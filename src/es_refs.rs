//! ES reference occurrence locator contracts (Phase 1).
//!
//! Public API types for localizing ElementId (and related) references
//! inside Extensible Storage / ElementSettings-adjacent payloads once an
//! oracle-backed decoder exists. **No ES byte layout is invented here.**
//!
//! See `docs/research/unified-research-report.md` (§15, §30) and
//! `research/es-remap/README.md`.

use crate::evidence::EvidenceTier;
use crate::identity::{DocumentIdentity, ScopedElementRef, SourceSpan};
use serde::{Deserialize, Serialize};

/// One segment in a logical path into an ES value tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EsPathSegment {
    /// Named field / schema key.
    Field { name: String },
    /// Array or list index.
    Index { index: u64 },
    /// Map key when the value tree is dictionary-shaped.
    MapKey { key: String },
    /// Opaque segment when the shape is not yet classified (fail closed).
    Opaque { label: String },
}

/// Locator for a single candidate ES-held reference occurrence.
///
/// Presence of this type in memory or JSON does **not** mean remapping
/// was observed — only that a research harness may record a candidate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EsReferenceOccurrence {
    pub document: DocumentIdentity,
    /// Host element that owns the ES storage entity, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<ScopedElementRef>,
    /// Referenced element id as interpreted by the observation (not proof).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub referenced: Option<ScopedElementRef>,
    /// Logical path inside the ES value tree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<EsPathSegment>,
    /// Byte span when localized; absent until a decoder exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    /// Evidence ceiling for this occurrence record.
    pub tier: EvidenceTier,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl EsReferenceOccurrence {
    /// Research stub: document-scoped placeholder with E0 tier.
    pub fn stub(document: DocumentIdentity) -> Self {
        Self {
            document,
            host: None,
            referenced: None,
            path: Vec::new(),
            span: None,
            tier: EvidenceTier::E0,
            notes: vec!["EsReferenceOccurrence stub — no ES decoder; remapping not claimed".into()],
        }
    }
}

/// Semantic mutation applied between a before/after fixture pair.
///
/// Fixture law: **one mutation per transition** (governing decision §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "mutation", rename_all = "snake_case")]
pub enum FixtureMutation {
    /// No semantic change — localization noise baseline.
    NoOp,
    /// Identity-preserving save / reserialize baseline.
    IdentitySave,
    /// Remap a single ElementId reference (R1 family).
    RemapElementId { from_id: u32, to_id: u32 },
    /// Null / clear a reference (N-family).
    NullReference { element_id: u32 },
    /// Copy / duplicate semantics (C-family) — details in manifest.
    CopyEntity { label: String },
    /// Explicitly unsupported / not yet classified — fail closed.
    Unsupported { reason: String },
}

/// Before/after fixture transition contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FixtureTransition {
    pub transition_id: String,
    pub before_fixture_id: String,
    pub after_fixture_id: String,
    pub mutation: FixtureMutation,
    /// Evidence tier for the transition record itself (usually E0 until oracle).
    pub tier: EvidenceTier,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl FixtureTransition {
    /// No-op baseline contract used by Phase A manifests.
    pub fn no_op_baseline(transition_id: impl Into<String>, fixture_id: impl Into<String>) -> Self {
        let id = fixture_id.into();
        Self {
            transition_id: transition_id.into(),
            before_fixture_id: id.clone(),
            after_fixture_id: id,
            mutation: FixtureMutation::NoOp,
            tier: EvidenceTier::E0,
            notes: vec!["No-op baseline — requires Revit oracle before byte claims".into()],
        }
    }
}

/// Phase 2 family identifiers (documentation / manifest only).
pub mod families {
    pub const N1: &str = "N1";
    pub const N2: &str = "N2";
    pub const N3: &str = "N3";
    pub const N4: &str = "N4";
    pub const R1: &str = "R1";
    pub const R2: &str = "R2";
    pub const C1: &str = "C1";
    pub const C2: &str = "C2";
    pub const C3A: &str = "C3a";
    pub const C4A: &str = "C4a";
    pub const ES_REMAP_00: &str = "ES-remap-00";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn occurrence_stub_serializes() {
        let occ = EsReferenceOccurrence::stub(DocumentIdentity::from_key("es-remap-00"));
        let v = serde_json::to_value(&occ).expect("ser");
        assert_eq!(v["tier"], "E0");
        let back: EsReferenceOccurrence = serde_json::from_value(v).expect("de");
        assert!(back.span.is_none());
        assert!(back.path.is_empty());
    }

    #[test]
    fn no_op_transition_is_single_mutation() {
        let t = FixtureTransition::no_op_baseline("t-noop", "S_All");
        assert!(matches!(t.mutation, FixtureMutation::NoOp));
        assert_eq!(t.before_fixture_id, t.after_fixture_id);
        let v = serde_json::to_value(&t).expect("ser");
        let back: FixtureTransition = serde_json::from_value(v).expect("de");
        assert_eq!(back.transition_id, "t-noop");
    }

    #[test]
    fn path_segments_round_trip() {
        let path = vec![
            EsPathSegment::Field {
                name: "entity".into(),
            },
            EsPathSegment::Index { index: 0 },
            EsPathSegment::MapKey {
                key: "WallRef".into(),
            },
        ];
        let v = serde_json::to_value(&path).expect("ser");
        let back: Vec<EsPathSegment> = serde_json::from_value(v).expect("de");
        assert_eq!(back.len(), 3);
    }
}
