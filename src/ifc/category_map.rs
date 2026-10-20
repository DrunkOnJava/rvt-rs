//! Revit class / category → IFC4 entity type mapping table.
//!
//! Every concrete [`crate::walker::ElementDecoder`] implementation
//! ultimately needs to know "when I emit this as IFC, what entity
//! type should it be?" This module is the single source of truth for
//! that mapping. Keeping it data-driven (a static table) instead of
//! scattered through per-class code means:
//!
//! - Adding a new Revit class gets one line here.
//! - IFC2X3 / IFC4 / IFC4.3 migration is a table swap.
//! - Tests can sweep every entry for spec conformance.
//!
//! See buildingSMART's MVD / IDM documentation for the canonical
//! entity choices per class category.

/// Mapping entry: Revit class → IFC entity type + optional
/// `PredefinedType` enum value.
///
/// `ifc_type` is the uppercase STEP entity name (e.g. `"IFCWALL"`,
/// `"IFCSLAB"`). `predefined_type` is the value that goes into the
/// entity's `PredefinedType` attribute when present (e.g.
/// `"FLOOR"` / `"ROOF"` for IfcSlab, `"BEAM"` for IfcBeam).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub revit_class: &'static str,
    pub ifc_type: &'static str,
    pub predefined_type: Option<&'static str>,
}

