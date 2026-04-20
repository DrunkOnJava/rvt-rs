//! `Door` + `Window` — wall-hosted opening elements. Both are
//! `FamilyInstance` subtypes at the class hierarchy level but ship
//! with enough stable scalar fields to decode directly without first
//! resolving the full family graph (which is task L5B-21).
//!
//! Each element references:
//! - A **host** (the wall it cuts through), via `m_host_id`
//! - A **symbol/type** (the family type definition), via `m_symbol_id`
//! - A **level**, inherited from the host wall's level
//! - A **location point** with a rotation angle
//!
//! Plus element-level overrides: `m_sill_height` for windows,
//! `m_flip_hand` and `m_flip_facing` for doors (left-swing vs
//! right-swing, interior vs exterior).
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! Door:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Wall (or other host) the door is cut into |
//! | `m_symbol_id` | ElementId | FamilySymbol (door type: "Single-Flush 36\" x 84\"") |
//! | `m_location_x`, `m_location_y`, `m_location_z` | f64 | Door location in project coords |
//! | `m_rotation` | f64 | Rotation angle in radians (usually aligned to host wall) |
//! | `m_flip_hand` | bool | Hinge on opposite side |
//! | `m_flip_facing` | bool | Swings into opposite space |
//!
//! Window:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Wall the window sits in |
//! | `m_symbol_id` | ElementId | FamilySymbol |
//! | `m_location_x`, `m_location_y`, `m_location_z` | f64 | Centre of the window in project coords |
//! | `m_rotation` | f64 | Rotation radians |
//! | `m_sill_height` | f64 | Height of windowsill above host level (feet) |

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::Point3;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Registered decoder for the `Door` class.
pub struct DoorDecoder;

