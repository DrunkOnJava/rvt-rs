//! Mode-aware IFC content policy (Lane Seven).
//!
//! [`ExportQualityMode`] is both a *validation gate* (see
//! [`super::ExportQualityMode::validate`]) and a *content policy*
//! for what the document exporter emits:
//!
//! | Mode | Content |
//! |---|---|
//! | `scaffold` | Framework + production elements; geometry when recovered. No diagnostic proxies. |
//! | `typed-no-geometry` | Mapped / typed elements only; geometry fields stripped. |
//! | `geometry` | Mapped / typed elements; attach Lane Six curves / loops / hosts / elevations when present. |
//! | `strict` | Same emission as `geometry`; validation then fails closed on incomplete output. |
//!
//! `HostObjAttr` and other low-confidence parent-class hits are never
//! emitted on the production path — use [`super::DiagnosticRvtDocExporter`]
//! for research proxies.

use super::category_map;
use super::entities::{self, Extrusion, Property, PropertySet, PropertyValue};
use super::from_decoded::{wall_segment_angle_radians, wall_segment_length_feet};
use super::{ExportQualityMode, MaterialInfo, Storey, UNRESOLVED_ARCWALL_THICKNESS_FEET};
use crate::elements::floor::Floor;
use crate::elements::level::Level;
use crate::elements::openings::{Door, Window};
use crate::elements::styling::Material;
use crate::elements::wall::Wall;
use crate::geometry::{
    recover_door_host, recover_floor_boundary, recover_level_elevation,
    recover_wall_location_curve_from_wall, recover_window_host,
};
use crate::walker::{DecodedElement, InstanceField};
use std::collections::{BTreeMap, HashMap};

/// Result of folding production walker hits into an IFC draft.
#[derive(Debug, Default)]
pub struct TypedProductionAppend {
    /// Revit element id → index in `entities` (for host linking).
    pub id_to_entity: HashMap<u32, usize>,
    /// Named materials recovered from partition / typed Material rows.
    pub materials: Vec<MaterialInfo>,
    /// Production class histogram (Level / Floor / Material / …).
    pub production_class_counts: BTreeMap<String, usize>,
    /// Floors/Rooms assigned to a storey via Level ElementId bind.
    /// Stays 0 on current corpora (partition Levels lack ElementIds).
    pub level_elementid_binds: usize,
    /// Layered elements' layers, by ElementId (RE-53).
    pub element_layers: BTreeMap<u32, super::ElementLayers>,
    /// The layers of walls their data does not place, by ElementId, for
    /// their materials only (RE-88).
    pub unplaced_wall_layers: BTreeMap<u32, Vec<super::LayerBand>>,
    /// The materials each element's type draws its geometry in, by
    /// ElementId (RE-82).
    pub element_type_materials: BTreeMap<u32, Vec<String>>,
    /// Each element's type, by ElementId (RE-110).
    pub element_type_ids: BTreeMap<u32, u32>,
    /// Each family instance's original symbol, by ElementId (RE-167).
    pub element_original_symbols: BTreeMap<u32, u32>,
    /// Each door's and window's plan facing, by ElementId (B72).
    pub element_facings: BTreeMap<u32, [f64; 2]>,
}

/// What a quality mode allows the document exporter to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportContentPolicy {
    /// Attach recovered placement / extrusion / host links when present.
    pub include_geometry: bool,
    /// Drop walker hits that have no [`category_map`] entry (no PROXY fallback).
    pub require_mapped_ifc_type: bool,
}

impl ExportContentPolicy {
    pub fn for_quality_mode(mode: ExportQualityMode) -> Self {
        match mode {
            ExportQualityMode::Scaffold => Self {
                include_geometry: true,
                require_mapped_ifc_type: false,
            },
            ExportQualityMode::TypedNoGeometry => Self {
                include_geometry: false,
                require_mapped_ifc_type: true,
            },
            ExportQualityMode::Geometry | ExportQualityMode::Strict => Self {
                include_geometry: true,
                require_mapped_ifc_type: true,
            },
        }
    }
}

/// Property-set name carried by a body recovered from a partition
/// element record's bounding box (#204 / #211).
pub const ELEMENT_RECORD_PROPERTY_SET: &str = "RvtElementRecordGeometry";

/// Property naming an element-record element's family (RE-38).
pub const FAMILY_NAME_PROPERTY: &str = "FamilyName";

/// Property naming an element-record element's type (RE-38).
pub const TYPE_NAME_PROPERTY: &str = "TypeName";

/// The `(family, type)` names RE-38 attached to `decoded`, when both are.
fn family_and_type(decoded: &DecodedElement) -> Option<(String, String)> {
    let field = |key: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(v) if name == key => Some(v.clone()),
            _ => None,
        })
    };
    Some((
        field(crate::partition_schema_mvp::FAMILY_NAME_FIELD)?,
        field(crate::partition_schema_mvp::TYPE_NAME_FIELD)?,
    ))
}

/// Decoded classes whose record bounding-box `z` extent is the
/// element's own thickness rather than an envelope height (#212).
pub const SLAB_THICKNESS_CLASSES: &[&str] = &["Floor", "BuildingPad"];

/// Value of the `ThicknessSource` property on a record-backed plate.
pub const RECORD_BBOX_THICKNESS_SOURCE: &str = "partition_element_record_bbox_z_extent";

/// Property carrying a recovered room number (#90, RE-29).
///
/// The STEP writer reads this back as `IfcSpace.Name`, as Revit's own
/// exporter writes it.
pub const ROOM_NUMBER_PROPERTY: &str = "RoomNumber";

/// Property carrying a room's Revit ElementId. `IfcSpace` declares no
/// `Tag`, the attribute every other element carries it in, and its `Name`
/// is the room number, so the id is written here.
pub const ROOM_ELEMENT_ID_PROPERTY: &str = "ElementId";

/// Property carrying a recovered room name (#90, RE-29).
///
/// The STEP writer reads this back to fill `IfcSpace.LongName`, the
/// slot Revit's own exporter puts the room name in.
pub const ROOM_NAME_PROPERTY: &str = "RoomName";

/// Classes that must never appear as production building elements.
pub fn is_misleading_proxy_class(class_name: &str) -> bool {
    matches!(
        class_name,
        "HostObjAttr" | "HostObject" | "Element" | "Symbol"
    )
}

/// Whether an IFC4 entity name is an `IfcDistributionElement` among those the
/// exporter writes: the only kind of element an `IfcDistributionPort` may be
/// tied to (RE-138). Equipment written as a building element proxy is not.
fn is_distribution_element(ifc_type: &str) -> bool {
    ifc_type.starts_with("IFCFLOW")
        || matches!(
            ifc_type,
            "IFCDUCTSEGMENT"
                | "IFCDUCTFITTING"
                | "IFCPIPESEGMENT"
                | "IFCPIPEFITTING"
                | "IFCAIRTERMINAL"
                | "IFCSANITARYTERMINAL"
                | "IFCLIGHTFIXTURE"
                | "IFCELECTRICAPPLIANCE"
                | "IFCALARM"
        )
}

/// Strip placement / body / host claims so an export cannot over-claim geometry.
pub fn strip_building_element_geometry(entities: &mut [entities::IfcEntity]) {
    for entity in entities.iter_mut() {
        if let entities::IfcEntity::BuildingElement {
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index,
            solid_shape,
            representation_map_index,
            ..
        } = entity
        {
            *location_feet = None;
            *rotation_radians = None;
            *extrusion = None;
            *host_element_index = None;
            *solid_shape = None;
            *representation_map_index = None;
        }
    }
}

