//! `BasePoint` / `SurveyPoint` / `ProjectPosition` — the three
//! classes that establish a Revit project's coordinate system.
//!
//! Understanding these is essential for every geometry extraction.
//! The transform chain is:
//!
//! ```text
//! world (survey) coords
//!     │
//!     │ ProjectPosition (rotation + translation)
//!     ▼
//! project (base) coords
//!     │
//!     │ per-element placement transform
//!     ▼
//! element local coords
//! ```
//!
//! - **BasePoint**: the origin of project coordinates. Every wall
//!   curve / floor boundary / door placement is relative to this.
//! - **SurveyPoint**: the origin of world/survey coordinates —
//!   typically shared across buildings on a site, aligned to true
//!   north.
//! - **ProjectPosition**: the transform between the two. Stores
//!   (rotation_angle, dx, dy, dz) so the IFC exporter can emit a
//!   correct IfcSite placement without hard-coding identity.
//!
//! All three emit into [`crate::geometry::Point3`] / `Vector3` /
//! `Transform3` for downstream consumers.

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::{Point3, Transform3, Vector3};
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

simple_decoder!(BasePointDecoder, "BasePoint");
simple_decoder!(SurveyPointDecoder, "SurveyPoint");
simple_decoder!(ProjectPositionDecoder, "ProjectPosition");

/// Typed view of a decoded BasePoint.
///
/// The project coordinate origin. Field names observed across the
/// 11-release corpus use `m_x` / `m_y` / `m_z` for position + an
/// elevation field that sometimes overlaps m_z (depends on whether
/// the file is a family or a project).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BasePoint {
    pub position: Option<Point3>,
    pub angle_radians: Option<f64>,
    pub is_project_base: Option<bool>,
}

impl BasePoint {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = extract_point_common(decoded);
        let position = if c.x.is_some() || c.y.is_some() || c.z.is_some() {
            Some(Point3::new(
                c.x.unwrap_or(0.0),
                c.y.unwrap_or(0.0),
                c.z.unwrap_or(0.0),
            ))
        } else {
            None
        };
        Self {
            position,
            angle_radians: c.angle,
            is_project_base: c.is_proj,
        }
    }
}

/// Typed view of a decoded SurveyPoint.
///
/// The world/survey coordinate origin. Like BasePoint but with an
/// `angle_to_true_north` field that describes project-to-true-north
/// rotation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SurveyPoint {
    pub position: Option<Point3>,
    pub angle_to_true_north: Option<f64>,
    pub elevation: Option<f64>,
}

impl SurveyPoint {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = extract_point_common(decoded);
        let position = if c.x.is_some() || c.y.is_some() || c.z.is_some() {
            Some(Point3::new(
                c.x.unwrap_or(0.0),
                c.y.unwrap_or(0.0),
                c.z.unwrap_or(0.0),
            ))
        } else {
            None
        };
        let elevation = c.z.or_else(|| {
            // Some versions expose a dedicated m_elevation separate
            // from m_z; fall back to that when present.
            decoded.fields.iter().find_map(|(n, v)| {
                if normalise_field_name(n) == "elevation" {
                    if let InstanceField::Float { value, .. } = v {
                        return Some(*value);
                    }
                }
                None
            })
        });
        Self {
            position,
            angle_to_true_north: c.angle,
            elevation,
        }
    }
}

/// Typed view of a decoded ProjectPosition.
///
/// Stores rotation (angle from true north) + translation
/// (dx, dy, dz) between project and survey coordinate systems.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProjectPosition {
    pub rotation_radians: Option<f64>,
    pub translation: Option<Vector3>,
}

impl ProjectPosition {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut rotation_radians = None;
        let mut tx = None;
        let mut ty = None;
        let mut tz = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("angle" | "rotation", InstanceField::Float { value, .. }) => {
                    rotation_radians = Some(*value);
                }
                ("dx" | "offsetx", InstanceField::Float { value, .. }) => tx = Some(*value),
                ("dy" | "offsety", InstanceField::Float { value, .. }) => ty = Some(*value),
                ("dz" | "offsetz" | "elevation", InstanceField::Float { value, .. }) => {
                    tz = Some(*value);
                }
                _ => {}
            }
        }
        let translation = if tx.is_some() || ty.is_some() || tz.is_some() {
            Some(Vector3::new(
                tx.unwrap_or(0.0),
                ty.unwrap_or(0.0),
                tz.unwrap_or(0.0),
            ))
        } else {
            None
        };
        Self {
            rotation_radians,
            translation,
        }
    }

    /// Compose into a Transform3 that maps project coordinates to
    /// survey coordinates. Identity when rotation + translation
    /// are both None.
    pub fn to_transform(&self) -> Transform3 {
        let (sin, cos) = self
            .rotation_radians
            .map(|a| a.sin_cos())
            .unwrap_or((0.0, 1.0));
        let t = self.translation.unwrap_or(Vector3::new(0.0, 0.0, 0.0));
        Transform3 {
            origin: Point3::new(t.x, t.y, t.z),
            x_axis: Vector3::new(cos, sin, 0.0),
            y_axis: Vector3::new(-sin, cos, 0.0),
            z_axis: Vector3::new(0.0, 0.0, 1.0),
            scale: 1.0,
        }
    }
}

