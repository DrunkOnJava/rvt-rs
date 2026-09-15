//! Native spatial observations. Cached instance transforms, placement fields,
//! and explicit membership references stay separate; no bounding-box containment
//! or shared-coordinate transform is inferred.
use crate::native_document::Record;
use crate::native_equipment::Identity;
use crate::native_metadata::identifier;
use crate::native_parameters::ObjectGraph;
use anyhow::{Result, ensure};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub source_element_id: u64,
    pub source_object: usize,
    pub source_field: String,
    pub body_sha256: String,
    pub stream: String,
    pub group_record_offset: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct Transform {
    pub origin: [f64; 3],
    pub basis_x: [f64; 3],
    pub basis_y: [f64; 3],
    pub basis_z: [f64; 3],
    pub coordinate_space: &'static str,
    pub length_unit: &'static str,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct StoredPoint {
    pub point: [f64; 3],
    pub length_unit: &'static str,
    pub semantics: &'static str,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Reference {
    pub raw_target_id: Option<i64>,
    pub target_identity: Option<Identity>,
    pub status: String,
    pub observation: &'static str,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub element_id: u64,
    pub code: String,
    pub message: String,
}
/// The native `RoomElem` class carries rooms, areas, and MEP spaces.  The
/// class name alone is therefore insufficient for identity or containment.
#[derive(Debug, Clone, Serialize)]
pub struct NativeRoomSpaceDescriptor {
    pub kind: String,
    pub zone_scheme_id: i64,
    pub area_scheme_id: i64,
    pub level_id: i64,
    pub upper_level_id: i64,
    pub phase_id: i64,
    pub height: Option<f64>,
    pub lower_offset: Option<f64>,
    pub upper_offset: Option<f64>,
    pub cached_circuit_id: i64,
    pub locationless: Option<bool>,
    pub location_point: Option<[f64; 2]>,
    pub raw_metadata: BTreeMap<String, Value>,
    pub volume_bounding_elements: Vec<VolumeBoundingReference>,
}
#[derive(Debug, Clone, Serialize)]
pub struct VolumeBoundingReference {
    pub host_or_link_instance_id: i64,
    pub linked_element_id: i64,
    pub source: Source,
}
#[derive(Debug, Clone, Serialize)]
pub struct ElementContext {
    pub identity: Identity,
    pub class_name: String,
    pub transform: Option<Transform>,
    pub stored_location_point: Option<StoredPoint>,
    pub references: BTreeMap<String, Reference>,
    pub stored_flags: BTreeMap<String, bool>,
    pub subcomponent_ids: Option<Vec<i64>>,
    pub saved_subinstance_entries: Option<Value>,
    pub saved_link_location: Option<Value>,
    pub saved_link_location_source: Option<Source>,
    pub diagnostics: Vec<Diagnostic>,
    pub room_space: Option<NativeRoomSpaceDescriptor>,
}
#[derive(Debug, Serialize)]
pub struct Inventory {
    pub format: &'static str,
    pub complete_supported_context: bool,
    pub complete_spatial_parity: bool,
    pub elements: Vec<ElementContext>,
    pub diagnostics: Vec<Diagnostic>,
    pub ignored_record_classes: BTreeMap<String, usize>,
    pub supported_scope: Vec<&'static str>,
    pub unresolved_scope: Vec<&'static str>,
}
#[derive(Default)]
pub struct InventoryBuilder {
    identities: BTreeMap<u64, Identity>,
    elements: BTreeMap<u64, ElementContext>,
    ignored_record_classes: BTreeMap<String, usize>,
}
impl InventoryBuilder {
    pub fn ingest(&mut self, record: &Record) -> Result<()> {
        if record.channel != 102 {
            return Ok(());
        }
        let identity = Identity {
            element_id: record.identity.element_id,
            unique_id: record.identity.unique_id.clone(),
        };
        ensure!(
            self.identities
                .insert(identity.element_id, identity.clone())
                .is_none(),
            "duplicate current spatial-context record"
        );
        let class = record.class_name.as_deref().unwrap_or("<unregistered>");
        if !matches!(
            class,
            "FamilyInstance" | "RoomElem" | "RvtLinkInstance" | "RvtLinkSymbol"
        ) {
            *self.ignored_record_classes.entry(class.into()).or_default() += 1;
            return Ok(());
        }
        let mut context = ElementContext {
            identity,
            class_name: class.into(),
            transform: None,
            stored_location_point: None,
            references: BTreeMap::new(),
            stored_flags: BTreeMap::new(),
            subcomponent_ids: None,
            saved_subinstance_entries: None,
            saved_link_location: None,
            saved_link_location_source: None,
            diagnostics: Vec::new(),
            room_space: None,
        };
        match &record.graph {
            Some(graph) => {
                let projected = match class {
                    "RoomElem" => project_room(record, graph, &mut context),
                    "RvtLinkInstance" | "RvtLinkSymbol" => {
                        project_link(record, graph, &mut context)
                    }
                    _ => project(record, graph, &mut context),
                };
                if let Err(error) = projected {
                    context.diagnostics.push(problem(
                        context.identity.element_id,
                        "unsupported_spatial_graph",
                        &error.to_string(),
                    ));
                }
            }
            None => context.diagnostics.push(problem(
                context.identity.element_id,
                "unsupported_spatial_graph",
                record
                    .diagnostic
                    .as_deref()
                    .unwrap_or("native graph absent"),
            )),
        }
        self.elements.insert(context.identity.element_id, context);
        Ok(())
    }
    pub fn finish(mut self) -> Result<Inventory> {
        for context in self.elements.values_mut() {
            for reference in context.references.values_mut() {
                if let Some(raw) = reference.raw_target_id {
                    if let Ok(id) = u64::try_from(raw) {
                        if let Some(identity) = self.identities.get(&id) {
                            reference.target_identity = Some(identity.clone());
                            reference.status = "resolved_current_element".into();
                        } else {
                            reference.status = "unresolved_positive_reference".into();
                            context.diagnostics.push(problem(
                                context.identity.element_id,
                                "unresolved_reference",
                                &format!(
                                    "{} targets absent current element {id}",
                                    reference.source.source_field
                                ),
                            ));
                        }
                    } else {
                        reference.status = "negative_serialized_reference".into();
                    }
                }
            }
        }
        let mut consistency = Vec::new();
        for context in self.elements.values() {
            if let Some(parent_id) = context
                .references
                .get("supercomponent")
                .and_then(|reference| reference.raw_target_id)
                .and_then(|id| u64::try_from(id).ok())
                && let Some(parent) = self.elements.get(&parent_id)
                && let Some(children) = &parent.subcomponent_ids
                && !children.contains(&(context.identity.element_id as i64))
            {
                consistency.push(problem(
                    context.identity.element_id,
                    "supercomponent_backlink_missing",
                    "parent's saved subinstance table does not contain this child",
                ));
            }
            if let Some(children) = &context.subcomponent_ids {
                for &child_id in children {
                    if let Ok(child_id) = u64::try_from(child_id)
                        && let Some(child) = self.elements.get(&child_id)
                        && let Some(reference) = child.references.get("supercomponent")
                        && reference.raw_target_id != Some(context.identity.element_id as i64)
                    {
                        consistency.push(problem(
                            context.identity.element_id,
                            "subcomponent_backlink_mismatch",
                            "saved child names a different supercomponent",
                        ));
                    }
                }
            }
        }
        for diagnostic in consistency {
            self.elements
                .get_mut(&diagnostic.element_id)
                .unwrap()
                .diagnostics
                .push(diagnostic);
        }
        let elements: Vec<_> = self.elements.into_values().collect();
        let diagnostics: Vec<_> = elements
            .iter()
            .flat_map(|element| element.diagnostics.clone())
            .collect();
        Ok(Inventory {
            format: "rvt-native-spatial-context/v1",
            complete_supported_context: diagnostics.is_empty(),
            complete_spatial_parity: false,
            elements,
            diagnostics,
            ignored_record_classes: self.ignored_record_classes,
            supported_scope: vec![
                "family_instance_cached_document_transform",
                "saved_placement_point",
                "level_host_phase_view_references",
                "nested_family_membership",
                "model_group_membership",
                "saved_room_space_references",
                "room_level_phase_references",
                "linked_model_placement_and_saved_source_location",
            ],
            unresolved_scope: vec![
                "positive_space_reference_qualification",
                "room_boundary_geometry",
                "assembly_membership",
                "linked_document_contents_and_identity_reconciliation",
                "shared_project_coordinates",
                "non_family_geometry_location",
            ],
        })
    }
}
fn source(record: &Record, source_object: usize, field: &str) -> Source {
    Source {
        source_element_id: record.identity.element_id,
        source_object,
        source_field: field.into(),
        body_sha256: record.source.body_sha256.clone(),
        stream: record.source.stream.clone(),
        group_record_offset: record.source.group_record_offset,
    }
}
fn target(graph: &ObjectGraph, source_object: usize, pointer: &Value) -> Result<Option<usize>> {
    if pointer["pointer_token"].as_u64() == Some(0) {
        return Ok(None);
    }
    let offset = pointer["offset"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("spatial pointer offset absent"))? as usize;
    let edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.source_object_index == source_object && edge.pointer_offset == offset)
        .collect();
    ensure!(
        edges.len() == 1,
        "spatial pointer must resolve through exactly one owner edge"
    );
    ensure!(
        edges[0].target_object_index < graph.objects.len(),
        "spatial pointer outside graph"
    );
    Ok(Some(edges[0].target_object_index))
}
fn point(value: &Value) -> Result<[f64; 3]> {
    let values = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("spatial vector absent"))?;
    ensure!(
        values.len() == 3,
        "spatial vector must contain three coordinates"
    );
    let mut result = [0.; 3];
    for (i, value) in values.iter().enumerate() {
        result[i] = value
            .as_f64()
            .ok_or_else(|| anyhow::anyhow!("nonnumeric spatial coordinate"))?;
        ensure!(result[i].is_finite(), "nonfinite spatial coordinate");
    }
    Ok(result)
}
fn project(record: &Record, graph: &ObjectGraph, context: &mut ElementContext) -> Result<()> {
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty spatial graph"))?;
    ensure!(
        root.class_name == "FamilyInstance",
        "spatial owner class mismatch"
    );
    for (kind, field) in [
        ("type", "m_masterSymbolId"),
        ("level", "m_assocLevelId"),
        ("host", "m_hostId"),
        ("supercomponent", "m_superInstanceId"),
        ("owner_view", "m_ownerDBViewId"),
        ("unplaced_owner", "m_unplacedOwnerId"),
        ("created_phase", "m_createdPhaseId"),
        ("demolished_phase", "m_demolishedPhaseId"),
        ("design_option", "m_designOptionId"),
    ] {
        if let Some(value) = root.fields.get(field) {
            context.references.insert(
                kind.into(),
                Reference {
                    raw_target_id: Some(identifier(value)?),
                    target_identity: None,
                    status: "unresolved".into(),
                    observation: "saved_reference",
                    source: source(record, 0, field),
                },
            );
        }
    }
    if let Some(pointer) = root.fields.get("m_pDesignPropManager")
        && let Some(index) = target(graph, 0, pointer)?
    {
        let manager = &graph.objects[index];
        if manager.class_name == "FamInstDesignPropertyManager" {
            for (kind, field) in [("room", "m_idRoom"), ("space", "m_idSpace")] {
                if let Some(value) = manager.fields.get(field) {
                    context.references.insert(
                        kind.into(),
                        Reference {
                            raw_target_id: Some(identifier(value)?),
                            target_identity: None,
                            status: "unresolved".into(),
                            observation: "saved_design_property_manager_reference",
                            source: source(record, index, &format!("m_pDesignPropManager.{field}")),
                        },
                    );
                }
            }
        }
    }
    for field in ["m_flippedX", "m_flippedY", "m_workPlaneFlipped"] {
        if let Some(value) = root.fields.get(field) {
            context.stored_flags.insert(
                field.into(),
                value
                    .as_bool()
                    .ok_or_else(|| anyhow::anyhow!("invalid spatial flag"))?,
            );
        }
    }
    if let Some(value) = root.fields.get("m_instOrigin") {
        context.stored_location_point = Some(StoredPoint {
            point: point(value)?,
            length_unit: "feet",
            semantics: "saved_placement_field_may_differ_from_cached_transform",
            source: source(record, 0, "m_instOrigin"),
        });
    }
    if let Some(entries) = root.fields.get("m_subInstTable") {
        let rows = entries
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("subinstance table absent"))?;
        context.subcomponent_ids = Some(
            rows.iter()
                .map(|row| identifier(&row["first"]))
                .collect::<Result<_>>()?,
        );
        context.saved_subinstance_entries = Some(entries.clone());
    }
    let pointer = root
        .fields
        .get("m_pInstanceInfo")
        .ok_or_else(|| anyhow::anyhow!("instance info pointer absent"))?;
    let index =
        target(graph, 0, pointer)?.ok_or_else(|| anyhow::anyhow!("instance info is null"))?;
    let info = &graph.objects[index];
    ensure!(
        info.class_name == "InstanceInfo",
        "instance info target class mismatch"
    );
    let transform = &info.fields["m_Trf"];
    let rows = transform["m_3x3"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("instance transform matrix absent"))?;
    ensure!(rows.len() == 3, "instance transform must have three rows");
    let matrix = [point(&rows[0])?, point(&rows[1])?, point(&rows[2])?];
    context.transform = Some(Transform {
        origin: point(&transform["m_or"])?,
        basis_x: [matrix[0][0], matrix[1][0], matrix[2][0]],
        basis_y: [matrix[0][1], matrix[1][1], matrix[2][1]],
        basis_z: [matrix[0][2], matrix[1][2], matrix[2][2]],
        coordinate_space: "document_internal",
        length_unit: "feet",
        source: source(record, index, "m_pInstanceInfo.m_Trf"),
    });
    let mut membership = None;
    if let Some(pointer) = root.fields.get("m_cellList")
        && let Some(list_index) = target(graph, 0, pointer)?
    {
        let list = &graph.objects[list_index];
        ensure!(
            list.class_name == "CellList",
            "spatial cell-list class mismatch"
        );
        for pointer in list.fields["m_cells"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("spatial cell array absent"))?
        {
            if let Some(cell_index) = target(graph, list_index, pointer)? {
                let cell = &graph.objects[cell_index];
                if cell.class_name == "ElementGroupMembership" {
                    ensure!(
                        membership.is_none(),
                        "multiple owner group membership cells"
                    );
                    membership = Some(Reference {
                        raw_target_id: Some(identifier(&cell.fields["m_groupId"])?),
                        target_identity: None,
                        status: "unresolved".into(),
                        observation: "saved_group_membership_cell",
                        source: source(
                            record,
                            cell_index,
                            "m_cellList.m_cells.ElementGroupMembership.m_groupId",
                        ),
                    });
                }
            }
        }
    }
    context.references.insert(
        "group".into(),
        membership.unwrap_or_else(|| Reference {
            raw_target_id: None,
            target_identity: None,
            status: "absent_owner_group_membership_cell".into(),
            observation: "absence_in_complete_owner_cell_list",
            source: source(record, 0, "m_cellList"),
        }),
    );
    Ok(())
}
fn project_room(record: &Record, graph: &ObjectGraph, context: &mut ElementContext) -> Result<()> {
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty room graph"))?;
    ensure!(root.class_name == "RoomElem", "room owner class mismatch");
    context.room_space = Some(room_space_descriptor(record, root)?);
    for (kind, field) in [("level", "m_levelId"), ("phase", "m_phaseId")] {
        if let Some(value) = root.fields.get(field) {
            context.references.insert(
                kind.into(),
                Reference {
                    raw_target_id: Some(identifier(value)?),
                    target_identity: None,
                    status: "unresolved".into(),
                    observation: "saved_room_reference",
                    source: source(record, 0, field),
                },
            );
        }
    }
    Ok(())
}

