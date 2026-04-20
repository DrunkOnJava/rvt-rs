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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_schema() -> ClassEntry {
        let f64_prim = FieldType::Primitive {
            kind: 0x07,
            size: 8,
        };
        ClassEntry {
            name: "ReferencePlane".into(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_name".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_bubble_end_x".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_bubble_end_y".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_bubble_end_z".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_free_end_x".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_free_end_y".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_free_end_z".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_cut_vec_x".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_cut_vec_y".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_cut_vec_z".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(10),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_name = "Workplane 1"
        let name = "Workplane 1";
        b.extend_from_slice(&(name.chars().count() as u32).to_le_bytes());
        for ch in name.encode_utf16() {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        // bubble_end = (1.0, 2.0, 3.0)
        for v in [1.0_f64, 2.0, 3.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // free_end = (4.0, 2.0, 3.0)  — horizontal in X
        for v in [4.0_f64, 2.0, 3.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        // cut_vec = (0.0, 1.0, 0.0)  — normal in +Y (vertical plane parallel to XZ)
        for v in [0.0_f64, 1.0, 0.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    }

    #[test]
    fn reference_plane_decoder_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Grid".into(),
            ..synth_schema()
        };
        assert!(
            ReferencePlaneDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn reference_plane_decodes_points_and_normal() {
        let decoded = ReferencePlaneDecoder
            .decode(&synth_bytes(), &synth_schema(), &HandleIndex::new())
            .unwrap();
        let p = ReferencePlane::from_decoded(&decoded);
        assert_eq!(p.name.as_deref(), Some("Workplane 1"));
        assert_eq!(p.bubble_end, Some(Point3::new(1.0, 2.0, 3.0)));
        assert_eq!(p.free_end, Some(Point3::new(4.0, 2.0, 3.0)));
        assert_eq!(p.normal, Some(Vector3::new(0.0, 1.0, 0.0)));
    }

    #[test]
    fn in_plane_direction_computed() {
        let p = ReferencePlane {
            bubble_end: Some(Point3::new(1.0, 2.0, 3.0)),
            free_end: Some(Point3::new(4.0, 2.0, 3.0)),
            ..Default::default()
        };
        let d = p.in_plane_direction().unwrap();
        assert!((d.x - 3.0).abs() < 1e-12);
        assert!(d.y.abs() < 1e-12);
        assert!(d.z.abs() < 1e-12);
    }

    #[test]
    fn vertical_detection() {
        let vertical = ReferencePlane {
            normal: Some(Vector3::new(1.0, 0.0, 0.0)),
            ..Default::default()
        };
        let horizontal = ReferencePlane {
            normal: Some(Vector3::new(0.0, 0.0, 1.0)),
            ..Default::default()
        };
        let missing = ReferencePlane::default();
        assert_eq!(vertical.is_vertical(1e-9), Some(true));
        assert_eq!(horizontal.is_vertical(1e-9), Some(false));
        assert_eq!(missing.is_vertical(1e-9), None);
    }

    #[test]
    fn tolerates_empty_decoded() {
        let empty = DecodedElement {
            id: None,
            class: "ReferencePlane".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let p = ReferencePlane::from_decoded(&empty);
        assert!(p.name.is_none());
        assert!(p.bubble_end.is_none());
        assert!(p.free_end.is_none());
        assert!(p.normal.is_none());
    }

    #[test]
    fn class_name() {
        assert_eq!(ReferencePlaneDecoder.class_name(), "ReferencePlane");
    }
}