/// Parsed common fields from a BasePoint or SurveyPoint record —
/// the five values both classes share (position components +
/// rotation + optional is_project_base flag).
#[derive(Debug, Clone, Copy, Default)]
struct PointCommon {
    x: Option<f64>,
    y: Option<f64>,
    z: Option<f64>,
    angle: Option<f64>,
    is_proj: Option<bool>,
}

/// Extract the common fields that BasePoint + SurveyPoint share.
fn extract_point_common(decoded: &DecodedElement) -> PointCommon {
    let mut out = PointCommon::default();
    for (field_name, value) in &decoded.fields {
        match (normalise_field_name(field_name).as_str(), value) {
            ("x" | "px", InstanceField::Float { value, .. }) => out.x = Some(*value),
            ("y" | "py", InstanceField::Float { value, .. }) => out.y = Some(*value),
            ("z" | "pz", InstanceField::Float { value, .. }) => out.z = Some(*value),
            (
                "angle" | "anglefromnorth" | "angletotruenorth",
                InstanceField::Float { value, .. },
            ) => out.angle = Some(*value),
            ("isprojectbase" | "isbase" | "projectbase", InstanceField::Bool(b)) => {
                out.is_proj = Some(*b);
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_basepoint_schema() -> ClassEntry {
        ClassEntry {
            name: "BasePoint".to_string(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_x".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_y".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_z".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_angle".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_is_project_base".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
            ],
            tag: Some(7),
            parent: None,
            declared_field_count: Some(5),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_basepoint_bytes(x: f64, y: f64, z: f64, angle: f64, is_proj: bool) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&x.to_le_bytes());
        b.extend_from_slice(&y.to_le_bytes());
        b.extend_from_slice(&z.to_le_bytes());
        b.extend_from_slice(&angle.to_le_bytes());
        b.push(if is_proj { 1 } else { 0 });
        b
    }

    #[test]
    fn basepoint_roundtrip() {
        let decoded = BasePointDecoder
            .decode(
                &synth_basepoint_bytes(10.0, 20.0, 5.0, 0.1, true),
                &synth_basepoint_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let bp = BasePoint::from_decoded(&decoded);
        assert_eq!(bp.position, Some(Point3::new(10.0, 20.0, 5.0)));
        assert!((bp.angle_radians.unwrap() - 0.1).abs() < 1e-9);
        assert_eq!(bp.is_project_base, Some(true));
    }

    #[test]
    fn surveypoint_decodes_from_basepoint_layout() {
        // SurveyPoint has the same field shape as BasePoint for
        // position + angle; the "is_project_base" field is ignored.
        let schema = ClassEntry {
            name: "SurveyPoint".to_string(),
            ..synth_basepoint_schema()
        };
        let decoded = SurveyPointDecoder
            .decode(
                &synth_basepoint_bytes(100.0, 200.0, 0.0, 0.5, false),
                &schema,
                &HandleIndex::new(),
            )
            .unwrap();
        let sp = SurveyPoint::from_decoded(&decoded);
        assert_eq!(sp.position, Some(Point3::new(100.0, 200.0, 0.0)));
        assert!((sp.angle_to_true_north.unwrap() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn project_position_to_transform_identity_when_empty() {
        let pp = ProjectPosition::default();
        let t = pp.to_transform();
        assert_eq!(t.origin, Point3::new(0.0, 0.0, 0.0));
        assert_eq!(t.x_axis, Vector3::new(1.0, 0.0, 0.0));
        assert_eq!(t.y_axis, Vector3::new(0.0, 1.0, 0.0));
        assert_eq!(t.z_axis, Vector3::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn project_position_to_transform_with_values() {
        let pp = ProjectPosition {
            rotation_radians: Some(std::f64::consts::FRAC_PI_2),
            translation: Some(Vector3::new(10.0, 20.0, 0.0)),
        };
        let t = pp.to_transform();
        // PI/2 rotation: x_axis should point to (0, 1, 0); y_axis to (-1, 0, 0).
        assert!((t.x_axis.x - 0.0).abs() < 1e-10);
        assert!((t.x_axis.y - 1.0).abs() < 1e-10);
        assert!((t.y_axis.x - -1.0).abs() < 1e-10);
        assert!((t.y_axis.y - 0.0).abs() < 1e-10);
        assert_eq!(t.origin, Point3::new(10.0, 20.0, 0.0));
    }

    #[test]
    fn all_three_decoders_reject_wrong_schema() {
        let wrong = ClassEntry {
            name: "NotAPoint".to_string(),
            ..synth_basepoint_schema()
        };
        assert!(
            BasePointDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
        assert!(
            SurveyPointDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
        assert!(
            ProjectPositionDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn decoder_class_names() {
        assert_eq!(BasePointDecoder.class_name(), "BasePoint");
        assert_eq!(SurveyPointDecoder.class_name(), "SurveyPoint");
        assert_eq!(ProjectPositionDecoder.class_name(), "ProjectPosition");
    }

    #[test]
    fn basepoint_tolerates_missing_position() {
        let empty = DecodedElement {
            id: None,
            class: "BasePoint".to_string(),
            fields: vec![],
            byte_range: 0..0,
        };
        let bp = BasePoint::from_decoded(&empty);
        assert!(bp.position.is_none());
    }
}