/// Accumulate production walker hits into IFC entities + storeys + materials.
pub fn append_typed_production_elements(
    decoded_iter: impl Iterator<Item = DecodedElement>,
    entities: &mut Vec<entities::IfcEntity>,
    building_storeys: &mut Vec<Storey>,
    policy: ExportContentPolicy,
) -> TypedProductionAppend {
    let mut out = TypedProductionAppend::default();
    let mut pending_hosts: Vec<(usize, u32)> = Vec::new();
    // RE-84: doors and windows whose type draws no geometry.
    let mut without_geometry: std::collections::BTreeSet<usize> = Default::default();
    let mut pending_parts: Vec<(usize, u32)> = Vec::new();
    // RE-138: the joins of ducts and pipes, by entity index and ElementId.
    let mut pending_joins: Vec<(usize, u32, crate::partition_schema_mvp::ConnectorJoins)> =
        Vec::new();
    // B54: each duct and pipe, by entity index and ElementId.
    let mut curve_ends: Vec<(usize, u32)> = Vec::new();
    // RE-162: each MEP system's name and members, by its ElementId.
    let mut pending_systems: std::collections::BTreeMap<u32, (Option<String>, Vec<usize>)> =
        Default::default();
    // Bodies of aggregate wholes, held back until their parts are known: a
    // whole that no part names keeps its own body.
    let mut held_bodies: std::collections::BTreeMap<usize, Extrusion> =
        std::collections::BTreeMap::new();
    let mut level_bind = crate::level_bind::LevelStoreyBind::new();

    for decoded in decoded_iter {
        *out.production_class_counts
            .entry(decoded.class.clone())
            .or_insert(0) += 1;

        if is_misleading_proxy_class(&decoded.class) {
            continue;
        }

        if decoded.class == "Level" {
            if let Some(storey) = storey_from_level(&decoded) {
                let storey_index = building_storeys.len();
                // Fail-closed: only record when the Level carries an ElementId.
                level_bind.record_level(decoded.id, storey_index);
                building_storeys.push(storey);
            }
            continue;
        }

        // RE-53: a wall's layers, for the glTF export to draw in colour.
        if policy.include_geometry {
            if let (Some(id), Some(mut layers)) = (
                decoded.id,
                crate::partition_schema_mvp::element_layers_from_fields(&decoded.fields),
            ) {
                // The element's own system family name, localized
                // (RE-123), where one was given; else the class's.
                layers.system_family = decoded
                    .fields
                    .iter()
                    .find_map(|(name, value)| match value {
                        InstanceField::String(family)
                            if name == crate::partition_schema_mvp::FAMILY_NAME_FIELD =>
                        {
                            Some(family.clone())
                        }
                        _ => None,
                    })
                    .or_else(|| {
                        crate::partition_schema_mvp::system_family(&decoded.class, true)
                            .map(String::from)
                    });
                out.element_layers.insert(id, layers);
            } else if let (Some(id), Some(bands)) = (
                decoded.id,
                crate::partition_schema_mvp::unplaced_wall_layers_from_fields(&decoded.fields),
            ) {
                out.unplaced_wall_layers.insert(id, bands);
            }
        }

        // RE-82: the materials the element's type draws its geometry in.
        if let Some(id) = decoded.id {
            let names: Vec<String> = decoded
                .fields
                .iter()
                .filter_map(|(name, value)| match value {
                    InstanceField::String(text)
                        if name == crate::partition_schema_mvp::TYPE_MATERIAL_FIELD =>
                    {
                        Some(text.clone())
                    }
                    _ => None,
                })
                .collect();
            if !names.is_empty() {
                out.element_type_materials.insert(id, names);
            }
            // RE-110: the element's type, which the writer types it by.
            if let Some(type_id) = decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == crate::partition_schema_mvp::TYPE_ID_FIELD =>
                {
                    Some(*id)
                }
                _ => None,
            }) {
                out.element_type_ids.insert(id, type_id);
            }
            // RE-167: the original symbol, whose GlobalId the type takes.
            if let Some(original) = decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == crate::partition_schema_mvp::ORIGINAL_SYMBOL_FIELD =>
                {
                    Some(*id)
                }
                _ => None,
            }) {
                out.element_original_symbols.insert(id, original);
            }
            // B72: a door's or window's facing, for its type's flip.
            if let Some(facing) =
                crate::partition_schema_mvp::opening_facing_from_fields(&decoded.fields)
            {
                out.element_facings.insert(id, facing);
            }
        }

        // Materials are IfcMaterial rows, never building-element proxies.
        if decoded.class == "Material" {
            if let Some(info) = material_info_from_decoded(&decoded) {
                out.materials.push(info);
            }
            continue;
        }

        // ArcWall IFC emission stays on the partition path in
        // `ifc::mod` (storey index from recovered elevations). The
        // walker still yields ArcWall `DecodedElement`s for API
        // consumers; skipping here avoids duplicate IFCWALL rows.
        if decoded.class == "ArcWall" {
            continue;
        }

        // Opening-index rows are not typed Door/Window. Keep them in
        // `iter_elements` / production_class_counts for File Status, but
        // do not flood IFC with thousands of hostless IFCOPENINGELEMENT
        // stubs — emit only once host Wall ElementIds join (still open).
        if decoded.class == "ArcWallRectOpening" {
            continue;
        }

        let mapping = category_map::lookup(&decoded.class);
        if mapping.is_none() && policy.require_mapped_ifc_type {
            continue;
        }
        // Scaffold must not invent PROXY rows for unmapped partition
        // MVP classes (e.g. stray research tags).
        let Some(mapping) = mapping else {
            continue;
        };
        // A per-element Revit "IFC Export As" override redirects the
        // entity type (#212, RE-22). Only values `category_map`
        // recognises are honoured; anything else keeps the class
        // mapping, so an unknown override can never invent a type.
        let effective = ifc_export_override(&decoded).unwrap_or(mapping);
        let ifc_type = effective.ifc_type.to_string();
        // RE-45: the element's own or its type's IFC predefined type, when
        // it is an enumerator of the entity it exports as.
        let predefined_type = decoded
            .fields
            .iter()
            .find_map(|(name, value)| match value {
                InstanceField::String(text)
                    if name == crate::partition_schema_mvp::IFC_PREDEFINED_TYPE_FIELD =>
                {
                    category_map::predefined_type_for(effective.ifc_type, text)
                }
                _ => None,
            })
            .or(effective.predefined_type);

        // RE-65: a stair and its parts carry the name Revit gives them.
        let own_name = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(text)
                if name == crate::partition_schema_mvp::ELEMENT_NAME_FIELD =>
            {
                Some(text.clone())
            }
            _ => None,
        });
        // Revit names electrical equipment `Family:<Panel Name>:ElementId`
        // (RE1 Electrical: `262416-PANEL:RE-1:428352`), from an instance
        // parameter rvt-rs does not read, so it gets no `Family:Type` name.
        let named_by_type = decoded.class != "ElectricalEquipment";
        // RE-154: the type name. A door or window whose type draws no
        // geometry is written as its opening (RE-84), which takes none.
        let opening_only = matches!(decoded.class.as_str(), "Door" | "Window")
            && decoded.fields.iter().any(|(name, value)| {
                name == crate::partition_schema_mvp::TYPE_WITHOUT_GEOMETRY_FIELD
                    && matches!(value, InstanceField::Bool(true))
            });
        // A duct's type is read (RE-134) but not its system family, which
        // follows its shape; its Reference is still its type's name.
        let duct_type = (decoded.class == "Duct")
            .then(|| {
                decoded.fields.iter().find_map(|(name, value)| match value {
                    InstanceField::String(text)
                        if name == crate::partition_schema_mvp::TYPE_NAME_FIELD =>
                    {
                        Some(text.clone())
                    }
                    _ => None,
                })
            })
            .flatten();
        // The Reference is the type's name however Revit names the element:
        // electrical equipment named by its panel name has one too (B29).
        let reference_type = family_and_type(&decoded)
            .filter(|_| own_name.is_none() && !opening_only)
            .map(|(_, type_name)| type_name)
            .or(duct_type);
        let name = match (
            own_name,
            decoded.id,
            family_and_type(&decoded).filter(|_| named_by_type),
        ) {
            (Some(name), _, _) => name,
            // RE-38: `Family:Type:ElementId`, the name Revit's own export
            // gives the element.
            (None, Some(id), Some((family, type_name))) => format!("{family}:{type_name}:{id}"),
            (None, Some(id), None) => format!("{}-{}", decoded.class, id),
            (None, None, _) => format!("{}-unnamed", decoded.class),
        };
        let type_guid = decoded.id.map(|id| id.to_string());

        let mut location_feet = None;
        let mut rotation_radians = None;
        let mut extrusion = None;
        let mut property_set = None;
        let mut pending_host_id = None;
        let mut piece_bodies: Vec<Extrusion> = Vec::new();
        let mut void_bodies: Vec<VoidBody> = Vec::new();
        let mut held_body = None;
        let mut solid_shape = None;

        if policy.include_geometry {
            // Partition element records carry their own model bbox for
            // every category, so one envelope path serves all of them
            // (#204 columns, #211 walls / doors / windows). Checked
            // before the class arms so a schema-field decoder is never
            // asked to read element-record fields.
            let element_record_geometry = if is_partition_element_record(&decoded) {
                element_record_geometry_from_decoded(&decoded)
            } else {
                None
            };
            if let Some(RecordGeometry {
                location,
                rotation,
                body,
                solid,
                properties,
                pieces,
                void_openings,
            }) = element_record_geometry
            {
                location_feet = Some(location);
                rotation_radians = rotation;
                solid_shape = solid;
                piece_bodies = pieces;
                void_bodies = void_openings;
                // #323 / RE-46: a stair or curtain wall is carried by its
                // parts, as in Revit's export; its own record box would
                // double their volume. It is held until the parts are known.
                if crate::partition_schema_mvp::AGGREGATE_WHOLE_CLASSES
                    .contains(&decoded.class.as_str())
                {
                    held_body = Some(body);
                } else {
                    extrusion = Some(body);
                }
                property_set = Some(properties);
            } else {
                match decoded.class.as_str() {
                    "Wall" | "ArcWall" => {
                        if let Some(geom) = wall_geometry_from_decoded(&decoded) {
                            location_feet = Some(geom.location_feet);
                            rotation_radians = Some(geom.rotation_radians);
                            extrusion = Some(geom.extrusion);
                            property_set = geom.property_set;
                        }
                    }
                    "Floor" => {
                        if let Some(geom) = floor_boundary_annotation_from_decoded(&decoded) {
                            // Boundary without resolved thickness: record
                            // provenance only — do not claim an extruded body.
                            property_set = Some(geom);
                        }
                    }
                    _ => {}
                }
            }
            // Host-wall binding is independent of which geometry
            // carrier produced the body. Before #222 this lived in the
            // `else` arm above, so a door recovered from a partition
            // element record — which always has a record bbox body —
            // never reached it and could not be voided into its wall.
            match decoded.class.as_str() {
                "Door" => {
                    let door = Door::from_decoded(&decoded);
                    if let Some(host) = recover_door_host(&door).ok() {
                        pending_host_id = Some(host.host_element_id);
                    }
                }
                "Window" => {
                    let window = Window::from_decoded(&decoded);
                    if let Some(host) = recover_window_host(&window).ok() {
                        pending_host_id = Some(host.host_element_id);
                    }
                }
                _ => {}
            }
        }

        let entity_index = entities.len();
        if let Some(id) = decoded.id {
            out.id_to_entity.insert(id, entity_index);
        }
        if let Some(host_id) = pending_host_id {
            pending_hosts.push((entity_index, host_id));
            if decoded.fields.iter().any(|(name, value)| {
                name == crate::partition_schema_mvp::TYPE_WITHOUT_GEOMETRY_FIELD
                    && matches!(value, InstanceField::Bool(true))
            }) {
                without_geometry.insert(entity_index);
            }
        }
        if let Some(body) = held_body {
            held_bodies.insert(entity_index, body);
        }
        if let Some(whole) = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. }
                if name == crate::partition_schema_mvp::AGGREGATE_WHOLE_FIELD =>
            {
                Some(*id)
            }
            _ => None,
        }) {
            pending_parts.push((entity_index, whole));
        }
        if let Some(id) = decoded.id {
            let joins = crate::partition_schema_mvp::connector_joins_from_fields(&decoded.fields);
            if !joins.is_empty() {
                pending_joins.push((entity_index, id, joins));
            }
            if matches!(decoded.class.as_str(), "Duct" | "Pipe") {
                curve_ends.push((entity_index, id));
            }
        }
        // RE-162: the MEP systems the element is a member of.
        for (system, name) in crate::partition_schema_mvp::mep_systems_from_fields(&decoded.fields)
        {
            let entry = pending_systems
                .entry(system)
                .or_insert_with(|| (name, Vec::new()));
            entry.1.push(entity_index);
        }

        // Floor/Room → storey via Level ElementId only when both sides
        // carry ids that match. Partition MVP Levels are id-less today,
        // so this stays None (honest Unassigned) on available corpora.
        let storey_index = level_bind.storey_index_for(&decoded);
        if storey_index.is_some() {
            out.level_elementid_binds += 1;
        }

        let mut reference_sets = reference_type
            .map(|type_name| reference_property_sets(&ifc_type, &type_name))
            .unwrap_or_default();
        add_is_external(&mut reference_sets, &ifc_type, &decoded, opening_only);
        add_covering_finish(&mut reference_sets, &ifc_type, &decoded);
        let serial_sets = serial_number_property_sets(&decoded, &ifc_type);
        // RE-151: the name of the openings of a floor's tagged voids.
        let opening_name = (!void_bodies.is_empty()).then(|| {
            let type_id = decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == crate::partition_schema_mvp::TYPE_ID_FIELD =>
                {
                    Some(*id)
                }
                _ => None,
            });
            match (family_and_type(&decoded), type_id) {
                (Some((family, type_name)), Some(type_id)) => {
                    format!("{family}:{type_name}:{type_id}")
                }
                _ => format!("Opening for {name}"),
            }
        });
        entities.push(entities::IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            predefined_type: predefined_type.map(str::to_string),
            storey_index,
            material_index: None,
            property_set,
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape,
            representation_map_index: None,
        });
        // #331: each further piece of a sketch of separate loops is an
        // element of its own with the element's `Tag`, named `…:2`, `…:3`,
        // as in Revit's export.
        if !piece_bodies.is_empty() {
            if let Some(entities::IfcEntity::BuildingElement { .. }) = entities.last() {
                let first = entities.last().cloned().expect("just pushed");
                for (index, body) in piece_bodies.into_iter().enumerate() {
                    let mut piece = first.clone();
                    if let entities::IfcEntity::BuildingElement {
                        name, extrusion, ..
                    } = &mut piece
                    {
                        *name = format!("{name}:{}", index + 2);
                        *extrusion = Some(body);
                    }
                    entities.push(piece);
                }
            }
        }
        // RE-151: each tagged void of a floor's sketch is an opening voiding
        // the floor, or the piece it is in, named `Family:Type:<type id>` as
        // in Revit's export.
        if let Some(opening_name) = opening_name {
            for (piece, tag, outline_feet, depth_feet) in void_bodies {
                entities.push(entities::IfcEntity::VoidOpening {
                    host: entity_index + piece,
                    tag: tag.to_string(),
                    name: opening_name.clone(),
                    outline_feet,
                    depth_feet,
                });
            }
        }
        // RE-47: a stair's or flight's riser and tread dimensions, in the
        // standard set Revit's export writes them in.
        if let Some(set) = stair_property_set(&decoded) {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set,
            });
        }
        // RE-153: a room's Pset_SpaceCommon.
        if let Some(set) = space_common_property_set(&decoded) {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set,
            });
        }
        // #35: the two sets holding a room's name, as Revit's export gives
        // every space.
        if decoded.class == "Room" {
            let name = decoded
                .fields
                .iter()
                .find_map(|(field, value)| match value {
                    InstanceField::String(text) if field == "m_name" && !text.is_empty() => {
                        Some(text.as_str())
                    }
                    _ => None,
                });
            for set in name.map(PropertySet::name_sets).unwrap_or_default() {
                entities.push(entities::IfcEntity::ElementPropertySet {
                    element: entity_index,
                    set,
                });
            }
        }
        // A pipe's or duct's length along its axis, which Revit's export gives
        // in both flow-segment sets: a pipe's between its ends (RE-131), a
        // duct's the dimension of its box its section leaves (RE-134).
        let segment_length = match decoded.class.as_str() {
            "Pipe" => {
                crate::partition_schema_mvp::pipe_body_from_fields(&decoded.fields).map(|pipe| {
                    (0..3)
                        .map(|axis| (pipe.end[axis] - pipe.start[axis]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
            }
            "Duct" => decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. }
                    if name == crate::partition_schema_mvp::DUCT_LENGTH_FIELD =>
                {
                    Some(*value)
                }
                _ => None,
            }),
            _ => None,
        };
        // RE-165 (B44): a pipe fitting's nominal size, the one diameter its
        // connectors give, as Revit's export writes it: a list of one length.
        let fitting_diameter = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. }
                if name == crate::partition_schema_mvp::FITTING_NOMINAL_DIAMETER_FIELD =>
            {
                Some(*value)
            }
            _ => None,
        });
        if let Some(diameter) = fitting_diameter {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set: PropertySet {
                    name: "Pset_PipeFittingTypeCommon".into(),
                    properties: vec![Property {
                        name: "NominalDiameter".into(),
                        value: PropertyValue::List(vec![PropertyValue::PositiveLengthFeet(
                            diameter,
                        )]),
                    }],
                },
            });
        }
        // A duct's length is also in its Pset_DuctSegmentTypeCommon, as on
        // every RE1 Mechanical duct.
        let duct_set = (decoded.class == "Duct").then_some("Pset_DuctSegmentTypeCommon");
        // RE-157: a pipe's invert, here in model height; the storey's
        // elevation comes off once storeys are bound
        // (`pipe_inverts_above_storeys`).
        let invert = match decoded.class.as_str() {
            "Pipe" => pipe_invert_height(&decoded.fields),
            _ => None,
        };
        if let Some(length) = segment_length {
            let sets = ["Pset_FlowSegmentPipeSegment", "Pset_FlowSegmentDuctSegment"];
            for name in sets.into_iter().chain(duct_set) {
                let mut properties = vec![Property {
                    name: "Length".into(),
                    value: PropertyValue::PositiveLengthFeet(length),
                }];
                if let (Some(height), "Pset_FlowSegmentPipeSegment") = (invert, name) {
                    properties.push(Property {
                        name: INVERT_ELEVATION_PROPERTY.into(),
                        value: PropertyValue::LengthFeet(height),
                    });
                }
                entities.push(entities::IfcEntity::ElementPropertySet {
                    element: entity_index,
                    set: PropertySet {
                        name: name.into(),
                        properties,
                    },
                });
            }
        }
        // RE-154: the common property sets' Reference, the type's name;
        // RE-156: the shared parameter Serial Number.
        for set in reference_sets.into_iter().chain(serial_sets) {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set,
            });
        }
    }

    // #323: group each whole's parts into one aggregate.
    let mut parts_by_whole: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (part_index, whole_id) in pending_parts {
        if let Some(&whole_index) = out.id_to_entity.get(&whole_id) {
            parts_by_whole
                .entry(whole_index)
                .or_default()
                .push(part_index);
        }
    }
    // A whole no part names is not carried by parts: it keeps its body, as
    // Revit's export does for a stair family placed on its own (Snowdon
    // 1603717).
    for (index, body) in held_bodies {
        if parts_by_whole.contains_key(&index) {
            continue;
        }
        if let Some(entities::IfcEntity::BuildingElement { extrusion, .. }) =
            entities.get_mut(index)
        {
            *extrusion = Some(body);
        }
    }
    for (whole, parts) in parts_by_whole {
        entities.push(entities::IfcEntity::Aggregate { whole, parts });
    }

    // RE-138 (#528): a join is written when both elements are exported as
    // distribution elements, the only elements a port may be tied to, and once
    // however many of the two it was read from.
    let is_distribution = |index: usize| {
        matches!(
            entities.get(index),
            Some(entities::IfcEntity::BuildingElement { ifc_type, .. })
                if is_distribution_element(ifc_type)
        )
    };
    let mut connections: std::collections::BTreeSet<[(usize, u32, u32); 2]> = Default::default();
    for (entity, id, joins) in pending_joins {
        for (index, other_id, other_index) in joins {
            let Some(&other) = out.id_to_entity.get(&other_id) else {
                continue;
            };
            if !is_distribution(entity) || !is_distribution(other) {
                continue;
            }
            let mut ends = [(entity, id, index), (other, other_id, other_index)];
            ends.sort();
            connections.insert(ends);
        }
    }
    let curve_ends: Vec<(usize, u32)> = curve_ends
        .into_iter()
        .filter(|(element, _)| is_distribution(*element))
        .collect();
    for [(a, a_id, a_index), (b, b_id, b_index)] in connections {
        entities.push(entities::IfcEntity::PortConnection {
            a,
            a_id,
            a_index,
            b,
            b_id,
            b_index,
        });
    }
    // B54: both ends of every duct and pipe, joined or not.
    for (element, id) in curve_ends {
        for index in 0..2 {
            entities.push(entities::IfcEntity::Port { element, id, index });
        }
    }
    for (id, (name, members)) in pending_systems {
        entities.push(entities::IfcEntity::System { id, name, members });
    }

    if policy.include_geometry {
        for (entity_index, host_id) in pending_hosts {
            if let Some(&host_index) = out.id_to_entity.get(&host_id) {
                if let Some(entities::IfcEntity::BuildingElement {
                    ifc_type,
                    predefined_type,
                    host_element_index,
                    ..
                }) = entities.get_mut(entity_index)
                {
                    *host_element_index = Some(host_index);
                    // RE-84: a door or window whose type draws no geometry
                    // is only the hole it cuts, and Revit's export writes
                    // that opening alone, with no door or window.
                    if without_geometry.contains(&entity_index) {
                        *ifc_type = "IFCOPENINGELEMENT".into();
                        *predefined_type = Some("OPENING".into());
                    }
                }
            }
        }
    }

    out
}