pub(crate) fn classify_room_space(fields: &Value) -> Result<(String, i64, i64)> {
    let zone_scheme_id = identifier(
        fields
            .get("m_zoneSchemeId")
            .ok_or_else(|| anyhow::anyhow!("m_zoneSchemeId absent"))?,
    )?;
    let area_scheme_id = identifier(
        fields
            .get("m_areaSchemeId")
            .ok_or_else(|| anyhow::anyhow!("m_areaSchemeId absent"))?,
    )?;
    ensure!(
        zone_scheme_id == -1 || zone_scheme_id > 0,
        "invalid zone scheme sentinel"
    );
    ensure!(
        area_scheme_id == -1 || area_scheme_id > 0,
        "invalid area scheme sentinel"
    );
    let kind = match (zone_scheme_id > 0, area_scheme_id > 0) {
        (true, false) => "mep_space",
        (false, true) => "area",
        (false, false) => "room",
        (true, true) => "contradictory_room_space",
    };
    Ok((kind.into(), zone_scheme_id, area_scheme_id))
}
fn required_id(fields: &Value, name: &str) -> Result<i64> {
    identifier(
        fields
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("{name} absent"))?,
    )
}

fn optional_number(fields: &Value, name: &str) -> Result<Option<f64>> {
    let Some(value) = fields.get(name) else {
        return Ok(None);
    };
    let value = value
        .as_f64()
        .ok_or_else(|| anyhow::anyhow!("{name} is not numeric"))?;
    ensure!(value.is_finite(), "{name} is nonfinite");
    Ok(Some(value))
}

