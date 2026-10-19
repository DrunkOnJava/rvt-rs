//! The IFC4 standard property sets a parameter named for one of their
//! properties is written in (RE-194).
//!
//! Revit's exporter reads a project or shared parameter named
//! `<property set>.<property>`, such as `Pset_WallCommon.FireRating`, before
//! anything else for that property (`PropertySetEntry
//! ("Pset_WallCommon.FireRating", "FireRating")` in revit-ifc's property set
//! definitions, CLEANROOM.md source 5). Which entities a set applies to, and
//! each property's value type, are IFC4's property set definitions
//! (buildingSMART IFC4 ADD2 TC1), as revit-ifc's IFC4 definitions give them,
//! for the sets of the entities rvt-rs writes. A set applies to its entity
//! and that entity's subtypes. Only the properties of the kinds RE-194 reads
//! from a parameter are listed: booleans, text, counts, lengths and thermal
//! transmittance.

/// The value type IFC4 declares for a standard property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandardPropertyKind {
    /// `IfcBoolean`, from a Yes/No parameter.
    Boolean,
    /// `IfcIdentifier`, from a text parameter.
    Identifier,
    /// `IfcLabel`, from a text parameter.
    Label,
    /// `IfcText`, from a text parameter.
    Text,
    /// `IfcCountMeasure`, from an integer parameter.
    Count,
    /// `IfcLengthMeasure`, from a length parameter, stored in feet.
    Length,
    /// `IfcPositiveLengthMeasure`, from a length parameter, stored in feet.
    PositiveLength,
    /// `IfcThermalTransmittanceMeasure`, from a heat transfer coefficient
    /// parameter, stored in W/(m²·K).
    ThermalTransmittance,
}

use StandardPropertyKind::{
    Boolean, Count, Identifier, Label, Length, PositiveLength, Text, ThermalTransmittance,
};

/// One IFC4 property set: the entities it applies to, as the IFC types rvt-rs
/// writes, and its properties of the kinds in [`StandardPropertyKind`].
#[derive(Debug)]
pub struct StandardPropertySet {
    /// The set's name.
    pub name: &'static str,
    /// The entities it applies to, with their subtypes.
    pub entities: &'static [&'static str],
    /// Its properties and their kinds.
    pub properties: &'static [(&'static str, StandardPropertyKind)],
}