fn storey_from_level(decoded: &DecodedElement) -> Option<Storey> {
    let level = Level::from_decoded(decoded);
    // Drafting / non-building levels stay out of the spatial tree.
    if level.is_building_story == Some(false) {
        return None;
    }
    let name = level
        .name
        .clone()
        .or_else(|| decoded.id.map(|id| format!("Level-{id}")))?;
    // Prefer recovered elevation; name-only partition Levels (2024
    // without ArcWall trailers) still form storeys at 0.0 — callers
    // must treat that as elevation-unresolved, not surveyed height.
    let elevation_feet = recover_level_elevation(&level)
        .ok()
        .map(|e| e.elevation_feet)
        .or(level.elevation_feet)
        .unwrap_or(0.0);
    Some(Storey {
        name,
        elevation_feet,
    })
}

fn material_info_from_decoded(decoded: &DecodedElement) -> Option<MaterialInfo> {
    let material = Material::from_decoded(decoded);
    let name = material.name?;
    if name.trim().is_empty() {
        return None;
    }
    Some(MaterialInfo {
        name,
        color_packed: material.color,
        transparency: material.transparency,
    })
}

#[allow(dead_code)] // retained for when opening→host IFC emission lands
fn opening_index_property_set(decoded: &DecodedElement) -> PropertySet {
    let mut properties = vec![
        Property {
            name: "OpeningKindResolved".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "DoorWindowDiscriminated".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "Source".into(),
            value: PropertyValue::Text("partition_rect_opening_index".into()),
        },
    ];
    for (name, value) in &decoded.fields {
        match (name.as_str(), value) {
            ("m_related_id_a", InstanceField::ElementId { id, .. }) => {
                properties.push(Property {
                    name: "RelatedIdA".into(),
                    value: PropertyValue::Integer(i64::from(*id)),
                });
            }
            ("m_related_id_b", InstanceField::ElementId { id, .. }) => {
                properties.push(Property {
                    name: "RelatedIdB".into(),
                    value: PropertyValue::Integer(i64::from(*id)),
                });
            }
            ("m_host_id", InstanceField::ElementId { id, .. }) => {
                properties.push(Property {
                    name: "HostIdCandidate".into(),
                    value: PropertyValue::Integer(i64::from(*id)),
                });
            }
            ("m_related_id_a_in_elem_table", InstanceField::Bool(b)) => {
                properties.push(Property {
                    name: "RelatedIdAInElemTable".into(),
                    value: PropertyValue::Boolean(*b),
                });
            }
            ("m_related_id_b_in_elem_table", InstanceField::Bool(b)) => {
                properties.push(Property {
                    name: "RelatedIdBInElemTable".into(),
                    value: PropertyValue::Boolean(*b),
                });
            }
            ("m_host_elem_table_confirmed", InstanceField::Bool(b)) => {
                properties.push(Property {
                    name: "HostElemTableConfirmed".into(),
                    value: PropertyValue::Boolean(*b),
                });
            }
            ("m_index", InstanceField::Integer { value, .. }) => {
                properties.push(Property {
                    name: "Index".into(),
                    value: PropertyValue::Integer(*value),
                });
            }
            _ => {}
        }
    }
    PropertySet {
        name: "RvtArcWallRectOpening".into(),
        properties,
    }
}

struct RecoveredWallGeom {
    location_feet: [f64; 3],
    rotation_radians: f64,
    extrusion: Extrusion,
    property_set: Option<PropertySet>,
}

fn wall_geometry_from_decoded(decoded: &DecodedElement) -> Option<RecoveredWallGeom> {
    let wall = Wall::from_decoded(decoded);
    let curve = recover_wall_location_curve_from_wall(&wall, decoded).ok()?;
    let (start, end) = curve.line_endpoints_xy()?;
    let length_feet = wall_segment_length_feet(start, end);
    if !length_feet.is_finite() || length_feet < 0.01 {
        return None;
    }
    // Fail closed: do not invent a 10 ft height.
    let height_feet = wall.unconnected_height_feet?;
    if !height_feet.is_finite() || height_feet <= 0.0 {
        return None;
    }
    let z = wall
        .location_start
        .map(|p| p.z)
        .or(wall.base_offset_feet)
        .unwrap_or(0.0);
    let location_feet = [(start[0] + end[0]) / 2.0, (start[1] + end[1]) / 2.0, z];
    let rotation_radians = wall_segment_angle_radians(start, end);
    let mut properties = vec![
        Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "LocationCurveSource".into(),
            value: PropertyValue::Text(format!("{:?}", curve.source)),
        },
    ];
    if let Some(id) = curve.location_curve_id {
        properties.push(Property {
            name: "LocationCurveId".into(),
            value: PropertyValue::Integer(i64::from(id)),
        });
    }
    Some(RecoveredWallGeom {
        location_feet,
        rotation_radians,
        extrusion: Extrusion {
            width_feet: length_feet,
            depth_feet: UNRESOLVED_ARCWALL_THICKNESS_FEET,
            height_feet,
            profile_override: None,
        },
        property_set: Some(PropertySet {
            name: "RvtWallGeometry".into(),
            properties,
        }),
    })
}

/// `Pset_StairCommon` for a stair and `Pset_StairFlightCommon` for a
/// flight, holding the number of risers, riser height and tread length
/// their data records (RE-47), with the measure types those standard sets
/// declare. `None` when nothing was read.
fn stair_property_set(decoded: &DecodedElement) -> Option<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    let name = match decoded.class.as_str() {
        "Stair" => "Pset_StairCommon",
        "StairsRun" => "Pset_StairFlightCommon",
        _ => return None,
    };
    let mut properties = Vec::new();
    for (field, value) in &decoded.fields {
        let property = match (field.as_str(), value) {
            (mvp::STAIR_RISER_COUNT_FIELD, InstanceField::Integer { value, .. }) => Property {
                name: "NumberOfRiser".into(),
                value: PropertyValue::CountValue(*value),
            },
            (mvp::STAIR_RISER_HEIGHT_FIELD, InstanceField::Float { value, .. }) => Property {
                name: "RiserHeight".into(),
                value: PropertyValue::PositiveLengthFeet(*value),
            },
            (mvp::STAIR_TREAD_DEPTH_FIELD, InstanceField::Float { value, .. }) => Property {
                name: "TreadLength".into(),
                value: PropertyValue::PositiveLengthFeet(*value),
            },
            _ => continue,
        };
        properties.push(property);
    }
    (!properties.is_empty()).then(|| PropertySet {
        name: name.into(),
        properties,
    })
}

/// The property sets whose `Reference` Revit's export gives an element of
/// `ifc_type` (RE-154), each holding the type's name: `Pset_QuantityTakeOff`,
/// the entity's common set, and on walls and slabs its reinforcement-pitch
/// set, which declares `Reference` an `IfcLabel`. Only the entities measured
/// on the RE1 models' exports are listed, with the IFC4 subtypes rvt-rs writes
/// for Revit's flow, furnishing and control elements; any other gets none.
fn reference_property_sets(ifc_type: &str, type_name: &str) -> Vec<PropertySet> {
    let common: &[(&str, bool)] = match ifc_type {
        "IFCWALL" => &[
            ("Pset_WallCommon", true),
            ("Pset_ReinforcementBarPitchOfWall", false),
        ],
        "IFCSLAB" => &[
            ("Pset_SlabCommon", true),
            ("Pset_ReinforcementBarPitchOfSlab", false),
        ],
        "IFCDOOR" => &[("Pset_DoorCommon", true)],
        "IFCCOVERING" => &[("Pset_CoveringCommon", true)],
        "IFCCURTAINWALL" => &[("Pset_CurtainWallCommon", true)],
        "IFCMEMBER" => &[("Pset_MemberCommon", true)],
        "IFCPLATE" => &[("Pset_PlateCommon", true)],
        "IFCRAILING" => &[("Pset_RailingCommon", true)],
        "IFCBUILDINGELEMENTPROXY" => &[("Pset_BuildingElementProxyCommon", true)],
        // Revit's export writes these as IfcFlowTerminal, IfcFlowFitting and
        // IfcFlowSegment; rvt-rs writes the IFC4 subtypes.
        "IFCFLOWTERMINAL"
        | "IFCFLOWFITTING"
        | "IFCFLOWSEGMENT"
        | "IFCDUCTSEGMENT"
        | "IFCPIPESEGMENT"
        | "IFCDUCTFITTING"
        | "IFCPIPEFITTING"
        | "IFCAIRTERMINAL"
        | "IFCSANITARYTERMINAL"
        | "IFCLIGHTFIXTURE"
        | "IFCELECTRICAPPLIANCE" => &[("Pset_DistributionFlowElementCommon", true)],
        // IfcFurnishingElement and IfcDistributionControlElement in Revit's.
        "IFCFURNISHINGELEMENT" | "IFCFURNITURE" | "IFCDISTRIBUTIONCONTROLELEMENT" | "IFCALARM" => {
            &[]
        }
        _ => return Vec::new(),
    };
    std::iter::once(("Pset_QuantityTakeOff", true))
        .chain(common.iter().copied())
        .map(|(name, identifier)| PropertySet {
            name: name.into(),
            properties: vec![Property {
                name: "Reference".into(),
                value: if identifier {
                    PropertyValue::Identifier(type_name.to_string())
                } else {
                    PropertyValue::Label(type_name.to_string())
                },
            }],
        })
        .collect()
}

/// The sets Revit's export gives an element's shared parameter `Serial
/// Number` in (RE-156): `Pset_ManufacturerOccurrence`, and on a
/// building-element proxy also `Pset_PrecastConcreteElementGeneral`, each
/// with `SerialNumber` (`IfcIdentifier`). Empty when the element has none.
fn serial_number_property_sets(decoded: &DecodedElement, ifc_type: &str) -> Vec<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    let serial = decoded
        .fields
        .iter()
        .find_map(|(field, value)| match value {
            InstanceField::String(text) if field == mvp::SERIAL_NUMBER_FIELD => Some(text),
            _ => None,
        });
    let Some(serial) = serial.filter(|text| !text.is_empty()) else {
        return Vec::new();
    };
    let proxy = ifc_type == "IFCBUILDINGELEMENTPROXY";
    [
        "Pset_ManufacturerOccurrence",
        "Pset_PrecastConcreteElementGeneral",
    ]
    .into_iter()
    .filter(|set| proxy || *set == "Pset_ManufacturerOccurrence")
    .map(|set| PropertySet {
        name: set.into(),
        properties: vec![Property {
            name: "SerialNumber".into(),
            value: PropertyValue::Identifier(serial.clone()),
        }],
    })
    .collect()
}

/// Add `IsExternal` (`IfcBoolean`) to a door's `Pset_DoorCommon` or a
/// slab's `Pset_SlabCommon` among `sets`, from its type's Function, 0
/// interior or 1 exterior (RE-158), starting the set when the element has
/// none (no type name for its `Reference`). Nothing for another Function,
/// which RE1 does not show, when the type's Function was not read, or for a
/// door written as its opening alone (RE-84).
fn add_is_external(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    opening_only: bool,
) {
    if opening_only {
        return;
    }
    let common = match ifc_type {
        "IFCDOOR" => "Pset_DoorCommon",
        "IFCSLAB" => "Pset_SlabCommon",
        _ => return,
    };
    let function = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. }
            if name == crate::partition_schema_mvp::TYPE_FUNCTION_FIELD =>
        {
            Some(*value)
        }
        _ => None,
    });
    let external = match function {
        Some(0) => false,
        Some(1) => true,
        _ => return,
    };
    let property = Property {
        name: "IsExternal".into(),
        value: PropertyValue::Boolean(external),
    };
    match sets.iter_mut().find(|set| set.name == common) {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: common.into(),
            properties: vec![property],
        }),
    }
}

/// Add `Finish` (`IfcText`) to a covering's `Pset_CoveringCommon` among
/// `sets`: its type's finish layers' materials (B41).
fn add_covering_finish(sets: &mut [PropertySet], ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCCOVERING" {
        return;
    }
    let finish = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text)
            if name == crate::partition_schema_mvp::COVERING_FINISH_FIELD =>
        {
            Some(text.clone())
        }
        _ => None,
    });
    let Some(finish) = finish else {
        return;
    };
    if let Some(set) = sets
        .iter_mut()
        .find(|set| set.name == "Pset_CoveringCommon")
    {
        set.properties.push(Property {
            name: "Finish".into(),
            value: PropertyValue::Text(finish),
        });
    }
}

/// The property of `Pset_FlowSegmentPipeSegment` holding a pipe's invert.
const INVERT_ELEVATION_PROPERTY: &str = "InvertElevation";
/// How far a pipe's two ends may differ, feet, along an axis it is taken to
/// lie across.
const PIPE_AXIS_TOLERANCE_FEET: f64 = 1e-6;

