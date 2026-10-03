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
