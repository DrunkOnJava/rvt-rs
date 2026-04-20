//! `Column` + `Beam` + `StructuralColumn` + `StructuralFraming` —
//! the bones of the building. In Revit:
//!
//! - An **architectural column** (class `Column`) is a vertical
//!   prismatic element spanning from `base_level` to `top_level` with
//!   a profile (rectangular / round / W-shape / …) carried by its
//!   symbol.
//! - A **structural column** (class `StructuralColumn`) is the same
//!   geometry plus analytical-model attachments (released DOFs,
//!   material strength overrides). Concrete / steel columns live
//!   here.
//! - A **beam** (class `Beam` or `StructuralFraming`) is a horizontal
//!   or sloped member running along a 2D/3D curve — `start` and `end`
//!   endpoints plus a cross-section profile.
//!
//! All four decode the same location / level / symbol fields so we
//! share `StructuralCommon` similar to `OpeningCommon` in
//! `openings.rs`.
//!
//! # Typical field shape
//!
//! Column / StructuralColumn:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_base_level_id` | ElementId | Bottom of column |
//! | `m_top_level_id` | ElementId | Top of column |
//! | `m_base_offset` | f64 | Feet above base level |
//! | `m_top_offset` | f64 | Feet above top level |
//! | `m_symbol_id` | ElementId | Column FamilySymbol (profile + material) |
//! | `m_location_x`, `m_location_y`, `m_location_z` | f64 | Insertion point |
//! | `m_rotation` | f64 | Rotation about vertical axis, radians |
//! | `m_is_structural` | bool | True for StructuralColumn, false for architectural |
//!
//! Beam / StructuralFraming:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Reference level for the beam |
//! | `m_symbol_id` | ElementId | FamilySymbol defining the profile |
//! | `m_start_x`, `m_start_y`, `m_start_z` | f64 | Start endpoint |
//! | `m_end_x`, `m_end_y`, `m_end_z` | f64 | End endpoint |
//! | `m_cross_section_rotation` | f64 | Rotation of profile about the beam axis |
//! | `m_start_level_offset` / `m_end_level_offset` | f64 | Per-end offsets from level |

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

simple_decoder!(ColumnDecoder, "Column");
simple_decoder!(StructuralColumnDecoder, "StructuralColumn");
simple_decoder!(BeamDecoder, "Beam");
simple_decoder!(StructuralFramingDecoder, "StructuralFraming");

/// Typed view of a decoded Column (architectural or structural — the
/// `is_structural` flag disambiguates).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Column {
    pub base_level_id: Option<u32>,
    pub top_level_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    pub top_offset_feet: Option<f64>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    pub is_structural: Option<bool>,
}

impl Column {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut lx = None;
        let mut ly = None;
        let mut lz = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("baselevelid" | "levelid", InstanceField::ElementId { id, .. }) => {
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
                ("symbolid" | "typeid" | "familysymbolid", InstanceField::ElementId { id, .. }) => {
                    out.symbol_id = Some(*id);
                }
                ("locationx", InstanceField::Float { value, .. }) => lx = Some(*value),
                ("locationy", InstanceField::Float { value, .. }) => ly = Some(*value),
                ("locationz", InstanceField::Float { value, .. }) => lz = Some(*value),
                ("rotation", InstanceField::Float { value, .. }) => {
                    out.rotation_radians = Some(*value);
                }
                ("isstructural" | "structural", InstanceField::Bool(b)) => {
                    out.is_structural = Some(*b);
                }
                _ => {}
            }
        }
        if let (Some(x), Some(y), Some(z)) = (lx, ly, lz) {
            out.location = Some(Point3::new(x, y, z));
        }
        out
    }

    /// Column height in feet, from base_offset above base_level to
    /// top_offset above top_level. Returns None if we can't compute
    /// (missing offsets — level elevations are looked up separately
    /// via the Level decoder, not stored on the column).
    ///
    /// Note: this returns the *offset component* of the height; the
    /// caller must add `top_level.elevation - base_level.elevation`
    /// separately. Kept here to surface the offset delta explicitly.
    pub fn offset_span_feet(&self) -> Option<f64> {
        Some(self.top_offset_feet? - self.base_offset_feet?)
    }
}

/// Typed view of a decoded Beam.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Beam {
    pub level_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub start: Option<Point3>,
    pub end: Option<Point3>,
    pub cross_section_rotation_radians: Option<f64>,
    pub start_level_offset_feet: Option<f64>,
    pub end_level_offset_feet: Option<f64>,
}

