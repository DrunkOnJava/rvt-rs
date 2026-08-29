//! JSON projections of Lane Five MVP typed views.
//!
//! Used by the Python bindings and the `rvt-elements` CLI so both
//! surfaces share one dictionary shape. Non-MVP classes return
//! `None` — callers keep the generic `fields` dump and omit `typed`.

use super::MVP_TYPED_CLASSES;
use super::floor::Floor;
use super::level::Level;
use super::openings::{Door, Window};
use super::styling::Material;
use super::wall::{LocationLine, StructuralUsage, Wall};
use super::zones::Zone;
use crate::geometry::Point3;
use crate::walker::DecodedElement;
use serde_json::{Value, json};

/// `true` when `class_name` is in the Lane Five schema-driven MVP set.
pub fn is_mvp_typed_class(class_name: &str) -> bool {
    MVP_TYPED_CLASSES.iter().any(|c| *c == class_name)
}

/// Project a decoded element into its MVP typed JSON view, if any.
///
/// Missing or wrong-typed wire fields surface as JSON `null` (via
/// omitted Optional fields becoming `null` in the object). Geometry
/// is never invented — Door/Window `location` is only present when
/// the decoder recovered XYZ.
pub fn mvp_typed_view(decoded: &DecodedElement) -> Option<Value> {
    match decoded.class.as_str() {
        "Level" => Some(level_json(&Level::from_decoded(decoded))),
        "Wall" => Some(wall_json(&Wall::from_decoded(decoded))),
        "Floor" => Some(floor_json(&Floor::from_decoded(decoded))),
        "Door" => Some(door_json(&Door::from_decoded(decoded))),
        "Window" => Some(window_json(&Window::from_decoded(decoded))),
        "Room" => Some(zone_json(&Zone::from_decoded(decoded))),
        "Material" => Some(material_json(&Material::from_decoded(decoded))),
        _ => None,
    }
}

fn opt_u32(v: Option<u32>) -> Value {
    match v {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

fn opt_f64(v: Option<f64>) -> Value {
    match v {
        Some(n) => json!(n),
        None => Value::Null,
    }
}

fn opt_bool(v: Option<bool>) -> Value {
    match v {
        Some(b) => json!(b),
        None => Value::Null,
    }
}

fn opt_string(v: &Option<String>) -> Value {
    match v {
        Some(s) => json!(s),
        None => Value::Null,
    }
}

fn point3_json(p: Point3) -> Value {
    json!({ "x": p.x, "y": p.y, "z": p.z })
}

fn structural_usage_str(u: StructuralUsage) -> &'static str {
    match u {
        StructuralUsage::NonBearing => "non_bearing",
        StructuralUsage::Bearing => "bearing",
        StructuralUsage::Shear => "shear",
        StructuralUsage::Combined => "combined",
    }
}

fn location_line_str(l: LocationLine) -> &'static str {
    match l {
        LocationLine::WallCenterline => "wall_centerline",
        LocationLine::CoreCenterline => "core_centerline",
        LocationLine::FinishFaceExterior => "finish_face_exterior",
        LocationLine::FinishFaceInterior => "finish_face_interior",
        LocationLine::CoreFaceExterior => "core_face_exterior",
        LocationLine::CoreFaceInterior => "core_face_interior",
    }
}

fn level_json(v: &Level) -> Value {
    json!({
        "name": opt_string(&v.name),
        "elevation_feet": opt_f64(v.elevation_feet),
        "level_type_id": opt_u32(v.level_type_id),
        "is_building_story": opt_bool(v.is_building_story),
    })
}

fn wall_json(v: &Wall) -> Value {
    json!({
        "base_level_id": opt_u32(v.base_level_id),
        "base_offset_feet": opt_f64(v.base_offset_feet),
        "top_level_id": opt_u32(v.top_level_id),
        "top_offset_feet": opt_f64(v.top_offset_feet),
        "unconnected_height_feet": opt_f64(v.unconnected_height_feet),
        "structural_usage": match v.structural_usage {
            Some(u) => json!(structural_usage_str(u)),
            None => Value::Null,
        },
        "location_line": match v.location_line {
            Some(l) => json!(location_line_str(l)),
            None => Value::Null,
        },
        "type_id": opt_u32(v.type_id),
        "host_id": opt_u32(v.host_id),
    })
}

