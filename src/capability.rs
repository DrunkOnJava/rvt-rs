//! The capability manifest: what rvt-rs reads, at the status
//! `docs/support-matrix.json` allows.
//!
//! Emits research + product capability rows aligned with
//! `docs/support-matrix.json` honesty ceilings. Does **not** claim ES
//! remapping, compound openings, or converter-grade IFC.
//!
//! Schema: `docs/schemas/capability-manifest.schema.json` (and the ES
//! promotion stub at `docs/schemas/es-capability.schema.json`).

use serde::{Deserialize, Serialize};

/// `docs/support-matrix.json` as built into this crate: every capability with
/// its status ceiling, evidence and user impact. The one list of what rvt-rs
/// can do; README, `docs/status.md` and `docs/supported-profile.md` summarise
/// it, and `tests/support_matrix.rs` keeps it honest.
pub const SUPPORT_MATRIX_JSON: &str = include_str!("../docs/support-matrix.json");

/// How rvt-rs reads a Revit release's element records (#871): the
/// [`SUPPORT_MATRIX_JSON`] row that holds them and that row's status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ReleaseSupport {
    /// The row's status: `verified`, `partial` or `experimental`, or
    /// `unsupported` where no element records of the release are read.
    pub status: String,
    /// The row's id, or `None` where no element records of the release are
    /// read.
    pub capability: Option<String>,
}

/// The [`SUPPORT_MATRIX_JSON`] row that holds a release's element records,
/// or `None` for a release whose element records are not read.
pub fn release_capability_id(revit_version: u32) -> Option<&'static str> {
    match revit_version {
        2008..=2010 => Some("element-records-revit-2008-2010"),
        2011..=2014 => Some("element-records-revit-2011-2014"),
        2015..=2018 => Some("element-records-revit-2015-2018"),
        2019..=2022 => Some("element-records-revit-2019-2022"),
        2023 => Some("element-records-revit-2023"),
        2024 => Some("typed-project-elements"),
        2025 => Some("element-records-revit-2025"),
        2026 => Some("element-records-revit-2026"),
        2027 => Some("element-records-revit-2027"),
        _ => None,
    }
}

/// How a release's element records are read, from the support matrix built
/// into this crate, so it cannot disagree with the matrix.
pub fn release_support(revit_version: u32) -> ReleaseSupport {
    let capability = release_capability_id(revit_version);
    let status = capability
        .and_then(|id| {
            let matrix: serde_json::Value = serde_json::from_str(SUPPORT_MATRIX_JSON).ok()?;
            matrix["capabilities"]
                .as_array()?
                .iter()
                .find(|row| row["id"] == id)?["status"]
                .as_str()
                .map(str::to_string)
        })
        .unwrap_or_else(|| "unsupported".to_string());
    ReleaseSupport {
        status,
        capability: capability.map(str::to_string),
    }
}

/// Manifest status vocabulary (subset of support-matrix + ES schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityStatus {
    Unsupported,
    Research,
    Experimental,
    Partial,
    Verified,
}

impl CapabilityStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Research => "research",
            Self::Experimental => "experimental",
            Self::Partial => "partial",
            Self::Verified => "verified",
        }
    }
}

/// One capability row in the doctor / CLI snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CapabilityRecord {
    pub capability_id: String,
    pub status: CapabilityStatus,
    pub evidence_tier: EvidenceTier,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub non_claims: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub support_matrix_row: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Versioned capability manifest for CLI / doctor emission.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CapabilityManifest {
    pub schema_version: u32,
    pub manifest_id: String,
    pub experimental: bool,
    pub capabilities: Vec<CapabilityRecord>,
    pub relation_domains: RelationDomainRegistry,
    pub honesty: Vec<String>,
}

