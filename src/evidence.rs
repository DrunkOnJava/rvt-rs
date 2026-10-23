//! Named evidence tiers and lightweight ledgers of research observations.
//!
//! Ledgers record research observations and typed-edge *candidates*.
//! They do **not** assert that ES remapping or BIM relations are verified.
//!
//! See `docs/research/unified-research-report.md` (§2, §3, §23).

pub use crate::capability::EvidenceTier;
use crate::identity::{ScopedElementRef, SourceSpan};
use serde::{Deserialize, Serialize};

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

    /// Serialize to pretty JSON for research artifacts.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON (round-trip helper).
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
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

    /// Serialize to pretty JSON for research artifacts.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize from JSON (round-trip helper).
    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
