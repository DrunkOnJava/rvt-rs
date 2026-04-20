//! `Grid` + `GridType` — Revit's 2D datum grid lines. Architects use
//! them to tag columns and walls (A-1, A-2, B-1…) and the IFC exporter
//! needs their endpoints to emit `IfcGrid` with two `IfcGridAxis` lists.
//!
//! A grid is an infinite-in-direction line (or arc) with a "bubble"
//! head at one or both ends carrying its label. In the file the
//! concrete geometry is two endpoints + a curve kind; the bubble side
//! and head style come from the referenced `GridType`.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! Grid:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Label shown in bubble ("A", "1", "B.5") |
//! | `m_curve_kind` | Primitive u32 | 0 = line, 1 = arc |
//! | `m_start_x`, `m_start_y` | Primitive f64 | Start endpoint (model units — feet) |
//! | `m_end_x`, `m_end_y` | Primitive f64 | End endpoint |
//! | `m_elevation` | Primitive f64 | Z-plane the grid lives on |
//! | `m_show_head_start` | Primitive bool | Bubble at start end? |
//! | `m_show_head_end` | Primitive bool | Bubble at end end? |
//! | `m_type_id` | ElementId | Reference to the `GridType` |
//!
//! GridType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Type name ("6.5mm Bubble", …) |
//! | `m_bubble_loc` | Primitive u32 | 0=none 1=start 2=end 3=both |
//! | `m_line_weight` | Primitive u32 | Projection line weight |
//! | `m_line_pattern_id` | ElementId | `LinePattern` reference |
//!
//! Endpoints are stored in project coordinates (i.e. after the
//! `ProjectPosition` transform from `reference_points.rs`). Keep that
//! in mind when composing with `Transform3::IDENTITY` for IFC export.

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::Point3;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

macro_rules! simple_decoder {
    ($Struct:ident, $name:literal) => {
        pub struct $Struct;

        impl ElementDecoder for $Struct {
            fn class_name(&self) -> &'static str {
                $name
            }

            fn decode(
                &self,
                bytes: &[u8],
                schema: &formats::ClassEntry,
                _index: &HandleIndex,
            ) -> Result<DecodedElement> {
                if schema.name != $name {
                    return Err(Error::BasicFileInfo(format!(
                        "{} received wrong schema: {}",
                        stringify!($Struct),
                        schema.name
                    )));
                }
                Ok(crate::walker::decode_instance(bytes, 0, schema))
            }
        }
    };
}

simple_decoder!(GridDecoder, "Grid");
simple_decoder!(GridTypeDecoder, "GridType");

/// Which end(s) of the grid show a label bubble.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BubbleLocation {
    #[default]
    None,
    Start,
    End,
    Both,
}

impl BubbleLocation {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Start,
            2 => Self::End,
            3 => Self::Both,
            _ => Self::None,
        }
    }
}

/// Whether the grid line is straight or curved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GridCurveKind {
    #[default]
    Line,
    Arc,
}

impl GridCurveKind {
    fn from_code(code: u32) -> Self {
        if code == 1 { Self::Arc } else { Self::Line }
    }
}

/// Typed view of a decoded Grid.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Grid {
    pub name: Option<String>,
    pub curve_kind: Option<GridCurveKind>,
    pub start: Option<Point3>,
    pub end: Option<Point3>,
    pub show_head_start: Option<bool>,
    pub show_head_end: Option<bool>,
    pub type_id: Option<u32>,
}

impl Grid {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut sx = None;
        let mut sy = None;
        let mut ex = None;
        let mut ey = None;
        let mut elev = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("curvekind", InstanceField::Integer { value, .. }) => {
                    out.curve_kind = Some(GridCurveKind::from_code(*value as u32));
                }
                ("startx", InstanceField::Float { value, .. }) => sx = Some(*value),
                ("starty", InstanceField::Float { value, .. }) => sy = Some(*value),
                ("endx", InstanceField::Float { value, .. }) => ex = Some(*value),
                ("endy", InstanceField::Float { value, .. }) => ey = Some(*value),
                ("elevation", InstanceField::Float { value, .. }) => elev = Some(*value),
                ("showheadstart", InstanceField::Bool(b)) => out.show_head_start = Some(*b),
                ("showheadend", InstanceField::Bool(b)) => out.show_head_end = Some(*b),
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                _ => {}
            }
        }
        let z = elev.unwrap_or(0.0);
        if let (Some(x), Some(y)) = (sx, sy) {
            out.start = Some(Point3::new(x, y, z));
        }
        if let (Some(x), Some(y)) = (ex, ey) {
            out.end = Some(Point3::new(x, y, z));
        }
        out
    }

    /// Horizontal length in model units (feet). `None` if either
    /// endpoint is missing or the curve is an arc (arc length needs
    /// the centre, which isn't in the stable field set yet).
    pub fn length_feet(&self) -> Option<f64> {
        if !matches!(self.curve_kind, None | Some(GridCurveKind::Line)) {
            return None;
        }
        let (s, e) = (self.start?, self.end?);
        let dx = e.x - s.x;
        let dy = e.y - s.y;
        Some((dx * dx + dy * dy).sqrt())
    }
}