/// A pipe's invert in model height, feet (RE-157): the lower end of a
/// vertical pipe, or a horizontal pipe's axis less half its inner diameter.
/// `None` for a sloped pipe, or a horizontal one whose inner diameter was not
/// read.
fn pipe_invert_height(fields: &[(String, InstanceField)]) -> Option<f64> {
    use crate::partition_schema_mvp as mvp;
    let pipe = mvp::pipe_body_from_fields(fields)?;
    let (start, end) = (pipe.start, pipe.end);
    let near = |a: f64, b: f64| (a - b).abs() <= PIPE_AXIS_TOLERANCE_FEET;
    let level = near(start[2], end[2]);
    if !level && near(start[0], end[0]) && near(start[1], end[1]) {
        return Some(start[2].min(end[2]));
    }
    if !level {
        return None;
    }
    let inner = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == mvp::PIPE_INNER_DIAMETER_FIELD => {
            Some(*value)
        }
        _ => None,
    })?;
    Some(start[2] - inner / 2.0)
}

/// Take each pipe's storey elevation off its `InvertElevation`, which
/// Revit's export gives above the pipe's storey (RE-157). A pipe on no storey
/// keeps its height above the model's zero.
pub(super) fn pipe_inverts_above_storeys(entities: &mut [entities::IfcEntity], storeys: &[Storey]) {
    let elevations: Vec<f64> = entities
        .iter()
        .map(|entity| match entity {
            entities::IfcEntity::BuildingElement {
                storey_index: Some(index),
                ..
            } => storeys
                .get(*index)
                .map_or(0.0, |storey| storey.elevation_feet),
            _ => 0.0,
        })
        .collect();
    for entity in entities.iter_mut() {
        let entities::IfcEntity::ElementPropertySet { element, set } = entity else {
            continue;
        };
        if set.name != "Pset_FlowSegmentPipeSegment" {
            continue;
        }
        let elevation = elevations.get(*element).copied().unwrap_or(0.0);
        for property in &mut set.properties {
            if let (INVERT_ELEVATION_PROPERTY, PropertyValue::LengthFeet(height)) =
                (property.name.as_str(), &mut property.value)
            {
                *height -= elevation;
            }
        }
    }
}

/// The entities a room's space contains in Revit's export (B63): on RE1
/// Architecture its furniture, sanitary terminals and equipment written as
/// proxies, and no wall, door, slab, covering or curtain wall.
const ROOM_CONTENT_TYPES: [&str; 3] = [
    "IFCFURNITURE",
    "IFCSANITARYTERMINAL",
    "IFCBUILDINGELEMENTPROXY",
];

/// A closed plan loop, model feet.
type PlanLoop = Vec<(f64, f64)>;

/// A space's plan outline and voids in model coordinates: its body's profile
/// moved to its placement.
fn space_plan_outline(
    location: &[f64; 3],
    rotation: f64,
    extrusion: &entities::Extrusion,
) -> Option<(PlanLoop, Vec<PlanLoop>)> {
    let (sin, cos) = rotation.sin_cos();
    let place = |ring: &[(f64, f64)]| -> PlanLoop {
        ring.iter()
            .map(|(x, y)| {
                (
                    location[0] + x * cos - y * sin,
                    location[1] + x * sin + y * cos,
                )
            })
            .collect()
    };
    match &extrusion.profile_override {
        Some(entities::ProfileDef::ArbitraryClosed { points }) => Some((place(points), Vec::new())),
        Some(entities::ProfileDef::ArbitraryWithVoids { points, voids }) => Some((
            place(points),
            voids.iter().map(|ring| place(ring)).collect(),
        )),
        None => {
            let (w, d) = (extrusion.width_feet / 2.0, extrusion.depth_feet / 2.0);
            Some((place(&[(-w, -d), (w, -d), (w, d), (-w, d)]), Vec::new()))
        }
        Some(_) => None,
    }
}

/// Contain each room's furniture, fixtures and equipment in its space, as
/// Revit's export does (B63): a family instance written as one of
/// [`ROOM_CONTENT_TYPES`], hosted by nothing, whose location lies in the plan
/// outline of a space on its storey, or, where it lies in none, whose plan box
/// overlaps exactly one, as a cabinet recessed into a room's wall does (RE1
/// Architecture's 375441, 0.08 ft outside its room). Everything else stays in
/// its storey.
pub(super) fn contain_in_spaces(entities: &mut Vec<entities::IfcEntity>) {
    use crate::element_record_plan_profiles::inside;
    let mut spaces = Vec::new();
    for (index, entity) in entities.iter().enumerate() {
        let entities::IfcEntity::BuildingElement {
            ifc_type,
            storey_index: Some(storey),
            location_feet: Some(location),
            rotation_radians,
            extrusion: Some(extrusion),
            ..
        } = entity
        else {
            continue;
        };
        if ifc_type != "IFCSPACE" {
            continue;
        }
        if let Some((outer, voids)) =
            space_plan_outline(location, rotation_radians.unwrap_or(0.0), extrusion)
        {
            spaces.push((index, *storey, outer, voids));
        }
    }
    let is_family_instance = |set: &Option<PropertySet>| {
        set.as_ref().is_some_and(|set| {
            set.properties.iter().any(|p| {
                p.name == "RevitClass"
                    && matches!(&p.value, PropertyValue::Text(class) if class == "FamilyInstance")
            })
        })
    };
    let mut contents: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, entity) in entities.iter().enumerate() {
        let entities::IfcEntity::BuildingElement {
            ifc_type,
            storey_index: Some(storey),
            location_feet: Some(location),
            rotation_radians,
            extrusion,
            host_element_index: None,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        if !ROOM_CONTENT_TYPES.contains(&ifc_type.as_str()) || !is_family_instance(property_set) {
            continue;
        }
        let holds = |outer: &PlanLoop, voids: &[PlanLoop], point: (f64, f64)| {
            inside(outer, point) && !voids.iter().any(|ring| inside(ring, point))
        };
        let point = (location[0], location[1]);
        let on_storey = || {
            spaces
                .iter()
                .filter(|(_, space_storey, ..)| space_storey == storey)
        };
        let mut space = on_storey()
            .find(|(_, _, outer, voids)| holds(outer, voids, point))
            .map(|(space, ..)| *space);
        if space.is_none() {
            let plan_box = extrusion
                .as_ref()
                .filter(|body| body.profile_override.is_none())
                .and_then(|body| {
                    space_plan_outline(location, rotation_radians.unwrap_or(0.0), body)
                })
                .map(|(outline, _)| outline);
            if let Some(plan_box) = plan_box {
                let overlapping: Vec<usize> = on_storey()
                    .filter(|(_, _, outer, voids)| {
                        plan_box.iter().any(|&corner| holds(outer, voids, corner))
                            || outer.iter().any(|&vertex| inside(&plan_box, vertex))
                    })
                    .map(|(space, ..)| *space)
                    .collect();
                if let [only] = overlapping[..] {
                    space = Some(only);
                }
            }
        }
        if let Some(space) = space {
            contents.entry(space).or_default().push(index);
        }
    }
    for (space, elements) in contents {
        entities.push(entities::IfcEntity::SpaceContainment { space, elements });
    }
}

/// A room's `Pset_SpaceCommon` (RE-153), as Revit's export writes it:
/// `Reference` (`IfcIdentifier`) its name and number, and `FloorCovering`
/// (`IfcLabel`) its Floor Finish. `None` when neither was read.
fn space_common_property_set(decoded: &DecodedElement) -> Option<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    if decoded.class != "Room" {
        return None;
    }
    let text = |wanted: &str| {
        decoded
            .fields
            .iter()
            .find_map(|(field, value)| match value {
                InstanceField::String(text) if field == wanted && !text.is_empty() => {
                    Some(text.clone())
                }
                _ => None,
            })
    };
    let mut properties = Vec::new();
    if let (Some(name), Some(number)) = (text("m_name"), text(mvp::ROOM_NUMBER_FIELD)) {
        properties.push(Property {
            name: "Reference".into(),
            value: PropertyValue::Identifier(format!("{name} {number}")),
        });
    }
    if let Some(finish) = text(mvp::ROOM_FLOOR_FINISH_FIELD) {
        properties.push(Property {
            name: "FloorCovering".into(),
            value: PropertyValue::Label(finish),
        });
    }
    (!properties.is_empty()).then(|| PropertySet {
        name: "Pset_SpaceCommon".into(),
        properties,
    })
}

/// The IFC entity mapping a per-element "IFC Export As" override
/// names, when the decoded element carries one and
/// [`category_map::lookup_export_override`] recognises the value.
fn ifc_export_override(decoded: &DecodedElement) -> Option<&'static category_map::Mapping> {
    decoded
        .fields
        .iter()
        .find_map(|(name, value)| match (name.as_str(), value) {
            (crate::partition_schema_mvp::IFC_EXPORT_AS_FIELD, InstanceField::String(text)) => {
                category_map::lookup_export_override(text)
            }
            _ => None,
        })
}

/// True when this element came from a partition element record.
fn is_partition_element_record(decoded: &DecodedElement) -> bool {
    decoded.fields.iter().any(|(name, value)| {
        matches!(
            (name.as_str(), value),
            ("m_source", InstanceField::String(source))
                if source == "partition_element_record"
        )
    })
}

/// Placement + body from a partition element record (#204 columns,
/// #211 walls / doors / windows).
///
/// The record carries the element's model bounding box, so the
/// placement is its plan centre at the box base and the body is the
/// box itself — an envelope, not a recovered family profile or a
/// wall location curve. The property set says so rather than letting
/// a consumer read the rectangle as a modelled section.
/// What an element record gives an element: its placement, its body and
/// the properties saying where they came from.
struct RecordGeometry {
    location: [f64; 3],
    rotation: Option<f64>,
    body: Extrusion,
    /// A body the extrusion cannot express (a sloped beam, RE-49). `body`
    /// stays the record box for consumers that read only extrusions.
    solid: Option<entities::SolidShape>,
    properties: PropertySet,
    pieces: Vec<Extrusion>,
    /// Each tagged void of a floor's sketch (RE-151): the piece it is in, its
    /// tag, its outline in the body's frame and the floor's thickness.
    void_openings: Vec<VoidBody>,
}

/// A tagged void of a floor's sketch (RE-151): the piece it is in, its tag,
/// its outline in the body's frame, and the floor's thickness, feet.
type VoidBody = (usize, u32, Vec<(f64, f64)>, f64);

/// `BodySource` of a body that is the element record's bounding box, where
/// no carrier refines it (#409).
pub const RECORD_BBOX_BODY_SOURCE: &str = "partition_element_record_bbox";

/// `BodySource` of a family instance turned off the model's axes, drawn as
/// the rectangle at its angle whose box is its record box (RE-87).
pub const INSTANCE_TURNED_BODY_SOURCE: &str = "partition_family_instance_turned_box";

/// `BodySource` of a curtain mullion or panel turned or tilted off the
/// model's axes, drawn as the box along its own axes (RE-106).
pub const INSTANCE_ORIENTED_BODY_SOURCE: &str = "partition_family_instance_oriented_box";

/// Below this, `|det|` of the axes' absolute components leaves the box along
/// them undetermined by the record box (RE-106).
const ORIENTED_BOX_MIN_DET: f64 = 0.1;

/// The half-extents along `axes` (X, Y and Z, each in model coordinates) of
/// the box whose axis-aligned box is `size` (RE-106): the box centred on the
/// record box's centre whose extent along each model axis, `sum |a_i| e_i`,
/// is the record box's. `None` where the axes leave it undetermined or a
/// half-extent would not be positive.
fn oriented_box(axes: [[f64; 3]; 3], size: [f64; 3]) -> Option<([[f64; 3]; 3], [f64; 3])> {
    let m = |row: usize, col: usize| axes[col][row].abs();
    let det = m(0, 0) * (m(1, 1) * m(2, 2) - m(1, 2) * m(2, 1))
        - m(0, 1) * (m(1, 0) * m(2, 2) - m(1, 2) * m(2, 0))
        + m(0, 2) * (m(1, 0) * m(2, 1) - m(1, 1) * m(2, 0));
    if !det.is_finite() || det.abs() < ORIENTED_BOX_MIN_DET {
        return None;
    }
    let h = size.map(|s| s / 2.0);
    // Cramer's rule on sum_i |axis_i[row]| e_i = h[row].
    let solve = |col: usize| {
        let pick = |row: usize, c: usize| if c == col { h[row] } else { m(row, c) };
        (pick(0, 0) * (pick(1, 1) * pick(2, 2) - pick(1, 2) * pick(2, 1))
            - pick(0, 1) * (pick(1, 0) * pick(2, 2) - pick(1, 2) * pick(2, 0))
            + pick(0, 2) * (pick(1, 0) * pick(2, 1) - pick(1, 1) * pick(2, 0)))
            / det
    };
    let half = [solve(0), solve(1), solve(2)];
    half.iter()
        .all(|e| e.is_finite() && *e > 0.0)
        .then_some((axes, half))
}

/// The box along `axes` with half-extents `half`, centred on the record
/// box's centre, `height / 2` above the element's placement: its X by Y
/// section extruded along its Z.
fn oriented_box_solid(axes: [[f64; 3]; 3], half: [f64; 3], height: f64) -> entities::SolidShape {
    let [x, _, z] = axes;
    let centre = [0.0, 0.0, height / 2.0];
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::Rectangle {
            width_feet: 2.0 * half[0],
            depth_feet: 2.0 * half[1],
        },
        origin_feet: [
            centre[0] - half[2] * z[0],
            centre[1] - half[2] * z[1],
            centre[2] - half[2] * z[2],
        ],
        axis: z,
        ref_direction: x,
        depth_feet: 2.0 * half[2],
    }
}

/// Properties holding a turned family instance's plan origin from its
/// transform, model feet (RE-87, RE-89).
pub const INSTANCE_ORIGIN_PROPERTIES: [&str; 2] = ["InstanceOriginX", "InstanceOriginY"];

/// Properties holding a window's opening from its type and transform
/// (RE-93, [`crate::partition_schema_mvp::FillerOpening`]): the plan point
/// it is centred on and the direction of its width, model feet and a unit
/// vector, then its base elevation, width and height, feet.
pub const FILLER_OPENING_PROPERTIES: [&str; 7] = [
    "OpeningCentreX",
    "OpeningCentreY",
    "OpeningAxisX",
    "OpeningAxisY",
    "OpeningBaseElevation",
    "OpeningWidth",
    "OpeningHeight",
];

