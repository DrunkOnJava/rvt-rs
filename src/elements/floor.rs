//! `Floor` — a horizontal slab element. In Revit a floor is typically
//! a multi-layer compound structure extruded from a 2D boundary
//! sketch, hosted on a Level.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level — slab top is at `level.elevation + height_offset` |
//! | `m_height_offset` | f64 | Offset from level in feet (can be negative for depressed slabs) |
//! | `m_structural` | Primitive bool | Participates in analytical model? |
//! | `m_is_slab_edge` | Primitive bool | True for balcony edges, ramps, floor edges |
//! | `m_type_id` | ElementId | Reference to the floor's type |
//! | `m_span_direction` | f64 | Radians — direction one-way slab spans (for reinforcement) |
//!
//! The 2D boundary sketch (edges of the floor in plan view) lives in
//! the ADocument element table, reachable via the element's history
//! chain. Geometry assembly is not done here.

use super::level::normalise_field_name;
use crate::walker::{DecodedElement, InstanceField};

/// Typed view of a decoded Floor instance.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Floor {
    pub level_id: Option<crate::ElementId>,
    /// Offset from host level's elevation. Can be negative.
    pub height_offset_feet: Option<f64>,
    pub structural: Option<bool>,
    pub is_slab_edge: Option<bool>,
    pub type_id: Option<crate::ElementId>,
    pub span_direction_radians: Option<f64>,
}

impl Floor {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(crate::ElementId::from(*id));
                }
                ("heightoffset" | "offset", InstanceField::Float { value, .. }) => {
                    out.height_offset_feet = Some(*value);
                }
                ("structural" | "isstructural", InstanceField::Bool(b)) => {
                    out.structural = Some(*b);
                }
                ("isslabedge" | "slabedge", InstanceField::Bool(b)) => {
                    out.is_slab_edge = Some(*b);
                }
                ("typeid", InstanceField::ElementId { id, .. }) => {
                    out.type_id = Some(crate::ElementId::from(*id))
                }
                ("spandirection", InstanceField::Float { value, .. }) => {
                    out.span_direction_radians = Some(*value);
                }
                _ => {}
            }
        }
        out
    }

    /// True for depressed slabs (e.g. shower floors, depressed bays).
    pub fn is_depressed(&self) -> Option<bool> {
        self.height_offset_feet.map(|h| h < 0.0)
    }
}
