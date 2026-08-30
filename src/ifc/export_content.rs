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

/// Decoded classes whose record bounding-box `z` extent is the
/// element's own thickness rather than an envelope height (#212).
pub const SLAB_THICKNESS_CLASSES: &[&str] = &["Floor", "BuildingPad"];

/// Value of the `ThicknessSource` property on a record-backed plate.
pub const RECORD_BBOX_THICKNESS_SOURCE: &str = "partition_element_record_bbox_z_extent";

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

        let name = match decoded.id {
            Some(id) => format!("{}-{}", decoded.class, id),
            None => format!("{}-unnamed", decoded.class),
        };
        let type_guid = decoded.id.map(|id| id.to_string());

        let mut location_feet = None;
        let mut rotation_radians = None;
        let mut extrusion = None;
        let mut property_set = None;
        let mut pending_host_id = None;

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
            if let Some((location, body, properties)) = element_record_geometry {
                location_feet = Some(location);
                extrusion = Some(body);
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
            predefined_type: effective.predefined_type.map(str::to_string),
            storey_index,
            material_index: None,
            property_set,
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        });
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

/// The IFC entity mapping a per-element "IFC Export As" override
/// names, when the decoded element carries one and
/// [`category_map::lookup_export_override`] recognises the value.
fn ifc_export_override(decoded: &DecodedElement) -> Option<&'static category_map::Mapping> {
    decoded
        .fields
        .iter()
        .find_map(|(name, value)| match (name.as_str(), value) {
            ("m_ifc_export_as", InstanceField::String(text)) => {
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
fn element_record_geometry_from_decoded(
    decoded: &DecodedElement,
) -> Option<([f64; 3], Extrusion, PropertySet)> {
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
    let mut type_symbol_id = None;
    let mut type_profile = (None, None);
    for (name, value) in &decoded.fields {
        match (name.as_str(), value) {
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
                crate::partition_schema_mvp::TYPE_SYMBOL_FIELD,
                InstanceField::ElementId { id, .. },
            ) => type_symbol_id = Some(*id),
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
    // The sketched plan profile, when the element's `OST_SketchLines`
    // records closed one (#31, RE-25). It is recovered in project
    // plan coordinates and the body is placed at the record's plan
    // centre, so the profile is expressed relative to that centre.
    let profile = crate::element_record_plan_profiles::plan_profile_from_fields(&decoded.fields);
    let profile_override = profile.as_ref().map(|profile| {
        let outer: Vec<(f64, f64)> = profile
            .outer_xy
            .iter()
            .map(|(px, py)| (px - x, py - y))
            .collect();
        let voids: Vec<Vec<(f64, f64)>> = profile
            .inner_xy
            .iter()
            .map(|ring| ring.iter().map(|(px, py)| (px - x, py - y)).collect())
            .collect();
        if voids.is_empty() {
            entities::ProfileDef::ArbitraryClosed { points: outer }
        } else {
            entities::ProfileDef::ArbitraryWithVoids {
                points: outer,
                voids,
            }
        }
    });
    // The family/type symbol's section, when the instance joined to
    // one and the section agrees with the instance envelope (#215,
    // RE-26). The rectangle it gives is the same shape the envelope
    // gave; what changes is that the type is now the authority for it.
    let type_section = match type_profile {
        (Some(w), Some(d)) if w > 0.0 && d > 0.0 => Some((w, d)),
        _ => None,
    };
    let (width, depth) = type_section.unwrap_or((width, depth));
    let mut properties = vec![
        // `BodySource` stays the record box unless a wall resolved its
        // joins: the placement, the plan envelope and the extrusion
        // depth are otherwise all still read from the box. What the
        // sketch lines replace is the plan *profile*, which
        // `ProfileResolved` / `ProfileSource` report separately.
        Property {
            name: "BodySource".into(),
            value: PropertyValue::Text(
                wall_body_source
                    .clone()
                    .unwrap_or_else(|| "partition_element_record_bbox".into()),
            ),
        },
        Property {
            name: "ProfileResolved".into(),
            value: PropertyValue::Boolean(profile.is_some() || type_section.is_some()),
        },
        Property {
            name: "LevelBindResolved".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "BoundingBoxHeight".into(),
            value: PropertyValue::LengthFeet(height),
        },
    ];
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
    }
    if let (Some((section_width, section_depth)), Some(symbol)) = (type_section, type_symbol_id) {
        properties.push(Property {
            name: "ProfileSource".into(),
            value: PropertyValue::Text(crate::partition_schema_mvp::TYPE_PROFILE_SOURCE.into()),
        });
        properties.push(Property {
            name: "TypeSymbolElementId".into(),
            value: PropertyValue::Integer(i64::from(symbol)),
        });
        properties.push(Property {
            name: "TypeSectionWidthFeet".into(),
            value: PropertyValue::LengthFeet(section_width),
        });
        properties.push(Property {
            name: "TypeSectionDepthFeet".into(),
            value: PropertyValue::LengthFeet(section_depth),
        });
    }
    // A wall whose joins resolved reports the trim it took and the
    // thickness it read off the box's thin axis (RE-26).
    if let (Some(thickness), Some(start), Some(end)) =
        (wall_thickness, wall_trim_start, wall_trim_end)
    {
        properties.push(Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(true),
        });
        properties.push(Property {
            name: "ThicknessFeet".into(),
            value: PropertyValue::LengthFeet(thickness),
        });
        properties.push(Property {
            name: "ThicknessSource".into(),
            value: PropertyValue::Text(
                crate::element_record_wall_joins::WALL_THICKNESS_SOURCE.into(),
            ),
        });
        properties.push(Property {
            name: "JoinTrimStartFeet".into(),
            value: PropertyValue::LengthFeet(start),
        });
        properties.push(Property {
            name: "JoinTrimEndFeet".into(),
            value: PropertyValue::LengthFeet(end),
        });
    }
    if let Some(stream) = source_stream {
        properties.push(Property {
            name: "SourceStream".into(),
            value: PropertyValue::Text(stream),
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
            name: "ThicknessFeet".into(),
            value: PropertyValue::LengthFeet(height),
        });
        properties.push(Property {
            name: "ThicknessSource".into(),
            value: PropertyValue::Text(RECORD_BBOX_THICKNESS_SOURCE.into()),
        });
    }
    Some((
        [x, y, z],
        Extrusion {
            width_feet: width,
            depth_feet: depth,
            height_feet: height,
            profile_override,
        },
        PropertySet {
            name: ELEMENT_RECORD_PROPERTY_SET.into(),
            properties,
        },
    ))
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
                assert_eq!(predefined_type.as_deref(), None);
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
        let (_, body, properties) =
            element_record_geometry_from_decoded(&slab).expect("record geometry");
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
        let (_, _, properties) =
            element_record_geometry_from_decoded(&column).expect("record geometry");
        assert!(
            !properties
                .properties
                .iter()
                .any(|p| p.name == "ThicknessResolved"),
            "an envelope height is not a thickness"
        );
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
