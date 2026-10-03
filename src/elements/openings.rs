//! `Door` + `Window` — wall-hosted opening elements. Both are
//! `FamilyInstance` subtypes at the class hierarchy level but ship
//! with enough stable scalar fields to decode directly without first
//! resolving the full family graph (which is task L5B-21).
//!
//! Each element references:
//! - A **host** (the wall it cuts through), via `m_host_id`
//! - A **symbol/type** (the family type definition), via `m_symbol_id`
//! - A **level**, inherited from the host wall's level
//! - A **location point** with a rotation angle
//!
//! Plus element-level overrides: `m_sill_height` for windows,
//! `m_flip_hand` and `m_flip_facing` for doors (left-swing vs
//! right-swing, interior vs exterior).
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! Door:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Wall (or other host) the door is cut into |
//! | `m_symbol_id` | ElementId | FamilySymbol (door type: "Single-Flush 36\" x 84\"") |
//! | `m_location_x`, `m_location_y`, `m_location_z` | f64 | Door location in project coords |
//! | `m_rotation` | f64 | Rotation angle in radians (usually aligned to host wall) |
//! | `m_flip_hand` | bool | Hinge on opposite side |
//! | `m_flip_facing` | bool | Swings into opposite space |
//!
//! Window:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Wall the window sits in |
//! | `m_symbol_id` | ElementId | FamilySymbol |
//! | `m_location_x`, `m_location_y`, `m_location_z` | f64 | Centre of the window in project coords |
//! | `m_rotation` | f64 | Rotation radians |
//! | `m_sill_height` | f64 | Height of windowsill above host level (feet) |

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::Point3;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Registered decoder for the `Door` class.
pub struct DoorDecoder;

impl ElementDecoder for DoorDecoder {
    fn class_name(&self) -> &'static str {
        "Door"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Door" {
            return Err(Error::BasicFileInfo(format!(
                "DoorDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `Window` class.
pub struct WindowDecoder;

impl ElementDecoder for WindowDecoder {
    fn class_name(&self) -> &'static str {
        "Window"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Window" {
            return Err(Error::BasicFileInfo(format!(
                "WindowDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Fields shared by every wall-hosted opening element (Door,
/// Window, and future Opening). Gathered in one pass so neither
/// `Door::from_decoded` nor `Window::from_decoded` has to re-scan.
#[derive(Debug, Clone, Copy, Default)]
struct OpeningCommon {
    location: Option<Point3>,
    rotation_radians: Option<f64>,
    level_id: Option<u32>,
    host_id: Option<u32>,
    symbol_id: Option<u32>,
}

fn collect_common(decoded: &DecodedElement) -> OpeningCommon {
    let mut lx = None;
    let mut ly = None;
    let mut lz = None;
    let mut out = OpeningCommon::default();
    for (field_name, value) in &decoded.fields {
        match (normalise_field_name(field_name).as_str(), value) {
            ("locationx", InstanceField::Float { value, .. }) => lx = Some(*value),
            ("locationy", InstanceField::Float { value, .. }) => ly = Some(*value),
            ("locationz", InstanceField::Float { value, .. }) => lz = Some(*value),
            ("rotation", InstanceField::Float { value, .. }) => {
                out.rotation_radians = Some(*value);
            }
            ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                out.level_id = Some(*id);
            }
            ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
            ("symbolid" | "typeid" | "familysymbolid", InstanceField::ElementId { id, .. }) => {
                out.symbol_id = Some(*id);
            }
            _ => {}
        }
    }
    if let (Some(x), Some(y), Some(z)) = (lx, ly, lz) {
        out.location = Some(Point3::new(x, y, z));
    }
    out
}

/// Typed view of a decoded Door.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Door {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    pub flip_hand: Option<bool>,
    pub flip_facing: Option<bool>,
}

impl Door {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        let mut out = Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
            rotation_radians: c.rotation_radians,
            flip_hand: None,
            flip_facing: None,
        };
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("fliphand", InstanceField::Bool(b)) => out.flip_hand = Some(*b),
                ("flipfacing", InstanceField::Bool(b)) => out.flip_facing = Some(*b),
                _ => {}
            }
        }
        out
    }

    /// True when the door has been flipped from its family-default
    /// orientation (either hand or facing).
    pub fn is_flipped(&self) -> Option<bool> {
        match (self.flip_hand, self.flip_facing) {
            (Some(h), Some(f)) => Some(h ^ f),
            (Some(h), None) => Some(h),
            (None, Some(f)) => Some(f),
            (None, None) => None,
        }
    }
}

/// Typed view of a decoded Window.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Window {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    /// Distance from host level's elevation up to the windowsill.
    pub sill_height_feet: Option<f64>,
}

impl Window {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        let mut out = Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
            rotation_radians: c.rotation_radians,
            sill_height_feet: None,
        };
        for (field_name, value) in &decoded.fields {
            if let ("sillheight", InstanceField::Float { value, .. }) =
                (normalise_field_name(field_name).as_str(), value)
            {
                out.sill_height_feet = Some(*value);
            }
        }
        out
    }

    /// Sill height in inches — convenience for US-customary callers.
    pub fn sill_height_inches(&self) -> Option<f64> {
        self.sill_height_feet.map(|ft| ft * 12.0)
    }
}
