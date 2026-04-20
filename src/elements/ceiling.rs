//! `Ceiling` + `CeilingType` — horizontal surface hung below a level,
//! typically used for dropped acoustic tile ceilings or bulkhead
//! soffits. Like `Floor` it's a 2D sketched boundary extruded
//! downward by a thickness.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! Ceiling:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level — ceiling top is at `level.elevation + height_offset` |
//! | `m_height_offset` | f64 | Drop below level in feet (negative for suspended ceilings) |
//! | `m_type_id` | ElementId | Reference to `CeilingType` |
//! | `m_room_bounding` | Primitive bool | Bounds rooms? Affects room area reporting |
//!
//! CeilingType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Compound Ceiling", "2' x 2' ACT System", "Gypsum Board" |
//! | `m_thickness` | f64 | Assembly thickness in feet |

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Registered decoder for the `Ceiling` class.
pub struct CeilingDecoder;

impl ElementDecoder for CeilingDecoder {
    fn class_name(&self) -> &'static str {
        "Ceiling"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Ceiling" {
            return Err(Error::BasicFileInfo(format!(
                "CeilingDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `CeilingType` class.
pub struct CeilingTypeDecoder;

impl ElementDecoder for CeilingTypeDecoder {
    fn class_name(&self) -> &'static str {
        "CeilingType"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "CeilingType" {
            return Err(Error::BasicFileInfo(format!(
                "CeilingTypeDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded Ceiling instance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ceiling {
    pub level_id: Option<u32>,
    /// Offset from the host level. Typically negative (ceiling is
    /// below the level). `-0.66667` = "suspended 8" below level".
    pub height_offset_feet: Option<f64>,
    pub type_id: Option<u32>,
    pub room_bounding: Option<bool>,
}

impl Ceiling {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("heightoffset" | "offset", InstanceField::Float { value, .. }) => {
                    out.height_offset_feet = Some(*value);
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                ("roombounding" | "isroombounding", InstanceField::Bool(b)) => {
                    out.room_bounding = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// Drop distance below level in inches (always positive). None
    /// when offset is missing or ≥ 0 (ceiling flush with level top).
    pub fn drop_inches(&self) -> Option<f64> {
        let h = self.height_offset_feet?;
        if h >= 0.0 { None } else { Some(-h * 12.0) }
    }
}

/// Typed view of a decoded CeilingType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CeilingType {
    pub name: Option<String>,
    pub thickness_feet: Option<f64>,
}

impl CeilingType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("thickness" | "width", InstanceField::Float { value, .. }) => {
                    out.thickness_feet = Some(*value);
                }
                _ => {}
            }
        }
        out
    }

    pub fn thickness_inches(&self) -> Option<f64> {
        self.thickness_feet.map(|ft| ft * 12.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_ceiling_schema() -> ClassEntry {
        ClassEntry {
            name: "Ceiling".into(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_height_offset".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_type_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_room_bounding".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(4),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_ceiling_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_level_id = [0, 5]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&5u32.to_le_bytes());
        // m_height_offset = -0.75  (9" drop)
        b.extend_from_slice(&(-0.75_f64).to_le_bytes());
        // m_type_id = [0, 8]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&8u32.to_le_bytes());
        // m_room_bounding = true
        b.push(1);
        b
    }

    #[test]
    fn ceiling_decoder_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Floor".into(),
            ..synth_ceiling_schema()
        };
        assert!(
            CeilingDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn ceiling_decodes_and_computes_drop() {
        let decoded = CeilingDecoder
            .decode(
                &synth_ceiling_bytes(),
                &synth_ceiling_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let c = Ceiling::from_decoded(&decoded);
        assert_eq!(c.level_id, Some(5));
        assert_eq!(c.height_offset_feet, Some(-0.75));
        assert_eq!(c.type_id, Some(8));
        assert_eq!(c.room_bounding, Some(true));
        assert_eq!(c.drop_inches(), Some(9.0));
    }

    #[test]
    fn drop_inches_none_when_not_suspended() {
        let flush = Ceiling {
            height_offset_feet: Some(0.0),
            ..Default::default()
        };
        let above = Ceiling {
            height_offset_feet: Some(0.5),
            ..Default::default()
        };
        let missing = Ceiling::default();
        assert_eq!(flush.drop_inches(), None);
        assert_eq!(above.drop_inches(), None);
        assert_eq!(missing.drop_inches(), None);
    }

    #[test]
    fn ceiling_type_tolerates_empty() {
        let empty = DecodedElement {
            id: None,
            class: "CeilingType".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let ct = CeilingType::from_decoded(&empty);
        assert!(ct.name.is_none() && ct.thickness_feet.is_none());
        assert_eq!(ct.thickness_inches(), None);
    }

    #[test]
    fn class_names() {
        assert_eq!(CeilingDecoder.class_name(), "Ceiling");
        assert_eq!(CeilingTypeDecoder.class_name(), "CeilingType");
    }
}
