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
use super::{ExportQualityMode, Storey, UNRESOLVED_ARCWALL_THICKNESS_FEET};
use crate::elements::floor::Floor;
use crate::elements::level::Level;
use crate::elements::openings::{Door, Window};
use crate::elements::wall::Wall;
use crate::geometry::{
    recover_door_host, recover_floor_boundary, recover_level_elevation,
    recover_wall_location_curve_from_wall, recover_window_host,
};
use crate::walker::DecodedElement;
use std::collections::HashMap;

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

/// Accumulate production walker hits into IFC entities + storeys.
///
/// Returns a map of Revit element id → index in `entities` for host linking.
pub fn append_typed_production_elements(
    decoded_iter: impl Iterator<Item = DecodedElement>,
    entities: &mut Vec<entities::IfcEntity>,
    building_storeys: &mut Vec<Storey>,
    policy: ExportContentPolicy,
) -> HashMap<u32, usize> {
    let mut id_to_entity: HashMap<u32, usize> = HashMap::new();
    let mut pending_hosts: Vec<(usize, u32)> = Vec::new();

    for decoded in decoded_iter {
        if is_misleading_proxy_class(&decoded.class) {
            continue;
        }

        if decoded.class == "Level" {
            if let Some(storey) = storey_from_level(&decoded) {
                building_storeys.push(storey);
            }
            continue;
        }

        let mapping = category_map::lookup(&decoded.class);
        if mapping.is_none() && policy.require_mapped_ifc_type {
            continue;
        }
        let ifc_type = mapping
            .map(|m| m.ifc_type.to_string())
            .unwrap_or_else(|| "IFCBUILDINGELEMENTPROXY".to_string());

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
            match decoded.class.as_str() {
                "Wall" => {
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
            id_to_entity.insert(id, entity_index);
        }
        if let Some(host_id) = pending_host_id {
            pending_hosts.push((entity_index, host_id));
        }

        entities.push(entities::IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            storey_index: None,
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
            if let Some(&host_index) = id_to_entity.get(&host_id) {
                if let Some(entities::IfcEntity::BuildingElement {
                    host_element_index, ..
                }) = entities.get_mut(entity_index)
                {
                    *host_element_index = Some(host_index);
                }
            }
        }
    }

    id_to_entity
}

fn storey_from_level(decoded: &DecodedElement) -> Option<Storey> {
    let level = Level::from_decoded(decoded);
    let elev = recover_level_elevation(&level).ok()?;
    let name = elev
        .name
        .or(level.name)
        .unwrap_or_else(|| match decoded.id {
            Some(id) => format!("Level-{id}"),
            None => "Level".into(),
        });
    Some(Storey {
        name,
        elevation_feet: elev.elevation_feet,
    })
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
