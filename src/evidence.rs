//! Named evidence tiers and lightweight ledgers (Phase 1).
//!
//! Ledgers record research observations and typed-edge *candidates*.
//! They do **not** assert that ES remapping or BIM relations are verified.
//!
//! See `docs/research/unified-research-report.md` (§2, §3, §23).

use crate::identity::{ScopedElementRef, SourceSpan};
use serde::{Deserialize, Serialize};

/// Evidence ceiling labels from the unified research report (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EvidenceTier {
    /// Speculative / untested hypothesis.
    E0,
    /// Single-environment observation.
    E1,
    /// Multi-file or multi-release observation; no independent oracle.
    E2,
    /// Independently reproduced on redistributable / owned fixtures.
    E3,
    /// Oracle-backed + automated regression.
    E4,
    /// Promoted capability with release gate + support-matrix row.
    E5,
}

impl EvidenceTier {
    /// Human-readable label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::E0 => "E0",
            Self::E1 => "E1",
            Self::E2 => "E2",
            Self::E3 => "E3",
            Self::E4 => "E4",
            Self::E5 => "E5",
        }
    }

    /// True when the tier is high enough for a `verified` capability claim.
    pub fn meets_verified_gate(self) -> bool {
        self >= Self::E4
    }
}

/// One evidence note attached to a research observation or edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub tier: EvidenceTier,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    /// Explicit non-claims so honesty survives serialization.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub non_claims: Vec<String>,
}

impl EvidenceRecord {
    pub fn research_stub(summary: impl Into<String>) -> Self {
        Self {
            tier: EvidenceTier::E0,
            summary: summary.into(),
            fixture_id: None,
            span: None,
            non_claims: vec![
                "Does not claim ES ElementId remapping works".into(),
                "Does not claim converter-grade IFC".into(),
            ],
        }
    }
}

/// Kind of typed research edge (not a universal parent pointer).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// ES-held ElementId reference candidate.
    EsElementIdRef,
    /// ES value-tree containment (separate from BIM topology).
    EsValueTree,
    /// BIM host/opening/connect — only after evidence gates.
    BimRelation,
    /// ElemTable ownership candidate (#152) — scored separately from ES.
    ElemTableOwnership,
    /// Catch-all for research-only edges.
    Other(String),
}

/// Lightweight typed edge candidate for the research ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedEdge {
    pub kind: EdgeKind,
    pub from: ScopedElementRef,
    pub to: ScopedElementRef,
    pub tier: EvidenceTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// Append-only evidence ledger (in-memory; serialization helper for research).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EvidenceLedger {
    pub records: Vec<EvidenceRecord>,
}

impl EvidenceLedger {
    pub fn push(&mut self, record: EvidenceRecord) {
        self.records.push(record);
    }

    pub fn max_tier(&self) -> Option<EvidenceTier> {
        self.records.iter().map(|r| r.tier).max()
    }
}

/// Append-only typed-edge ledger.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeLedger {
    pub edges: Vec<TypedEdge>,
}

impl EdgeLedger {
    pub fn push(&mut self, edge: TypedEdge) {
        self.edges.push(edge);
    }

    /// ES edges must not be mixed into ElemTable ownership scoring (#152 wall).
    pub fn es_edges(&self) -> impl Iterator<Item = &TypedEdge> {
        self.edges
            .iter()
            .filter(|e| matches!(e.kind, EdgeKind::EsElementIdRef | EdgeKind::EsValueTree))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::DocumentIdentity;

    #[test]
    fn verified_gate_requires_e4() {
        assert!(!EvidenceTier::E3.meets_verified_gate());
        assert!(EvidenceTier::E4.meets_verified_gate());
    }

    #[test]
    fn ledger_round_trips_and_separates_es_edges() {
        let doc = DocumentIdentity::from_key("doc");
        let mut edges = EdgeLedger::default();
        edges.push(TypedEdge {
            kind: EdgeKind::EsElementIdRef,
            from: ScopedElementRef::from_element_id(doc.clone(), 1),
            to: ScopedElementRef::from_element_id(doc.clone(), 2),
            tier: EvidenceTier::E0,
            span: None,
            notes: vec!["stub".into()],
        });
        edges.push(TypedEdge {
            kind: EdgeKind::ElemTableOwnership,
            from: ScopedElementRef::from_element_id(doc.clone(), 3),
            to: ScopedElementRef::from_element_id(doc, 4),
            tier: EvidenceTier::E1,
            span: None,
            notes: vec![],
        });
        assert_eq!(edges.es_edges().count(), 1);
        let json = serde_json::to_value(&edges).expect("ser");
        let back: EdgeLedger = serde_json::from_value(json).expect("de");
        assert_eq!(back.edges.len(), 2);
    }
}
