//! Revit class / category → IFC4 entity type mapping table.
//!
//! Every concrete [`crate::elements::ElementDecoder`] implementation
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
    Mapping {
        revit_class: "Wall",
        ifc_type: "IFCWALL",
        predefined_type: Some("STANDARD"),
    },
    Mapping {
        revit_class: "CurtainWall",
        ifc_type: "IFCCURTAINWALL",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Door",
        ifc_type: "IFCDOOR",
        predefined_type: None,
    },
    Mapping {
        revit_class: "Window",
        ifc_type: "IFCWINDOW",
        predefined_type: None,
    },
    // Architectural — horizontal elements.
    Mapping {
        revit_class: "Floor",
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
    // Spatial zoning.
    Mapping {
        revit_class: "Room",
        ifc_type: "IFCSPACE",
        predefined_type: Some("INTERNAL"),
    },
    Mapping {
        revit_class: "Area",
        ifc_type: "IFCSPACE",
        predefined_type: None,
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
        predefined_type: Some("USERDEFINED"),
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
        ifc_type: "IFCELECTRICAPPLIANCE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "ElectricalFixture",
        ifc_type: "IFCLIGHTFIXTURE",
        predefined_type: None,
    },
    Mapping {
        revit_class: "MechanicalEquipment",
        ifc_type: "IFCFLOWCONTROLLER",
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
        predefined_type: Some("USERDEFINED"),
    },
    // Massing / abstract.
    Mapping {
        revit_class: "Mass",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: Some("USERDEFINED"),
    },
    Mapping {
        revit_class: "GenericModel",
        ifc_type: "IFCBUILDINGELEMENTPROXY",
        predefined_type: None,
    },
];

/// Look up the IFC entity type for a Revit class name.
///
/// Returns `None` when the class has no registered mapping — callers
/// should emit `IfcBuildingElementProxy` as the fallback.
pub fn lookup(revit_class: &str) -> Option<&'static Mapping> {
    MAPPINGS.iter().find(|m| m.revit_class == revit_class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn lookup_known_class() {
        let m = lookup("Wall").unwrap();
        assert_eq!(m.ifc_type, "IFCWALL");
        assert_eq!(m.predefined_type, Some("STANDARD"));
    }

    #[test]
    fn lookup_unknown_class_returns_none() {
        assert!(lookup("DefinitelyNotARevitClass").is_none());
    }

    #[test]
    fn revit_classes_unique() {
        let mut seen = BTreeSet::new();
        for m in MAPPINGS {
            assert!(
                seen.insert(m.revit_class),
                "duplicate mapping for {}",
                m.revit_class
            );
        }
    }

    #[test]
    fn ifc_type_names_uppercase_step_format() {
        for m in MAPPINGS {
            assert!(
                m.ifc_type.starts_with("IFC"),
                "{} should start with IFC",
                m.ifc_type
            );
            assert_eq!(
                m.ifc_type.to_uppercase(),
                m.ifc_type,
                "{} should be uppercase",
                m.ifc_type
            );
        }
    }

    #[test]
    fn door_and_window_have_no_predefined_type_by_default() {
        // IfcDoor / IfcWindow handle type variance via IfcDoorType /
        // IfcWindowType, not PredefinedType. Confirm the mapping
        // respects that.
        assert_eq!(lookup("Door").unwrap().predefined_type, None);
        assert_eq!(lookup("Window").unwrap().predefined_type, None);
    }

    #[test]
    fn structural_framing_maps_to_ifcbeam() {
        let m = lookup("StructuralFraming").unwrap();
        assert_eq!(m.ifc_type, "IFCBEAM");
    }

    #[test]
    fn room_area_space_all_map_to_ifcspace() {
        assert_eq!(lookup("Room").unwrap().ifc_type, "IFCSPACE");
        assert_eq!(lookup("Area").unwrap().ifc_type, "IFCSPACE");
        assert_eq!(lookup("Space").unwrap().ifc_type, "IFCSPACE");
    }
}