impl CapabilityManifest {
    /// Built-in honest snapshot — ArcWall 2023 validated path, compound /
    /// ES / Door-Window unsupported or research-only.
    pub fn honest_snapshot() -> Self {
        let capabilities = vec![
            CapabilityRecord {
                capability_id: "arcwall.standard_2023".into(),
                status: CapabilityStatus::Verified,
                evidence_tier: EvidenceTier::E4,
                claims: vec![
                    "2023 standard ArcWall (variant 0x07fa) partition decode on gated path".into(),
                ],
                non_claims: vec![
                    "Does not cover 2024+ ArcWall envelope".into(),
                    "Does not decode compound variant 0x0821 openings".into(),
                ],
                support_matrix_row: Some("typed-project-elements".into()),
                notes: Some("RE-14.3 / arc_wall_record; version-gated".into()),
            },
            CapabilityRecord {
                capability_id: "arcwall.compound_0x0821".into(),
                status: CapabilityStatus::Unsupported,
                evidence_tier: EvidenceTier::E1,
                claims: vec![],
                non_claims: vec![
                    "No compound-opening decoder".into(),
                    "Marker tokenization is research-only when present".into(),
                ],
                support_matrix_row: Some("typed-door-window".into()),
                notes: Some(
                    "Lane C framing harness only — do not name decode_compound_openings".into(),
                ),
            },
            CapabilityRecord {
                capability_id: "es.elementid_remap".into(),
                status: CapabilityStatus::Unsupported,
                evidence_tier: EvidenceTier::E0,
                claims: vec![],
                non_claims: vec![
                    "ES ElementId remapping not implemented".into(),
                    "Phase 2 fixture families blocked on Revit API oracle".into(),
                ],
                support_matrix_row: None,
                notes: Some("research/es-remap scaffold + contracts only".into()),
            },
            CapabilityRecord {
                capability_id: "typed-door-window".into(),
                status: CapabilityStatus::Unsupported,
                evidence_tier: EvidenceTier::E0,
                claims: vec![],
                non_claims: vec!["RE-19 negative — no Door vs Window discriminator".into()],
                support_matrix_row: Some("typed-door-window".into()),
                notes: None,
            },
            CapabilityRecord {
                capability_id: "converter-grade-rvt-ifc".into(),
                status: CapabilityStatus::Unsupported,
                evidence_tier: EvidenceTier::E0,
                claims: vec![],
                non_claims: vec!["Not a production Revit→IFC converter".into()],
                support_matrix_row: Some("converter-grade-rvt-ifc".into()),
                notes: None,
            },
            CapabilityRecord {
                capability_id: "relations.typed_domains".into(),
                status: CapabilityStatus::Experimental,
                evidence_tier: EvidenceTier::E0,
                claims: vec![
                    "RelationDomain registry + SCC/condensation/quarantine stubs exist".into(),
                ],
                non_claims: vec![
                    "Not wired into production IFC or topology claims".into(),
                    format!(
                        "ES domains isolated from {}",
                        RelationDomain::ElemTableOwnership.as_str()
                    ),
                ],
                support_matrix_row: None,
                notes: Some("Phase 1 leftover architecture — experimental".into()),
            },
            CapabilityRecord {
                capability_id: "transmission_data.detect".into(),
                status: CapabilityStatus::Partial,
                evidence_tier: EvidenceTier::E2,
                claims: vec![
                    "UTF-16LE / empty / opaque classification for TransmissionData".into(),
                ],
                non_claims: vec!["Linked-model resolution unsupported".into()],
                support_matrix_row: None,
                notes: Some("Detect-only; empty list ≠ no links".into()),
            },
        ];

        Self {
            schema_version: 1,
            manifest_id: "rvt-rs-capability-manifest".into(),
            experimental: true,
            capabilities,
            relation_domains: RelationDomainRegistry::phase1_stub(),
            honesty: vec![
                "Statuses are fail-closed; verified requires E4+ and a support-matrix update."
                    .into(),
                "Credit research posture to @STE1200 / Discussion #112; product gates remain maintainer-owned."
                    .into(),
            ],
        }
    }

    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json_str(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }

    /// Lookup by capability id.
    pub fn get(&self, id: &str) -> Option<&CapabilityRecord> {
        self.capabilities.iter().find(|c| c.capability_id == id)
    }
}

/// Evidence ceiling labels from the unified research report (§2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
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

/// Isolated relation domain. Edges must not silently migrate across domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RelationDomain {
    /// ES-held ElementId / reference candidates.
    EsRefGraph,
    /// ES value-tree containment (separate from BIM topology).
    EsValueTree,
    /// BIM host/opening/connect — only after evidence gates.
    BimTopology,
    /// ElemTable ownership candidates (#152) — scored separately from ES.
    ElemTableOwnership,
    /// Catch-all research-only domain.
    ResearchOther,
}

impl RelationDomain {
    /// Stable id for manifests / CLI output.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EsRefGraph => "es_ref_graph",
            Self::EsValueTree => "es_value_tree",
            Self::BimTopology => "bim_topology",
            Self::ElemTableOwnership => "elem_table_ownership",
            Self::ResearchOther => "research_other",
        }
    }

    /// Domains that must never contribute to each other's promotion scores.
    pub fn isolation_peers(self) -> &'static [RelationDomain] {
        match self {
            Self::EsRefGraph | Self::EsValueTree => &[Self::ElemTableOwnership],
            Self::ElemTableOwnership => &[Self::EsRefGraph, Self::EsValueTree],
            Self::BimTopology | Self::ResearchOther => &[],
        }
    }
}

/// Registry of known relation domains with honesty notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RelationDomainRegistry {
    pub domains: Vec<RelationDomainEntry>,
    pub experimental: bool,
    pub notes: Vec<String>,
}

/// One registered domain row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RelationDomainEntry {
    pub domain: RelationDomain,
    pub id: String,
    /// Production wiring status; every domain is experimental today.
    pub status: DomainStatus,
    pub isolation_peers: Vec<RelationDomain>,
    pub notes: Vec<String>,
}

/// Domain promotion status (intentionally coarse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DomainStatus {
    /// Architecture stub only — not wired to product claims.
    Experimental,
    /// Research observations may be recorded; no verified claims.
    Research,
    /// Do not use for product / IFC defaults.
    Quarantined,
}

impl RelationDomainRegistry {
    /// The registry as it stands: every domain experimental, with its
    /// isolation peers and notes.
    pub fn phase1_stub() -> Self {
        let domains = [
            RelationDomain::EsRefGraph,
            RelationDomain::EsValueTree,
            RelationDomain::BimTopology,
            RelationDomain::ElemTableOwnership,
            RelationDomain::ResearchOther,
        ]
        .into_iter()
        .map(|domain| RelationDomainEntry {
            domain,
            id: domain.as_str().to_string(),
            status: DomainStatus::Experimental,
            isolation_peers: domain.isolation_peers().to_vec(),
            notes: match domain {
                RelationDomain::EsRefGraph => {
                    vec!["ES ElementId remapping not claimed (H-ES5 Phase 2 Revit-blocked)".into()]
                }
                RelationDomain::EsValueTree => {
                    vec!["Separate from BIM topology per governing decision §3.2".into()]
                }
                RelationDomain::BimTopology => {
                    vec!["Bound by RE-19 / RE-20 negatives; no invented Door/Window joins".into()]
                }
                RelationDomain::ElemTableOwnership => {
                    vec!["#152 scoring wall — ES refs must not contribute".into()]
                }
                RelationDomain::ResearchOther => {
                    vec!["Catch-all; never promote without an explicit domain move".into()]
                }
            },
        })
        .collect();
        Self {
            domains,
            experimental: true,
            notes: vec![
                "RelationDomainRegistry is experimental architecture — not production topology."
                    .into(),
                "Default IFC must not emit ES-derived edges (G-IFC).".into(),
            ],
        }
    }
}