impl ElementDecoder for DoorDecoder {
    fn class_name(&self) -> &'static str {
        "Door"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Door" {
            return Err(Error::BasicFileInfo(format!(
                "DoorDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `Window` class.
pub struct WindowDecoder;

impl ElementDecoder for WindowDecoder {
    fn class_name(&self) -> &'static str {
        "Window"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Window" {
            return Err(Error::BasicFileInfo(format!(
                "WindowDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Fields shared by every wall-hosted opening element (Door,
/// Window, and future Opening). Gathered in one pass so neither
/// `Door::from_decoded` nor `Window::from_decoded` has to re-scan.
#[derive(Debug, Clone, Copy, Default)]
struct OpeningCommon {
    location: Option<Point3>,
    rotation_radians: Option<f64>,
    level_id: Option<u32>,
    host_id: Option<u32>,
    symbol_id: Option<u32>,
}

fn collect_common(decoded: &DecodedElement) -> OpeningCommon {
    let mut lx = None;
    let mut ly = None;
    let mut lz = None;
    let mut out = OpeningCommon::default();
    for (field_name, value) in &decoded.fields {
        match (normalise_field_name(field_name).as_str(), value) {
            ("locationx", InstanceField::Float { value, .. }) => lx = Some(*value),
            ("locationy", InstanceField::Float { value, .. }) => ly = Some(*value),
            ("locationz", InstanceField::Float { value, .. }) => lz = Some(*value),
            ("rotation", InstanceField::Float { value, .. }) => {
                out.rotation_radians = Some(*value);
            }
            ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                out.level_id = Some(*id);
            }
            ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
            ("symbolid" | "typeid" | "familysymbolid", InstanceField::ElementId { id, .. }) => {
                out.symbol_id = Some(*id);
            }
            _ => {}
        }
    }
    if let (Some(x), Some(y), Some(z)) = (lx, ly, lz) {
        out.location = Some(Point3::new(x, y, z));
    }
    out
}

/// Typed view of a decoded Door.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Door {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    pub flip_hand: Option<bool>,
    pub flip_facing: Option<bool>,
}

impl Door {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        let mut out = Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
            rotation_radians: c.rotation_radians,
            flip_hand: None,
            flip_facing: None,
        };
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("fliphand", InstanceField::Bool(b)) => out.flip_hand = Some(*b),
                ("flipfacing", InstanceField::Bool(b)) => out.flip_facing = Some(*b),
                _ => {}
            }
        }
        out
    }

    /// True when the door has been flipped from its family-default
    /// orientation (either hand or facing).
    pub fn is_flipped(&self) -> Option<bool> {
        match (self.flip_hand, self.flip_facing) {
            (Some(h), Some(f)) => Some(h ^ f),
            (Some(h), None) => Some(h),
            (None, Some(f)) => Some(f),
            (None, None) => None,
        }
    }
}

/// Typed view of a decoded Window.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Window {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    /// Distance from host level's elevation up to the windowsill.
    pub sill_height_feet: Option<f64>,
}

impl Window {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        let mut out = Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
            rotation_radians: c.rotation_radians,
            sill_height_feet: None,
        };
        for (field_name, value) in &decoded.fields {
            if let ("sillheight", InstanceField::Float { value, .. }) =
                (normalise_field_name(field_name).as_str(), value)
            {
                out.sill_height_feet = Some(*value);
            }
        }
        out
    }

    /// Sill height in inches — convenience for US-customary callers.
    pub fn sill_height_inches(&self) -> Option<f64> {
        self.sill_height_feet.map(|ft| ft * 12.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_door_schema() -> ClassEntry {
        let f64_prim = FieldType::Primitive {
            kind: 0x07,
            size: 8,
        };
        ClassEntry {
            name: "Door".into(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_host_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_symbol_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_location_x".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_location_y".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_location_z".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_rotation".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim),
                },
                FieldEntry {
                    name: "m_flip_hand".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
                FieldEntry {
                    name: "m_flip_facing".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(9),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_door_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_level_id, m_host_id, m_symbol_id
        for id in [1_u32, 42_u32, 101_u32] {
            b.extend_from_slice(&0u32.to_le_bytes());
            b.extend_from_slice(&id.to_le_bytes());
        }
        // m_location = (5.0, 10.0, 0.0)
        for v in [5.0_f64, 10.0, 0.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // m_rotation = 0.0
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        // m_flip_hand = true, m_flip_facing = false
        b.push(1);
        b.push(0);
        b
    }

    #[test]
    fn door_decoder_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Window".into(),
            ..synth_door_schema()
        };
        assert!(
            DoorDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn door_decodes_and_tracks_flips() {
        let decoded = DoorDecoder
            .decode(
                &synth_door_bytes(),
                &synth_door_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let d = Door::from_decoded(&decoded);
        assert_eq!(d.level_id, Some(1));
        assert_eq!(d.host_id, Some(42));
        assert_eq!(d.symbol_id, Some(101));
        assert_eq!(d.location, Some(Point3::new(5.0, 10.0, 0.0)));
        assert_eq!(d.rotation_radians, Some(0.0));
        assert_eq!(d.flip_hand, Some(true));
        assert_eq!(d.flip_facing, Some(false));
        assert_eq!(d.is_flipped(), Some(true));
    }

    #[test]
    fn door_is_flipped_combinations() {
        // Both flipped: double negative → not flipped.
        let both = Door {
            flip_hand: Some(true),
            flip_facing: Some(true),
            ..Default::default()
        };
        assert_eq!(both.is_flipped(), Some(false));
        // Neither flag set.
        assert_eq!(Door::default().is_flipped(), None);
    }

    #[test]
    fn window_collects_location_and_sill() {
        let empty = DecodedElement {
            id: None,
            class: "Window".into(),
            fields: vec![
                (
                    "m_location_x".into(),
                    InstanceField::Float {
                        value: 1.0,
                        size: 8,
                    },
                ),
                (
                    "m_location_y".into(),
                    InstanceField::Float {
                        value: 2.0,
                        size: 8,
                    },
                ),
                (
                    "m_location_z".into(),
                    InstanceField::Float {
                        value: 0.0,
                        size: 8,
                    },
                ),
                (
                    "m_sill_height".into(),
                    InstanceField::Float {
                        value: 2.5,
                        size: 8,
                    },
                ),
            ],
            byte_range: 0..0,
        };
        let w = Window::from_decoded(&empty);
        assert_eq!(w.location, Some(Point3::new(1.0, 2.0, 0.0)));
        assert_eq!(w.sill_height_feet, Some(2.5));
        assert_eq!(w.sill_height_inches(), Some(30.0));
    }

    #[test]
    fn window_tolerates_missing_fields() {
        let empty = DecodedElement {
            id: None,
            class: "Window".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let w = Window::from_decoded(&empty);
        assert!(w.location.is_none());
        assert!(w.sill_height_feet.is_none());
        assert!(w.rotation_radians.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(DoorDecoder.class_name(), "Door");
        assert_eq!(WindowDecoder.class_name(), "Window");
    }
}
