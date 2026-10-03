//! `Room` + `Area` + `Space` — Revit's spatial-zoning classes.
//!
//! These are the "programmatic" elements — they describe *what a part
//! of the building is for* (Bedroom, Kitchen, Mech Room, Stairwell,
//! Fire Zone A, HVAC Zone 3) rather than *what's physically there*.
//! The IFC exporter maps all three to `IfcSpace` because the IFC4
//! schema collapses Revit's three variants into a single class.
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! Room:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Kitchen", "Bedroom 1" |
//! | `m_number` | String | Room number ("101", "B2-12") |
//! | `m_level_id` | ElementId | Level the room sits on |
//! | `m_upper_limit_id` | ElementId | Top level for the room's bounded height |
//! | `m_upper_offset` | f64 | Offset from upper level |
//! | `m_base_offset` | f64 | Offset from base level |
//! | `m_area` | f64 | Computed floor area (ft²) |
//! | `m_volume` | f64 | Computed volume (ft³) |
//!
//! Area + Space use the same fields with minor naming differences,
//! all collapsed through `normalise_field_name`.

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

simple_decoder!(RoomDecoder, "Room");
simple_decoder!(AreaDecoder, "Area");
simple_decoder!(SpaceDecoder, "Space");

/// Typed Room view — alias of [`Zone`] (Room/Area/Space share shape).
pub type Room = Zone;

/// Shared typed view for Room/Area/Space — same shape, same code path.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Zone {
    pub name: Option<String>,
    pub number: Option<String>,
    pub level_id: Option<u32>,
    pub upper_limit_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    pub upper_offset_feet: Option<f64>,
    pub area_square_feet: Option<f64>,
    pub volume_cubic_feet: Option<f64>,
}

impl Zone {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("number" | "roomnumber", InstanceField::String(s)) => {
                    out.number = Some(s.clone());
                }
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("upperlimitid" | "upperlevelid", InstanceField::ElementId { id, .. }) => {
                    out.upper_limit_id = Some(*id);
                }
                ("baseoffset", InstanceField::Float { value, .. }) => {
                    out.base_offset_feet = Some(*value);
                }
                ("upperoffset" | "limitoffset", InstanceField::Float { value, .. }) => {
                    out.upper_offset_feet = Some(*value);
                }
                ("area", InstanceField::Float { value, .. }) => {
                    out.area_square_feet = Some(*value);
                }
                ("volume", InstanceField::Float { value, .. }) => {
                    out.volume_cubic_feet = Some(*value);
                }
                _ => {}
            }
        }
        out
    }

    /// Human-readable label — "Number: Name" or just Name/Number if
    /// only one is present.
    pub fn label(&self) -> Option<String> {
        match (&self.number, &self.name) {
            (Some(n), Some(name)) => Some(format!("{n}: {name}")),
            (Some(n), None) => Some(n.clone()),
            (None, Some(name)) => Some(name.clone()),
            (None, None) => None,
        }
    }
}
