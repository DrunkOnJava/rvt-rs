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