/// Lookup table, checked at test time for uniqueness + completeness.
///
/// Ordered by a rough "primary Revit category" sort so reviewers
/// scanning the list see related entries together.
pub const MAPPINGS: &[Mapping] = &[
    // Spatial containers (project scaffolding — already handled by
    // the RvtDocExporter but listed here for the lookup to cover
    // the full Revit vocabulary).
    Mapping {
        revit_class: "Level",
        ifc_type: "IFCBUILDINGSTOREY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Grid",
        ifc_type: "IFCGRID",
        predefined_type: None,
    },
    // Architectural — walls and hosted elements.
    //
    // `.NOTDEFINED.` rather than `.STANDARD.` is the authoring
    // witness's own choice, not a shrug (#220): on
    // `2024_Core_Interior_slim.ifc` Revit's exporter writes
    // `.NOTDEFINED.` on all 360 `IFCWALL` rows, including the ones
    // whose body is a plain `SweptSolid` that `.STANDARD.` would
    // describe. Agreement with the recorded export is what the
    // verdicts score, so where an edge shows the witness's answer we
    // take it.
    Mapping {
        revit_class: "Wall",
        ifc_type: "IFCWALL",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "ArcWall",
        ifc_type: "IFCWALL",
        predefined_type: Some("NOTDEFINED"),
    },
    // Also a wall a mullion names (RE-46). Revit's IFC4 export of Snowdon
    // Towers writes `.NOTDEFINED.` on all 60 of its `IFCCURTAINWALL` rows;
    // 42 are bodiless aggregates of their panels and mullions, and the
    // other 18 are panels that hold a basic wall (`CurtainPanelWall`).
    Mapping {
        revit_class: "CurtainWall",
        ifc_type: "IFCCURTAINWALL",
        predefined_type: Some("NOTDEFINED"),
    },
    // A curtain panel that holds a basic wall (RE-64). Revit's export of
    // Snowdon Towers writes all 18 as `IFCCURTAINWALL` with their own body,
    // nested under the curtain wall they are panels of (RE-72); these are
    // the only curtain walls there that carry a body.
    Mapping {
        revit_class: "CurtainPanelWall",
        ifc_type: "IFCCURTAINWALL",
        predefined_type: Some("NOTDEFINED"),
    },
    // Revit writes `.DOOR.` on all 132 `IFCDOOR` rows and `.WINDOW.`
    // on all 6 `IFCWINDOW` rows of the same export (#220); emitting
    // `$` there dropped a value the witness had already shown.
    Mapping {
        revit_class: "Door",
        ifc_type: "IFCDOOR",
        predefined_type: Some("DOOR"),
    },
    Mapping {
        revit_class: "Window",
        ifc_type: "IFCWINDOW",
        predefined_type: Some("WINDOW"),
    },
    // 2024 ArcWallRectOpening index rows — not typed Door/Window.
    Mapping {
        revit_class: "ArcWallRectOpening",
        ifc_type: "IFCOPENINGELEMENT",
        predefined_type: None,
    },
    // Architectural — horizontal elements.
    Mapping {
        revit_class: "Floor",
        ifc_type: "IFCSLAB",
        predefined_type: Some("FLOOR"),
    },
    // A Revit building pad is not a floor, but Revit's own exporter
    // emits it as `IfcSlab` with `PredefinedType = .FLOOR.` — see the
    // `Pad:Site Pad:21975` row in the Core Interior reference export
    // (#212, RE-22). The class stays `BuildingPad` so the mapping is
    // visible here rather than hidden behind a relabelled decode.
    Mapping {
        revit_class: "BuildingPad",
        ifc_type: "IFCSLAB",
        predefined_type: Some("FLOOR"),
    },
    Mapping {
        revit_class: "Roof",
        ifc_type: "IFCROOF",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Ceiling",
        ifc_type: "IFCCOVERING",
        predefined_type: Some("CEILING"),
    },
    // Architectural — circulation.
    Mapping {
        revit_class: "Stair",
        ifc_type: "IFCSTAIR",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Railing",
        ifc_type: "IFCRAILING",
        predefined_type: None,
    },
    // Curtain-wall parts (RE-33): Revit's IFC4 export writes a mullion as
    // `IfcMember` `.MULLION.` and a panel as `IfcPlate` `.CURTAIN_PANEL.`.
    Mapping {
        revit_class: "CurtainWallMullion",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("MULLION"),
    },
    Mapping {
        revit_class: "CurtainWallPanel",
        ifc_type: "IFCPLATE",
        predefined_type: Some("CURTAIN_PANEL"),
    },
    // `OST_Cornices`: Revit's IFC4 export writes a wall sweep as
    // `IfcBuildingElementProxy` `.NOTDEFINED.` (RE-33).
    Mapping {
        revit_class: "WallSweep",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "Ramp",
        ifc_type: "IFCRAMP",
        predefined_type: None,
    },
    // Structural — load-bearing elements.
    Mapping {
        revit_class: "Column",
        ifc_type: "IFCCOLUMN",
        predefined_type: Some("COLUMN"),
    },
    Mapping {
        revit_class: "StructuralColumn",
        ifc_type: "IFCCOLUMN",
        predefined_type: Some("COLUMN"),
    },
    Mapping {
        revit_class: "StructuralFraming",
        ifc_type: "IFCBEAM",
        predefined_type: Some("BEAM"),
    },
    Mapping {
        revit_class: "StructuralFoundation",
        ifc_type: "IFCFOOTING",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Rebar",
        ifc_type: "IFCREINFORCINGBAR",
        predefined_type: None,
    },
    // Structural — secondary members (IFC-10).
    // IfcMember is IFC4's entity for structural elements that aren't
    // the primary load path: bracing, trusses, studs, girts, purlins,
    // mullions, posts. Revit distinguishes these via the Family
    // Symbol / PredefinedType on the StructuralFraming class. When
    // the walker can identify the subtype, route through one of these
    // entries; otherwise StructuralFraming stays on the IfcBeam row
    // above.
    Mapping {
        revit_class: "Brace",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("BRACE"),
    },
    Mapping {
        revit_class: "StructuralTruss",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("CHORD"),
    },
    Mapping {
        revit_class: "Purlin",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("PURLIN"),
    },
    Mapping {
        revit_class: "Post",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("POST"),
    },
    Mapping {
        revit_class: "Stud",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("STUD"),
    },
    Mapping {
        revit_class: "Strut",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("STRUT"),
    },
    Mapping {
        revit_class: "Girt",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("MEMBER"),
    },
    Mapping {
        revit_class: "Rafter",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("RAFTER"),
    },
    // Spatial zoning.
    //
    // `.SPACE.` rather than `.INTERNAL.` (#220): Revit writes
    // `.SPACE.` on all 116 `IFCSPACE` rows of
    // `2024_Core_Interior_slim.ifc`, and on the 30 Areas of the BIM4LCA
    // office (#763). `Space` keeps `None`: no recorded edge shows what
    // the witness does with that Revit class, and inventing a value is
    // the thing this table exists to avoid.
    Mapping {
        revit_class: "Room",
        ifc_type: "IFCSPACE",
        predefined_type: Some("SPACE"),
    },
    Mapping {
        revit_class: "Area",
        ifc_type: "IFCSPACE",
        predefined_type: Some("SPACE"),
    },
    Mapping {
        revit_class: "Space",
        ifc_type: "IFCSPACE",
        predefined_type: None,
    },
    // Furnishings / equipment.
    Mapping {
        revit_class: "Furniture",
        ifc_type: "IFCFURNITURE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "FurnitureSystem",
        ifc_type: "IFCFURNITURE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Casework",
        ifc_type: "IFCFURNITURE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "LightingFixture",
        ifc_type: "IFCLIGHTFIXTURE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "ElectricalEquipment",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "ElectricalFixture",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "MechanicalEquipment",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    // #96: the IFC4 subtype of what Revit's export of RE1 Electrical (IFC2x3)
    // writes: fire alarm devices are IfcDistributionControlElement typed
    // IfcAlarmType, so IfcAlarm; data devices (its thermostats) are
    // IfcFlowTerminal typed IfcElectricApplianceType, so IfcElectricAppliance.
    // Lighting devices, electrical fixtures and electrical and mechanical
    // equipment are proxies there, and stay proxies.
    Mapping {
        revit_class: "LightingDevice",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "FireAlarmDevice",
        ifc_type: "IFCALARM",
        predefined_type: None,
    },
    Mapping {
        revit_class: "DataDevice",
        ifc_type: "IFCELECTRICAPPLIANCE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "PlumbingFixture",
        ifc_type: "IFCSANITARYTERMINAL",
        predefined_type: None,
    },
    Mapping {
        revit_class: "SpecialtyEquipment",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    // Ducts and pipes (RE-33), as their IFC4 distribution-element types.
    Mapping {
        revit_class: "Duct",
        ifc_type: "IFCDUCTSEGMENT",
        predefined_type: None,
    },
    Mapping {
        revit_class: "DuctFitting",
        ifc_type: "IFCDUCTFITTING",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Pipe",
        ifc_type: "IFCPIPESEGMENT",
        predefined_type: None,
    },
    Mapping {
        revit_class: "PipeFitting",
        ifc_type: "IFCPIPEFITTING",
        predefined_type: None,
    },
    // Massing / abstract.
    Mapping {
        revit_class: "Mass",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "GenericModel",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    // RE-37: the entities Revit's own IFC4 export writes for these
    // categories on Snowdon Towers (air terminals from the IFC4 subtype of
    // the IFC2x3 `IfcFlowTerminal` Revit writes on RE1 Mechanical).
    Mapping {
        revit_class: "DuctTerminal",
        ifc_type: "IFCAIRTERMINAL",
        predefined_type: None,
    },
    Mapping {
        revit_class: "FoodServiceEquipment",
        ifc_type: "IFCELECTRICAPPLIANCE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Planting",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Parking",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Entourage",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Hardscape",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    Mapping {
        revit_class: "VerticalCirculation",
        ifc_type: "IFCTRANSPORTELEMENT",
        predefined_type: None,
    },
    // RE-40: Revit's own IFC4 export writes every Snowdon slab edge outside
    // a non-primary design option as a proxy.
    Mapping {
        revit_class: "SlabEdge",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
    // #323: a stair's parts, aggregated under its `IfcStair` as in Revit's
    // IFC4 export of Snowdon Towers (flights, `.LANDING.` slabs,
    // `.STRINGER.` members).
    Mapping {
        revit_class: "StairsRun",
        ifc_type: "IFCSTAIRFLIGHT",
        predefined_type: None,
    },
    Mapping {
        revit_class: "StairsLanding",
        ifc_type: "IFCSLAB",
        predefined_type: Some("LANDING"),
    },
    Mapping {
        revit_class: "StairsStringer",
        ifc_type: "IFCMEMBER",
        predefined_type: Some("STRINGER"),
    },
];

/// Look up the IFC entity type for a Revit class name.
///
/// Returns `None` when the class has no registered mapping — callers
/// should emit `IfcBuildingElementProxy` as the fallback.
pub fn lookup(revit_class: &str) -> Option<&'static Mapping> {
    MAPPINGS.iter().find(|m| m.revit_class == revit_class)
}

/// IFC entity types a per-element Revit "IFC Export As" override may
/// redirect a decoded element to (#212, RE-22).
///
/// The override carrier is general —
/// [`crate::partition_ifc_export_overrides`] returns whatever value
/// string the parameter block holds — but acting on a value is a
/// claim, so only values proven against a reference export are
/// listed. An unlisted value leaves the element on its class mapping,
/// and the export diagnostics count it in their warnings, by value and
/// class, rather than guessing (#792).
///
/// `IfcShadingDevice` is proven on floors, not on every element: on
/// `2024_Core_Interior.rvt` the twenty `OST_Floors` instances that
/// carry it are exactly the twenty `IFCSHADINGDEVICE` rows in
/// Revit's own export, and the remaining seventy-nine plus the one
/// `OST_BuildingPad` instance are exactly its eighty `IFCSLAB` rows.
/// `PredefinedType` follows the authoring witness (#220 rule, #235):
/// Revit writes `.NOTDEFINED.` on all twenty `IFCSHADINGDEVICE` rows of
/// `IFC Exports/2024_Core_Interior_slim.ifc`, the same recorded choice
/// that moved `IFCWALL` off `.STANDARD.`. Before #235 this slot was `$`
/// on the grounds that rvt-rs decodes no value for it; the witness is
/// the value. On curtain walls it is disproven for Revit 2024: the 124
/// BIM4LCA office curtain walls that carry it are `IfcCurtainWall` in
/// Revit's export ([`export_override_applies`], RE-188).
///
/// RE-45 adds two targets, each proven against a reference export:
/// `IfcSlab`, which the two Core Interior floors carrying `ifcSlab` (with
/// predefined type `ROOF`) are in Revit's export, and `IfcCovering`, which
/// the three `teste_export_2025` walls whose type carries `IfcCoveringType`
/// (with `CLADDING`) are. The override's own predefined type replaces the
/// target's default when it is an enumerator of that entity
/// ([`predefined_type_for`]).
///
/// #792 adds five targets, proven on the two BIM4LCA office files (Revit
/// 2024) against Revit's export of each: `IfcWasteTerminal` on 78 plumbing
/// fixtures whose types carry `IfcWasteTerminalType` (Revit writes
/// `FLOORTRAP` on 62 and `ROOFDRAIN` on 16), `IfcElectricAppliance` on 51
/// items of electrical equipment, `IfcFireSuppressionTerminal` on 40 items of
/// specialty equipment (`HOSEREEL`), `IfcSanitaryTerminal` on 15 casework
/// elements (`WASHHANDBASIN`) and `IfcAirTerminal` on one item of electrical
/// equipment. Each value's elements are exactly the ones Revit's export
/// writes as its entity, and their predefined types are the ones it writes.
///
/// #792 adds two more: `IfcStair`, which 2 generic models of each office
/// file carry and Revit writes as `IfcStair .NOTDEFINED.`, and `IfcDoor`,
/// which a curtain panel carries with `DOOR` and Revit writes as
/// `IfcDoor .DOOR.`. Every element of Revit's export of either file that
/// carries one of the two values as its own is that entity there (16 and
/// 268), as the other elements carrying `IfcStair` or `IfcDoor` already were.
///
/// #763 adds `IfcSpace`, which 42 `Turning Free Space` family instances of
/// each office file carry, with `USERDEFINED` as their own predefined type,
/// and which Revit writes as `IfcSpace .USERDEFINED.` typed by an
/// `IfcSpaceType` named after the family type ([`space_predefined_type_for`]).
/// A Room, Area or Space is a space already and keeps its own mapping.
pub const EXPORT_OVERRIDE_TARGETS: &[Mapping] = &[
    Mapping {
        revit_class: "IfcShadingDevice",
        ifc_type: "IFCSHADINGDEVICE",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcSlab",
        ifc_type: "IFCSLAB",
        predefined_type: Some("FLOOR"),
    },
    Mapping {
        revit_class: "IfcCovering",
        ifc_type: "IFCCOVERING",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcWasteTerminal",
        ifc_type: "IFCWASTETERMINAL",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcElectricAppliance",
        ifc_type: "IFCELECTRICAPPLIANCE",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcFireSuppressionTerminal",
        ifc_type: "IFCFIRESUPPRESSIONTERMINAL",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcSanitaryTerminal",
        ifc_type: "IFCSANITARYTERMINAL",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcAirTerminal",
        ifc_type: "IFCAIRTERMINAL",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcStair",
        ifc_type: "IFCSTAIR",
        predefined_type: Some("NOTDEFINED"),
    },
    Mapping {
        revit_class: "IfcDoor",
        ifc_type: "IFCDOOR",
        predefined_type: Some("DOOR"),
    },
    Mapping {
        revit_class: "IfcSpace",
        ifc_type: "IFCSPACE",
        predefined_type: Some("NOTDEFINED"),
    },
];

/// The IFC4 `IfcSpaceTypeEnum` enumerator `value` names, for an element
/// written as `IfcSpace` through its own "Export to IFC As" value (#763):
/// Revit writes the 42 `Turning Free Space` instances of the office files,
/// whose own predefined type is `USERDEFINED`, as `IfcSpace .USERDEFINED.`.
/// Rooms are not looked up here: their own value is not measured against
/// Revit's export, so they keep [`Mapping::predefined_type`]. `None` for a
/// value that is not an enumerator.
pub fn space_predefined_type_for(value: &str) -> Option<&'static str> {
    const SPACE_TYPES: &[&str] = &[
        "SPACE",
        "PARKING",
        "GFA",
        "INTERNAL",
        "EXTERNAL",
        "USERDEFINED",
        "NOTDEFINED",
    ];
    let value = value.trim();
    SPACE_TYPES
        .iter()
        .copied()
        .find(|enumerator| enumerator.eq_ignore_ascii_case(value))
}

/// IFC4 `PredefinedType` enumerators of the entities an export override
/// can name, as the IFC4 schema lists them (`IfcSlabTypeEnum`,
/// `IfcCoveringTypeEnum`, `IfcShadingDeviceTypeEnum`, `IfcRoofTypeEnum`,
/// `IfcWasteTerminalTypeEnum`, `IfcElectricApplianceTypeEnum`,
/// `IfcFireSuppressionTerminalTypeEnum`, `IfcSanitaryTerminalTypeEnum`,
/// `IfcAirTerminalTypeEnum`). `IfcStair` and `IfcDoor`, honoured as targets
/// since #792, are not listed, because listing them would apply every door's
/// and stair's own predefined type as well, which is not measured. An
/// element exported as either through its value takes the target's
/// default, the one Revit writes on the elements measured.
const PREDEFINED_TYPE_ENUMERATORS: &[(&str, &[&str])] = &[
    (
        "IFCSLAB",
        &[
            "FLOOR",
            "ROOF",
            "LANDING",
            "BASESLAB",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCCOVERING",
        &[
            "CEILING",
            "FLOORING",
            "CLADDING",
            "ROOFING",
            "MOLDING",
            "SKIRTINGBOARD",
            "INSULATION",
            "MEMBRANE",
            "SLEEVING",
            "WRAPPING",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCSHADINGDEVICE",
        &["JALOUSIE", "SHUTTER", "AWNING", "USERDEFINED", "NOTDEFINED"],
    ),
    (
        "IFCROOF",
        &[
            "FLAT_ROOF",
            "SHED_ROOF",
            "GABLE_ROOF",
            "HIP_ROOF",
            "HIPPED_GABLE_ROOF",
            "GAMBREL_ROOF",
            "MANSARD_ROOF",
            "BARREL_ROOF",
            "RAINBOW_ROOF",
            "BUTTERFLY_ROOF",
            "PAVILION_ROOF",
            "DOME_ROOF",
            "FREEFORM",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCWASTETERMINAL",
        &[
            "FLOORTRAP",
            "FLOORWASTE",
            "GULLYSUMP",
            "GULLYTRAP",
            "ROOFDRAIN",
            "WASTEDISPOSALUNIT",
            "WASTETRAP",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCELECTRICAPPLIANCE",
        &[
            "DISHWASHER",
            "ELECTRICCOOKER",
            "FREESTANDINGELECTRICHEATER",
            "FREESTANDINGFAN",
            "FREESTANDINGWATERHEATER",
            "FREESTANDINGWATERCOOLER",
            "FREEZER",
            "FRIDGE_FREEZER",
            "HANDDRYER",
            "KITCHENMACHINE",
            "MICROWAVE",
            "PHOTOCOPIER",
            "REFRIGERATOR",
            "TUMBLEDRYER",
            "VENDINGMACHINE",
            "WASHINGMACHINE",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCFIRESUPPRESSIONTERMINAL",
        &[
            "BREECHINGINLET",
            "FIREHYDRANT",
            "HOSEREEL",
            "SPRINKLER",
            "SPRINKLERDEFLECTOR",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCSANITARYTERMINAL",
        &[
            "BATH",
            "BIDET",
            "CISTERN",
            "SHOWER",
            "SINK",
            "SANITARYFOUNTAIN",
            "TOILETPAN",
            "URINAL",
            "WASHHANDBASIN",
            "WCSEAT",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
    (
        "IFCAIRTERMINAL",
        &[
            "DIFFUSER",
            "GRILLE",
            "LOUVRE",
            "REGISTER",
            "USERDEFINED",
            "NOTDEFINED",
        ],
    ),
];

/// The IFC4 enumerator `value` names for `ifc_type`'s `PredefinedType`
/// (RE-45), compared case-insensitively because the Revit parameter is
/// user-entered text. `None` when the entity is not listed or the value is
/// not one of its enumerators, so the caller keeps its own default.
pub fn predefined_type_for(ifc_type: &str, value: &str) -> Option<&'static str> {
    let value = value.trim();
    PREDEFINED_TYPE_ENUMERATORS
        .iter()
        .find(|(entity, _)| *entity == ifc_type)?
        .1
        .iter()
        .copied()
        .find(|enumerator| enumerator.eq_ignore_ascii_case(value))
}

/// Whether Revit's exporter acts on an "Export to IFC As" value on an
/// element of `revit_class`, in a file Revit `revit_version` saved (RE-188).
///
/// Revit 2024's exporter never reads a curtain wall's value: revit-ifc's
/// `CurtainSystemExporter.ExportBase`, from `IFC_v24.1.0` to `IFC_v24.3.40`,
/// creates every curtain wall as `IfcCurtainWall`. The 124 curtain walls
/// of the two BIM4LCA office files set to `IfcShadingDevice` are
/// `IfcCurtainWall` in Revit's export of each. From `IFC_v25.2.0` on,
/// `ExportBase` takes the entity from the same lookup as other elements,
/// so a 2025 or 2026 curtain wall keeps its value here. That is read from
/// the exporter's source: no Revit 2025 or 2026 export of a curtain wall
/// carrying a value has been measured. The file's release stands for the
/// exporter's, since exporting from a later Revit upgrades the file first.
/// An unknown release keeps the value.
///
/// Nor does Revit 2024's export act on a wall sweep's value (RE-192). On
/// both BIM4LCA office files 9 wall sweeps each carry their own
/// `IfcCovering` (predefined type `ROOFING`) in their own data object, and
/// Revit's export of each file writes all 9 as `IfcBuildingElementProxy`.
/// This is measured, not read from the exporter's source: revit-ifc's
/// `WallSweepExporter` exports a sweep through `GenericElementExporter`,
/// which takes the entity from the same lookup as other elements, so the
/// value seems not to reach that lookup. No other release's wall sweep
/// carrying a value has been measured.
pub fn export_override_applies(revit_class: &str, revit_version: Option<u32>) -> bool {
    !(matches!(
        revit_class,
        crate::partition_schema_mvp::CURTAIN_WALL_CLASS | "WallSweep"
    ) && revit_version == Some(2024))
}

/// Look up the IFC entity type a "IFC Export As" override value names.
///
/// The comparison is case-insensitive because the parameter value is
/// user-entered text; everything else about it is exact.
pub fn lookup_export_override(value: &str) -> Option<&'static Mapping> {
    EXPORT_OVERRIDE_TARGETS
        .iter()
        .find(|m| m.revit_class.eq_ignore_ascii_case(value.trim()))
}

/// True when the mapping routes a Revit class through `IfcMember`
/// (as opposed to `IfcBeam` / `IfcColumn` / `IfcFooting`). Useful
/// for downstream code that wants to emit IfcMember-specific
/// property-sets (`Pset_MemberCommon`) or group members into an
/// `IfcRelAggregates` under a truss / frame assembly.
///
/// IFC-10: IfcMember is IFC4's dedicated entity for secondary
/// structural elements. Routing through it (rather than the
/// fallback `IfcBeam`) lets validators and downstream tools
/// distinguish primary beams from bracing / purlins / studs /
/// trusses, which matters for load-path visualization and
/// structural schedules.
pub fn is_ifc_member(revit_class: &str) -> bool {
    lookup(revit_class)
        .map(|m| m.ifc_type == "IFCMEMBER")
        .unwrap_or(false)
}
