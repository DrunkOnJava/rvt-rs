//! `Wall` + `WallType` — the single highest-leverage element class in
//! any architectural Revit model. Decoded here into stable typed
//! views; geometry assembly (extrusion from location curve + layered
//! compound structure) lives in `src/geometry/` and is task GEO-27.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! Wall:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Base level (from `Level` decoder) |
//! | `m_base_offset` | f64 | Height above base level (feet) |
//! | `m_top_level_id` | ElementId | Top level, or 0 if "unconnected" |
//! | `m_top_offset` | f64 | Height offset above top level |
//! | `m_unconnected_height` | f64 | Used when `m_top_level_id == 0` |
//! | `m_structural_usage` | Primitive u32 | 0=NonBearing 1=Bearing 2=Shear 3=Combined |
//! | `m_orientation` | Primitive u32 | 0=Interior 1=Exterior |
//! | `m_location_line` | Primitive u32 | 0=Centerline 1=Core-Centerline 2=Finish-Exterior … |
//! | `m_type_id` | ElementId | Reference to `WallType` |
//! | `m_host_id` | ElementId | Host element, 0 = model-hosted |
//!
//! The 2D `location curve` (the line in plan view that the wall was
//! drawn along) is stored by the ADocument walker separately — the
//! `location_curve_id` here is a handle into that table. Wiring that
//! up is task L5B-01 + GEO-27.
//!
//! WallType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Generic - 8\"", "Basic Wall", … |
//! | `m_kind` | Primitive u32 | 0=Basic 1=Curtain 2=Stacked |
//! | `m_function` | Primitive u32 | 0=Interior 1=Exterior 2=Foundation 3=Retaining 4=Soffit 5=CoreShaft |
//! | `m_width` | f64 | Total wall thickness in feet (sum of layers) |
//! | `m_structural` | Primitive bool | Load-bearing by default? |
//!
//! Multi-layer compound structure (`m_compound`) is a nested structure
//! that the walker's Vector variant (L5B-08) will surface later. For
//! now we capture the scalar fields.

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// How a wall's load-bearing role is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StructuralUsage {
    #[default]
    NonBearing,
    Bearing,
    Shear,
    Combined,
}

impl StructuralUsage {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Bearing,
            2 => Self::Shear,
            3 => Self::Combined,
            _ => Self::NonBearing,
        }
    }
}

/// Which side of the location line represents the "face" Revit shows
/// in outlines and which side receives finish layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocationLine {
    #[default]
    WallCenterline,
    CoreCenterline,
    FinishFaceExterior,
    FinishFaceInterior,
    CoreFaceExterior,
    CoreFaceInterior,
}

impl LocationLine {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::CoreCenterline,
            2 => Self::FinishFaceExterior,
            3 => Self::FinishFaceInterior,
            4 => Self::CoreFaceExterior,
            5 => Self::CoreFaceInterior,
            _ => Self::WallCenterline,
        }
    }
}

/// The three wall classifications Revit stores at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallKind {
    #[default]
    Basic,
    Curtain,
    Stacked,
}

impl WallKind {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Curtain,
            2 => Self::Stacked,
            _ => Self::Basic,
        }
    }

    /// `true` when the wall is a plain Basic wall (most common case).
    pub fn is_basic(self) -> bool {
        matches!(self, Self::Basic)
    }
}

/// The functional role a wall plays architecturally — drives IFC
/// predefined-type tagging (`INTERNAL`, `PARTITIONING`, `PARAPET`…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallFunction {
    #[default]
    Interior,
    Exterior,
    Foundation,
    Retaining,
    Soffit,
    CoreShaft,
}

impl WallFunction {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Exterior,
            2 => Self::Foundation,
            3 => Self::Retaining,
            4 => Self::Soffit,
            5 => Self::CoreShaft,
            _ => Self::Interior,
        }
    }

    /// Map to the closest IFC `IfcWallTypeEnum` predefined type.
    pub fn to_ifc_predefined(self) -> &'static str {
        match self {
            Self::Interior => "PARTITIONING",
            Self::Exterior => "STANDARD",
            Self::Foundation => "ELEMENTEDWALL",
            Self::Retaining => "SOLIDWALL",
            Self::Soffit => "PARAPET",
            Self::CoreShaft => "SHEAR",
        }
    }
}

/// Registered decoder for the `Wall` class.
pub struct WallDecoder;