/// Classes whose body another rule draws, which a family instance's turn
/// leaves alone: columns (their type's section), beams (their line), and
/// curtain-wall panels and mullions (their grid).
const TURNED_BOX_EXCLUDED_CLASSES: &[&str] = &[
    "Column",
    "StructuralColumn",
    "StructuralFraming",
    "CurtainWallPanel",
    "CurtainWallMullion",
];

/// The rectangle at the plan angle of `x_axis` whose axis-aligned box is
/// `width` × `depth` (RE-87): `(angle, along X, along Y)`. A rectangle
/// `w` × `d` turned by the angle has a box `w|cos| + d|sin|` wide and
/// `w|sin| + d|cos|` deep, which gives `w` and `d` back unless the angle is
/// near 45 degrees. `None` there, or where either side would not be
/// positive.
fn turned_box(x_axis: [f64; 2], width: f64, depth: f64) -> Option<(f64, f64, f64)> {
    let length = x_axis[0].hypot(x_axis[1]);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let (c, s) = ((x_axis[0] / length).abs(), (x_axis[1] / length).abs());
    let det = c * c - s * s;
    if det.abs() < 0.1 {
        return None;
    }
    let along = (width * c - depth * s) / det;
    let across = (depth * c - width * s) / det;
    (along > 1e-6 && across > 1e-6).then(|| (x_axis[1].atan2(x_axis[0]), along, across))
}

/// `BodySource` of a beam whose body runs along its location line (RE-49).
pub const BEAM_AXIS_BODY_SOURCE: &str = "partition_beam_axis";

/// A sloped beam's body: its section swept along its centreline, with the
/// section's depth kept in the line's vertical plane. `location` is the
/// element's placement, which the directrix is relative to.
fn beam_swept_solid(
    beam: &crate::partition_beam_axes::BeamBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let (a, b) = beam.centreline();
    let local = |p: [f64; 3]| [p[0] - location[0], p[1] - location[1], p[2] - location[2]];
    entities::SolidShape::SweptPath {
        // The fixed reference sets the profile's X axis, so X is the depth.
        profile: entities::ProfileDef::Rectangle {
            width_feet: beam.depth_feet,
            depth_feet: beam.width_feet,
        },
        directrix_points_feet: vec![local(a), local(b)],
        fixed_reference: [0.0, 0.0, 1.0],
    }
}

/// `BodySource` of a pipe drawn as the cylinder its connector entries and
/// record box make (RE-131).
pub const PIPE_AXIS_BODY_SOURCE: &str = "partition_pipe_axis";

/// A pipe's body (RE-131): its circle extruded from one end to the other.
/// `location` is the element's placement, which the extrusion's own placement
/// is relative to.
fn pipe_cylinder_solid(
    pipe: &crate::partition_pipe_axes::PipeBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let length = pipe.length_feet();
    let axis = [
        (pipe.end[0] - pipe.start[0]) / length,
        (pipe.end[1] - pipe.start[1]) / length,
        (pipe.end[2] - pipe.start[2]) / length,
    ];
    // The circle has no orientation, so the profile's X axis is any direction
    // square to the axis: the axis crossed with the model axis it is least
    // along.
    let least = (0..3)
        .min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
        .unwrap_or(0);
    let mut across = [0.0; 3];
    across[least] = 1.0;
    let mut x = [
        axis[1] * across[2] - axis[2] * across[1],
        axis[2] * across[0] - axis[0] * across[2],
        axis[0] * across[1] - axis[1] * across[0],
    ];
    let norm = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt().max(1e-12);
    for value in &mut x {
        *value /= norm;
    }
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::Circle {
            radius_feet: pipe.radius_feet,
        },
        origin_feet: [
            pipe.start[0] - location[0],
            pipe.start[1] - location[1],
            pipe.start[2] - location[2],
        ],
        axis,
        ref_direction: x,
        depth_feet: length,
    }
}

/// `BodySource` of a structural column drawn as its type's I section
/// (RE-104).
pub const COLUMN_I_SECTION_BODY_SOURCE: &str = "family_type_i_section";

/// How closely an I-section column's section, turned to its X axis, must
/// fill its record box in plan (RE-104), feet.
pub const COLUMN_SECTION_TOLERANCE_FEET: f64 = 0.01;

/// An upright structural column's I section and the plan angle of its X
/// axis, which its flanges run along (RE-104): only where the section turned
/// by that angle is `box_width` × `box_depth` in plan, as its record box is.
fn column_i_section(
    fields: &[(String, InstanceField)],
    box_width: f64,
    box_depth: f64,
) -> Option<(f64, crate::partition_type_parameters::ISection)> {
    let section = crate::partition_schema_mvp::beam_i_section_from_fields(fields)?;
    let [ax, ay] = crate::partition_schema_mvp::COLUMN_I_AXIS_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    let (ax, ay) = (ax?, ay?);
    let length = ax.hypot(ay);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let (c, s) = ((ax / length).abs(), (ay / length).abs());
    let (w, d) = (section.width_feet, section.depth_feet);
    let fits = (w * c + d * s - box_width).abs() <= COLUMN_SECTION_TOLERANCE_FEET
        && (w * s + d * c - box_depth).abs() <= COLUMN_SECTION_TOLERANCE_FEET;
    fits.then(|| (ay.atan2(ax), section))
}

/// An I-section beam's body (RE-103): its type's I extruded along its
/// centreline from one end, the profile's X axis level across the line, so
/// the flanges run across and the web stands upright in the line's vertical
/// plane. `location` is the element's placement. An extrusion fixes the
/// profile's axes by its placement; a fixed-reference sweep would leave
/// them to how a reader projects the reference.
fn beam_i_solid(
    beam: &crate::partition_beam_axes::BeamBody,
    section: &crate::partition_type_parameters::ISection,
    location: [f64; 3],
) -> entities::SolidShape {
    let (a, _) = beam.centreline();
    let d = beam.direction;
    let plan = d[0].hypot(d[1]).max(1e-12);
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::IShape {
            overall_width_feet: section.width_feet,
            overall_depth_feet: section.depth_feet,
            web_thickness_feet: section.web_feet,
            flange_thickness_feet: section.flange_feet,
        },
        origin_feet: [a[0] - location[0], a[1] - location[1], a[2] - location[2]],
        axis: d,
        ref_direction: [-d[1] / plan, d[0] / plan, 0.0],
        depth_feet: beam.length_feet,
    }
}

/// `BodySource` of a stair run drawn as its treads and risers (RE-52).
pub const STAIR_RUN_BODY_SOURCE: &str = "partition_stair_run_sketch";

/// `BodySource` of a shed roof drawn along its slope (RE-56).
pub const ROOF_SLOPE_BODY_SOURCE: &str = "partition_roof_slope";

/// How closely a shed roof's rise and thickness must reproduce its record
/// box's height for its slope to be drawn, feet.
const ROOF_SLOPE_CLOSURE_FEET: f64 = 1e-4;

/// A shed roof's body (RE-56): its plan outline, rising from the edge that
/// defines its slope at that slope, with its type's thickness measured
/// square to the slope and upright sides. `outer` and `inner` are the
/// outline relative to the element's placement at `(x, y)` and the record
/// box's base, and `height` is the box's height.
///
/// `None` unless the outline lies on one side of the edge and the rise
/// across it plus the thickness's vertical extent reproduces `height`
/// within [`ROOF_SLOPE_CLOSURE_FEET`], so the body is exactly as tall as
/// the one Revit recorded.
fn roof_slope_solid(
    slope: crate::partition_schema_mvp::RoofSlope,
    [x, y]: [f64; 2],
    outer: &[(f64, f64)],
    inner: &[Vec<(f64, f64)>],
    height: f64,
) -> Option<entities::SolidShape> {
    let [sx, sy] = slope.edge_start;
    let [ex, ey] = slope.edge_end;
    let length = (ex - sx).hypot(ey - sy);
    let angle = slope.angle_radians;
    if !(length > 1e-9 && angle.is_finite() && angle > 0.0 && angle < std::f64::consts::FRAC_PI_2) {
        return None;
    }
    let (ux, uy) = ((ex - sx) / length, (ey - sy) / length);
    // Distance from the edge, local coordinates, positive into the roof.
    let offset = |(px, py): (f64, f64)| (px + x - sx) * -uy + (py + y - sy) * ux;
    let (low, high) = outer
        .iter()
        .map(|&p| offset(p))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| {
            (lo.min(d), hi.max(d))
        });
    let (sign, far) = if high.abs() >= low.abs() {
        (1.0, high)
    } else {
        (-1.0, -low)
    };
    let near = if sign > 0.0 { low } else { -high };
    if near < -1e-6 {
        return None;
    }
    let rise = angle.tan();
    let vertical = slope.thickness_feet / angle.cos();
    if (far * rise + vertical - height).abs() > ROOF_SLOPE_CLOSURE_FEET {
        return None;
    }
    let mesh = super::body_geometry::sloped_slab_mesh(
        outer,
        inner,
        |p| sign * offset(p) * rise,
        vertical,
    )?;
    Some(entities::SolidShape::FacetedBrep {
        vertices_feet: mesh.vertices,
        triangles: mesh
            .triangles
            .into_iter()
            .map(|[a, b, c]| entities::BrepTriangle(a, b, c))
            .collect(),
    })
}

/// `BodySource` of a wall drawn its type's thickness either side of its
/// centreline (RE-54).
pub const WALL_CENTRELINE_BODY_SOURCE: &str = "partition_wall_centreline";

/// `BodySource` of a tapered wall drawn as its cross-section along its
/// centreline (RE-86).
pub const WALL_TAPERED_BODY_SOURCE: &str = "partition_wall_tapered_section";

/// `BodySource` of a wall drawn from its centreline whose layers end where
/// its layered butt joins put them (RE-71).
pub const WALL_LAYER_JOIN_BODY_SOURCE: &str = "partition_wall_centreline_layer_joins";

/// `BodySource` of a curved wall drawn as the ring sector its arc makes
/// (RE-75).
pub const WALL_ARC_BODY_SOURCE: &str = "partition_wall_arc";

/// How far a curved wall's drawn outline may stray inside its arcs, feet.
pub const WALL_ARC_TOLERANCE_FEET: f64 = 0.0005;

/// A curved wall's plan outline: the ring sector between its arc's radius
/// less and plus half its thickness, from its start angle to its end angle,
/// each arc drawn as chords that stray at most [`WALL_ARC_TOLERANCE_FEET`]
/// inside it. `None` for a degenerate arc or one thicker than its radius.
fn wall_arc_outline(arc: &crate::partition_schema_mvp::WallArc) -> Option<Vec<(f64, f64)>> {
    let half = arc.thickness_feet * 0.5;
    let sweep = arc.angles[1] - arc.angles[0];
    let outer = arc.radius + half;
    if !(sweep.is_finite() && sweep > 0.0 && sweep <= std::f64::consts::TAU)
        || !(half.is_finite() && half > 0.0 && arc.radius > half)
    {
        return None;
    }
    let step = 2.0
        * (1.0 - WALL_ARC_TOLERANCE_FEET / outer)
            .clamp(-1.0, 1.0)
            .acos();
    let count = ((sweep / step).ceil() as usize).clamp(2, 512);
    let angle = |index: usize| arc.angles[0] + sweep * index as f64 / count as f64;
    let mut points: Vec<(f64, f64)> = (0..=count)
        .map(|index| {
            let [x, y] = arc.point(angle(index), half);
            (x, y)
        })
        .collect();
    points.extend((0..=count).rev().map(|index| {
        let [x, y] = arc.point(angle(index), -half);
        (x, y)
    }));
    Some(points)
}

/// `ThicknessSource` of a wall whose thickness is its type's layers summed
/// (RE-54).
pub const WALL_TYPE_THICKNESS_SOURCE: &str = "wall_type_compound_structure";

/// How far outside a wall's record box its centreline's midpoint may lie,
/// beyond the wall's own thickness, before the line is taken to belong to
/// something else.
const WALL_AXIS_BOX_TOLERANCE_FEET: f64 = 1e-6;

/// How closely a rectangle along an angled wall's line must reproduce its
/// record box for the box to set its length.
const WALL_BOX_CLOSURE_FEET: f64 = 1e-4;

/// A wall's body from its centreline (RE-54): a rectangle its type's
/// thickness across, centred on the line.
#[derive(Debug, Clone, Copy, PartialEq)]
struct WallCentrelineBody {
    centre: [f64; 2],
    /// Plan rotation of the rectangle's width axis; `None` keeps the model
    /// axes.
    rotation: Option<f64>,
    width_feet: f64,
    depth_feet: f64,
    thickness_feet: f64,
    /// How far past its line's start and end the body reaches at a butt
    /// join its join lists decide (RE-70) or a T joint (RE-73).
    join_reach_feet: [Option<f64>; 2],
}

