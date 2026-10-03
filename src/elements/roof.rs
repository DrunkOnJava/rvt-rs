//! `Roof` + `RoofType` — sloped or flat upper building envelope. In
//! Revit, roofs are serialized in two flavors:
//!
//! - **Footprint roof** — sketched in plan view, with one or more
//!   edges marked as "slope-defining" so Revit auto-generates the
//!   planes and ridge/hip geometry.
//! - **Extrusion roof** — swept from a 2D profile along a path,
//!   typical for barrel vaults or shed roofs with a simple axis.
//!
//! Both share the same class in the file (the `m_roof_type` field
//! distinguishes which flavour). Geometry assembly (slope planes,
//! ridge/valley topology, eave cuts) is GEO-29.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! Roof:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Base level |
//! | `m_base_offset` | f64 | Offset from base level (feet) |
//! | `m_cutoff_level_id` | ElementId | Top level that truncates the roof, 0 for no cutoff |
//! | `m_cutoff_offset` | f64 | Offset from cutoff level |
//! | `m_roof_type` | Primitive u32 | 0=Footprint 1=Extrusion |
//! | `m_type_id` | ElementId | Reference to `RoofType` |
//! | `m_rafter_cut` | Primitive u32 | 0=PlumbCut 1=TwoCutSquare 2=TwoCutPlumb |
//!
//! RoofType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Generic - 12\"", "Basic Roof — Steel Bar Joist" |
//! | `m_thickness` | f64 | Total roof-assembly thickness in feet |
//! | `m_function` | Primitive u32 | 0=Interior 1=Exterior (rare; most roofs are exterior) |

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// How a roof is geometrically defined at the element level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoofKind {
    #[default]
    Footprint,
    Extrusion,
}

impl RoofKind {
    pub fn from_code(code: u32) -> Self {
        if code == 1 {
            Self::Extrusion
        } else {
            Self::Footprint
        }
    }
}

/// How Revit trims the rafters at the eave.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RafterCut {
    #[default]
    PlumbCut,
    TwoCutSquare,
    TwoCutPlumb,
}

impl RafterCut {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::TwoCutSquare,
            2 => Self::TwoCutPlumb,
            _ => Self::PlumbCut,
        }
    }
}

/// Registered decoder for the `Roof` class.
pub struct RoofDecoder;

impl ElementDecoder for RoofDecoder {
    fn class_name(&self) -> &'static str {
        "Roof"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Roof" {
            return Err(Error::BasicFileInfo(format!(
                "RoofDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `RoofType` class.
pub struct RoofTypeDecoder;

impl ElementDecoder for RoofTypeDecoder {
    fn class_name(&self) -> &'static str {
        "RoofType"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "RoofType" {
            return Err(Error::BasicFileInfo(format!(
                "RoofTypeDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded Roof instance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Roof {
    pub level_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    /// `0` or `None` means the roof has no cutoff — it follows the
    /// sketched/extruded geometry all the way up.
    pub cutoff_level_id: Option<u32>,
    pub cutoff_offset_feet: Option<f64>,
    pub kind: Option<RoofKind>,
    pub type_id: Option<u32>,
    pub rafter_cut: Option<RafterCut>,
}

impl Roof {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "baselevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("baseoffset", InstanceField::Float { value, .. }) => {
                    out.base_offset_feet = Some(*value);
                }
                ("cutofflevelid", InstanceField::ElementId { id, .. }) => {
                    out.cutoff_level_id = Some(*id);
                }
                ("cutoffoffset", InstanceField::Float { value, .. }) => {
                    out.cutoff_offset_feet = Some(*value);
                }
                ("rooftype" | "kind", InstanceField::Integer { value, .. }) => {
                    out.kind = Some(RoofKind::from_code(*value as u32));
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                ("raftercut", InstanceField::Integer { value, .. }) => {
                    out.rafter_cut = Some(RafterCut::from_code(*value as u32));
                }
                _ => {}
            }
        }
        out
    }

    /// `true` when the roof is not truncated by a cutoff level (the
    /// common case — "roof sketched freely, ridge at whatever height
    /// the slopes yield").
    pub fn has_cutoff(&self) -> bool {
        !matches!(self.cutoff_level_id, None | Some(0))
    }
}

/// Typed view of a decoded RoofType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoofType {
    pub name: Option<String>,
    pub thickness_feet: Option<f64>,
    /// 0 = Interior (very rare for roofs), 1 = Exterior. None when
    /// the schema didn't carry this field.
    pub is_exterior: Option<bool>,
}

impl RoofType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("thickness" | "width", InstanceField::Float { value, .. }) => {
                    out.thickness_feet = Some(*value);
                }
                ("function", InstanceField::Integer { value, .. }) => {
                    out.is_exterior = Some(*value == 1);
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
