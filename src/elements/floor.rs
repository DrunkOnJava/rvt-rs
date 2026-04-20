//! `Floor` + `FloorType` — a horizontal slab element. In Revit a
//! floor is typically a multi-layer compound structure extruded from
//! a 2D boundary sketch, hosted on a Level.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! Floor:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level — slab top is at `level.elevation + height_offset` |
//! | `m_height_offset` | f64 | Offset from level in feet (can be negative for depressed slabs) |
//! | `m_structural` | Primitive bool | Participates in analytical model? |
//! | `m_is_slab_edge` | Primitive bool | True for balcony edges, ramps, floor edges |
//! | `m_type_id` | ElementId | Reference to `FloorType` |
//! | `m_span_direction` | f64 | Radians — direction one-way slab spans (for reinforcement) |
//!
//! FloorType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Generic 12\"", "Wood Joist 10\" — Wood Finish" |
//! | `m_function` | Primitive u32 | 0=Interior 1=Exterior |
//! | `m_thickness` | f64 | Total slab thickness in feet (sum of layers) |
//! | `m_structural` | Primitive bool | Default load-bearing? |
//!
//! The 2D boundary sketch (edges of the floor in plan view) lives in
//! the ADocument element table, reachable via the element's history
//! chain — geometry assembly is task GEO-28.

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Floor function classification — mirrors Revit's two-value enum
/// (Revit doesn't split floors as finely as walls).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FloorFunction {
    #[default]
    Interior,
    Exterior,
}

impl FloorFunction {
    pub fn from_code(code: u32) -> Self {
        if code == 1 {
            Self::Exterior
        } else {
            Self::Interior
        }
    }

    /// Map to IfcSlabTypeEnum.
    pub fn to_ifc_predefined(self) -> &'static str {
        match self {
            Self::Interior => "FLOOR",
            Self::Exterior => "ROOF",
        }
    }
}

/// Registered decoder for the `Floor` class.
pub struct FloorDecoder;

impl ElementDecoder for FloorDecoder {
    fn class_name(&self) -> &'static str {
        "Floor"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Floor" {
            return Err(Error::BasicFileInfo(format!(
                "FloorDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `FloorType` class.
pub struct FloorTypeDecoder;

impl ElementDecoder for FloorTypeDecoder {
    fn class_name(&self) -> &'static str {
        "FloorType"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "FloorType" {
            return Err(Error::BasicFileInfo(format!(
                "FloorTypeDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded Floor instance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Floor {
    pub level_id: Option<u32>,
    /// Offset from host level's elevation. Can be negative.
    pub height_offset_feet: Option<f64>,
    pub structural: Option<bool>,
    pub is_slab_edge: Option<bool>,
    pub type_id: Option<u32>,
    pub span_direction_radians: Option<f64>,
}

impl Floor {
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
                ("structural" | "isstructural", InstanceField::Bool(b)) => {
                    out.structural = Some(*b);
                }
                ("isslabedge" | "slabedge", InstanceField::Bool(b)) => {
                    out.is_slab_edge = Some(*b);
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
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

/// Typed view of a decoded FloorType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FloorType {
    pub name: Option<String>,
    pub function: Option<FloorFunction>,
    pub thickness_feet: Option<f64>,
    pub structural: Option<bool>,
}

impl FloorType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("function", InstanceField::Integer { value, .. }) => {
                    out.function = Some(FloorFunction::from_code(*value as u32));
                }
                ("thickness" | "width", InstanceField::Float { value, .. }) => {
                    out.thickness_feet = Some(*value);
                }
                ("structural" | "isstructural", InstanceField::Bool(b)) => {
                    out.structural = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// Total slab thickness in inches.
    pub fn thickness_inches(&self) -> Option<f64> {
        self.thickness_feet.map(|ft| ft * 12.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_floor_schema() -> ClassEntry {
        ClassEntry {
            name: "Floor".into(),
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
                    name: "m_structural".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
                FieldEntry {
                    name: "m_type_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(4),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_floor_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_level_id = [0, 10]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&10u32.to_le_bytes());
        // m_height_offset = -0.5  (depressed slab)
        b.extend_from_slice(&(-0.5_f64).to_le_bytes());
        // m_structural = true
        b.push(1);
        // m_type_id = [0, 3]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&3u32.to_le_bytes());
        b
    }

    #[test]
    fn floor_decoder_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Wall".into(),
            ..synth_floor_schema()
        };
        assert!(
            FloorDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn floor_decodes_and_detects_depressed() {
        let decoded = FloorDecoder
            .decode(
                &synth_floor_bytes(),
                &synth_floor_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let f = Floor::from_decoded(&decoded);
        assert_eq!(f.level_id, Some(10));
        assert_eq!(f.height_offset_feet, Some(-0.5));
        assert_eq!(f.structural, Some(true));
        assert_eq!(f.type_id, Some(3));
        assert_eq!(f.is_depressed(), Some(true));
    }

    #[test]
    fn floor_depressed_detection_boundary() {
        let flat = Floor {
            height_offset_feet: Some(0.0),
            ..Default::default()
        };
        let raised = Floor {
            height_offset_feet: Some(1.5),
            ..Default::default()
        };
        let missing = Floor::default();
        assert_eq!(flat.is_depressed(), Some(false));
        assert_eq!(raised.is_depressed(), Some(false));
        assert_eq!(missing.is_depressed(), None);
    }

    #[test]
    fn floor_type_thickness_conversion() {
        let ft = FloorType {
            thickness_feet: Some(1.0),
            ..Default::default()
        };
        assert_eq!(ft.thickness_inches(), Some(12.0));
        assert_eq!(FloorType::default().thickness_inches(), None);
    }

    #[test]
    fn floor_function_mapping() {
        assert_eq!(FloorFunction::from_code(0), FloorFunction::Interior);
        assert_eq!(FloorFunction::from_code(1), FloorFunction::Exterior);
        assert_eq!(FloorFunction::Interior.to_ifc_predefined(), "FLOOR");
        assert_eq!(FloorFunction::Exterior.to_ifc_predefined(), "ROOF");
    }

    #[test]
    fn floor_type_tolerates_empty() {
        let empty = DecodedElement {
            id: None,
            class: "FloorType".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let ft = FloorType::from_decoded(&empty);
        assert!(ft.name.is_none() && ft.thickness_feet.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(FloorDecoder.class_name(), "Floor");
        assert_eq!(FloorTypeDecoder.class_name(), "FloorType");
    }
}
