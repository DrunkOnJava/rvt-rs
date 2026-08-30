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
    Mapping {
        revit_class: "CurtainWall",
        ifc_type: "IFCCURTAINWALL",
        predefined_type: None,
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
    // `2024_Core_Interior_slim.ifc`. `Area` and `Space` keep `None` —
    // no recorded edge shows what the witness does with those two
    // Revit classes, and inventing a value is the thing this table
    // exists to avoid.
    Mapping {
        revit_class: "Room",
        ifc_type: "IFCSPACE",
        predefined_type: Some("SPACE"),
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

/// IFC entity types a per-element Revit "IFC Export As" override may
/// redirect a decoded element to (#212, RE-22).
///
/// The override carrier is general —
/// [`crate::partition_ifc_export_overrides`] returns whatever value
/// string the parameter block holds — but acting on a value is a
/// claim, so only values proven against a reference export are
/// listed. An unlisted value leaves the element on its class mapping
/// and is reported as an unhonoured override rather than guessed at.
///
/// `IfcShadingDevice` is the one proven value: on
/// `2024_Core_Interior.rvt` the twenty `OST_Floors` instances that
/// carry it are exactly the twenty `IFCSHADINGDEVICE` rows in
/// Revit's own export, and the remaining seventy-nine plus the one
/// `OST_BuildingPad` instance are exactly its eighty `IFCSLAB` rows.
/// `IfcShadingDevice` declares no `PredefinedType` value rvt-rs can
/// decode, so none is written (`$`, not an invented `.NOTDEFINED.`).
pub const EXPORT_OVERRIDE_TARGETS: &[Mapping] = &[Mapping {
    revit_class: "IfcShadingDevice",
    ifc_type: "IFCSHADINGDEVICE",
    predefined_type: None,
}];

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn export_override_maps_the_one_proven_value() {
        let m = lookup_export_override("IfcShadingDevice").unwrap();
        assert_eq!(m.ifc_type, "IFCSHADINGDEVICE");
        assert_eq!(m.predefined_type, None);
        // The parameter value is user-entered text.
        assert!(lookup_export_override("  ifcshadingdevice  ").is_some());
    }

    #[test]
    fn unproven_export_override_values_are_not_honoured() {
        // A value the reference export never demonstrated must leave
        // the element on its class mapping rather than invent a type.
        assert!(lookup_export_override("IfcCovering").is_none());
        assert!(lookup_export_override("IfcSlab").is_none());
        assert!(lookup_export_override("").is_none());
    }

    #[test]
    fn building_pad_maps_to_ifcslab_floor() {
        let m = lookup("BuildingPad").unwrap();
        assert_eq!(m.ifc_type, "IFCSLAB");
        assert_eq!(m.predefined_type, Some("FLOOR"));
    }

    #[test]
    fn lookup_known_class() {
        let m = lookup("Wall").unwrap();
        assert_eq!(m.ifc_type, "IFCWALL");
        assert_eq!(m.predefined_type, Some("NOTDEFINED"));
    }

    /// #220: the four mappings that now follow the authoring witness
    /// instead of the closest-reading IFC4 enum. Each value is what
    /// Revit's own exporter writes on `2024_Core_Interior_slim.ifc`
    /// (360 walls `.NOTDEFINED.`, 116 spaces `.SPACE.`, 132 doors
    /// `.DOOR.`, 6 windows `.WINDOW.`).
    #[test]
    fn witness_following_predefined_types() {
        for (class, ifc_type, expected) in [
            ("Wall", "IFCWALL", "NOTDEFINED"),
            ("ArcWall", "IFCWALL", "NOTDEFINED"),
            ("Room", "IFCSPACE", "SPACE"),
            ("Door", "IFCDOOR", "DOOR"),
            ("Window", "IFCWINDOW", "WINDOW"),
        ] {
            let m = lookup(class).unwrap();
            assert_eq!(m.ifc_type, ifc_type, "{class} entity type");
            assert_eq!(m.predefined_type, Some(expected), "{class} PredefinedType");
        }
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
    fn door_and_window_follow_the_witness_predefined_type() {
        // This used to assert `None`, on the reading that IfcDoor /
        // IfcWindow carry their type variance on IfcDoorType /
        // IfcWindowType rather than PredefinedType. That reading is
        // defensible and still wrong for this table (#220): Revit's own
        // exporter populates both — the type object *and* `.DOOR.` /
        // `.WINDOW.` on the occurrence, on all 132 doors and all 6
        // windows of the recorded artifact. Writing `$` where the
        // witness wrote a value is a dropped observation, not restraint.
        assert_eq!(lookup("Door").unwrap().predefined_type, Some("DOOR"));
        assert_eq!(lookup("Window").unwrap().predefined_type, Some("WINDOW"));
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

    // IFC-10: IfcMember secondary-structural routing.
    #[test]
    fn brace_maps_to_ifcmember_brace() {
        let m = lookup("Brace").unwrap();
        assert_eq!(m.ifc_type, "IFCMEMBER");
        assert_eq!(m.predefined_type, Some("BRACE"));
    }

    #[test]
    fn structural_truss_maps_to_ifcmember_chord() {
        let m = lookup("StructuralTruss").unwrap();
        assert_eq!(m.ifc_type, "IFCMEMBER");
        assert_eq!(m.predefined_type, Some("CHORD"));
    }

    #[test]
    fn secondary_members_route_to_ifcmember() {
        // All 8 IfcMember subtypes should route through IFCMEMBER
        // with the correct IFC4 PredefinedType. If any of these
        // flip back to IfcBeam, validators will complain about
        // bracing-in-load-path misclassification.
        let cases = [
            ("Brace", "BRACE"),
            ("StructuralTruss", "CHORD"),
            ("Purlin", "PURLIN"),
            ("Post", "POST"),
            ("Stud", "STUD"),
            ("Strut", "STRUT"),
            ("Girt", "MEMBER"),
            ("Rafter", "RAFTER"),
        ];
        for (revit, expected_pt) in cases {
            let m = lookup(revit).unwrap_or_else(|| panic!("{revit} missing from MAPPINGS"));
            assert_eq!(m.ifc_type, "IFCMEMBER", "{revit} should be IFCMEMBER");
            assert_eq!(
                m.predefined_type,
                Some(expected_pt),
                "{revit} predefined_type mismatch"
            );
        }
    }

    #[test]
    fn is_ifc_member_true_for_member_classes() {
        assert!(is_ifc_member("Brace"));
        assert!(is_ifc_member("StructuralTruss"));
        assert!(is_ifc_member("Stud"));
    }

    #[test]
    fn is_ifc_member_false_for_primary_beams() {
        // StructuralFraming stays on IfcBeam — callers that want to
        // route framing-as-bracing need a separate decision point
        // (e.g. Symbol family-name pattern match) and should NOT
        // rely on is_ifc_member() returning true for the base class.
        assert!(!is_ifc_member("StructuralFraming"));
        assert!(!is_ifc_member("Column"));
        assert!(!is_ifc_member("Beam"));
    }

    #[test]
    fn is_ifc_member_false_for_unknown_classes() {
        assert!(!is_ifc_member("DefinitelyNotARevitClass"));
    }

    #[test]
    fn ifc_member_predefined_types_are_spec_legal() {
        // IFC4 IfcMemberTypeEnum values — any future additions must
        // stay within this set. Updating IFC schema bumps (IFC4.3,
        // IFC5) would edit this list.
        let legal = [
            "BRACE",
            "CHORD",
            "COLLAR",
            "MEMBER",
            "MULLION",
            "PLATE",
            "POST",
            "PURLIN",
            "RAFTER",
            "STRINGER",
            "STRUT",
            "STUD",
            "USERDEFINED",
            "NOTDEFINED",
        ];
        for m in MAPPINGS.iter().filter(|m| m.ifc_type == "IFCMEMBER") {
            let pt = m
                .predefined_type
                .expect("IfcMember entries must have an explicit PredefinedType");
            assert!(
                legal.contains(&pt),
                "{} has illegal IfcMember PredefinedType: {}",
                m.revit_class,
                pt
            );
        }
    }
}
