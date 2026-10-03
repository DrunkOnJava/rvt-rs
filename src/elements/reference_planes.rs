//! `ReferencePlane` — a user-defined 2D plane in the project. Unlike
//! `Grid` (which is itself infinite but lives on a single elevation)
//! a reference plane is a full 3D plane used as a work plane for
//! sketching, for constraining elements, and as the host for
//! face-hosted families.
//!
//! The file stores the plane as two endpoints plus a cut vector. The
//! plane normal is `cut_vec`; the plane's in-plane direction is
//! `end - start`, normalised. Together they define a right-handed
//! frame whose origin is `start`.
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | User-visible name ("Workplane 1", …) |
//! | `m_bubble_end_x`, `m_bubble_end_y`, `m_bubble_end_z` | f64 | Bubble-end point (labelled end) |
//! | `m_free_end_x`, `m_free_end_y`, `m_free_end_z` | f64 | Free-end point |
//! | `m_cut_vec_x`, `m_cut_vec_y`, `m_cut_vec_z` | f64 | Plane normal |
//! | `m_is_template` | Primitive bool | Template planes aren't view-specific |
//! | `m_owner_view_id` | ElementId | View this plane was created in (0 if model) |
//!
//! The IFC exporter consumes these to emit `IfcPlane` wrapped in an
//! `IfcAxis2Placement3D` for any face-hosted family that references
//! the plane.

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::{Point3, Vector3};
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Registered decoder for the `ReferencePlane` class.
pub struct ReferencePlaneDecoder;

impl ElementDecoder for ReferencePlaneDecoder {
    fn class_name(&self) -> &'static str {
        "ReferencePlane"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "ReferencePlane" {
            return Err(Error::BasicFileInfo(format!(
                "ReferencePlaneDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded ReferencePlane.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReferencePlane {
    pub name: Option<String>,
    /// The "bubble" end is the one that carries the plane's name tag.
    pub bubble_end: Option<Point3>,
    pub free_end: Option<Point3>,
    /// Normal to the plane.
    pub normal: Option<Vector3>,
    pub is_template: Option<bool>,
    /// View the plane was created in; `0` or absent means model-level.
    pub owner_view_id: Option<u32>,
}

impl ReferencePlane {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut bx = None;
        let mut by = None;
        let mut bz = None;
        let mut fx = None;
        let mut fy = None;
        let mut fz = None;
        let mut nx = None;
        let mut ny = None;
        let mut nz = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("bubbleendx", InstanceField::Float { value, .. }) => bx = Some(*value),
                ("bubbleendy", InstanceField::Float { value, .. }) => by = Some(*value),
                ("bubbleendz", InstanceField::Float { value, .. }) => bz = Some(*value),
                ("freeendx", InstanceField::Float { value, .. }) => fx = Some(*value),
                ("freeendy", InstanceField::Float { value, .. }) => fy = Some(*value),
                ("freeendz", InstanceField::Float { value, .. }) => fz = Some(*value),
                ("cutvecx", InstanceField::Float { value, .. }) => nx = Some(*value),
                ("cutvecy", InstanceField::Float { value, .. }) => ny = Some(*value),
                ("cutvecz", InstanceField::Float { value, .. }) => nz = Some(*value),
                ("istemplate", InstanceField::Bool(b)) => out.is_template = Some(*b),
                ("ownerviewid", InstanceField::ElementId { id, .. }) => {
                    out.owner_view_id = Some(*id);
                }
                _ => {}
            }
        }
        if let (Some(x), Some(y), Some(z)) = (bx, by, bz) {
            out.bubble_end = Some(Point3::new(x, y, z));
        }
        if let (Some(x), Some(y), Some(z)) = (fx, fy, fz) {
            out.free_end = Some(Point3::new(x, y, z));
        }
        if let (Some(x), Some(y), Some(z)) = (nx, ny, nz) {
            out.normal = Some(Vector3::new(x, y, z));
        }
        out
    }

    /// In-plane direction from `bubble_end` to `free_end`. Returns
    /// `None` if either endpoint is missing.
    ///
    /// This vector is NOT normalised — callers that need a unit
    /// vector should normalise it themselves (we avoid the divide
    /// here to preserve exact zero lengths in tests).
    pub fn in_plane_direction(&self) -> Option<Vector3> {
        let b = self.bubble_end?;
        let f = self.free_end?;
        Some(Vector3::new(f.x - b.x, f.y - b.y, f.z - b.z))
    }

    /// `true` when the plane is vertical (normal has no Z component
    /// within `eps`). Most user work planes in architectural models
    /// are either purely horizontal or purely vertical.
    pub fn is_vertical(&self, eps: f64) -> Option<bool> {
        let n = self.normal?;
        Some(n.z.abs() < eps)
    }
}