/// The body of a wall with centreline `axis` whose record box is centred on
/// `(x, y)` with plan extents `width` × `depth`.
///
/// Across the wall, the body is the type's thickness centred on the line.
/// Along it:
/// - an axis-parallel wall keeps the box's extent, which its joins trimmed
///   (RE-26). A box thinner than the type is a wall Revit cut, and keeps
///   the box.
/// - a wall at an angle is the rectangle the box closes on, when one of the
///   type's thickness along the line fits the box exactly and is centred on
///   the line; otherwise it runs from one end of its line to the other,
///   unless the box is more than twice the wall's thickness (at least a
///   foot) wider than that rectangle's.
/// - an end at a butt join its join lists decide reaches half the other
///   wall's thickness past the line's end, or stops that far short of it
///   (RE-70), for either kind of wall; an end at a T joint stops that far
///   short (RE-73).
///
/// `None` when the line is degenerate, its midpoint lies outside the box,
/// or the box is thinner than the type or too wide.
fn wall_centreline_body(
    axis: crate::partition_schema_mvp::WallAxis,
    [x, y]: [f64; 2],
    width: f64,
    depth: f64,
) -> Option<WallCentrelineBody> {
    let [sx, sy] = axis.start;
    let [ex, ey] = axis.end;
    let thickness = axis.thickness_feet;
    let length = (ex - sx).hypot(ey - sy);
    if !(length.is_finite() && length > 1e-9 && thickness.is_finite() && thickness > 0.0) {
        return None;
    }
    let (mx, my) = ((sx + ex) / 2.0, (sy + ey) / 2.0);
    let slack = thickness + WALL_AXIS_BOX_TOLERANCE_FEET;
    if (mx - x).abs() > width / 2.0 + slack || (my - y).abs() > depth / 2.0 + slack {
        return None;
    }
    let (ux, uy) = ((ex - sx) / length, (ey - sy) / length);
    let thinner = |across: f64| across < thickness - WALL_AXIS_BOX_TOLERANCE_FEET;
    // RE-70: an end its join lists decide sits that far past the line's
    // end, whatever the box says; the other end keeps its own.
    let joined = |centre: [f64; 2], run: f64| {
        let middle = (centre[0] - sx) * ux + (centre[1] - sy) * uy;
        let [start, end] = axis.join_reach_feet;
        let low = start.map_or(middle - run / 2.0, |reach| -reach);
        let high = end.map_or(middle + run / 2.0, |reach| length + reach);
        if high - low <= WALL_AXIS_BOX_TOLERANCE_FEET {
            return (centre, run);
        }
        let at = (low + high) / 2.0;
        ([sx + ux * at, sy + uy * at], high - low)
    };
    if uy.abs() <= 1e-9 {
        let (centre, run) = joined([x, sy], width);
        return (!thinner(depth)).then_some(WallCentrelineBody {
            centre,
            rotation: None,
            width_feet: run,
            depth_feet: thickness,
            thickness_feet: thickness,
            join_reach_feet: axis.join_reach_feet,
        });
    }
    if ux.abs() <= 1e-9 {
        let (centre, run) = joined([sx, y], depth);
        return (!thinner(width)).then_some(WallCentrelineBody {
            centre,
            rotation: None,
            width_feet: thickness,
            depth_feet: run,
            thickness_feet: thickness,
            join_reach_feet: axis.join_reach_feet,
        });
    }
    // The box of a rectangle `run` long and `thickness` across along
    // (ux, uy) is `run·|ux| + thickness·|uy|` wide and
    // `run·|uy| + thickness·|ux|` deep.
    let run_from_width = (width - thickness * uy.abs()) / ux.abs();
    let run_from_depth = (depth - thickness * ux.abs()) / uy.abs();
    let off_line = (x - sx) * -uy + (y - sy) * ux;
    let (centre, run) = if (run_from_width - run_from_depth).abs() <= WALL_BOX_CLOSURE_FEET
        && off_line.abs() <= WALL_BOX_CLOSURE_FEET
        && run_from_width > 0.0
    {
        ([x, y], run_from_width)
    } else {
        // A box far wider than the line's own rectangle holds a body that
        // is not that rectangle: a leaning or profiled wall.
        let limit = 2.0 * thickness.max(1.0);
        let excess_width = width - (length * ux.abs() + thickness * uy.abs());
        let excess_depth = depth - (length * uy.abs() + thickness * ux.abs());
        if excess_width > limit || excess_depth > limit {
            return None;
        }
        ([mx, my], length)
    };
    let (centre, run) = joined(centre, run);
    Some(WallCentrelineBody {
        centre,
        rotation: Some(uy.atan2(ux)),
        width_feet: run,
        depth_feet: thickness,
        thickness_feet: thickness,
        join_reach_feet: axis.join_reach_feet,
    })
}

/// A tapered wall's plan footprint as a [`WallCentrelineBody`] (RE-86): the
/// rectangle between its interior face and its exterior face at the base,
/// `height` × tan(angle) further out than at the top. It is checked against
/// the record box as a vertical wall of that width would be, on the line
/// moved half that much towards the exterior. Its `thickness_feet` stays
/// the type's, at the top.
fn tapered_wall_body(
    axis: crate::partition_schema_mvp::WallAxis,
    taper: crate::partition_schema_mvp::WallTaper,
    centre: [f64; 2],
    width: f64,
    depth: f64,
    height: f64,
) -> Option<WallCentrelineBody> {
    let spread = height * taper.angle_radians.tan();
    if !(spread.is_finite() && spread > 0.0) {
        return None;
    }
    let [ex, ey] = taper.exterior.map(|value| value * spread / 2.0);
    let shifted = crate::partition_schema_mvp::WallAxis {
        start: [axis.start[0] + ex, axis.start[1] + ey],
        end: [axis.end[0] + ex, axis.end[1] + ey],
        thickness_feet: axis.thickness_feet + spread,
        ..axis
    };
    wall_centreline_body(shifted, centre, width, depth).map(|body| WallCentrelineBody {
        thickness_feet: axis.thickness_feet,
        ..body
    })
}

/// A tapered wall's solid (RE-86): its cross-section, the type's thickness
/// about the line at the top and wider towards the exterior at the base,
/// extruded along the line through `body`'s run. It is placed in the frame
/// of the line's direction, centred on `body`.
fn tapered_wall_solid(
    body: &WallCentrelineBody,
    axis: &crate::partition_schema_mvp::WallAxis,
    taper: crate::partition_schema_mvp::WallTaper,
    height: f64,
) -> Option<entities::SolidShape> {
    let [sx, sy] = axis.start;
    let length = (axis.end[0] - sx).hypot(axis.end[1] - sy);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let along = [(axis.end[0] - sx) / length, (axis.end[1] - sy) / length];
    // The body's run: the rectangle's width, unless an axis-parallel body
    // runs along the model's Y axis.
    let run = if body.rotation.is_none() && along[1].abs() > 1e-9 {
        body.depth_feet
    } else {
        body.width_feet
    };
    let left = [-along[1], along[0]];
    let side = if taper.exterior[0] * left[0] + taper.exterior[1] * left[1] >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let half = axis.thickness_feet / 2.0;
    let spread = height * taper.angle_radians.tan();
    let inner = -side * (half + spread / 2.0);
    let mut points = vec![
        (inner, 0.0),
        (side * (half + spread / 2.0), 0.0),
        (side * (half - spread / 2.0), height),
        (inner, height),
    ];
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
        .sum();
    if area < 0.0 {
        points.reverse();
    }
    (run > 0.0 && height > 0.0).then(|| entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::ArbitraryClosed { points },
        origin_feet: [-run / 2.0, 0.0, 0.0],
        axis: [1.0, 0.0, 0.0],
        ref_direction: [0.0, 1.0, 0.0],
        depth_feet: run,
    })
}

/// A wall body's plan outline where its layered butt joins end each layer
/// on its own line (RE-71), in the body's local frame: a staircase of one
/// band per layer across the wall. An end no layered join decides keeps
/// `body`'s end for every layer.
///
/// `None` when the layers do not add up to the body's thickness, the
/// exterior is not across the line, or a layer would have no length.
fn layered_wall_profile(
    body: &WallCentrelineBody,
    axis: &crate::partition_schema_mvp::WallAxis,
    ends: &crate::partition_schema_mvp::WallLayerEnds,
) -> Option<Vec<(f64, f64)>> {
    let [sx, sy] = axis.start;
    let length = (axis.end[0] - sx).hypot(axis.end[1] - sy);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let along = [(axis.end[0] - sx) / length, (axis.end[1] - sy) / length];
    let across = ends.exterior;
    let total: f64 = ends.widths.iter().sum();
    if (along[0] * across[0] + along[1] * across[1]).abs() > 1e-6
        || (across[0].hypot(across[1]) - 1.0).abs() > 1e-6
        || (total - body.thickness_feet).abs() > WALL_BOX_CLOSURE_FEET
    {
        return None;
    }
    let run = if body.rotation.is_some() || along[1].abs() <= 1e-9 {
        body.width_feet
    } else {
        body.depth_feet
    };
    let middle = (body.centre[0] - sx) * along[0] + (body.centre[1] - sy) * along[1];
    let [start, end] = &ends.reach_feet;
    let mut top = total / 2.0;
    let mut bands = Vec::with_capacity(ends.widths.len());
    for (index, width) in ends.widths.iter().enumerate() {
        // Each layer ends along both of its edges, square or slanted (RE-74).
        let low = start.as_ref().map_or([middle - run / 2.0; 2], |reach| {
            [-reach[index][0], -reach[index][1]]
        });
        let high = end.as_ref().map_or([middle + run / 2.0; 2], |reach| {
            [length + reach[index][0], length + reach[index][1]]
        });
        if high[0] - low[0] <= WALL_AXIS_BOX_TOLERANCE_FEET
            || high[1] - low[1] <= WALL_AXIS_BOX_TOLERANCE_FEET
        {
            return None;
        }
        bands.push((low, high, top, top - width));
        top -= width;
    }
    // Along the exterior face to the end, down the end's steps, back along
    // the interior face and up the start's steps.
    let mut outline: Vec<(f64, f64)> = Vec::with_capacity(bands.len() * 4);
    for (low, high, upper, lower) in &bands {
        if outline.is_empty() {
            outline.push((low[0], *upper));
        }
        outline.push((high[0], *upper));
        outline.push((high[1], *lower));
    }
    for (low, _, upper, lower) in bands.iter().rev() {
        outline.push((low[1], *lower));
        outline.push((low[0], *upper));
    }
    outline.pop();
    outline.dedup_by(|a, b| (a.0 - b.0).abs() <= 1e-12 && (a.1 - b.1).abs() <= 1e-12);
    // Where neighbouring layers end together, their shared end is one edge.
    let corners: Vec<(f64, f64)> = (0..outline.len())
        .filter(|&index| {
            let previous = outline[(index + outline.len() - 1) % outline.len()];
            let point = outline[index];
            let next = outline[(index + 1) % outline.len()];
            ((point.0 - previous.0) * (next.1 - point.1)
                - (point.1 - previous.1) * (next.0 - point.0))
                .abs()
                > 1e-12
        })
        .map(|index| outline[index])
        .collect();
    let outline = corners;
    let (cos, sin) = body
        .rotation
        .map_or((1.0, 0.0), |angle| (angle.cos(), angle.sin()));
    let mut points: Vec<(f64, f64)> = outline
        .into_iter()
        .map(|(t, a)| {
            let x = sx + along[0] * t + across[0] * a - body.centre[0];
            let y = sy + along[1] * t + across[1] * a - body.centre[1];
            (x * cos + y * sin, -x * sin + y * cos)
        })
        .collect();
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
        .sum();
    if area < 0.0 {
        points.reverse();
    }
    Some(points)
}

/// A stair run's treads and risers: its side view extruded across the run.
/// `location` is the element's placement, which the solid's own placement
/// is relative to. That placement's Z axis runs across the run and its X
/// axis is up, so the profile's Y axis is the one crossed with the other:
/// the climb direction or its reverse, by which side of the run the sketch
/// starts on.
fn stair_run_solid(
    run: &crate::partition_schema_mvp::StairRunBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let y_axis = [run.across[1], -run.across[0]];
    let along = if y_axis[0] * run.climb[0] + y_axis[1] * run.climb[1] >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let mut points: Vec<(f64, f64)> = run.profile.iter().map(|&[u, z]| (z, along * u)).collect();
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
        .sum();
    if area < 0.0 {
        points.reverse();
    }
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::ArbitraryClosed { points },
        origin_feet: [
            run.origin[0] - location[0],
            run.origin[1] - location[1],
            run.origin[2] - location[2],
        ],
        axis: [run.across[0], run.across[1], 0.0],
        ref_direction: [0.0, 0.0, 1.0],
        depth_feet: run.width_feet,
    }
}

