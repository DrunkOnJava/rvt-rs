//! Honest capability manifest snapshot (Phase 1 leftovers).
//!
//! Emits research + product capability rows aligned with
//! `docs/support-matrix.json` honesty ceilings. Does **not** claim ES
//! remapping, compound openings, or converter-grade IFC.
//!
//! Schema: `docs/schemas/capability-manifest.schema.json` (and the ES
//! promotion stub at `docs/schemas/es-capability.schema.json`).

use crate::evidence::EvidenceTier;
use crate::relations::{RelationDomain, RelationDomainRegistry};
use serde::{Deserialize, Serialize};

/// Manifest status vocabulary (subset of support-matrix + ES schema).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honest_snapshot_marks_es_and_compound_unsupported() {
        let m = CapabilityManifest::honest_snapshot();
        assert_eq!(
            m.get("es.elementid_remap").unwrap().status,
            CapabilityStatus::Unsupported
        );
        assert_eq!(
            m.get("arcwall.compound_0x0821").unwrap().status,
            CapabilityStatus::Unsupported
        );
        assert_eq!(
            m.get("arcwall.standard_2023").unwrap().status,
            CapabilityStatus::Verified
        );
        assert!(m.relation_domains.experimental);
    }

    #[test]
    fn manifest_json_round_trip() {
        let m = CapabilityManifest::honest_snapshot();
        let s = m.to_json_string().expect("ser");
        let back = CapabilityManifest::from_json_str(&s).expect("de");
        assert_eq!(back.capabilities.len(), m.capabilities.len());
        assert!(s.contains("es.elementid_remap"));
        assert!(s.contains("unsupported"));
    }
}