/// The IFC4 property sets of the entities rvt-rs writes, with the properties
/// a parameter's value can be written as.
pub const STANDARD_PROPERTY_SETS: &[StandardPropertySet] = &[
    StandardPropertySet {
        name: "Pset_BeamCommon",
        entities: &["IFCBEAM", "IFCBEAMSTANDARDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("Span", PositiveLength),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_BuildingElementProxyCommon",
        entities: &["IFCBUILDINGELEMENTPROXY"],
        properties: &[
            ("Reference", Identifier),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_ColumnCommon",
        entities: &["IFCCOLUMN", "IFCCOLUMNSTANDARDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_CoveringCommon",
        entities: &["IFCCOVERING"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("FlammabilityRating", Label),
            ("FragilityRating", Label),
            ("Combustible", Boolean),
            ("SurfaceSpreadOfFlame", Label),
            ("Finish", Text),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_CurtainWallCommon",
        entities: &["IFCCURTAINWALL"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("FireRating", Label),
            ("Combustible", Boolean),
            ("SurfaceSpreadOfFlame", Label),
            ("ThermalTransmittance", ThermalTransmittance),
            ("IsExternal", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_DoorCommon",
        entities: &["IFCDOOR", "IFCDOORSTANDARDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("FireRating", Label),
            ("AcousticRating", Label),
            ("SecurityRating", Label),
            ("DurabilityRating", Label),
            ("HygrothermalRating", Label),
            ("WaterTightnessRating", Label),
            ("MechanicalLoadRating", Label),
            ("WindLoadRating", Label),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("HandicapAccessible", Boolean),
            ("FireExit", Boolean),
            ("HasDrive", Boolean),
            ("SelfClosing", Boolean),
            ("SmokeStop", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_DoorWindowGlazingType",
        entities: &[
            "IFCDOOR",
            "IFCDOORSTANDARDCASE",
            "IFCWINDOW",
            "IFCWINDOWSTANDARDCASE",
        ],
        properties: &[
            ("GlassLayers", Count),
            ("GlassThickness1", PositiveLength),
            ("GlassThickness2", PositiveLength),
            ("GlassThickness3", PositiveLength),
            ("FillGas", Label),
            ("GlassColor", Label),
            ("IsTempered", Boolean),
            ("IsLaminated", Boolean),
            ("IsCoated", Boolean),
            ("IsWired", Boolean),
            ("ThermalTransmittanceSummer", ThermalTransmittance),
            ("ThermalTransmittanceWinter", ThermalTransmittance),
        ],
    },
    StandardPropertySet {
        name: "Pset_MemberCommon",
        entities: &["IFCMEMBER", "IFCMEMBERSTANDARDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("Span", PositiveLength),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_PlateCommon",
        entities: &["IFCPLATE", "IFCPLATESTANDARDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_RailingCommon",
        entities: &["IFCRAILING"],
        properties: &[
            ("Reference", Identifier),
            ("Height", PositiveLength),
            ("Diameter", PositiveLength),
            ("IsExternal", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_RampCommon",
        entities: &["IFCRAMP"],
        properties: &[
            ("Reference", Identifier),
            ("RequiredHeadroom", PositiveLength),
            ("HandicapAccessible", Boolean),
            ("HasNonSkidSurface", Boolean),
            ("FireExit", Boolean),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_RampFlightCommon",
        entities: &["IFCRAMPFLIGHT"],
        properties: &[
            ("Reference", Identifier),
            ("Headroom", PositiveLength),
            ("ClearWidth", PositiveLength),
        ],
    },
    StandardPropertySet {
        name: "Pset_RoofCommon",
        entities: &["IFCROOF"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
        ],
    },
    StandardPropertySet {
        name: "Pset_SlabCommon",
        entities: &["IFCSLAB", "IFCSLABSTANDARDCASE", "IFCSLABELEMENTEDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("FireRating", Label),
            ("Combustible", Boolean),
            ("SurfaceSpreadOfFlame", Label),
            ("Compartmentation", Boolean),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_SpaceCommon",
        entities: &["IFCSPACE"],
        properties: &[
            ("Reference", Identifier),
            ("IsExternal", Boolean),
            ("PubliclyAccessible", Boolean),
            ("HandicapAccessible", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_SpaceCoveringRequirements",
        entities: &["IFCSPACE"],
        properties: &[
            ("FloorCovering", Label),
            ("FloorCoveringThickness", PositiveLength),
            ("WallCovering", Label),
            ("WallCoveringThickness", PositiveLength),
            ("CeilingCovering", Label),
            ("CeilingCoveringThickness", PositiveLength),
            ("SkirtingBoard", Label),
            ("SkirtingBoardHeight", PositiveLength),
            ("Molding", Label),
            ("MoldingHeight", PositiveLength),
            ("ConcealedFlooring", Boolean),
            ("ConcealedCeiling", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_SpaceOccupancyRequirements",
        entities: &["IFCSPACE"],
        properties: &[
            ("OccupancyType", Label),
            ("OccupancyNumber", Count),
            ("OccupancyNumberPeak", Count),
            ("MinimumHeadroom", Length),
            ("IsOutlookDesirable", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_StairCommon",
        entities: &["IFCSTAIR"],
        properties: &[
            ("Reference", Identifier),
            ("NumberOfRiser", Count),
            ("NumberOfTreads", Count),
            ("RiserHeight", PositiveLength),
            ("TreadLength", PositiveLength),
            ("NosingLength", Length),
            ("WalkingLineOffset", PositiveLength),
            ("TreadLengthAtOffset", PositiveLength),
            ("TreadLengthAtInnerSide", PositiveLength),
            ("WaistThickness", PositiveLength),
            ("RequiredHeadroom", PositiveLength),
            ("HandicapAccessible", Boolean),
            ("HasNonSkidSurface", Boolean),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("LoadBearing", Boolean),
            ("FireRating", Label),
            ("FireExit", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_StairFlightCommon",
        entities: &["IFCSTAIRFLIGHT"],
        properties: &[
            ("Reference", Identifier),
            ("NumberOfRiser", Count),
            ("NumberOfTreads", Count),
            ("RiserHeight", PositiveLength),
            ("TreadLength", PositiveLength),
            ("NosingLength", Length),
            ("WalkingLineOffset", PositiveLength),
            ("TreadLengthAtOffset", PositiveLength),
            ("TreadLengthAtInnerSide", PositiveLength),
            ("Headroom", PositiveLength),
            ("WaistThickness", PositiveLength),
        ],
    },
    StandardPropertySet {
        name: "Pset_WallCommon",
        entities: &["IFCWALL", "IFCWALLSTANDARDCASE", "IFCWALLELEMENTEDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("FireRating", Label),
            ("Combustible", Boolean),
            ("SurfaceSpreadOfFlame", Label),
            ("ThermalTransmittance", ThermalTransmittance),
            ("IsExternal", Boolean),
            ("LoadBearing", Boolean),
            ("ExtendToStructure", Boolean),
            ("Compartmentation", Boolean),
        ],
    },
    StandardPropertySet {
        name: "Pset_WindowCommon",
        entities: &["IFCWINDOW", "IFCWINDOWSTANDARDCASE"],
        properties: &[
            ("Reference", Identifier),
            ("AcousticRating", Label),
            ("FireRating", Label),
            ("SecurityRating", Label),
            ("IsExternal", Boolean),
            ("ThermalTransmittance", ThermalTransmittance),
            ("HasSillExternal", Boolean),
            ("HasSillInternal", Boolean),
            ("HasDrive", Boolean),
            ("SmokeStop", Boolean),
            ("FireExit", Boolean),
            ("WaterTightnessRating", Label),
            ("MechanicalLoadRating", Label),
            ("WindLoadRating", Label),
        ],
    },
];

fn standard_set(name: &str) -> Option<&'static StandardPropertySet> {
    STANDARD_PROPERTY_SETS.iter().find(|set| set.name == name)
}

/// The kind of `property` in the standard set named `set`, `None` for a set
/// or property not listed.
pub fn property_kind(set: &str, property: &str) -> Option<StandardPropertyKind> {
    standard_set(set)?
        .properties
        .iter()
        .find(|(name, _)| *name == property)
        .map(|(_, kind)| *kind)
}

/// Whether the standard set named `set` applies to an element written as
/// `ifc_type`.
pub fn applies_to(set: &str, ifc_type: &str) -> bool {
    standard_set(set).is_some_and(|set| set.entities.contains(&ifc_type))
}
