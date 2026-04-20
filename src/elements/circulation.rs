//! `Stair` + `StairType` + `Railing` + `RailingType` — circulation
//! elements that connect levels. Together they account for most of
//! what Revit calls "circulation" in a typical model.
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! Stair:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_base_level_id` | ElementId | Level the stair starts from |
//! | `m_top_level_id` | ElementId | Level the stair reaches |
//! | `m_base_offset` | f64 | Offset from base level |
//! | `m_top_offset` | f64 | Offset from top level |
//! | `m_desired_riser_count` | Primitive u32 | Designer-specified riser count |
//! | `m_actual_riser_count` | Primitive u32 | What Revit generated after constraint resolution |
//! | `m_actual_tread_depth` | f64 | Calibrated tread depth in feet |
//! | `m_actual_riser_height` | f64 | Calibrated riser height in feet |
//! | `m_type_id` | ElementId | Reference to StairType |
//!
//! StairType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Assembled Stair", "Monolithic Stair" |
//! | `m_function` | Primitive u32 | 0=Interior 1=Exterior |
//!
//! Railing:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Host element (stair, floor, ramp) or 0 for free-standing |
//! | `m_height_offset` | f64 | Vertical offset from host |
//! | `m_type_id` | ElementId | Reference to RailingType |
//!
//! RailingType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Handrail — Pipe", "Guardrail — 42\"" |
//! | `m_top_rail_height` | f64 | Height of top rail above host (feet) |

use super::level::normalise_field_name;
use crate::formats;
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

simple_decoder!(StairDecoder, "Stair");
simple_decoder!(StairTypeDecoder, "StairType");
simple_decoder!(RailingDecoder, "Railing");
simple_decoder!(RailingTypeDecoder, "RailingType");

/// Typed view of a decoded Stair.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stair {
    pub base_level_id: Option<u32>,
    pub top_level_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    pub top_offset_feet: Option<f64>,
    pub desired_riser_count: Option<u32>,
    pub actual_riser_count: Option<u32>,
    pub actual_tread_depth_feet: Option<f64>,
    pub actual_riser_height_feet: Option<f64>,
    pub type_id: Option<u32>,
}

impl Stair {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
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
                ("desiredrisercount", InstanceField::Integer { value, .. }) => {
                    out.desired_riser_count = Some(*value as u32);
                }
                ("actualrisercount", InstanceField::Integer { value, .. }) => {
                    out.actual_riser_count = Some(*value as u32);
                }
                ("actualtreaddepth", InstanceField::Float { value, .. }) => {
                    out.actual_tread_depth_feet = Some(*value);
                }
                ("actualriserheight", InstanceField::Float { value, .. }) => {
                    out.actual_riser_height_feet = Some(*value);
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                _ => {}
            }
        }
        out
    }

    /// True when Revit adjusted the riser count from what the designer
    /// asked for — a sign the geometry was over-constrained and
    /// something gave way.
    pub fn was_adjusted(&self) -> Option<bool> {
        Some(self.desired_riser_count? != self.actual_riser_count?)
    }

    /// Total rise in feet = riser_count × riser_height.
    pub fn total_rise_feet(&self) -> Option<f64> {
        Some(self.actual_riser_count? as f64 * self.actual_riser_height_feet?)
    }
}

/// Typed view of a StairType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StairType {
    pub name: Option<String>,
    pub is_exterior: Option<bool>,
}

impl StairType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("function", InstanceField::Integer { value, .. }) => {
                    out.is_exterior = Some(*value == 1);
                }
                _ => {}
            }
        }
        out
    }
}

/// Typed view of a decoded Railing.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Railing {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub height_offset_feet: Option<f64>,
    pub type_id: Option<u32>,
}

impl Railing {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                ("heightoffset" | "offset", InstanceField::Float { value, .. }) => {
                    out.height_offset_feet = Some(*value);
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                _ => {}
            }
        }
        out
    }

    /// True when the railing has no host — a free-standing rail, not
    /// riding along a stair or floor.
    pub fn is_free_standing(&self) -> bool {
        matches!(self.host_id, None | Some(0))
    }
}

/// Typed view of a RailingType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RailingType {
    pub name: Option<String>,
    pub top_rail_height_feet: Option<f64>,
}

impl RailingType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("topraillheight" | "topheight", InstanceField::Float { value, .. }) => {
                    out.top_rail_height_feet = Some(*value);
                }
                _ => {}
            }
        }
        out
    }

    pub fn top_rail_height_inches(&self) -> Option<f64> {
        self.top_rail_height_feet.map(|ft| ft * 12.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stair_decoder_rejects_wrong_schema() {
        use crate::formats::ClassEntry;
        let wrong = ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        assert!(
            StairDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn stair_adjusted_detection() {
        let unchanged = Stair {
            desired_riser_count: Some(16),
            actual_riser_count: Some(16),
            ..Default::default()
        };
        let changed = Stair {
            desired_riser_count: Some(16),
            actual_riser_count: Some(17),
            ..Default::default()
        };
        let missing = Stair::default();
        assert_eq!(unchanged.was_adjusted(), Some(false));
        assert_eq!(changed.was_adjusted(), Some(true));
        assert_eq!(missing.was_adjusted(), None);
    }

    #[test]
    fn stair_total_rise_computation() {
        let s = Stair {
            actual_riser_count: Some(16),
            actual_riser_height_feet: Some(0.5833), // 7" riser
            ..Default::default()
        };
        assert!((s.total_rise_feet().unwrap() - 9.3328).abs() < 1e-4);
        let missing = Stair::default();
        assert_eq!(missing.total_rise_feet(), None);
    }

    #[test]
    fn stair_from_decoded_populates_all_fields() {
        let fields = vec![
            (
                "m_base_level_id".into(),
                InstanceField::ElementId { tag: 0, id: 1 },
            ),
            (
                "m_top_level_id".into(),
                InstanceField::ElementId { tag: 0, id: 2 },
            ),
            (
                "m_desired_riser_count".into(),
                InstanceField::Integer {
                    value: 16,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_actual_riser_count".into(),
                InstanceField::Integer {
                    value: 16,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_actual_tread_depth".into(),
                InstanceField::Float {
                    value: 0.9167,
                    size: 8,
                },
            ),
            (
                "m_actual_riser_height".into(),
                InstanceField::Float {
                    value: 0.5833,
                    size: 8,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Stair".into(),
            fields,
            byte_range: 0..0,
        };
        let s = Stair::from_decoded(&decoded);
        assert_eq!(s.base_level_id, Some(1));
        assert_eq!(s.top_level_id, Some(2));
        assert_eq!(s.desired_riser_count, Some(16));
        assert_eq!(s.actual_riser_count, Some(16));
        assert_eq!(s.actual_tread_depth_feet, Some(0.9167));
        assert!((s.actual_riser_height_feet.unwrap() - 0.5833).abs() < 1e-9);
    }

    #[test]
    fn railing_free_standing_detection() {
        let free = Railing {
            host_id: Some(0),
            ..Default::default()
        };
        let hosted = Railing {
            host_id: Some(42),
            ..Default::default()
        };
        let missing = Railing::default();
        assert!(free.is_free_standing());
        assert!(!hosted.is_free_standing());
        assert!(missing.is_free_standing());
    }

    #[test]
    fn railing_type_height_conversion() {
        let rt = RailingType {
            top_rail_height_feet: Some(3.5),
            ..Default::default()
        };
        assert_eq!(rt.top_rail_height_inches(), Some(42.0));
        assert_eq!(RailingType::default().top_rail_height_inches(), None);
    }

    #[test]
    fn stair_type_tolerates_empty() {
        let empty = DecodedElement {
            id: None,
            class: "StairType".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let st = StairType::from_decoded(&empty);
        assert!(st.name.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(StairDecoder.class_name(), "Stair");
        assert_eq!(StairTypeDecoder.class_name(), "StairType");
        assert_eq!(RailingDecoder.class_name(), "Railing");
        assert_eq!(RailingTypeDecoder.class_name(), "RailingType");
    }
}