fn floor_json(v: &Floor) -> Value {
    json!({
        "level_id": opt_u32(v.level_id),
        "height_offset_feet": opt_f64(v.height_offset_feet),
        "structural": opt_bool(v.structural),
        "is_slab_edge": opt_bool(v.is_slab_edge),
        "type_id": opt_u32(v.type_id),
        "span_direction_radians": opt_f64(v.span_direction_radians),
    })
}

fn door_json(v: &Door) -> Value {
    json!({
        "level_id": opt_u32(v.level_id),
        "host_id": opt_u32(v.host_id),
        "symbol_id": opt_u32(v.symbol_id),
        "location": match v.location {
            Some(p) => point3_json(p),
            None => Value::Null,
        },
        "rotation_radians": opt_f64(v.rotation_radians),
        "flip_hand": opt_bool(v.flip_hand),
        "flip_facing": opt_bool(v.flip_facing),
    })
}

fn window_json(v: &Window) -> Value {
    json!({
        "level_id": opt_u32(v.level_id),
        "host_id": opt_u32(v.host_id),
        "symbol_id": opt_u32(v.symbol_id),
        "location": match v.location {
            Some(p) => point3_json(p),
            None => Value::Null,
        },
        "rotation_radians": opt_f64(v.rotation_radians),
        "sill_height_feet": opt_f64(v.sill_height_feet),
    })
}

fn zone_json(v: &Zone) -> Value {
    json!({
        "name": opt_string(&v.name),
        "number": opt_string(&v.number),
        "level_id": opt_u32(v.level_id),
        "upper_limit_id": opt_u32(v.upper_limit_id),
        "base_offset_feet": opt_f64(v.base_offset_feet),
        "upper_offset_feet": opt_f64(v.upper_offset_feet),
        "area_square_feet": opt_f64(v.area_square_feet),
        "volume_cubic_feet": opt_f64(v.volume_cubic_feet),
    })
}

fn material_json(v: &Material) -> Value {
    json!({
        "name": opt_string(&v.name),
        "color": opt_u32(v.color),
        "transparency": opt_f64(v.transparency),
        "shininess": opt_f64(v.shininess),
        "appearance_asset_id": opt_u32(v.appearance_asset_id),
        "physical_asset_id": opt_u32(v.physical_asset_id),
        "thermal_asset_id": opt_u32(v.thermal_asset_id),
        "surface_pattern_id": opt_u32(v.surface_pattern_id),
        "cut_pattern_id": opt_u32(v.cut_pattern_id),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::walker::InstanceField;

    #[test]
    fn mvp_typed_view_projects_level_height() {
        let decoded = DecodedElement {
            id: Some(1),
            class: "Level".into(),
            byte_range: 0..8,
            fields: vec![
                ("m_name".into(), InstanceField::String("L1".into())),
                (
                    "m_height".into(),
                    InstanceField::Float {
                        value: 10.0,
                        size: 8,
                    },
                ),
            ],
        };
        let typed = mvp_typed_view(&decoded).expect("Level is MVP");
        assert_eq!(typed["name"], json!("L1"));
        assert_eq!(typed["elevation_feet"], json!(10.0));
    }

    #[test]
    fn mvp_typed_view_skips_non_mvp() {
        let decoded = DecodedElement {
            id: None,
            class: "HostObjAttr".into(),
            byte_range: 0..0,
            fields: vec![],
        };
        assert!(mvp_typed_view(&decoded).is_none());
    }

    #[test]
    fn is_mvp_covers_lane_five_set() {
        for class in MVP_TYPED_CLASSES {
            assert!(is_mvp_typed_class(class));
        }
        assert!(!is_mvp_typed_class("ArcWall"));
    }
}
