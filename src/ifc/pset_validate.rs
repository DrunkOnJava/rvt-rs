//! Reserved `Pset_` name validation stubs (Lane D lightweight).
//!
//! buildingSMART reserves the `Pset_` prefix for standardized property sets.
//! This module records **schema constraints scaffolding** only: which names we
//! currently emit, which are reserved-looking, and which are project-local.
//! It does **not** claim full IFC4 Pset template compliance or ES-backed
//! property recovery (default IFC omits ES).

use serde::{Deserialize, Serialize};

/// Classification of a property-set name for honesty / linting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PsetNameKind {
    /// buildingSMART-style `Pset_<Class>Common` (or known Common variants).
    BuildingsmartCommon,
    /// Autodesk-aligned `Pset_RevitType_*` aliasing (documented convention).
    RevitTypeAlias,
    /// Project / toolkit diagnostic sets (e.g. `Pset_RvtRsDiagnosticCandidate`).
    ToolkitDiagnostic,
    /// Starts with `Pset_` but is not in the known allow-list — reserved-looking.
    ReservedUnknown,
    /// Does not use the `Pset_` prefix.
    NonPset,
}

/// Known Common sets currently referenced by rvt-rs emitters (allow-list stub).
pub const KNOWN_PSET_COMMON: &[&str] = &[
    "Pset_WallCommon",
    "Pset_DoorCommon",
    "Pset_WindowCommon",
    "Pset_StairCommon",
    "Pset_ColumnCommon",
    "Pset_BeamCommon",
    "Pset_MemberCommon",
];

/// Classify a property-set name without claiming template completeness.
pub fn classify_pset_name(name: &str) -> PsetNameKind {
    if !name.starts_with("Pset_") {
        return PsetNameKind::NonPset;
    }
    if KNOWN_PSET_COMMON.contains(&name) {
        return PsetNameKind::BuildingsmartCommon;
    }
    if name.starts_with("Pset_RevitType_") {
        return PsetNameKind::RevitTypeAlias;
    }
    if name.starts_with("Pset_RvtRs") {
        return PsetNameKind::ToolkitDiagnostic;
    }
    PsetNameKind::ReservedUnknown
}

/// True when the name looks reserved (`Pset_`) but is not on our allow-list.
pub fn is_unknown_reserved_pset(name: &str) -> bool {
    matches!(classify_pset_name(name), PsetNameKind::ReservedUnknown)
}

/// One row for mapping-example / doctor output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PsetMappingExample {
    pub revit_concept: String,
    pub ifc_pset: String,
    pub kind: PsetNameKind,
    pub notes: String,
}

/// Small honest mapping examples (ES omitted by default).
pub fn mapping_examples() -> Vec<PsetMappingExample> {
    vec![
        PsetMappingExample {
            revit_concept: "Wall (ArcWall 2023 path)".into(),
            ifc_pset: "Pset_WallCommon".into(),
            kind: PsetNameKind::BuildingsmartCommon,
            notes: "Emitted from typed ArcWall path when available".into(),
        },
        PsetMappingExample {
            revit_concept: "Door (typed)".into(),
            ifc_pset: "Pset_DoorCommon".into(),
            kind: PsetNameKind::BuildingsmartCommon,
            notes: "Helper exists; typed Door recovery unsupported (RE-19)".into(),
        },
        PsetMappingExample {
            revit_concept: "ES / Extensible Storage".into(),
            ifc_pset: "(omitted)".into(),
            kind: PsetNameKind::NonPset,
            notes: "Default IFC omits ES edges (governing decision §3.3)".into(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_known_and_unknown_reserved() {
        assert_eq!(
            classify_pset_name("Pset_WallCommon"),
            PsetNameKind::BuildingsmartCommon
        );
        assert_eq!(
            classify_pset_name("Pset_RevitType_Wall"),
            PsetNameKind::RevitTypeAlias
        );
        assert_eq!(
            classify_pset_name("Pset_RvtRsDiagnosticCandidate"),
            PsetNameKind::ToolkitDiagnostic
        );
        assert!(is_unknown_reserved_pset("Pset_SomethingInvented"));
        assert_eq!(classify_pset_name("MyProps"), PsetNameKind::NonPset);
    }

    #[test]
    fn mapping_examples_omit_es_by_default() {
        let ex = mapping_examples();
        assert!(
            ex.iter()
                .any(|e| e.revit_concept.contains("ES") && e.ifc_pset.contains("omitted"))
        );
    }
}
