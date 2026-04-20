//! `CurtainWall` + `CurtainGrid` + `CurtainMullion` + `CurtainPanel`
//! — the four classes that together describe a curtain-wall
//! assembly in Revit. Common on commercial-office envelopes:
//!
//! - **CurtainWall**: the overall hosted envelope (acts like a
//!   big wall but internally is a grid of panels + mullions).
//! - **CurtainGrid**: the dividing-line grid. Stores U + V line
//!   counts plus the positions of the line intersections.
//! - **CurtainMullion**: a framing member running along one grid
//!   line segment (vertical or horizontal).
//! - **CurtainPanel**: a glass / spandrel / infill panel between
//!   two pairs of intersecting grid lines.
//!
//! CurtainWall → IFCCURTAINWALL. Mullion / Panel don't have
//! dedicated IFC4 types; Graphisoft / Autodesk both emit them as
//! `IfcMember` (mullions) and `IfcPlate` (panels) inside the
//! parent IfcCurtainWall aggregate. For now we decode the typed
//! views but leave the IFC aggregation to a follow-up once we
//! wire up IfcRelAggregates for curtain-wall components.
//!
//! # Typical field shape (stable 2016–2026)
//!
//! CurtainWall:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_base_offset` / `m_top_offset` | f64 | Offsets from base/top levels |
//! | `m_top_level_id` | ElementId | Top level (0 = unconnected) |
//! | `m_unconnected_height` | f64 | Height when top is unconnected |
//! | `m_type_id` | ElementId | Reference to the WallType (curtain wall type) |
//!
//! CurtainGrid:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_u_grid_line_count` | Primitive u32 | Number of U-direction grid lines |
//! | `m_v_grid_line_count` | Primitive u32 | Number of V-direction grid lines |
//! | `m_host_id` | ElementId | Parent CurtainWall |
//!
//! CurtainMullion:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_host_id` | ElementId | Parent CurtainGrid |
//! | `m_symbol_id` | ElementId | Mullion type (profile + material) |
//! | `m_is_vertical` | Primitive bool | True = V-direction; false = U |
//!
//! CurtainPanel:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_host_id` | ElementId | Parent CurtainGrid |
//! | `m_symbol_id` | ElementId | Panel type (glass / spandrel / solid) |
//! | `m_u_index` / `m_v_index` | Primitive u32 | Grid-cell coordinates of this panel |

use super::level::normalise_field_name;
use crate::formats;
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

simple_decoder!(CurtainWallDecoder, "CurtainWall");
simple_decoder!(CurtainGridDecoder, "CurtainGrid");
simple_decoder!(CurtainMullionDecoder, "CurtainMullion");
simple_decoder!(CurtainPanelDecoder, "CurtainPanel");

/// Typed view for CurtainWall. Shares most of the shape with
/// `crate::elements::wall::Wall` but lives as a separate type
/// because curtain walls don't have a `LocationLine` or
/// `StructuralUsage` — those concepts don't apply to mullion-
/// panel assemblies.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurtainWall {
    pub base_level_id: Option<u32>,
    pub top_level_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    pub top_offset_feet: Option<f64>,
    pub unconnected_height_feet: Option<f64>,
    pub type_id: Option<u32>,
}

impl CurtainWall {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "baselevelid", InstanceField::ElementId { id, .. }) => {
                    out.base_level_id = Some(*id);
                }
                ("toplevelid", InstanceField::ElementId { id, .. }) => {
                    out.top_level_id = Some(*id);
                }
                ("baseoffset", InstanceField::Float { value, .. }) => {
                    out.base_offset_feet = Some(*value);
                }
                ("topoffset", InstanceField::Float { value, .. }) => {
                    out.top_offset_feet = Some(*value);
                }
                ("unconnectedheight", InstanceField::Float { value, .. }) => {
                    out.unconnected_height_feet = Some(*value);
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                _ => {}
            }
        }
        out
    }

    pub fn is_unconnected(&self) -> bool {
        matches!(self.top_level_id, None | Some(0))
    }
}

/// Typed view for CurtainGrid — the dividing-line grid inside a
/// curtain wall. Each line count + the parent CurtainWall host
/// fully describe the grid topology.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurtainGrid {
    pub host_id: Option<u32>,
    pub u_line_count: Option<u32>,
    pub v_line_count: Option<u32>,
}

