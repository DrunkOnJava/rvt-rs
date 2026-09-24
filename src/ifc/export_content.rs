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
pub const ROOM_NUMBER_PROPERTY: &str = "RoomNumber";

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
    let mut pending_parts: Vec<(usize, u32)> = Vec::new();
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
                layers.system_family =
                    crate::partition_schema_mvp::system_family(&decoded.class, true)
                        .map(String::from);
                out.element_layers.insert(id, layers);
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
        let name = match (own_name, decoded.id, family_and_type(&decoded)) {
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
            }) = element_record_geometry
            {
                location_feet = Some(location);
                rotation_radians = rotation;
                solid_shape = solid;
                piece_bodies = pieces;
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

        // Floor/Room → storey via Level ElementId only when both sides
        // carry ids that match. Partition MVP Levels are id-less today,
        // so this stays None (honest Unassigned) on available corpora.
        let storey_index = level_bind.storey_index_for(&decoded);
        if storey_index.is_some() {
            out.level_elementid_binds += 1;
        }

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
        // RE-47: a stair's or flight's riser and tread dimensions, in the
        // standard set Revit's export writes them in.
        if let Some(set) = stair_property_set(&decoded) {
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

    if policy.include_geometry {
        for (entity_index, host_id) in pending_hosts {
            if let Some(&host_index) = out.id_to_entity.get(&host_id) {
                if let Some(entities::IfcEntity::BuildingElement {
                    host_element_index, ..
                }) = entities.get_mut(entity_index)
                {
                    *host_element_index = Some(host_index);
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

/// `BodySource` of a wall drawn from its centreline whose layers end where
/// its layered butt joins put them (RE-71).
pub const WALL_LAYER_JOIN_BODY_SOURCE: &str = "partition_wall_centreline_layer_joins";

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
        let low = start
            .as_ref()
            .map_or(middle - run / 2.0, |reach| -reach[index]);
        let high = end
            .as_ref()
            .map_or(middle + run / 2.0, |reach| length + reach[index]);
        if high - low <= WALL_AXIS_BOX_TOLERANCE_FEET {
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
            outline.push((*low, *upper));
        }
        outline.push((*high, *upper));
        outline.push((*high, *lower));
    }
    for (low, _, upper, lower) in bands.iter().rev() {
        outline.push((*low, *lower));
        outline.push((*low, *upper));
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
    let mut wall_body_source = None;
    let mut wall_thickness = None;
    let mut wall_trim_start = None;
    let mut wall_trim_end = None;
    let mut column_body_source = None;
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
    // RE-71: a wall whose layered butt joins end each layer on its own line
    // is the staircase they make.
    let layer_ends = wall_axis
        .and_then(|_| crate::partition_schema_mvp::wall_layer_ends_from_fields(&decoded.fields));
    let wall_centreline = wall_axis
        .and_then(|axis| wall_centreline_body(axis, [x, y], width, depth))
        .filter(|body| {
            layer_ends.is_some()
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
    let profile_override = profile
        .as_ref()
        .map(|profile| relative(&profile.outer_xy, &profile.inner_xy));
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
                    .or_else(|| wall_centreline.map(|_| WALL_CENTRELINE_BODY_SOURCE.into()))
                    .or_else(|| wall_body_source.clone())
                    .or_else(|| column_body_source.clone())
                    .or_else(|| beam.map(|_| BEAM_AXIS_BODY_SOURCE.into()))
                    .or_else(|| stair_run.as_ref().map(|_| STAIR_RUN_BODY_SOURCE.into()))
                    .or_else(|| roof_slope.as_ref().map(|_| ROOF_SLOPE_BODY_SOURCE.into()))
                    .unwrap_or_else(|| "partition_element_record_bbox".into()),
            ),
        },
        Property {
            name: "ProfileResolved".into(),
            value: PropertyValue::Boolean(
                profile.is_some()
                    || type_section.is_some()
                    || beam.is_some()
                    || stair_run.is_some()
                    || wall_centreline.is_some(),
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
                crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE.into(),
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
    // them (#90, RE-29). `RoomName` is what the writer puts in
    // `IfcSpace.LongName`, which is the slot Revit's own exporter uses
    // for it; the element `Name` keeps the `Room-<ElementId>` identity
    // every other class uses, because `IfcSpace` declares no `Tag`.
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
    if let Some(stream) = source_stream {
        properties.push(Property {
            name: "SourceStream".into(),
            value: PropertyValue::Text(stream),
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
    let (location, rotation, body, solid) = match (beam, wall_centreline) {
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
                Some(beam) if beam.is_horizontal() => (
                    Some(beam.plan_angle_radians()),
                    Extrusion::rectangle(beam.length_feet, beam.width_feet, beam.depth_feet),
                    None,
                ),
                Some(beam) => (None, record_body, Some(beam_swept_solid(&beam, [x, y, z]))),
                None => {
                    let solid = stair_run
                        .as_ref()
                        .map(|run| stair_run_solid(run, [x, y, z]))
                        .or(roof_slope);
                    (None, record_body, solid)
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walker::{DecodedElement, InstanceField};

    fn decoded(class: &str, id: Option<u32>) -> DecodedElement {
        DecodedElement {
            class: class.into(),
            id,
            fields: Vec::new(),
            byte_range: 0..0,
            provenance: Default::default(),
        }
    }

    /// #323: a stair carries no body of its own and aggregates the run
    /// that names it.
    /// #331: a floor sketched as two separate rectangles exports as two
    /// slabs with the element's `Tag`, the second named `…:2`, each with
    /// its own piece as the profile.
    #[test]
    fn a_floor_of_two_separate_pieces_exports_one_slab_per_piece() {
        use crate::partition_element_records::{
            CONTAINER_NONE, OST_FLOORS, PLACEMENT_KIND_INSTANCE, PartitionElementRecord,
        };
        let record = PartitionElementRecord {
            stream: "Partitions/68".into(),
            offset: 0,
            element_id: 1402063,
            flags: 0x0141,
            builtin_category: OST_FLOORS,
            container: CONTAINER_NONE,
            placement_kind: PLACEMENT_KIND_INSTANCE,
            bbox_feet: [-11.15, -0.97, 58.0, 11.15, 0.97, 58.42],
            preceding_reference: None,
            owner_reference: None,
            references: Vec::new(),
            id_from_enclosing_record: true,
            design_option: None,
        };
        let levels = std::collections::BTreeSet::new();
        let mut elements =
            crate::partition_schema_mvp::instances_from_records(vec![record], "Floor", &levels);
        let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
            vec![
                [x0, y0, x1, y0],
                [x1, y0, x1, y1],
                [x0, y1, x1, y1],
                [x0, y0, x0, y1],
            ]
        };
        let mut segments = rect(9.06, -0.97, 11.15, 0.97);
        segments.extend(rect(-11.15, -0.97, -9.06, 0.97));
        let profile = crate::element_record_plan_profiles::plan_profile_from_segments(&segments)
            .expect("closes");
        elements[0].fields.extend(profile.fields());
        let mut entities = Vec::new();
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements(elements.into_iter(), &mut entities, &mut storeys, policy);
        let slabs: Vec<(String, Option<String>, bool)> = entities
            .iter()
            .filter_map(|entity| match entity {
                entities::IfcEntity::BuildingElement {
                    name,
                    type_guid,
                    extrusion,
                    ..
                } => Some((
                    name.clone(),
                    type_guid.clone(),
                    matches!(
                        extrusion.as_ref().and_then(|e| e.profile_override.as_ref()),
                        Some(entities::ProfileDef::ArbitraryClosed { .. })
                    ),
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            slabs,
            vec![
                ("Floor-1402063".into(), Some("1402063".into()), true),
                ("Floor-1402063:2".into(), Some("1402063".into()), true),
            ]
        );
    }

    #[test]
    fn a_stair_aggregates_its_run_and_has_no_body() {
        use crate::partition_element_records::{
            CONTAINER_NONE, OST_STAIRS, OST_STAIRS_RUNS, PLACEMENT_KIND_INSTANCE,
            PartitionElementRecord,
        };
        let record = |element_id: u32, category: i64| PartitionElementRecord {
            stream: "Partitions/68".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0141,
            builtin_category: category,
            container: CONTAINER_NONE,
            placement_kind: PLACEMENT_KIND_INSTANCE,
            bbox_feet: [0.0, 0.0, 0.0, 10.0, 4.0, 9.0],
            preceding_reference: None,
            owner_reference: None,
            references: Vec::new(),
            id_from_enclosing_record: true,
            design_option: None,
        };
        let levels = std::collections::BTreeSet::new();
        let mut elements = crate::partition_schema_mvp::instances_from_records(
            vec![record(620883, OST_STAIRS)],
            "Stair",
            &levels,
        );
        let mut runs = crate::partition_schema_mvp::instances_from_records(
            vec![record(621141, OST_STAIRS_RUNS)],
            "StairsRun",
            &levels,
        );
        runs[0].fields.push((
            crate::partition_schema_mvp::AGGREGATE_WHOLE_FIELD.into(),
            InstanceField::ElementId { tag: 0, id: 620883 },
        ));
        elements.extend(runs);
        let mut entities = Vec::new();
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements(elements.into_iter(), &mut entities, &mut storeys, policy);
        let index_of = |tag: &str| {
            entities.iter().position(|entity| {
                matches!(entity, entities::IfcEntity::BuildingElement { type_guid, .. }
                    if type_guid.as_deref() == Some(tag))
            })
        };
        let stair = index_of("620883").expect("stair");
        let run = index_of("621141").expect("run");
        let body = |index: usize| match &entities[index] {
            entities::IfcEntity::BuildingElement { extrusion, .. } => extrusion.is_some(),
            _ => unreachable!(),
        };
        assert!(!body(stair), "the stair has no body");
        assert!(body(run), "the run keeps its record box");
        let aggregates: Vec<(usize, Vec<usize>)> = entities
            .iter()
            .filter_map(|entity| match entity {
                entities::IfcEntity::Aggregate { whole, parts } => Some((*whole, parts.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(aggregates, vec![(stair, vec![run])]);
    }

    /// RE-47: a stair's and a flight's riser and tread dimensions become
    /// `Pset_StairCommon` and `Pset_StairFlightCommon`, which the viewer
    /// panel lists beside the element's own set.
    #[test]
    fn stair_dimensions_become_the_standard_stair_sets() {
        use crate::partition_element_records::{
            CONTAINER_NONE, OST_STAIRS, OST_STAIRS_RUNS, PLACEMENT_KIND_INSTANCE,
            PartitionElementRecord,
        };
        use crate::partition_schema_mvp as mvp;
        let record = |element_id: u32, category: i64| PartitionElementRecord {
            stream: "Partitions/68".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0141,
            builtin_category: category,
            container: CONTAINER_NONE,
            placement_kind: PLACEMENT_KIND_INSTANCE,
            bbox_feet: [0.0, 0.0, 0.0, 10.0, 4.0, 9.0],
            preceding_reference: None,
            owner_reference: None,
            references: Vec::new(),
            id_from_enclosing_record: true,
            design_option: None,
        };
        let dimensions = |risers: i64| {
            vec![
                (
                    mvp::STAIR_RISER_COUNT_FIELD.to_string(),
                    InstanceField::Integer {
                        value: risers,
                        signed: false,
                        size: 4,
                    },
                ),
                (
                    mvp::STAIR_RISER_HEIGHT_FIELD.to_string(),
                    InstanceField::Float {
                        value: 11.0 / 19.0,
                        size: 8,
                    },
                ),
                (
                    mvp::STAIR_TREAD_DEPTH_FIELD.to_string(),
                    InstanceField::Float {
                        value: 11.0 / 12.0,
                        size: 8,
                    },
                ),
            ]
        };
        let levels = std::collections::BTreeSet::new();
        let mut elements =
            mvp::instances_from_records(vec![record(620883, OST_STAIRS)], "Stair", &levels);
        elements[0].fields.extend(dimensions(19));
        let mut runs = mvp::instances_from_records(
            vec![record(621141, OST_STAIRS_RUNS)],
            "StairsRun",
            &levels,
        );
        runs[0].fields.push((
            mvp::AGGREGATE_WHOLE_FIELD.into(),
            InstanceField::ElementId { tag: 0, id: 620883 },
        ));
        runs[0].fields.extend(dimensions(10));
        elements.extend(runs);
        let mut entities = Vec::new();
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements(elements.into_iter(), &mut entities, &mut storeys, policy);
        let sets: Vec<(usize, String, Vec<String>)> = entities
            .iter()
            .filter_map(|entity| match entity {
                entities::IfcEntity::ElementPropertySet { element, set } => Some((
                    *element,
                    set.name.clone(),
                    set.properties.iter().map(|p| p.value.to_step()).collect(),
                )),
                _ => None,
            })
            .collect();
        let index_of = |tag: &str| {
            entities.iter().position(|entity| {
                matches!(entity, entities::IfcEntity::BuildingElement { type_guid, .. }
                    if type_guid.as_deref() == Some(tag))
            })
        };
        let stair = index_of("620883").expect("stair");
        let run = index_of("621141").expect("run");
        assert_eq!(
            sets,
            vec![
                (
                    stair,
                    "Pset_StairCommon".to_string(),
                    vec![
                        "IFCCOUNTMEASURE(19)".to_string(),
                        "IFCPOSITIVELENGTHMEASURE(0.176463)".to_string(),
                        "IFCPOSITIVELENGTHMEASURE(0.279400)".to_string(),
                    ]
                ),
                (
                    run,
                    "Pset_StairFlightCommon".to_string(),
                    vec![
                        "IFCCOUNTMEASURE(10)".to_string(),
                        "IFCPOSITIVELENGTHMEASURE(0.176463)".to_string(),
                        "IFCPOSITIVELENGTHMEASURE(0.279400)".to_string(),
                    ]
                ),
            ]
        );
        let model = super::super::IfcModel {
            entities,
            ..super::super::IfcModel::default()
        };
        let panel = super::super::scene_graph::element_info_panel(&model, stair).expect("panel");
        let names: Vec<&str> = panel
            .further_property_groups
            .iter()
            .map(|group| group.name.as_str())
            .collect();
        assert_eq!(names, ["Pset_StairCommon"]);
    }

    /// A stair family placed on its own names no part and keeps its body,
    /// as Revit's export does for Snowdon 1603717; a curtain wall with a
    /// panel is carried by it (RE-46).
    #[test]
    fn a_whole_no_part_names_keeps_its_body() {
        use crate::partition_element_records::{
            CONTAINER_NONE, OST_CURTAIN_WALL_PANELS, OST_STAIRS, OST_WALLS,
            PLACEMENT_KIND_INSTANCE, PartitionElementRecord,
        };
        let record = |element_id: u32, category: i64| PartitionElementRecord {
            stream: "Partitions/68".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0141,
            builtin_category: category,
            container: CONTAINER_NONE,
            placement_kind: PLACEMENT_KIND_INSTANCE,
            bbox_feet: [0.0, 0.0, 0.0, 10.0, 4.0, 9.0],
            preceding_reference: None,
            owner_reference: None,
            references: Vec::new(),
            id_from_enclosing_record: true,
            design_option: None,
        };
        let levels = std::collections::BTreeSet::new();
        let mut elements = crate::partition_schema_mvp::instances_from_records(
            vec![record(1603717, OST_STAIRS)],
            "Stair",
            &levels,
        );
        elements.extend(crate::partition_schema_mvp::instances_from_records(
            vec![record(946544, OST_WALLS)],
            crate::partition_schema_mvp::CURTAIN_WALL_CLASS,
            &levels,
        ));
        let mut panels = crate::partition_schema_mvp::instances_from_records(
            vec![record(946600, OST_CURTAIN_WALL_PANELS)],
            "CurtainWallPanel",
            &levels,
        );
        panels[0].fields.push((
            crate::partition_schema_mvp::AGGREGATE_WHOLE_FIELD.into(),
            InstanceField::ElementId { tag: 0, id: 946544 },
        ));
        elements.extend(panels);
        let mut entities = Vec::new();
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements(elements.into_iter(), &mut entities, &mut storeys, policy);
        let element = |tag: &str| {
            entities.iter().find_map(|entity| match entity {
                entities::IfcEntity::BuildingElement {
                    type_guid,
                    ifc_type,
                    extrusion,
                    ..
                } if type_guid.as_deref() == Some(tag) => {
                    Some((ifc_type.clone(), extrusion.is_some()))
                }
                _ => None,
            })
        };
        assert_eq!(element("1603717"), Some(("IFCSTAIR".into(), true)));
        assert_eq!(element("946544"), Some(("IFCCURTAINWALL".into(), false)));
        assert_eq!(element("946600"), Some(("IFCPLATE".into(), true)));
    }

    #[test]
    fn ifc_export_override_redirects_the_entity_type() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let mut slab = decoded("Floor", Some(20953));
        slab.fields.push((
            "m_ifc_export_as".into(),
            InstanceField::String("IfcShadingDevice".into()),
        ));
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements([slab].into_iter(), &mut entities, &mut storeys, policy);
        match &entities[1] {
            entities::IfcEntity::BuildingElement {
                ifc_type,
                predefined_type,
                ..
            } => {
                assert_eq!(ifc_type, "IFCSHADINGDEVICE");
                // The witness writes `.NOTDEFINED.` on every shading device (#235).
                assert_eq!(predefined_type.as_deref(), Some("NOTDEFINED"));
            }
            _ => panic!("expected building element"),
        }
    }

    #[test]
    fn unrecognised_export_override_keeps_the_class_mapping() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let mut slab = decoded("Floor", Some(7));
        slab.fields.push((
            "m_ifc_export_as".into(),
            InstanceField::String("IfcNotAProvenTarget".into()),
        ));
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements([slab].into_iter(), &mut entities, &mut storeys, policy);
        match &entities[1] {
            entities::IfcEntity::BuildingElement {
                ifc_type,
                predefined_type,
                ..
            } => {
                assert_eq!(ifc_type, "IFCSLAB");
                assert_eq!(predefined_type.as_deref(), Some("FLOOR"));
            }
            _ => panic!("expected building element"),
        }
    }

    #[test]
    fn record_backed_slab_records_a_resolved_thickness() {
        let mut slab = decoded("Floor", Some(20311));
        for (name, value) in [
            ("m_locationX", 93.0),
            ("m_locationY", 69.5),
            ("m_locationZ", 75.8333),
            ("m_bboxWidth", 168.0),
            ("m_bboxDepth", 107.0),
            ("m_bboxHeight", 0.1667),
        ] {
            slab.fields
                .push((name.into(), InstanceField::Float { value, size: 8 }));
        }
        slab.fields.push((
            "m_source".into(),
            InstanceField::String("partition_element_record".into()),
        ));
        let RecordGeometry {
            body,
            properties,
            pieces,
            ..
        } = element_record_geometry_from_decoded(&slab).expect("record geometry");
        assert!(pieces.is_empty());
        assert!((body.height_feet - 0.1667).abs() < 1e-6);
        assert!(properties.properties.iter().any(|p| {
            p.name == "ThicknessResolved" && matches!(p.value, PropertyValue::Boolean(true))
        }));
        assert!(properties.properties.iter().any(|p| {
            p.name == "ThicknessSource"
                && matches!(&p.value, PropertyValue::Text(t) if t == RECORD_BBOX_THICKNESS_SOURCE)
        }));
    }

    #[test]
    fn non_plate_record_bodies_do_not_claim_a_thickness() {
        let mut column = decoded("Column", Some(20375));
        for (name, value) in [
            ("m_locationX", 24.0),
            ("m_locationY", 110.0),
            ("m_locationZ", 76.0),
            ("m_bboxWidth", 2.0),
            ("m_bboxDepth", 2.0),
            ("m_bboxHeight", 14.33),
        ] {
            column
                .fields
                .push((name.into(), InstanceField::Float { value, size: 8 }));
        }
        let RecordGeometry { properties, .. } =
            element_record_geometry_from_decoded(&column).expect("record geometry");
        assert!(
            !properties
                .properties
                .iter()
                .any(|p| p.name == "ThicknessResolved"),
            "an envelope height is not a thickness"
        );
    }

    /// A structural-framing element with its record box and, when given,
    /// the location line RE-49 attaches.
    fn beam(bbox: [f64; 6], line: Option<([f64; 3], [f64; 3])>) -> DecodedElement {
        let mut element = decoded("StructuralFraming", Some(627866));
        for (name, value) in [
            ("m_locationX", (bbox[0] + bbox[3]) / 2.0),
            ("m_locationY", (bbox[1] + bbox[4]) / 2.0),
            ("m_locationZ", bbox[2]),
            ("m_bboxWidth", bbox[3] - bbox[0]),
            ("m_bboxDepth", bbox[4] - bbox[1]),
            ("m_bboxHeight", bbox[5] - bbox[2]),
        ] {
            element
                .fields
                .push((name.into(), InstanceField::Float { value, size: 8 }));
        }
        element.fields.push((
            "m_source".into(),
            InstanceField::String("partition_element_record".into()),
        ));
        if let Some((start, end)) = line {
            use crate::partition_schema_mvp::{BEAM_AXIS_END_FIELDS, BEAM_AXIS_START_FIELDS};
            for (names, point) in [(BEAM_AXIS_START_FIELDS, start), (BEAM_AXIS_END_FIELDS, end)] {
                for (name, value) in names.iter().zip(point) {
                    element
                        .fields
                        .push(((*name).into(), InstanceField::Float { value, size: 8 }));
                }
            }
        }
        element
    }

    fn body_source(properties: &PropertySet) -> Option<&str> {
        properties
            .properties
            .iter()
            .find_map(|p| match (&p.value, p.name.as_str()) {
                (PropertyValue::Text(text), "BodySource") => Some(text.as_str()),
                _ => None,
            })
    }

    /// RE-49: a level beam rotated in plan exports as its section's plan
    /// rectangle along its location line, not as the box around it.
    #[test]
    fn a_beam_runs_along_its_location_line() {
        let angle = 30f64.to_radians();
        let (length, width, depth) = (20.0, 0.5, 1.5);
        let (c, s) = (angle.cos(), angle.sin());
        // The line along the top of the beam, and the box of the beam.
        let start = [0.0, 0.0, 10.0];
        let end = [length * c, length * s, 10.0];
        let half_x = width / 2.0 * s;
        let half_y = width / 2.0 * c;
        let bbox = [
            -half_x,
            -half_y,
            10.0 - depth,
            length * c + half_x,
            length * s + half_y,
            10.0,
        ];
        let element = beam(bbox, Some((start, end)));
        let RecordGeometry {
            location,
            rotation,
            body,
            solid,
            properties,
            ..
        } = element_record_geometry_from_decoded(&element).expect("record geometry");
        assert!((rotation.expect("rotated") - angle).abs() < 1e-12);
        assert!(solid.is_none());
        assert!(body.profile_override.is_none());
        assert!((body.width_feet - length).abs() < 1e-9);
        assert!((body.depth_feet - width).abs() < 1e-9);
        assert!((body.height_feet - depth).abs() < 1e-9);
        assert!((location[0] - length * c / 2.0).abs() < 1e-9);
        assert!((location[1] - length * s / 2.0).abs() < 1e-9);
        assert_eq!(location[2], 10.0 - depth);
        assert_eq!(body_source(&properties), Some(BEAM_AXIS_BODY_SOURCE));
        assert!(properties.properties.iter().any(|p| {
            p.name == "AxisLength"
                && matches!(p.value, PropertyValue::LengthFeet(v) if (v - length).abs() < 1e-9)
        }));

        // The element the exporter writes carries the rotation.
        let mut entities = Vec::new();
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements(
            std::iter::once(element),
            &mut entities,
            &mut storeys,
            policy,
        );
        let rotations: Vec<f64> = entities
            .iter()
            .filter_map(|entity| match entity {
                entities::IfcEntity::BuildingElement {
                    rotation_radians, ..
                } => *rotation_radians,
                _ => None,
            })
            .collect();
        assert!(matches!(rotations[..], [r] if (r - angle).abs() < 1e-12));
    }

    /// RE-49: a sloped beam sweeps its section along its centreline and
    /// keeps its record box as the extrusion.
    #[test]
    fn a_sloped_beam_sweeps_its_section_along_its_centreline() {
        let slope = 2f64.to_radians();
        let (length, width, depth) = (20.0, 0.25, 0.3);
        let d = [0.0, slope.cos(), -slope.sin()];
        let start = [5.0, 1.0, 30.0];
        let end = [5.0, 1.0 + length * d[1], 30.0 + length * d[2]];
        let (ey, ez) = (
            length * d[1] + depth * slope.sin(),
            length * d[2].abs() + depth * slope.cos(),
        );
        let centre = [5.0, (start[1] + end[1]) / 2.0, (start[2] + end[2]) / 2.0];
        let bbox = [
            centre[0] - width / 2.0,
            centre[1] - ey / 2.0,
            centre[2] - ez / 2.0,
            centre[0] + width / 2.0,
            centre[1] + ey / 2.0,
            centre[2] + ez / 2.0,
        ];
        let RecordGeometry {
            location,
            rotation,
            body,
            solid,
            properties,
            ..
        } = element_record_geometry_from_decoded(&beam(bbox, Some((start, end))))
            .expect("record geometry");
        assert!(rotation.is_none());
        assert!((body.height_feet - ez).abs() < 1e-9, "the record box stays");
        let Some(entities::SolidShape::SweptPath {
            profile:
                entities::ProfileDef::Rectangle {
                    width_feet,
                    depth_feet,
                },
            directrix_points_feet,
            fixed_reference,
        }) = solid
        else {
            panic!("expected a swept solid");
        };
        assert!((width_feet - depth).abs() < 1e-9);
        assert!((depth_feet - width).abs() < 1e-9);
        assert_eq!(fixed_reference, [0.0, 0.0, 1.0]);
        let [a, b] = directrix_points_feet[..] else {
            panic!("two directrix points");
        };
        for axis in 0..3 {
            // The directrix is the centreline, relative to the placement.
            let mid = (a[axis] + b[axis]) / 2.0 + location[axis];
            assert!((mid - centre[axis]).abs() < 1e-9);
        }
        let run = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        assert!((run - length).abs() < 1e-9);
        assert_eq!(body_source(&properties), Some(BEAM_AXIS_BODY_SOURCE));
    }

    /// RE-52: a run with a recorded side view is drawn as it, extruded
    /// across the run from the side its sketch starts on. The run climbs
    /// -Y and its sketch starts on its +X side, so the extrusion runs -X.
    #[test]
    fn a_stair_run_is_drawn_as_its_treads_and_risers() {
        use crate::partition_schema_mvp::{
            STAIR_RUN_ACROSS_FIELDS, STAIR_RUN_CLIMB_FIELDS, STAIR_RUN_ORIGIN_FIELDS,
            STAIR_RUN_PROFILE_FIELD, STAIR_RUN_WIDTH_FIELD,
        };
        // One step: a riser 0.5 ft high and a tread 1 ft deep, as a plain
        // L so every corner is easy to name.
        let profile = [[0.0, 0.0], [1.0, 0.0], [1.0, 0.5], [0.0, 0.5]];
        let origin = [10.0, 20.0, 3.0];
        let (climb, across, width) = ([0.0, -1.0], [-1.0, 0.0], 4.0);
        let bbox = [6.0, 19.0, 3.0, 10.0, 20.0, 3.5];
        let mut element = beam(bbox, None);
        element.class = "StairsRun".into();
        let float = |value: f64| InstanceField::Float { value, size: 8 };
        for (name, value) in STAIR_RUN_ORIGIN_FIELDS.iter().zip(origin) {
            element.fields.push(((*name).into(), float(value)));
        }
        for (name, value) in STAIR_RUN_CLIMB_FIELDS.iter().zip(climb) {
            element.fields.push(((*name).into(), float(value)));
        }
        for (name, value) in STAIR_RUN_ACROSS_FIELDS.iter().zip(across) {
            element.fields.push(((*name).into(), float(value)));
        }
        element
            .fields
            .push((STAIR_RUN_WIDTH_FIELD.into(), float(width)));
        element.fields.push((
            STAIR_RUN_PROFILE_FIELD.into(),
            InstanceField::Vector(
                profile
                    .iter()
                    .map(|&[u, z]| InstanceField::Vector(vec![float(u), float(z)]))
                    .collect(),
            ),
        ));
        let RecordGeometry {
            location,
            solid,
            properties,
            ..
        } = element_record_geometry_from_decoded(&element).expect("record geometry");
        assert_eq!(body_source(&properties), Some(STAIR_RUN_BODY_SOURCE));
        let solid = solid.expect("a solid");
        assert!(matches!(
            solid,
            entities::SolidShape::PlacedExtrusion {
                axis: [-1.0, 0.0, 0.0],
                ..
            }
        ));
        let mesh = super::super::body_geometry::body_mesh(
            super::super::body_geometry::Body::Solid(&solid),
        )
        .expect("mesh");
        // Every corner of the step, at both sides of the run.
        for [u, z] in profile {
            for v in [0.0, width] {
                let want = [
                    origin[0] + u * climb[0] + v * across[0] - location[0],
                    origin[1] + u * climb[1] + v * across[1] - location[1],
                    origin[2] + z - location[2],
                ];
                assert!(
                    mesh.vertices
                        .iter()
                        .any(|p| (0..3).all(|i| (p[i] - want[i]).abs() < 1e-9)),
                    "missing {want:?}"
                );
            }
        }
        assert!(
            mesh.vertices
                .iter()
                .all(|p| p[1] + location[1] <= origin[1] + 1e-9),
            "the run climbs -Y from its origin"
        );
    }

    /// RE-49: without a line, or with one no box along it reproduces, a
    /// beam keeps its record box.
    #[test]
    fn a_beam_without_a_solvable_line_keeps_its_box() {
        let bbox = [0.0, 0.0, 0.0, 10.0, 6.0, 1.0];
        for line in [None, Some(([0.0, 0.0, 1.0], [10.0, 6.0, 1.0]))] {
            let RecordGeometry {
                rotation,
                body,
                solid,
                properties,
                ..
            } = element_record_geometry_from_decoded(&beam(bbox, line)).expect("record geometry");
            assert!(rotation.is_none() && solid.is_none());
            assert_eq!((body.width_feet, body.depth_feet), (10.0, 6.0));
            assert_eq!(
                body_source(&properties),
                Some("partition_element_record_bbox")
            );
        }
    }

    #[test]
    fn host_obj_attr_is_misleading_proxy_class() {
        assert!(is_misleading_proxy_class("HostObjAttr"));
        assert!(!is_misleading_proxy_class("Wall"));
    }

    #[test]
    fn typed_no_geometry_policy_strips_geometry_flag() {
        let p = ExportContentPolicy::for_quality_mode(ExportQualityMode::TypedNoGeometry);
        assert!(!p.include_geometry);
        assert!(p.require_mapped_ifc_type);
    }

    #[test]
    fn append_skips_host_obj_attr_even_in_scaffold() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Scaffold);
        append_typed_production_elements(
            [decoded("HostObjAttr", Some(1)), decoded("Wall", Some(2))].into_iter(),
            &mut entities,
            &mut storeys,
            policy,
        );
        let names: Vec<_> = entities
            .iter()
            .filter_map(|e| match e {
                entities::IfcEntity::BuildingElement { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(names, vec!["Wall-2"]);
        assert!(!names.iter().any(|n| n.starts_with("HostObjAttr-")));
    }

    #[test]
    fn material_becomes_material_info_not_proxy() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let mut mat = decoded("Material", None);
        mat.fields
            .push(("m_name".into(), InstanceField::String("Concrete".into())));
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Scaffold);
        let append = append_typed_production_elements(
            [mat].into_iter(),
            &mut entities,
            &mut storeys,
            policy,
        );
        assert!(
            entities
                .iter()
                .filter(|e| matches!(e, entities::IfcEntity::BuildingElement { .. }))
                .count()
                == 0
        );
        assert_eq!(append.materials.len(), 1);
        assert_eq!(append.materials[0].name, "Concrete");
    }

    #[test]
    fn scaffold_skips_unmapped_without_proxy() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Scaffold);
        append_typed_production_elements(
            [decoded("DefinitelyNotMapped", Some(9))].into_iter(),
            &mut entities,
            &mut storeys,
            policy,
        );
        assert_eq!(
            entities
                .iter()
                .filter(|e| matches!(e, entities::IfcEntity::BuildingElement { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn strict_omits_unmapped_proxy_fallback() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Strict);
        append_typed_production_elements(
            [decoded("DefinitelyNotMapped", Some(9))].into_iter(),
            &mut entities,
            &mut storeys,
            policy,
        );
        assert_eq!(
            entities
                .iter()
                .filter(|e| matches!(e, entities::IfcEntity::BuildingElement { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn level_becomes_storey_not_building_element() {
        let mut entities = vec![entities::IfcEntity::Project {
            name: Some("t".into()),
            description: None,
            long_name: None,
        }];
        let mut storeys = Vec::new();
        let mut level = decoded("Level", Some(3));
        level
            .fields
            .push(("m_name".into(), InstanceField::String("L1".into())));
        level.fields.push((
            "m_elevation".into(),
            InstanceField::Float {
                value: 12.0,
                size: 8,
            },
        ));
        let policy = ExportContentPolicy::for_quality_mode(ExportQualityMode::Geometry);
        append_typed_production_elements([level].into_iter(), &mut entities, &mut storeys, policy);
        assert!(
            storeys
                .iter()
                .any(|s| s.name == "L1" && (s.elevation_feet - 12.0).abs() < 1e-9)
        );
        assert_eq!(
            entities
                .iter()
                .filter(|e| matches!(e, entities::IfcEntity::BuildingElement { .. }))
                .count(),
            0
        );
    }

    #[test]
    fn strip_geometry_clears_extrusion_and_hosts() {
        let mut entities = vec![entities::IfcEntity::BuildingElement {
            ifc_type: "IFCWALL".into(),
            name: "Wall-1".into(),
            type_guid: None,
            predefined_type: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: Some([1.0, 2.0, 3.0]),
            rotation_radians: Some(0.5),
            extrusion: Some(Extrusion {
                width_feet: 10.0,
                depth_feet: 0.5,
                height_feet: 8.0,
                profile_override: None,
            }),
            host_element_index: Some(0),
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }];
        strip_building_element_geometry(&mut entities);
        match &entities[0] {
            entities::IfcEntity::BuildingElement {
                location_feet,
                extrusion,
                host_element_index,
                ..
            } => {
                assert!(location_feet.is_none());
                assert!(extrusion.is_none());
                assert!(host_element_index.is_none());
            }
            _ => panic!("expected building element"),
        }
    }
}