impl ElementDecoder for WallDecoder {
    fn class_name(&self) -> &'static str {
        "Wall"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Wall" {
            return Err(Error::BasicFileInfo(format!(
                "WallDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `WallType` class.
pub struct WallTypeDecoder;

impl ElementDecoder for WallTypeDecoder {
    fn class_name(&self) -> &'static str {
        "WallType"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "WallType" {
            return Err(Error::BasicFileInfo(format!(
                "WallTypeDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded Wall instance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wall {
    pub base_level_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    /// `0` or `None` means the wall is "unconnected" at the top.
    pub top_level_id: Option<u32>,
    pub top_offset_feet: Option<f64>,
    pub unconnected_height_feet: Option<f64>,
    pub structural_usage: Option<StructuralUsage>,
    pub location_line: Option<LocationLine>,
    pub type_id: Option<u32>,
    pub host_id: Option<u32>,
}

impl Wall {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "baselevelid", InstanceField::ElementId { id, .. }) => {
                    out.base_level_id = Some(*id);
                }
                ("baseoffset", InstanceField::Float { value, .. }) => {
                    out.base_offset_feet = Some(*value);
                }
                ("toplevelid", InstanceField::ElementId { id, .. }) => {
                    out.top_level_id = Some(*id);
                }
                ("topoffset", InstanceField::Float { value, .. }) => {
                    out.top_offset_feet = Some(*value);
                }
                ("unconnectedheight", InstanceField::Float { value, .. }) => {
                    out.unconnected_height_feet = Some(*value);
                }
                ("structuralusage", InstanceField::Integer { value, .. }) => {
                    out.structural_usage = Some(StructuralUsage::from_code(*value as u32));
                }
                ("locationline", InstanceField::Integer { value, .. }) => {
                    out.location_line = Some(LocationLine::from_code(*value as u32));
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                _ => {}
            }
        }
        out
    }

    /// `true` when the wall's top isn't bound to a level — in that case
    /// height comes from `unconnected_height_feet`.
    pub fn is_unconnected(&self) -> bool {
        matches!(self.top_level_id, None | Some(0))
    }
}

/// Typed view of a decoded WallType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WallType {
    pub name: Option<String>,
    pub kind: Option<WallKind>,
    pub function: Option<WallFunction>,
    pub width_feet: Option<f64>,
    pub structural: Option<bool>,
}

impl WallType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("kind" | "walltype" | "walltypekind", InstanceField::Integer { value, .. }) => {
                    out.kind = Some(WallKind::from_code(*value as u32));
                }
                ("function" | "wallfunction", InstanceField::Integer { value, .. }) => {
                    out.function = Some(WallFunction::from_code(*value as u32));
                }
                ("width" | "thickness", InstanceField::Float { value, .. }) => {
                    out.width_feet = Some(*value);
                }
                ("structural" | "isstructural", InstanceField::Bool(b)) => {
                    out.structural = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// Total thickness in inches — convenience for US-customary
    /// readouts in UIs and reports.
    pub fn width_inches(&self) -> Option<f64> {
        self.width_feet.map(|ft| ft * 12.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_wall_schema() -> ClassEntry {
        ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_base_offset".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_top_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_unconnected_height".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_structural_usage".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    }),
                },
                FieldEntry {
                    name: "m_location_line".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
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
            declared_field_count: Some(7),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_wall_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_level_id = [tag=0, id=42]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&42u32.to_le_bytes());
        // m_base_offset = 0.0
        b.extend_from_slice(&0.0_f64.to_le_bytes());
        // m_top_level_id = [0, 0]  (unconnected)
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        // m_unconnected_height = 10.0  feet
        b.extend_from_slice(&10.0_f64.to_le_bytes());
        // m_structural_usage = 1 (Bearing)
        b.extend_from_slice(&1u32.to_le_bytes());
        // m_location_line = 0 (WallCenterline)
        b.extend_from_slice(&0u32.to_le_bytes());
        // m_type_id = [0, 7]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&7u32.to_le_bytes());
        b
    }

    #[test]
    fn wall_decoder_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Floor".into(),
            ..synth_wall_schema()
        };
        assert!(
            WallDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn wall_decodes_levels_and_height() {
        let decoded = WallDecoder
            .decode(
                &synth_wall_bytes(),
                &synth_wall_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let w = Wall::from_decoded(&decoded);
        assert_eq!(w.base_level_id, Some(42));
        assert_eq!(w.base_offset_feet, Some(0.0));
        assert_eq!(w.top_level_id, Some(0));
        assert_eq!(w.unconnected_height_feet, Some(10.0));
        assert_eq!(w.structural_usage, Some(StructuralUsage::Bearing));
        assert_eq!(w.location_line, Some(LocationLine::WallCenterline));
        assert_eq!(w.type_id, Some(7));
        assert!(w.is_unconnected());
    }

    #[test]
    fn wall_connected_to_top_level() {
        let w = Wall {
            top_level_id: Some(99),
            ..Default::default()
        };
        assert!(!w.is_unconnected());
    }

    #[test]
    fn enum_mappings() {
        assert_eq!(StructuralUsage::from_code(1), StructuralUsage::Bearing);
        assert_eq!(StructuralUsage::from_code(99), StructuralUsage::NonBearing);
        assert_eq!(LocationLine::from_code(3), LocationLine::FinishFaceInterior);
        assert_eq!(LocationLine::from_code(99), LocationLine::WallCenterline);
        assert_eq!(WallKind::from_code(1), WallKind::Curtain);
        assert_eq!(WallKind::from_code(2), WallKind::Stacked);
        assert_eq!(WallKind::from_code(0), WallKind::Basic);
        assert!(WallKind::Basic.is_basic());
        assert!(!WallKind::Curtain.is_basic());
        assert_eq!(WallFunction::from_code(1), WallFunction::Exterior);
        assert_eq!(WallFunction::Exterior.to_ifc_predefined(), "STANDARD");
        assert_eq!(WallFunction::Interior.to_ifc_predefined(), "PARTITIONING");
        assert_eq!(WallFunction::CoreShaft.to_ifc_predefined(), "SHEAR");
    }

    #[test]
    fn wall_type_width_conversion() {
        let wt = WallType {
            width_feet: Some(0.5),
            ..Default::default()
        };
        assert_eq!(wt.width_inches(), Some(6.0));
        let empty = WallType::default();
        assert_eq!(empty.width_inches(), None);
    }

    #[test]
    fn wall_type_tolerates_empty() {
        let empty = DecodedElement {
            id: None,
            class: "WallType".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let wt = WallType::from_decoded(&empty);
        assert!(wt.name.is_none() && wt.kind.is_none() && wt.width_feet.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(WallDecoder.class_name(), "Wall");
        assert_eq!(WallTypeDecoder.class_name(), "WallType");
    }
}