/// Typed view of a decoded GridType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GridType {
    pub name: Option<String>,
    pub bubble_location: Option<BubbleLocation>,
    pub line_weight: Option<u32>,
    pub line_pattern_id: Option<u32>,
}

impl GridType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("bubbleloc" | "bubblelocation", InstanceField::Integer { value, .. }) => {
                    out.bubble_location = Some(BubbleLocation::from_code(*value as u32));
                }
                ("lineweight" | "weight", InstanceField::Integer { value, .. }) => {
                    out.line_weight = Some(*value as u32);
                }
                ("linepatternid" | "patternid", InstanceField::ElementId { id, .. }) => {
                    out.line_pattern_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_grid_schema() -> ClassEntry {
        ClassEntry {
            name: "Grid".to_string(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_name".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_curve_kind".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    }),
                },
                FieldEntry {
                    name: "m_start_x".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_start_y".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_end_x".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_end_y".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_elevation".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_show_head_start".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
                FieldEntry {
                    name: "m_show_head_end".into(),
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

    fn synth_grid_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_name = "A"
        let name = "A";
        b.extend_from_slice(&(name.chars().count() as u32).to_le_bytes());
        for ch in name.encode_utf16() {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        // m_curve_kind = 0  (line)
        b.extend_from_slice(&0u32.to_le_bytes());
        // m_start = (0.0, 0.0), m_end = (10.0, 0.0), m_elevation = 0.0
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        b.extend_from_slice(&10.0_f64.to_le_bytes());
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        // m_show_head_start = true, m_show_head_end = false
        b.push(1);
        b.push(0);
        b
    }

    #[test]
    fn grid_decoder_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Level".to_string(),
            ..synth_grid_schema()
        };
        assert!(
            GridDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn grid_decodes_endpoints() {
        let decoded = GridDecoder
            .decode(
                &synth_grid_bytes(),
                &synth_grid_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let g = Grid::from_decoded(&decoded);
        assert_eq!(g.name.as_deref(), Some("A"));
        assert_eq!(g.curve_kind, Some(GridCurveKind::Line));
        assert_eq!(g.start, Some(Point3::new(0.0, 0.0, 0.0)));
        assert_eq!(g.end, Some(Point3::new(10.0, 0.0, 0.0)));
        assert_eq!(g.show_head_start, Some(true));
        assert_eq!(g.show_head_end, Some(false));
    }

    #[test]
    fn grid_length_line() {
        let g = Grid {
            start: Some(Point3::new(0.0, 0.0, 0.0)),
            end: Some(Point3::new(3.0, 4.0, 0.0)),
            curve_kind: Some(GridCurveKind::Line),
            ..Default::default()
        };
        assert!((g.length_feet().unwrap() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn grid_length_arc_returns_none() {
        let g = Grid {
            start: Some(Point3::new(0.0, 0.0, 0.0)),
            end: Some(Point3::new(3.0, 4.0, 0.0)),
            curve_kind: Some(GridCurveKind::Arc),
            ..Default::default()
        };
        assert_eq!(g.length_feet(), None);
    }

    #[test]
    fn bubble_location_mapping() {
        assert_eq!(BubbleLocation::from_code(0), BubbleLocation::None);
        assert_eq!(BubbleLocation::from_code(1), BubbleLocation::Start);
        assert_eq!(BubbleLocation::from_code(2), BubbleLocation::End);
        assert_eq!(BubbleLocation::from_code(3), BubbleLocation::Both);
        assert_eq!(BubbleLocation::from_code(99), BubbleLocation::None);
    }

    #[test]
    fn grid_type_tolerates_empty() {
        let empty = DecodedElement {
            id: None,
            class: "GridType".to_string(),
            fields: vec![],
            byte_range: 0..0,
        };
        let gt = GridType::from_decoded(&empty);
        assert!(gt.name.is_none() && gt.bubble_location.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(GridDecoder.class_name(), "Grid");
        assert_eq!(GridTypeDecoder.class_name(), "GridType");
    }
}
