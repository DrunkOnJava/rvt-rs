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
//! | Revit class | Registered as | Discipline |
//! |---|---|---|
//! | `ElectricalEquipment` | `SimpleDecoder::new("ElectricalEquipment")` | Electrical — panels, transformers |
//! | `ElectricalFixture` | `SimpleDecoder::new("ElectricalFixture")` | Electrical — fixtures, outlets |
//! | `LightingFixture` | `SimpleDecoder::new("LightingFixture")` | Electrical — luminaires (subtype of ElectricalFixture) |
//! | `LightingDevice` | `SimpleDecoder::new("LightingDevice")` | Electrical — switches, sensors |
//! | `Duct` | `SimpleDecoder::new("Duct")` | Mechanical — HVAC ducts |
//! | `DuctFitting` | `SimpleDecoder::new("DuctFitting")` | Mechanical — elbows, tees, transitions |
//! | `MechanicalEquipment` | `SimpleDecoder::new("MechanicalEquipment")` | Mechanical — boilers, chillers, AHUs |
//! | `Pipe` | `SimpleDecoder::new("Pipe")` | Plumbing — pipe segments |
//! | `PipeFitting` | `SimpleDecoder::new("PipeFitting")` | Plumbing — fittings |
//! | `PlumbingFixture` | `SimpleDecoder::new("PlumbingFixture")` | Plumbing — sinks, water closets |
//! | `SpecialtyEquipment` | `SimpleDecoder::new("SpecialtyEquipment")` | Cross-discipline — carts, lab benches |
//!
//! All of them share the same `FamilyInstance` wire shape, so each
//! is a [`super::SimpleDecoder`]. The
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
use crate::walker::{DecodedElement, InstanceField};

// L5B-36: Electrical family-instance decoders.

// L5B-37: Mechanical / Plumbing / Specialty family-instance decoders.

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