fn room_space_descriptor(
    record: &Record,
    root: &crate::native_parameters::GraphObject,
) -> Result<NativeRoomSpaceDescriptor> {
    let fields = &root.fields;
    let (kind, zone_scheme_id, area_scheme_id) = classify_room_space(fields)?;
    let location_point = fields
        .get("m_point")
        .and_then(|v| v.as_array())
        .and_then(|v| {
            if v.len() == 2 {
                Some([v[0].as_f64()?, v[1].as_f64()?])
            } else {
                None
            }
        });
    let mut raw_metadata = BTreeMap::new();
    for name in [
        "m_zoneSchemeId",
        "m_areaSchemeId",
        "m_levelId",
        "m_upperLevelId",
        "m_phaseId",
        "m_height",
        "m_lowerOffset",
        "m_upperOffset",
        "m_cachedCircuitId",
        "m_bIsLocationless",
        "m_location",
        "m_point",
        "m_pLocation",
        "m_SpaceRoomLocationInfo",
        "m_areaSpaceElemId",
        "m_roomBounding",
    ] {
        if let Some(value) = fields.get(name) {
            raw_metadata.insert(name.into(), value.clone());
        }
    }
    let mut volume_bounding_elements = Vec::new();
    if let Some(raw) = fields.get("m_volumeBoundingElems") {
        let values = raw
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("m_volumeBoundingElems is not an array"))?;
        for (index, value) in values.iter().enumerate() {
            let host = identifier(&value["m_linkInstOrHostId"])?;
            let linked = identifier(&value["m_linkRef"])?;
            // A native host reference has no linked document element and is
            // represented by linked=-1.  Zero and values below -1 are invalid
            // sentinels; positive linked IDs identify an actual link target.
            ensure!(
                host > 0 && (linked > 0 || linked == -1),
                "invalid volume bounding reference sentinel"
            );
            volume_bounding_elements.push(VolumeBoundingReference {
                host_or_link_instance_id: host,
                linked_element_id: linked,
                source: source(record, 0, &format!("m_volumeBoundingElems[{index}]")),
            });
        }
    }
    Ok(NativeRoomSpaceDescriptor {
        kind,
        zone_scheme_id,
        area_scheme_id,
        level_id: required_id(fields, "m_levelId")?,
        upper_level_id: required_id(fields, "m_upperLevelId")?,
        phase_id: required_id(fields, "m_phaseId")?,
        height: optional_number(fields, "m_height")?,
        lower_offset: optional_number(fields, "m_lowerOffset")?,
        upper_offset: optional_number(fields, "m_upperOffset")?,
        cached_circuit_id: required_id(
            fields
                .get("m_cachedCircuitId")
                .ok_or_else(|| anyhow::anyhow!("m_cachedCircuitId absent"))?,
            "m_id",
        )?,
        locationless: fields.get("m_bIsLocationless").and_then(Value::as_bool),
        location_point,
        raw_metadata,
        volume_bounding_elements,
    })
}
fn problem(element_id: u64, code: &str, message: &str) -> Diagnostic {
    Diagnostic {
        element_id,
        code: code.into(),
        message: message.into(),
    }
}