impl CurtainGrid {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                ("ugridlinecount" | "ucount", InstanceField::Integer { value, .. }) => {
                    out.u_line_count = Some(*value as u32);
                }
                ("vgridlinecount" | "vcount", InstanceField::Integer { value, .. }) => {
                    out.v_line_count = Some(*value as u32);
                }
                _ => {}
            }
        }
        out
    }

    /// Number of grid cells (U-1) × (V-1). None if either line
    /// count is missing. Gives the expected panel count.
    pub fn cell_count(&self) -> Option<u32> {
        let u = self.u_line_count?;
        let v = self.v_line_count?;
        Some(u.saturating_sub(1) * v.saturating_sub(1))
    }
}

/// Typed view for CurtainMullion — a single framing member.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurtainMullion {
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub is_vertical: Option<bool>,
}

impl CurtainMullion {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                ("symbolid" | "typeid", InstanceField::ElementId { id, .. }) => {
                    out.symbol_id = Some(*id);
                }
                ("isvertical", InstanceField::Bool(b)) => out.is_vertical = Some(*b),
                _ => {}
            }
        }
        out
    }
}

/// Typed view for CurtainPanel — a single glass / spandrel / infill
/// panel bound by two pairs of grid lines.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CurtainPanel {
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub u_index: Option<u32>,
    pub v_index: Option<u32>,
}

impl CurtainPanel {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                ("symbolid" | "typeid", InstanceField::ElementId { id, .. }) => {
                    out.symbol_id = Some(*id);
                }
                ("uindex", InstanceField::Integer { value, .. }) => {
                    out.u_index = Some(*value as u32);
                }
                ("vindex", InstanceField::Integer { value, .. }) => {
                    out.v_index = Some(*value as u32);
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

    #[test]
    fn curtain_wall_rejects_wrong_schema() {
        use crate::formats::ClassEntry;
        let wrong = ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        assert!(
            CurtainWallDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn curtain_wall_unconnected_detection() {
        let hanging = CurtainWall {
            top_level_id: Some(0),
            unconnected_height_feet: Some(30.0),
            ..Default::default()
        };
        let connected = CurtainWall {
            top_level_id: Some(5),
            ..Default::default()
        };
        assert!(hanging.is_unconnected());
        assert!(!connected.is_unconnected());
    }

    #[test]
    fn curtain_grid_cell_count() {
        // 4×3 grid of lines → 3×2 = 6 panel cells.
        let g = CurtainGrid {
            u_line_count: Some(4),
            v_line_count: Some(3),
            ..Default::default()
        };
        assert_eq!(g.cell_count(), Some(6));
        // Degenerate single-line cases don't panic on u32 subtraction.
        let degen = CurtainGrid {
            u_line_count: Some(1),
            v_line_count: Some(1),
            ..Default::default()
        };
        assert_eq!(degen.cell_count(), Some(0));
        let missing = CurtainGrid::default();
        assert_eq!(missing.cell_count(), None);
    }

    #[test]
    fn curtain_mullion_orientation() {
        let fields = vec![
            (
                "m_host_id".into(),
                InstanceField::ElementId { tag: 0, id: 42 },
            ),
            ("m_is_vertical".into(), InstanceField::Bool(true)),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "CurtainMullion".into(),
            fields,
            byte_range: 0..0,
        };
        let m = CurtainMullion::from_decoded(&decoded);
        assert_eq!(m.host_id, Some(42));
        assert_eq!(m.is_vertical, Some(true));
    }

    #[test]
    fn curtain_panel_indexed() {
        let fields = vec![
            (
                "m_u_index".into(),
                InstanceField::Integer {
                    value: 2,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_v_index".into(),
                InstanceField::Integer {
                    value: 1,
                    signed: false,
                    size: 4,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "CurtainPanel".into(),
            fields,
            byte_range: 0..0,
        };
        let p = CurtainPanel::from_decoded(&decoded);
        assert_eq!(p.u_index, Some(2));
        assert_eq!(p.v_index, Some(1));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "CurtainWall".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(CurtainWall::from_decoded(&empty).base_level_id.is_none());
        assert!(CurtainGrid::from_decoded(&empty).u_line_count.is_none());
        assert!(CurtainMullion::from_decoded(&empty).host_id.is_none());
        assert!(CurtainPanel::from_decoded(&empty).host_id.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(CurtainWallDecoder.class_name(), "CurtainWall");
        assert_eq!(CurtainGridDecoder.class_name(), "CurtainGrid");
        assert_eq!(CurtainMullionDecoder.class_name(), "CurtainMullion");
        assert_eq!(CurtainPanelDecoder.class_name(), "CurtainPanel");
    }
}