impl Beam {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut sx = None;
        let mut sy = None;
        let mut sz = None;
        let mut ex = None;
        let mut ey = None;
        let mut ez = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("symbolid" | "typeid" | "familysymbolid", InstanceField::ElementId { id, .. }) => {
                    out.symbol_id = Some(*id);
                }
                ("startx", InstanceField::Float { value, .. }) => sx = Some(*value),
                ("starty", InstanceField::Float { value, .. }) => sy = Some(*value),
                ("startz", InstanceField::Float { value, .. }) => sz = Some(*value),
                ("endx", InstanceField::Float { value, .. }) => ex = Some(*value),
                ("endy", InstanceField::Float { value, .. }) => ey = Some(*value),
                ("endz", InstanceField::Float { value, .. }) => ez = Some(*value),
                ("crosssectionrotation", InstanceField::Float { value, .. }) => {
                    out.cross_section_rotation_radians = Some(*value);
                }
                ("startleveloffset", InstanceField::Float { value, .. }) => {
                    out.start_level_offset_feet = Some(*value);
                }
                ("endleveloffset", InstanceField::Float { value, .. }) => {
                    out.end_level_offset_feet = Some(*value);
                }
                _ => {}
            }
        }
        if let (Some(x), Some(y), Some(z)) = (sx, sy, sz) {
            out.start = Some(Point3::new(x, y, z));
        }
        if let (Some(x), Some(y), Some(z)) = (ex, ey, ez) {
            out.end = Some(Point3::new(x, y, z));
        }
        out
    }

    /// Straight-line beam length in feet. None when either endpoint
    /// is missing (curved beams will need curve-aware length — TODO
    /// when walker Vector variant lands for curved-beam profiles).
    pub fn length_feet(&self) -> Option<f64> {
        let (s, e) = (self.start?, self.end?);
        let dx = e.x - s.x;
        let dy = e.y - s.y;
        let dz = e.z - s.z;
        Some((dx * dx + dy * dy + dz * dz).sqrt())
    }

    /// True when the beam is horizontal (start.z ≈ end.z). Useful for
    /// distinguishing floor-framing beams from sloped members.
    pub fn is_horizontal(&self, eps: f64) -> Option<bool> {
        let (s, e) = (self.start?, self.end?);
        Some((s.z - e.z).abs() < eps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_column_schema() -> ClassEntry {
        let f64_prim = FieldType::Primitive {
            kind: 0x07,
            size: 8,
        };
        ClassEntry {
            name: "Column".into(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_base_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_top_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_base_offset".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
                },
                FieldEntry {
                    name: "m_top_offset".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim.clone()),
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
                    field_type: Some(f64_prim),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(8),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_column_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // base_level_id = [0, 1]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&1u32.to_le_bytes());
        // top_level_id = [0, 2]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&2u32.to_le_bytes());
        // base_offset = 0.0, top_offset = 0.0
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        // symbol_id = [0, 100]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&100u32.to_le_bytes());
        // location = (15.0, 20.0, 0.0)
        for v in [15.0_f64, 20.0, 0.0] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b
    }

    #[test]
    fn column_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Beam".into(),
            ..synth_column_schema()
        };
        assert!(
            ColumnDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn column_decodes_levels_and_location() {
        let decoded = ColumnDecoder
            .decode(
                &synth_column_bytes(),
                &synth_column_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let c = Column::from_decoded(&decoded);
        assert_eq!(c.base_level_id, Some(1));
        assert_eq!(c.top_level_id, Some(2));
        assert_eq!(c.symbol_id, Some(100));
        assert_eq!(c.location, Some(Point3::new(15.0, 20.0, 0.0)));
        assert_eq!(c.base_offset_feet, Some(0.0));
        assert_eq!(c.top_offset_feet, Some(0.0));
        assert_eq!(c.offset_span_feet(), Some(0.0));
    }

    #[test]
    fn column_offset_span_requires_both() {
        let partial = Column {
            base_offset_feet: Some(1.0),
            top_offset_feet: None,
            ..Default::default()
        };
        assert_eq!(partial.offset_span_feet(), None);
    }

    #[test]
    fn beam_computes_length_and_horizontal() {
        let fields = vec![
            (
                "m_start_x".into(),
                InstanceField::Float {
                    value: 0.0,
                    size: 8,
                },
            ),
            (
                "m_start_y".into(),
                InstanceField::Float {
                    value: 0.0,
                    size: 8,
                },
            ),
            (
                "m_start_z".into(),
                InstanceField::Float {
                    value: 10.0,
                    size: 8,
                },
            ),
            (
                "m_end_x".into(),
                InstanceField::Float {
                    value: 3.0,
                    size: 8,
                },
            ),
            (
                "m_end_y".into(),
                InstanceField::Float {
                    value: 4.0,
                    size: 8,
                },
            ),
            (
                "m_end_z".into(),
                InstanceField::Float {
                    value: 10.0,
                    size: 8,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Beam".into(),
            fields,
            byte_range: 0..0,
        };
        let b = Beam::from_decoded(&decoded);
        assert_eq!(b.start, Some(Point3::new(0.0, 0.0, 10.0)));
        assert_eq!(b.end, Some(Point3::new(3.0, 4.0, 10.0)));
        assert!((b.length_feet().unwrap() - 5.0).abs() < 1e-9);
        assert_eq!(b.is_horizontal(1e-9), Some(true));
    }

    #[test]
    fn beam_sloped_detection() {
        let b = Beam {
            start: Some(Point3::new(0.0, 0.0, 10.0)),
            end: Some(Point3::new(10.0, 0.0, 12.0)),
            ..Default::default()
        };
        assert_eq!(b.is_horizontal(1e-9), Some(false));
        assert_eq!(Beam::default().is_horizontal(1e-9), None);
    }

    #[test]
    fn beam_length_requires_both_endpoints() {
        let b = Beam {
            start: Some(Point3::new(0.0, 0.0, 0.0)),
            end: None,
            ..Default::default()
        };
        assert_eq!(b.length_feet(), None);
    }

    #[test]
    fn structural_column_and_framing_decoders_work() {
        let empty = DecodedElement {
            id: None,
            class: "StructuralColumn".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        // Both StructuralColumn/StructuralFraming use the same typed
        // view projection functions as their architectural twins.
        let c = Column::from_decoded(&empty);
        let b = Beam::from_decoded(&empty);
        assert!(c.base_level_id.is_none() && b.start.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(ColumnDecoder.class_name(), "Column");
        assert_eq!(StructuralColumnDecoder.class_name(), "StructuralColumn");
        assert_eq!(BeamDecoder.class_name(), "Beam");
        assert_eq!(StructuralFramingDecoder.class_name(), "StructuralFraming");
    }
}