fn project_link(record: &Record, graph: &ObjectGraph, context: &mut ElementContext) -> Result<()> {
    let root = graph
        .objects
        .first()
        .ok_or_else(|| anyhow::anyhow!("empty link graph"))?;
    if root.class_name == "RvtLinkSymbol" {
        let location = root
            .fields
            .get("m_otherDocModelLocation")
            .ok_or_else(|| anyhow::anyhow!("link source location absent"))?;
        ensure!(
            location["m_lastKnownModelPath"]["m_path"].is_string(),
            "saved link path absent"
        );
        context.saved_link_location = Some(location.clone());
        context.saved_link_location_source = Some(source(record, 0, "m_otherDocModelLocation"));
        return Ok(());
    }
    ensure!(
        root.class_name == "RvtLinkInstance",
        "link instance class mismatch"
    );
    let index = target(graph, 0, &root.fields["m_pInstanceInfo"])?
        .ok_or_else(|| anyhow::anyhow!("link instance info is null"))?;
    let info = &graph.objects[index];
    ensure!(
        info.class_name == "InstanceInfo",
        "link transform target class mismatch"
    );
    context.references.insert(
        "type".into(),
        Reference {
            raw_target_id: Some(identifier(&info.fields["m_symbolId"])?),
            target_identity: None,
            status: "unresolved".into(),
            observation: "saved_link_symbol_reference",
            source: source(record, index, "m_symbolId"),
        },
    );
    let transform = &info.fields["m_Trf"];
    let rows = transform["m_3x3"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("link transform matrix absent"))?;
    ensure!(rows.len() == 3, "link transform requires three rows");
    let matrix = [point(&rows[0])?, point(&rows[1])?, point(&rows[2])?];
    context.transform = Some(Transform {
        origin: point(&transform["m_or"])?,
        basis_x: [matrix[0][0], matrix[1][0], matrix[2][0]],
        basis_y: [matrix[0][1], matrix[1][1], matrix[2][1]],
        basis_z: [matrix[0][2], matrix[1][2], matrix[2][2]],
        coordinate_space: "linked_document_to_host_document",
        length_unit: "feet",
        source: source(record, index, "m_Trf"),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn record(id: u64, fields: Value) -> Record {
        let graph: ObjectGraph = serde_json::from_value(json!({"consumed_bytes":50,"objects":[
            {"class_tag":20,"class_name":"FamilyInstance","token":0,"start":2,"fields_end":10,"fields":fields},
            {"class_tag":21,"class_name":"InstanceInfo","token":4294967295u32,"start":10,"fields_end":50,"fields":{"m_Trf":{"m_or":[22.,10.,0.],"m_3x3":[[0.,-1.,0.],[1.,0.,0.],[0.,0.,1.]]}}}],
            "edges":[{"source_object_index":0,"pointer_offset":2,"pointer_token":4294967295u32,"target_object_index":1,"target_class_tag":21}]})).unwrap();
        Record {
            identity: crate::native_index::Identity {
                element_id: id,
                original_id_suffix: id,
                creation_episode: 0,
                stored_revision: 0,
                other_revision: 0,
                row_offset: 0,
                raw_fields: vec![],
                owning_element_id: -1,
                partition_id: 0,
                unique_id: format!("synthetic-{id}"),
            },
            derived_default_ifc_guid: None,
            derived_identifier_diagnostic: None,
            effective_ifc_parameter: None,
            effective_ifc_parameter_diagnostic: None,
            channel: 102,
            class_name: Some("FamilyInstance".into()),
            status: "complete_bounded_graph".into(),
            diagnostic: None,
            source: crate::native_document::RecordSource {
                stream: "Partitions/0".into(),
                group: crate::native_segments::GroupSource {
                    content_key: None,
                    channel: 102,
                    first_marker_offset: 0,
                    segment_count: 1,
                    declared_objects: 1,
                    declared_body_bytes: 50,
                },
                group_record_offset: 0,
                body_bytes: 50,
                body_sha256: "synthetic".into(),
            },
            graph: Some(graph),
            saved_metadata: None,
            metadata_diagnostic: None,
        }
    }
    #[test]
    fn matrix_columns_and_stale_placement_are_distinct() {
        let input = record(
            1,
            json!({"m_pInstanceInfo":{"offset":2,"pointer_token":4294967295u32},"m_instOrigin":[38.,10.,0.],"m_hostId":-4,"m_cellList":{"pointer_token":0}}),
        );
        let mut builder = InventoryBuilder::default();
        builder.ingest(&input).unwrap();
        let result = builder.finish().unwrap();
        let element = &result.elements[0];
        let transform = element.transform.as_ref().unwrap();
        assert_eq!(transform.basis_x, [0., 1., 0.]);
        assert_eq!(transform.basis_y, [-1., 0., 0.]);
        assert_eq!(transform.origin, [22., 10., 0.]);
        assert_eq!(
            element.stored_location_point.as_ref().unwrap().point,
            [38., 10., 0.]
        );
        assert_eq!(element.references["host"].raw_target_id, Some(-4));
        assert_eq!(
            element.references["host"].status,
            "negative_serialized_reference"
        );
        assert_eq!(element.references["group"].raw_target_id, None);
    }
    #[test]
    fn wrong_transform_owner_and_missing_positive_reference_are_explicit() {
        let mut input = record(
            1,
            json!({"m_pInstanceInfo":{"offset":2,"pointer_token":4294967295u32},"m_hostId":999}),
        );
        input.graph.as_mut().unwrap().edges[0].source_object_index = 1;
        let mut builder = InventoryBuilder::default();
        builder.ingest(&input).unwrap();
        let result = builder.finish().unwrap();
        assert!(!result.complete_supported_context);
        assert!(result.elements[0].transform.is_none());
        assert_eq!(
            result.elements[0].references["host"].status,
            "unresolved_positive_reference"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|d| d.code == "unsupported_spatial_graph")
        );
    }
    #[test]
    fn room_elem_scheme_ids_classify_space_without_promoting_boundary() {
        let root = crate::native_parameters::GraphObject {
            class_tag: 1,
            class_name: "RoomElem".into(),
            token: 0,
            start: 0,
            fields_end: 0,
            fields: json!({
                "m_zoneSchemeId": {"m_id": {"m_id": 700}}, "m_areaSchemeId": {"m_id": -1},
                "m_levelId": {"m_id": 701}, "m_upperLevelId": {"m_id": 701},
                "m_phaseId": {"m_id": 3}, "m_height": 8.0,
                "m_lowerOffset": 0.0, "m_upperOffset": 8.0,
                "m_cachedCircuitId": {"m_id": -1}, "m_bIsLocationless": false,
                "m_point": [12.5, -4.25],
                "m_SpaceRoomLocationInfo": {"pointer_token": 0}
            }),
        };
        let input = record(703, root.fields.clone());
        let descriptor = room_space_descriptor(&input, &root).unwrap();
        assert_eq!(descriptor.kind, "mep_space");
        assert_eq!(descriptor.zone_scheme_id, 700);
        assert_eq!(descriptor.area_scheme_id, -1);
        assert_eq!(descriptor.location_point, Some([12.5, -4.25]));
        assert_eq!(descriptor.cached_circuit_id, -1);
    }

    #[test]
    fn volume_bounding_refs_allow_native_host_and_reject_bad_sentinels() {
        let fields = json!({
            "m_zoneSchemeId": {"m_id": {"m_id": 700}}, "m_areaSchemeId": {"m_id": -1},
            "m_levelId": {"m_id": 701}, "m_upperLevelId": {"m_id": 701},
            "m_phaseId": {"m_id": 3}, "m_cachedCircuitId": {"m_id": -1},
            "m_volumeBoundingElems": [{"m_linkInstOrHostId": {"m_id": 702}, "m_linkRef": {"m_id": -1}}]
        });
        let input = record(703, fields.clone());
        let root = input.graph.as_ref().unwrap().objects.first().unwrap();
        let descriptor = room_space_descriptor(&input, root).unwrap();
        assert_eq!(descriptor.volume_bounding_elements.len(), 1);
        assert_eq!(descriptor.volume_bounding_elements[0].linked_element_id, -1);

        for linked in [0, -2] {
            let mut bad = fields.clone();
            bad["m_volumeBoundingElems"][0]["m_linkRef"]["m_id"] = json!(linked);
            let input = record(703, bad);
            let root = input.graph.as_ref().unwrap().objects.first().unwrap();
            assert!(room_space_descriptor(&input, root).is_err());
        }
    }
}

// A link location is a serialized reference claim, never an instruction to open
// an external file. Its model-identity bytes are retained without uniqueness claims.
