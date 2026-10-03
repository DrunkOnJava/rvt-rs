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