fn element_record_geometry_from_decoded(decoded: &DecodedElement) -> Option<RecordGeometry> {
    let class = decoded.class.as_str();
    let mut width = None;
    let mut depth = None;
    let mut height = None;
    let mut x = None;
    let mut y = None;
    let mut z = None;
    let mut source_stream = None;
    let mut revit_class = None;
    let mut type_parameters: Vec<(String, String)> = Vec::new();
    let mut wall_body_source = None;
    let mut wall_thickness = None;
    let mut wall_trim_start = None;
    let mut wall_trim_end = None;
    let mut column_body_source = None;
    let mut beam_body_source = None;
    let mut column_cut_walls = None;
    let mut type_symbol_id = None;
    let mut type_profile = (None, None);
    let mut level_id = None;
    let mut level_bind_source: Option<String> = None;
    let mut room_name = None;
    let mut room_number = None;
    let mut family_name = None;
    let mut family_name_source: Option<String> = None;
    let mut type_name = None;
    for (name, value) in &decoded.fields {
        match (name.as_str(), value) {
            ("m_name", InstanceField::String(v)) => room_name = Some(v.clone()),
            (crate::partition_schema_mvp::FAMILY_NAME_FIELD, InstanceField::String(v)) => {
                family_name = Some(v.clone());
            }
            (crate::partition_schema_mvp::FAMILY_NAME_SOURCE_FIELD, InstanceField::String(v)) => {
                family_name_source = Some(v.clone());
            }
            (crate::partition_schema_mvp::TYPE_NAME_FIELD, InstanceField::String(v)) => {
                type_name = Some(v.clone());
            }
            (crate::partition_schema_mvp::ROOM_NUMBER_FIELD, InstanceField::String(v)) => {
                room_number = Some(v.clone());
            }
            (
                crate::element_record_wall_joins::WALL_BODY_SOURCE_FIELD,
                InstanceField::String(v),
            ) => {
                wall_body_source = Some(v.clone());
            }
            (
                crate::element_record_wall_joins::WALL_THICKNESS_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                wall_thickness = Some(*value);
            }
            (
                crate::element_record_wall_joins::WALL_TRIM_START_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                wall_trim_start = Some(*value);
            }
            (
                crate::element_record_wall_joins::WALL_TRIM_END_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                wall_trim_end = Some(*value);
            }
            (
                crate::element_record_column_cuts::COLUMN_BODY_SOURCE_FIELD,
                InstanceField::String(v),
            ) => {
                column_body_source = Some(v.clone());
            }
            (crate::element_record_beam_cuts::BEAM_BODY_SOURCE_FIELD, InstanceField::String(v)) => {
                beam_body_source = Some(v.clone());
            }
            (
                crate::element_record_column_cuts::COLUMN_CUT_WALL_COUNT_FIELD,
                InstanceField::Integer { value, .. },
            ) => {
                column_cut_walls = Some(*value);
            }
            (
                crate::partition_schema_mvp::TYPE_SYMBOL_FIELD,
                InstanceField::ElementId { id, .. },
            ) => type_symbol_id = Some(*id),
            (
                crate::element_record_level_refs::LEVEL_REFERENCE_FIELD,
                InstanceField::ElementId { id, .. },
            ) => level_id = Some(*id),
            (
                crate::element_record_level_refs::LEVEL_BIND_SOURCE_FIELD,
                InstanceField::String(source),
            ) => level_bind_source = Some(source.clone()),
            (
                crate::partition_schema_mvp::TYPE_PROFILE_WIDTH_FIELD,
                InstanceField::Float { value, .. },
            ) => type_profile.0 = Some(*value),
            (
                crate::partition_schema_mvp::TYPE_PROFILE_DEPTH_FIELD,
                InstanceField::Float { value, .. },
            ) => type_profile.1 = Some(*value),
            ("m_bboxWidth", InstanceField::Float { value, .. }) => width = Some(*value),
            ("m_bboxDepth", InstanceField::Float { value, .. }) => depth = Some(*value),
            ("m_bboxHeight", InstanceField::Float { value, .. }) => height = Some(*value),
            ("m_locationX", InstanceField::Float { value, .. }) => x = Some(*value),
            ("m_locationY", InstanceField::Float { value, .. }) => y = Some(*value),
            ("m_locationZ", InstanceField::Float { value, .. }) => z = Some(*value),
            (crate::partition_schema_mvp::REVIT_CLASS_FIELD, InstanceField::String(value)) => {
                revit_class = Some(value.clone());
            }
            (name, InstanceField::String(value))
                if name.starts_with(crate::partition_schema_mvp::TYPE_PARAMETER_FIELD_PREFIX) =>
            {
                type_parameters.push((
                    name[crate::partition_schema_mvp::TYPE_PARAMETER_FIELD_PREFIX.len()..]
                        .to_string(),
                    value.clone(),
                ));
            }
            ("m_source_stream", InstanceField::String(value)) => {
                source_stream = Some(value.clone());
            }
            _ => {}
        }
    }
    let (x, y, z) = (x?, y?, z?);
    let (width, depth, height) = (width?, depth?, height?);
    if !(width.is_finite() && depth.is_finite() && height.is_finite()) {
        return None;
    }
    // Fail closed rather than emit a degenerate solid.
    if width <= 0.0 || depth <= 0.0 || height <= 0.0 {
        return None;
    }
    // RE-49: a beam whose location line the record box supports runs along
    // that line instead of filling its box.
    let beam = if class == "StructuralFraming" {
        crate::partition_schema_mvp::beam_axis_from_fields(&decoded.fields).and_then(
            |(start, end)| {
                let bbox = [
                    x - width / 2.0,
                    y - depth / 2.0,
                    z,
                    x + width / 2.0,
                    y + depth / 2.0,
                    z + height,
                ];
                crate::partition_beam_axes::beam_body(bbox, start, end)
            },
        )
    } else {
        None
    };
    // RE-131: a pipe is the cylinder its connector entries and its record
    // box make, drawn as one; a pipe they do not make one for keeps its box.
    let pipe = if class == "Pipe" {
        crate::partition_schema_mvp::pipe_body_from_fields(&decoded.fields)
    } else {
        None
    };
    // RE-103: a beam whose type is an I section, drawn as it where its body
    // along its line holds the section.
    let i_beam = beam.and_then(|beam| {
        let section = crate::partition_schema_mvp::beam_i_section_from_fields(&decoded.fields)?;
        let (start, end) = crate::partition_schema_mvp::beam_axis_from_fields(&decoded.fields)?;
        let bbox = [
            x - width / 2.0,
            y - depth / 2.0,
            z,
            x + width / 2.0,
            y + depth / 2.0,
            z + height,
        ];
        crate::partition_beam_axes::i_section_body(beam, &section, bbox, start, end)
            .map(|body| (body, section))
    });
    // RE-104: an upright structural column whose type is an I section,
    // turned to its X axis where that fills its record box.
    let i_column = if class == "StructuralColumn" {
        column_i_section(&decoded.fields, width, depth)
    } else {
        None
    };
    // RE-52: a straight run with separate treads and risers is drawn as
    // them.
    let stair_run = if class == "StairsRun" {
        crate::partition_schema_mvp::stair_run_body_from_fields(&decoded.fields)
    } else {
        None
    };
    // RE-54: a wall whose centreline and type thickness are known is that
    // thickness either side of the line. It replaces the box only where the
    // two differ, so a wall the box already drew right is unchanged.
    let wall_axis = if class == "Wall" {
        crate::partition_schema_mvp::wall_axis_from_fields(&decoded.fields)
    } else {
        None
    };
    // RE-86: a tapered wall is its type's thickness about its line at its
    // top and wider towards its exterior below. Without its exterior side
    // it is not drawn from its line at all.
    let wall_taper = wall_axis
        .and_then(|_| crate::partition_schema_mvp::wall_taper_from_fields(&decoded.fields));
    let wall_axis = wall_axis.filter(|_| {
        wall_taper.is_some() || !crate::partition_schema_mvp::wall_is_tapered(&decoded.fields)
    });
    // RE-71: a wall whose layered butt joins end each layer on its own line
    // is the staircase they make.
    let layer_ends = wall_axis
        .filter(|_| wall_taper.is_none())
        .and_then(|_| crate::partition_schema_mvp::wall_layer_ends_from_fields(&decoded.fields));
    let wall_centreline = wall_axis
        .and_then(|axis| match wall_taper {
            Some(taper) => tapered_wall_body(axis, taper, [x, y], width, depth, height),
            None => wall_centreline_body(axis, [x, y], width, depth),
        })
        .filter(|body| {
            layer_ends.is_some()
                || wall_taper.is_some()
                || body.rotation.is_some()
                || (body.centre[0] - x).abs() > 1e-9
                || (body.centre[1] - y).abs() > 1e-9
                || (body.width_feet - width).abs() > 1e-9
                || (body.depth_feet - depth).abs() > 1e-9
        });
    let wall_profile = match (wall_centreline, wall_axis, layer_ends.as_ref()) {
        (Some(body), Some(axis), Some(ends)) => layered_wall_profile(&body, &axis, ends),
        _ => None,
    };
    // The sketched plan profile, when the element's `OST_SketchLines`
    // records closed one (#31, RE-25). It is recovered in project
    // plan coordinates and the body is placed at the record's plan
    // centre, so the profile is expressed relative to that centre.
    let profile = crate::element_record_plan_profiles::plan_profile_from_fields(&decoded.fields);
    let relative = |outer: &[(f64, f64)], inner: &[Vec<(f64, f64)>]| {
        let points: Vec<(f64, f64)> = outer.iter().map(|(px, py)| (px - x, py - y)).collect();
        let voids: Vec<Vec<(f64, f64)>> = inner
            .iter()
            .map(|ring| ring.iter().map(|(px, py)| (px - x, py - y)).collect())
            .collect();
        if voids.is_empty() {
            entities::ProfileDef::ArbitraryClosed { points }
        } else {
            entities::ProfileDef::ArbitraryWithVoids { points, voids }
        }
    };
    // RE-75: a curved wall is the ring sector its arc and its type's
    // thickness make.
    let wall_arc = if class == "Wall" {
        crate::partition_schema_mvp::wall_arc_from_fields(&decoded.fields)
            .as_ref()
            .and_then(wall_arc_outline)
    } else {
        None
    };
    let profile_override = profile
        .as_ref()
        .map(|profile| relative(&profile.outer_xy, &profile.inner_xy))
        .or_else(|| wall_arc.as_ref().map(|outline| relative(outline, &[])));
    // RE-87: a family instance turned off the model's axes is the rectangle
    // at its angle that its record box bounds.
    // RE-106: a curtain mullion or panel turned or tilted off the model's
    // axes, as the box along its own axes whose world box is its record box.
    let oriented = crate::partition_schema_mvp::curtain_axes_from_fields(&decoded.fields)
        .and_then(|axes| oriented_box(axes, [width, depth, height]));
    let turned = if TURNED_BOX_EXCLUDED_CLASSES.contains(&class)
        || profile_override.is_some()
        || stair_run.is_some()
        || beam.is_some()
        || matches!(type_profile, (Some(_), Some(_)))
    {
        None
    } else {
        crate::partition_schema_mvp::instance_x_axis_from_fields(&decoded.fields)
            .and_then(|axis| turned_box(axis, width, depth))
    };
    // RE-56: a shed roof whose slope reproduces its record box rises along
    // it.
    let roof_slope = if class == "Roof" {
        profile.as_ref().and_then(|profile| {
            let slope = crate::partition_schema_mvp::roof_slope_from_fields(&decoded.fields)?;
            let outer: Vec<(f64, f64)> = profile
                .outer_xy
                .iter()
                .map(|(px, py)| (px - x, py - y))
                .collect();
            let inner: Vec<Vec<(f64, f64)>> = profile
                .inner_xy
                .iter()
                .map(|ring| ring.iter().map(|(px, py)| (px - x, py - y)).collect())
                .collect();
            roof_slope_solid(slope, [x, y], &outer, &inner, height)
        })
    } else {
        None
    };
    // #331: the further pieces of a sketch of separate loops, each its own
    // body at the element's placement, as Revit exports one element per
    // piece.
    let piece_bodies: Vec<Extrusion> = profile
        .as_ref()
        .map(|profile| {
            profile
                .pieces
                .iter()
                .map(|piece| Extrusion {
                    width_feet: width,
                    depth_feet: depth,
                    height_feet: height,
                    profile_override: Some(relative(&piece.outer_xy, &piece.inner_xy)),
                })
                .collect()
        })
        .unwrap_or_default();
    // RE-151: a floor's tagged void is also an opening voiding it, as Revit's
    // export writes it, the void's outline through the floor's thickness.
    let void_openings: Vec<VoidBody> = profile
        .as_ref()
        .filter(|_| class == "Floor")
        .map(|profile| {
            profile
                .void_openings
                .iter()
                .map(|void| {
                    let outline = void.outline_xy.iter().map(|(px, py)| (px - x, py - y));
                    (void.piece, void.tag, outline.collect(), height)
                })
                .collect()
        })
        .unwrap_or_default();
    // The family/type symbol's section, when the instance joined to
    // one and the section agrees with the instance envelope (#215,
    // RE-26). The rectangle it gives is the same shape the envelope
    // gave; what changes is that the type is now the authority for it.
    let type_section = match type_profile {
        (Some(w), Some(d)) if w > 0.0 && d > 0.0 => Some((w, d)),
        _ => None,
    };
    // A column that its joined walls cut emits the **cut** rectangle,
    // not the family section: the section is still the type's, and is
    // still reported below, but it is no longer the shape of the
    // solid (#239, RE-29 §5).
    let (width, depth) = if column_body_source.is_some() {
        (width, depth)
    } else {
        type_section.unwrap_or((width, depth))
    };
    let mut properties = vec![
        // `BodySource` stays the record box unless a wall resolved its
        // joins: the placement, the plan envelope and the extrusion
        // depth are otherwise all still read from the box. What the
        // sketch lines replace is the plan *profile*, which
        // `ProfileResolved` / `ProfileSource` report separately.
        Property {
            name: "BodySource".into(),
            value: PropertyValue::Text(
                wall_profile
                    .as_ref()
                    .map(|_| WALL_LAYER_JOIN_BODY_SOURCE.into())
                    .or_else(|| {
                        wall_centreline.map(|_| {
                            if wall_taper.is_some() {
                                WALL_TAPERED_BODY_SOURCE.into()
                            } else {
                                WALL_CENTRELINE_BODY_SOURCE.into()
                            }
                        })
                    })
                    .or_else(|| wall_arc.as_ref().map(|_| WALL_ARC_BODY_SOURCE.into()))
                    .or_else(|| wall_body_source.clone())
                    .or_else(|| column_body_source.clone())
                    .or_else(|| beam_body_source.clone())
                    .or_else(|| beam.map(|_| BEAM_AXIS_BODY_SOURCE.into()))
                    .or_else(|| pipe.map(|_| PIPE_AXIS_BODY_SOURCE.into()))
                    .or_else(|| stair_run.as_ref().map(|_| STAIR_RUN_BODY_SOURCE.into()))
                    .or_else(|| roof_slope.as_ref().map(|_| ROOF_SLOPE_BODY_SOURCE.into()))
                    .or_else(|| i_column.map(|_| COLUMN_I_SECTION_BODY_SOURCE.into()))
                    .or_else(|| turned.map(|_| INSTANCE_TURNED_BODY_SOURCE.into()))
                    .or_else(|| oriented.map(|_| INSTANCE_ORIENTED_BODY_SOURCE.into()))
                    .unwrap_or_else(|| RECORD_BBOX_BODY_SOURCE.into()),
            ),
        },
        Property {
            name: "ProfileResolved".into(),
            value: PropertyValue::Boolean(
                profile.is_some()
                    || type_section.is_some()
                    || beam.is_some()
                    || pipe.is_some()
                    || stair_run.is_some()
                    || wall_centreline.is_some()
                    || wall_arc.is_some()
                    || i_column.is_some(),
            ),
        },
        Property {
            name: "LevelBindResolved".into(),
            value: PropertyValue::Boolean(level_id.is_some()),
        },
        Property {
            name: "BoundingBoxHeight".into(),
            value: PropertyValue::LengthFeet(height),
        },
    ];
    // #219 / RE-27: the host Level the record's counted reference list
    // names. `ifc::apply_record_level_reference_storeys` reads it back
    // off the emitted element, so the two sides share one carrier.
    if let Some(id) = level_id {
        properties.push(Property {
            name: crate::element_record_level_refs::LEVEL_ELEMENT_ID_PROPERTY.into(),
            value: PropertyValue::Integer(i64::from(id)),
        });
        properties.push(Property {
            name: "LevelBindSource".into(),
            value: PropertyValue::Text(level_bind_source.unwrap_or_else(|| {
                crate::element_record_level_refs::LEVEL_REFERENCE_SOURCE.into()
            })),
        });
    }
    if let Some(profile) = profile.as_ref() {
        properties.push(Property {
            name: "ProfileSource".into(),
            value: PropertyValue::Text(
                decoded
                    .fields
                    .iter()
                    .find_map(|(name, value)| {
                        match value {
                        InstanceField::String(source)
                            if name
                                == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD =>
                        {
                            Some(source.clone())
                        }
                        _ => None,
                    }
                    })
                    .unwrap_or_else(|| {
                        crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE.into()
                    }),
            ),
        });
        properties.push(Property {
            name: "ProfileVertexCount".into(),
            value: PropertyValue::Integer(profile.vertex_count() as i64),
        });
        properties.push(Property {
            name: "ProfileVoidCount".into(),
            value: PropertyValue::Integer(profile.inner_xy.len() as i64),
        });
        if !profile.pieces.is_empty() {
            properties.push(Property {
                name: "ProfilePieceCount".into(),
                value: PropertyValue::Integer(1 + profile.pieces.len() as i64),
            });
        }
    }
    if let (Some((section_width, section_depth)), Some(symbol)) = (type_section, type_symbol_id) {
        properties.push(Property {
            name: "ProfileSource".into(),
            value: PropertyValue::Text(
                column_body_source
                    .clone()
                    .unwrap_or_else(|| crate::partition_schema_mvp::TYPE_PROFILE_SOURCE.into()),
            ),
        });
        properties.push(Property {
            name: "TypeSymbolElementId".into(),
            value: PropertyValue::Integer(i64::from(symbol)),
        });
        properties.push(Property {
            name: "TypeSectionWidth".into(),
            value: PropertyValue::LengthFeet(section_width),
        });
        properties.push(Property {
            name: "TypeSectionDepth".into(),
            value: PropertyValue::LengthFeet(section_depth),
        });
    }
    // RE-89: a turned instance reports its origin, which a window's opening
    // in a tapered wall is centred on.
    if turned.is_some() {
        if let Some(origin) =
            crate::partition_schema_mvp::instance_origin_from_fields(&decoded.fields)
        {
            for (name, value) in INSTANCE_ORIGIN_PROPERTIES.iter().zip(origin) {
                properties.push(Property {
                    name: (*name).into(),
                    value: PropertyValue::LengthFeet(value),
                });
            }
        }
    }
    // RE-93: a window reports the opening its type and transform give it.
    if let Some(opening) = crate::partition_schema_mvp::filler_opening_from_fields(&decoded.fields)
    {
        let values = [
            PropertyValue::LengthFeet(opening.centre[0]),
            PropertyValue::LengthFeet(opening.centre[1]),
            PropertyValue::Real(opening.axis[0]),
            PropertyValue::Real(opening.axis[1]),
            PropertyValue::LengthFeet(opening.base_feet),
            PropertyValue::PositiveLengthFeet(opening.width_feet),
            PropertyValue::PositiveLengthFeet(opening.height_feet),
        ];
        for (name, value) in FILLER_OPENING_PROPERTIES.iter().zip(values) {
            properties.push(Property {
                name: (*name).into(),
                value,
            });
        }
    }
    // A wall drawn from its centreline reports its type's thickness
    // (RE-54); one whose joins resolved reports the trim it took and, unless
    // its centreline redrew it, the thickness it read off the box's thin
    // axis (RE-26).
    let thickness = wall_centreline
        .map(|body| (body.thickness_feet, WALL_TYPE_THICKNESS_SOURCE))
        .or_else(|| {
            wall_thickness.map(|t| (t, crate::element_record_wall_joins::WALL_THICKNESS_SOURCE))
        });
    if let Some((thickness, source)) = thickness.filter(|_| {
        wall_centreline.is_some() || (wall_trim_start.is_some() && wall_trim_end.is_some())
    }) {
        properties.push(Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(true),
        });
        properties.push(Property {
            name: "Thickness".into(),
            value: PropertyValue::LengthFeet(thickness),
        });
        properties.push(Property {
            name: "ThicknessSource".into(),
            value: PropertyValue::Text(source.into()),
        });
    }
    // RE-70, RE-73: an end a join decides names the wall it butt-joins and
    // whether it runs through, and, where the body was drawn to it, how
    // far past the line it reaches (negative where it stops short).
    let reaches = wall_centreline.map_or([None, None], |body| body.join_reach_feet);
    for (slot, end) in ["Start", "End"].into_iter().enumerate() {
        let partner = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. }
                if name == crate::partition_schema_mvp::WALL_JOIN_PARTNER_FIELDS[slot] =>
            {
                Some(*id)
            }
            _ => None,
        });
        let through = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Bool(through)
                if name == crate::partition_schema_mvp::WALL_JOIN_THROUGH_FIELDS[slot] =>
            {
                Some(*through)
            }
            _ => None,
        });
        if let (Some(partner), Some(through)) = (partner, through) {
            properties.push(Property {
                name: format!("Join{end}WallElementId"),
                value: PropertyValue::Integer(i64::from(partner)),
            });
            properties.push(Property {
                name: format!("Join{end}RunsThrough"),
                value: PropertyValue::Boolean(through),
            });
        }
        if let Some(reach) = reaches[slot] {
            properties.push(Property {
                name: format!("JoinReach{end}"),
                value: PropertyValue::LengthFeet(reach),
            });
        }
    }
    if let (Some(_), Some(start), Some(end)) = (wall_thickness, wall_trim_start, wall_trim_end) {
        properties.push(Property {
            name: "JoinTrimStart".into(),
            value: PropertyValue::LengthFeet(start),
        });
        properties.push(Property {
            name: "JoinTrimEnd".into(),
            value: PropertyValue::LengthFeet(end),
        });
    }
    // A room's number and name, when its own parameter block carried
    // them (#90, RE-29, RE-117). The writer puts the number in
    // `IfcSpace.Name` and the name in `IfcSpace.LongName`, as Revit's own
    // exporter does; the ElementId, which an `IfcSpace` has no `Tag` for,
    // is its own property.
    if class == "Room" {
        if let Some(id) = decoded.id {
            properties.push(Property {
                name: ROOM_ELEMENT_ID_PROPERTY.into(),
                value: PropertyValue::Integer(i64::from(id)),
            });
        }
    }
    if let Some(number) = room_number {
        properties.push(Property {
            name: ROOM_NUMBER_PROPERTY.into(),
            value: PropertyValue::Text(number),
        });
    }
    if let Some(name) = room_name {
        properties.push(Property {
            name: ROOM_NAME_PROPERTY.into(),
            value: PropertyValue::Text(name),
        });
    }
    // A column whose joined walls cut it reports how many took part
    // (#239, RE-29 section 5). The section above is the family's; this
    // is what is left of the prism after the walls are subtracted.
    if let Some(walls) = column_cut_walls {
        properties.push(Property {
            name: "JoinCutWallCount".into(),
            value: PropertyValue::Integer(walls),
        });
    }
    // RE-49: the beam's length along its location line and the section the
    // record box leaves across it.
    if let Some(beam) = beam {
        for (name, value) in [
            ("AxisLength", beam.length_feet),
            ("SectionWidth", beam.width_feet),
            ("SectionDepth", beam.depth_feet),
        ] {
            properties.push(Property {
                name: name.into(),
                value: PropertyValue::LengthFeet(value),
            });
        }
    }
    if let Some(section) = i_beam
        .map(|(_, section)| section)
        .or(i_column.map(|(_, section)| section))
    {
        properties.push(Property {
            name: "SectionShape".into(),
            value: PropertyValue::Text("I".into()),
        });
        for (name, value) in [
            ("SectionWebThickness", section.web_feet),
            ("SectionFlangeThickness", section.flange_feet),
        ] {
            properties.push(Property {
                name: name.into(),
                value: PropertyValue::LengthFeet(value),
            });
        }
    }
    if let Some(stream) = source_stream {
        properties.push(Property {
            name: "SourceStream".into(),
            value: PropertyValue::Text(stream),
        });
    }
    // RE-76: the class the element's record names in the file's own schema.
    if let Some(class) = revit_class {
        properties.push(Property {
            name: "RevitClass".into(),
            value: PropertyValue::Text(class),
        });
    }
    // RE-77: the type's text parameters, under the names Revit shows.
    for (name, value) in type_parameters {
        properties.push(Property {
            name,
            value: PropertyValue::Text(value),
        });
    }
    // RE-38: the family and type the partition name entries give this
    // element, the two halves of the `Family:Type` Revit's export uses. A
    // system-family type (#322) has a name but no family in the file, so it
    // writes only `TypeName`.
    if let (Some(family), Some(_)) = (&family_name, &type_name) {
        properties.push(Property {
            name: FAMILY_NAME_PROPERTY.into(),
            value: PropertyValue::Text(family.clone()),
        });
        // RE-63: a system family's name is derived, not read.
        if let Some(source) = family_name_source {
            properties.push(Property {
                name: "FamilyNameSource".into(),
                value: PropertyValue::Text(source),
            });
        }
    }
    if let Some(type_name) = type_name {
        properties.push(Property {
            name: TYPE_NAME_PROPERTY.into(),
            value: PropertyValue::Text(type_name),
        });
    }
    // For a plate the recorded vertical extent *is* the element's
    // thickness, which closes the `floor_slab_extrusion_thickness`
    // gap the plan-loop path had to leave open (#31, #212). Measured
    // against the reference export's `IfcExtrudedAreaSolid.Depth`:
    // 79 of 80 slabs agree exactly, and the 80th
    // (`Floor:Basement Slab`, 22756) is exported as two stacked
    // solids of 0.3333 ft and 1.1667 ft whose depths sum to the
    // recorded 1.5 ft. It is still the box extent, not a modelled
    // layer set. The plan profile is a separate question, answered
    // above: the sketch boundary when the element's
    // `OST_SketchLines` records close one (#31, RE-25), the box
    // rectangle when they do not.
    if SLAB_THICKNESS_CLASSES.contains(&class) {
        properties.push(Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(true),
        });
        properties.push(Property {
            name: "Thickness".into(),
            value: PropertyValue::LengthFeet(height),
        });
        properties.push(Property {
            name: "ThicknessSource".into(),
            value: PropertyValue::Text(RECORD_BBOX_THICKNESS_SOURCE.into()),
        });
    }
    let record_body = Extrusion {
        width_feet: width,
        depth_feet: depth,
        height_feet: height,
        profile_override,
    };
    let tapered = match (wall_centreline, wall_axis, wall_taper) {
        (Some(body), Some(axis), Some(taper)) => tapered_wall_solid(&body, &axis, taper, height),
        _ => None,
    };
    let (location, rotation, body, solid) = match (beam, wall_centreline) {
        (_, Some(wall)) if tapered.is_some() => (
            [wall.centre[0], wall.centre[1], z],
            wall_axis.map(|axis| (axis.end[1] - axis.start[1]).atan2(axis.end[0] - axis.start[0])),
            Extrusion::rectangle(wall.width_feet, wall.depth_feet, height),
            tapered,
        ),
        (_, Some(wall)) => (
            [wall.centre[0], wall.centre[1], z],
            wall.rotation,
            Extrusion {
                profile_override: wall_profile
                    .clone()
                    .map(|points| entities::ProfileDef::ArbitraryClosed { points }),
                ..Extrusion::rectangle(wall.width_feet, wall.depth_feet, height)
            },
            None,
        ),
        (beam, None) => {
            let (rotation, body, solid) = match beam {
                // A level beam is its section's plan rectangle along the line,
                // extruded through its depth from the record box's base.
                Some(_) if i_beam.is_some() => (
                    None,
                    record_body,
                    i_beam.map(|(body, section)| beam_i_solid(&body, &section, [x, y, z])),
                ),
                Some(beam) if beam.is_horizontal() => (
                    Some(beam.plan_angle_radians()),
                    Extrusion::rectangle(beam.length_feet, beam.width_feet, beam.depth_feet),
                    None,
                ),
                Some(beam) => (None, record_body, Some(beam_swept_solid(&beam, [x, y, z]))),
                None if pipe.is_some() => (
                    None,
                    record_body,
                    pipe.map(|body| pipe_cylinder_solid(&body, [x, y, z])),
                ),
                None if oriented.is_some() => (
                    None,
                    record_body,
                    oriented.map(|(axes, half)| oriented_box_solid(axes, half, height)),
                ),
                None if i_column.is_some() => {
                    let (angle, section) = i_column.expect("checked above");
                    (
                        Some(angle),
                        Extrusion {
                            profile_override: Some(entities::ProfileDef::IShape {
                                overall_width_feet: section.width_feet,
                                overall_depth_feet: section.depth_feet,
                                web_thickness_feet: section.web_feet,
                                flange_thickness_feet: section.flange_feet,
                            }),
                            ..Extrusion::rectangle(section.width_feet, section.depth_feet, height)
                        },
                        None,
                    )
                }
                None => match (turned, roof_slope) {
                    (Some((angle, along, across)), None) => (
                        Some(angle),
                        Extrusion::rectangle(along, across, height),
                        None,
                    ),
                    (_, roof_slope) => {
                        let solid = stair_run
                            .as_ref()
                            .map(|run| stair_run_solid(run, [x, y, z]))
                            .or(roof_slope);
                        (None, record_body, solid)
                    }
                },
            };
            ([x, y, z], rotation, body, solid)
        }
    };
    Some(RecordGeometry {
        location,
        rotation,
        body,
        solid,
        properties: PropertySet {
            name: ELEMENT_RECORD_PROPERTY_SET.into(),
            properties,
        },
        pieces: piece_bodies,
        void_openings,
    })
}

/// Document a recovered floor boundary without inventing slab thickness.
fn floor_boundary_annotation_from_decoded(decoded: &DecodedElement) -> Option<PropertySet> {
    let _floor = Floor::from_decoded(decoded);
    let boundary = recover_floor_boundary(decoded).ok()?;
    if boundary.vertices_xy.len() < 3 {
        return None;
    }
    Some(PropertySet {
        name: "RvtFloorGeometry".into(),
        properties: vec![
            Property {
                name: "BoundaryVertexCount".into(),
                value: PropertyValue::Integer(boundary.vertices_xy.len() as i64),
            },
            Property {
                name: "BoundaryClosed".into(),
                value: PropertyValue::Boolean(boundary.closed),
            },
            Property {
                name: "BoundarySource".into(),
                value: PropertyValue::Text(format!("{:?}", boundary.source)),
            },
            Property {
                name: "ThicknessResolved".into(),
                value: PropertyValue::Boolean(false),
            },
        ],
    })
}
