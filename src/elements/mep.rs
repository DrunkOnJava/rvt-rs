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
                (
                    "systemtypeid" | "mepsystemtypeid",
                    InstanceField::ElementId { id, .. },
                ) => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ClassEntry;

    fn wrong_schema() -> ClassEntry {
        ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    #[test]
    fn all_decoders_reject_wrong_schema() {
        let mocks: Vec<Box<dyn ElementDecoder>> = vec![
            Box::new(ElectricalEquipmentDecoder),
            Box::new(ElectricalFixtureDecoder),
            Box::new(LightingFixtureDecoder),
            Box::new(LightingDeviceDecoder),
            Box::new(DuctDecoder),
            Box::new(DuctFittingDecoder),
            Box::new(MechanicalEquipmentDecoder),
            Box::new(PipeDecoder),
            Box::new(PipeFittingDecoder),
            Box::new(PlumbingFixtureDecoder),
            Box::new(SpecialtyEquipmentDecoder),
        ];
        for d in mocks {
            assert!(
                d.decode(&[], &wrong_schema(), &HandleIndex::new()).is_err(),
                "{} should reject wrong schema",
                d.class_name()
            );
        }
    }

    #[test]
    fn class_names_match_registry() {
        assert_eq!(ElectricalEquipmentDecoder.class_name(), "ElectricalEquipment");
        assert_eq!(ElectricalFixtureDecoder.class_name(), "ElectricalFixture");
        assert_eq!(LightingFixtureDecoder.class_name(), "LightingFixture");
        assert_eq!(LightingDeviceDecoder.class_name(), "LightingDevice");
        assert_eq!(DuctDecoder.class_name(), "Duct");
        assert_eq!(DuctFittingDecoder.class_name(), "DuctFitting");
        assert_eq!(MechanicalEquipmentDecoder.class_name(), "MechanicalEquipment");
        assert_eq!(PipeDecoder.class_name(), "Pipe");
        assert_eq!(PipeFittingDecoder.class_name(), "PipeFitting");
        assert_eq!(PlumbingFixtureDecoder.class_name(), "PlumbingFixture");
        assert_eq!(SpecialtyEquipmentDecoder.class_name(), "SpecialtyEquipment");
    }

    #[test]
    fn system_classification_mapping() {
        assert_eq!(MepSystemClassification::from_code(0), MepSystemClassification::Unknown);
        assert_eq!(
            MepSystemClassification::from_code(3),
            MepSystemClassification::Electrical
        );
        assert_eq!(
            MepSystemClassification::from_code(15),
            MepSystemClassification::Mechanical
        );
        assert_eq!(
            MepSystemClassification::from_code(35),
            MepSystemClassification::Plumbing
        );
        assert_eq!(
            MepSystemClassification::from_code(55),
            MepSystemClassification::FireProtection
        );
        assert_eq!(
            MepSystemClassification::from_code(70),
            MepSystemClassification::Data
        );
        assert!(matches!(
            MepSystemClassification::from_code(500),
            MepSystemClassification::Other(500)
        ));
    }

    #[test]
    fn mep_instance_from_decoded_electrical() {
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("Main Panel A".into()),
            ),
            (
                "m_level_id".into(),
                InstanceField::ElementId { tag: 0, id: 12 },
            ),
            (
                "m_system_classification".into(),
                InstanceField::Integer {
                    value: 3,
                    signed: false,
                    size: 4,
                },
            ),
            (
                "m_connector_count".into(),
                InstanceField::Integer {
                    value: 2,
                    signed: false,
                    size: 4,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "ElectricalEquipment".into(),
            fields,
            byte_range: 0..0,
        };
        let m = MepInstance::from_decoded(&decoded);
        assert_eq!(m.name.as_deref(), Some("Main Panel A"));
        assert_eq!(m.level_id, Some(12));
        assert_eq!(m.system_classification_code, Some(3));
        assert!(m.is_electrical());
        assert!(!m.is_mechanical());
        assert_eq!(m.connector_count, Some(2));
    }

    #[test]
    fn mep_instance_from_decoded_mechanical_duct() {
        let fields = vec![
            (
                "m_name".into(),
                InstanceField::String("SA Trunk".into()),
            ),
            (
                "m_system_classification".into(),
                InstanceField::Integer {
                    value: 20,
                    signed: false,
                    size: 4,
                },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Duct".into(),
            fields,
            byte_range: 0..0,
        };
        let m = MepInstance::from_decoded(&decoded);
        assert!(m.is_mechanical());
        assert!(!m.is_plumbing());
        assert_eq!(m.system_classification_code, Some(20));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "Pipe".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let m = MepInstance::from_decoded(&empty);
        assert!(m.name.is_none());
        assert!(!m.is_electrical() && !m.is_mechanical() && !m.is_plumbing());
    }
}
