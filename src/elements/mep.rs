//! MEP (Mechanical / Electrical / Plumbing) FamilyInstance decoders.
//!
//! Revit splits the MEP discipline across Category IDs that all
//! share the `FamilyInstance` wire shape plus a discipline-specific
//! `m_system_classification` tag (Electrical circuit, HVAC duct,
//! Hydronic pipe, …). This module ships typed decoders for the
//! common MEP subtypes so downstream analytics can distinguish
//! "any family instance" from "specifically a light fixture" or
//! "specifically a pipe fitting" without post-filtering by name.
//!
//! Coverage:
//!
//! | Revit class | Decoder | Discipline |
//! |---|---|---|
//! | `ElectricalEquipment` | `ElectricalEquipmentDecoder` | Electrical — panels, transformers |
//! | `ElectricalFixture` | `ElectricalFixtureDecoder` | Electrical — fixtures, outlets |
//! | `LightingFixture` | `LightingFixtureDecoder` | Electrical — luminaires (subtype of ElectricalFixture) |
//! | `LightingDevice` | `LightingDeviceDecoder` | Electrical — switches, sensors |
//! | `Duct` | `DuctDecoder` | Mechanical — HVAC ducts |
//! | `DuctFitting` | `DuctFittingDecoder` | Mechanical — elbows, tees, transitions |
//! | `MechanicalEquipment` | `MechanicalEquipmentDecoder` | Mechanical — boilers, chillers, AHUs |
//! | `Pipe` | `PipeDecoder` | Plumbing — pipe segments |
//! | `PipeFitting` | `PipeFittingDecoder` | Plumbing — fittings |
//! | `PlumbingFixture` | `PlumbingFixtureDecoder` | Plumbing — sinks, water closets |
//! | `SpecialtyEquipment` | `SpecialtyEquipmentDecoder` | Cross-discipline — carts, lab benches |
//!
//! All decoders share the same `FamilyInstance` wire shape, so
//! they are implemented via the `simple_decoder!` macro. The
//! typed view below (`MepInstance`) is a best-effort surface for
//! discipline-common fields; class-specific fields are left in
//! the generic `DecodedElement.fields` for callers that want them.
//!
//! # Typical common fields
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Instance name ("L1 — Type 60W", "Duct — 6x8") |
//! | `m_level_id` | ElementId | Host level |
//! | `m_system_classification` | Primitive u32 | Discipline code (see `MepSystemClassification`) |
//! | `m_system_type_id` | ElementId | PipeSystemType / DuctSystemType / ElectricalSystem |
//! | `m_connector_count` | Primitive u32 | Number of MEP connectors on this instance |

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

// L5B-36: Electrical family-instance decoders.
simple_decoder!(ElectricalEquipmentDecoder, "ElectricalEquipment");
simple_decoder!(ElectricalFixtureDecoder, "ElectricalFixture");
simple_decoder!(LightingFixtureDecoder, "LightingFixture");
simple_decoder!(LightingDeviceDecoder, "LightingDevice");

// L5B-37: Mechanical / Plumbing / Specialty family-instance decoders.
simple_decoder!(DuctDecoder, "Duct");
simple_decoder!(DuctFittingDecoder, "DuctFitting");
simple_decoder!(MechanicalEquipmentDecoder, "MechanicalEquipment");
simple_decoder!(PipeDecoder, "Pipe");
simple_decoder!(PipeFittingDecoder, "PipeFitting");
simple_decoder!(PlumbingFixtureDecoder, "PlumbingFixture");
simple_decoder!(SpecialtyEquipmentDecoder, "SpecialtyEquipment");

/// Revit's `System Classification` enum, collapsed to the
/// discipline families we care about. The full Revit enum has
/// dozens of values (`DomesticHotWater`, `DomesticColdWater`,
/// `StormDrainage`, `Sanitary`, `SupplyAir`, `ReturnAir`,
/// `ExhaustAir`, `ChilledWater`, `HotWater`, `SteamHigh`,
/// `SteamLow`, `HVACCondensate`, `HVACVentilation`,
/// `Electrical_Power`, `Electrical_Lighting`, …). We bucket by
/// coarse discipline; callers who need fine-grained subtypes can
/// read the raw u32 directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MepSystemClassification {
    #[default]
    Unknown,
    Electrical,
    Mechanical,
    Plumbing,
    FireProtection,
    Data,
    /// Undefined / not yet bucketed.
    Other(u32),
}

impl MepSystemClassification {
    /// Best-effort bucketing from Revit's raw `System Classification`
    /// u32. Mappings are approximate and based on observed values in
    /// the 2016-2026 corpus; callers needing exact semantics should
    /// consult the raw code instead.
    pub fn from_code(code: u32) -> Self {
        match code {
            1..=9 => Self::Electrical,
            10..=29 => Self::Mechanical,
            30..=49 => Self::Plumbing,
            50..=59 => Self::FireProtection,
            60..=79 => Self::Data,
            0 => Self::Unknown,
            _ => Self::Other(code),
        }
    }
}

/// Typed view of any MEP family instance. Discipline-specific
/// subtypes can project this further; for now all 11 decoders
/// share it, which is enough to surface host level, system ref,
/// and connector count uniformly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MepInstance {
    pub name: Option<String>,
    pub level_id: Option<u32>,
    pub system_classification: Option<MepSystemClassification>,
    /// Raw classification code. Always populated when
    /// `system_classification` is non-`None` — surfaced separately
    /// for callers who need the exact Revit enum value, not the
    /// coarse bucket.
    pub system_classification_code: Option<u32>,
    pub system_type_id: Option<u32>,
    pub connector_count: Option<u32>,
}

impl MepInstance {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("levelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                (
                    "systemclassification" | "classification",
                    InstanceField::Integer { value, .. },
                ) => {
                    let code = *value as u32;
                    out.system_classification = Some(MepSystemClassification::from_code(code));
                    out.system_classification_code = Some(code);
                }
                ("systemtypeid" | "mepsystemtypeid", InstanceField::ElementId { id, .. }) => {
                    out.system_type_id = Some(*id);
                }
                ("connectorcount", InstanceField::Integer { value, .. }) => {
                    out.connector_count = Some(*value as u32);
                }
                _ => {}
            }
        }
        out
    }

    /// True when this instance belongs to the Electrical bucket
    /// (Power, Lighting, Low-Voltage). Useful for
    /// discipline-filtered IFC export or system-type inventories.
    pub fn is_electrical(&self) -> bool {
        matches!(
            self.system_classification,
            Some(MepSystemClassification::Electrical)
        )
    }

    /// True when this instance is Mechanical (HVAC / ducts).
    pub fn is_mechanical(&self) -> bool {
        matches!(
            self.system_classification,
            Some(MepSystemClassification::Mechanical)
        )
    }

    /// True when this instance is Plumbing (pipes, fixtures).
    pub fn is_plumbing(&self) -> bool {
        matches!(
            self.system_classification,
            Some(MepSystemClassification::Plumbing)
        )
    }
}
