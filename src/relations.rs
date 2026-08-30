//! Experimental typed-relation domains, SCC condensation, and quarantine.
//!
//! Governing decisions (§3 of `docs/research/unified-research-report.md`):
//! relationships are **typed edges with provenance**, not a universal parent
//! pointer. ES reference graphs, ES value trees, BIM topology, and ElemTable
//! ownership must stay in separate [`RelationDomain`]s so research ledgers do
//! not cross-contaminate scoring (especially ES vs #152).
//!
//! **Experimental / not production.** These helpers are architecture stubs for
//! Phase 1 leftovers. They do **not** claim ES remapping, Door/Window joins,
//! compound openings, or converter-grade IFC are solved. Nothing here is wired
//! into default IFC emission.

use crate::evidence::{EdgeKind, EvidenceTier, TypedEdge};
use crate::identity::ScopedElementRef;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// Isolated relation domain. Edges must not silently migrate across domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationDomain {
    /// ES-held ElementId / reference candidates (Lane A).
    EsRefGraph,
    /// ES value-tree containment (separate from BIM topology).
    EsValueTree,
    /// BIM host/opening/connect — only after evidence gates (Lane C).
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

    /// Map a ledger [`EdgeKind`] into a domain (fail closed on `Other`).
    pub fn for_edge_kind(kind: &EdgeKind) -> Self {
        match kind {
            EdgeKind::EsElementIdRef => Self::EsRefGraph,
            EdgeKind::EsValueTree => Self::EsValueTree,
            EdgeKind::BimRelation => Self::BimTopology,
            EdgeKind::ElemTableOwnership => Self::ElemTableOwnership,
            EdgeKind::Other(_) => Self::ResearchOther,
        }
    }
}

/// Registry of known relation domains with honesty notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationDomainRegistry {
    pub domains: Vec<RelationDomainEntry>,
    pub experimental: bool,
    pub notes: Vec<String>,
}

/// One registered domain row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationDomainEntry {
    pub domain: RelationDomain,
    pub id: String,
    /// Production wiring status — always experimental for Phase 1 leftovers.
    pub status: DomainStatus,
    pub isolation_peers: Vec<RelationDomain>,
    pub notes: Vec<String>,
}

/// Domain promotion status (intentionally coarse).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DomainStatus {
    /// Architecture stub only — not wired to product claims.
    Experimental,
    /// Research observations may be recorded; no verified claims.
    Research,
    /// Do not use for product / IFC defaults.
    Quarantined,
}

impl RelationDomainRegistry {
    /// Seed registry with honest Phase 1 leftovers status.
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

/// Opaque node id inside one domain graph (stable for serialization).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RelationNodeId(pub String);

impl RelationNodeId {
    pub fn from_scoped(r: &ScopedElementRef) -> Self {
        let eid = r
            .element_id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "unknown".into());
        Self(format!("{}#{eid}", r.document.document_key))
    }
}

/// Directed edge inside a single [`RelationDomain`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DomainEdge {
    pub from: RelationNodeId,
    pub to: RelationNodeId,
    pub kind: EdgeKind,
    pub tier: EvidenceTier,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl DomainEdge {
    pub fn from_typed(edge: &TypedEdge) -> Self {
        Self {
            from: RelationNodeId::from_scoped(&edge.from),
            to: RelationNodeId::from_scoped(&edge.to),
            kind: edge.kind.clone(),
            tier: edge.tier,
            notes: edge.notes.clone(),
        }
    }
}

/// Reason an edge or component was quarantined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineReason {
    /// Edge domain does not match the graph domain.
    CrossDomain,
    /// Evidence tier too low for the requested operation.
    LowEvidence { tier: String, required: String },
    /// Cycle / mutual reachability marked for human review.
    StronglyConnectedComponent { component_id: usize },
    /// Explicit research non-claim.
    HonestyWall { detail: String },
}

/// Quarantined edge or SCC summary.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuarantineEntry {
    pub reason: QuarantineReason,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<DomainEdge>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<RelationNodeId>,
    pub notes: Vec<String>,
}

/// Directed graph confined to one [`RelationDomain`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationGraph {
    pub domain: RelationDomain,
    pub nodes: BTreeSet<RelationNodeId>,
    pub edges: Vec<DomainEdge>,
    pub quarantine: Vec<QuarantineEntry>,
}

impl RelationGraph {
    pub fn new(domain: RelationDomain) -> Self {
        Self {
            domain,
            nodes: BTreeSet::new(),
            edges: Vec::new(),
            quarantine: Vec::new(),
        }
    }

    /// Insert an edge when its [`EdgeKind`] maps to this domain; else quarantine.
    pub fn push_typed(&mut self, edge: &TypedEdge) {
        let domain = RelationDomain::for_edge_kind(&edge.kind);
        let domain_edge = DomainEdge::from_typed(edge);
        self.nodes.insert(domain_edge.from.clone());
        self.nodes.insert(domain_edge.to.clone());
        if domain != self.domain {
            self.quarantine.push(QuarantineEntry {
                reason: QuarantineReason::CrossDomain,
                edge: Some(domain_edge),
                nodes: Vec::new(),
                notes: vec![format!(
                    "refused: edge domain {} ≠ graph domain {}",
                    domain.as_str(),
                    self.domain.as_str()
                )],
            });
            return;
        }
        self.edges.push(domain_edge);
    }

    /// Adjacency list for algorithms (node → successors).
    fn adjacency(&self) -> BTreeMap<RelationNodeId, Vec<RelationNodeId>> {
        let mut adj: BTreeMap<RelationNodeId, Vec<RelationNodeId>> = BTreeMap::new();
        for n in &self.nodes {
            adj.entry(n.clone()).or_default();
        }
        for e in &self.edges {
            adj.entry(e.from.clone()).or_default().push(e.to.clone());
            adj.entry(e.to.clone()).or_default();
        }
        adj
    }

    /// Tarjan strongly connected components.
    ///
    /// Returns components as lists of node ids. Singleton non-loop nodes are
    /// included (standard Tarjan). Components with `len >= 2` (or a self-loop)
    /// are condensation candidates for quarantine review.
    pub fn strongly_connected_components(&self) -> Vec<Vec<RelationNodeId>> {
        tarjan_scc(&self.adjacency())
    }

    /// Condensation DAG: each SCC becomes a supernode; edges between SCCs retained.
    pub fn condense(&self) -> Condensation {
        let components = self.strongly_connected_components();
        let mut node_to_comp: HashMap<RelationNodeId, usize> = HashMap::new();
        for (ci, comp) in components.iter().enumerate() {
            for n in comp {
                node_to_comp.insert(n.clone(), ci);
            }
        }

        let mut dag_edges: BTreeSet<(usize, usize)> = BTreeSet::new();
        for e in &self.edges {
            let Some(&a) = node_to_comp.get(&e.from) else {
                continue;
            };
            let Some(&b) = node_to_comp.get(&e.to) else {
                continue;
            };
            if a != b {
                dag_edges.insert((a, b));
            }
        }

        let mut cyclic: Vec<usize> = Vec::new();
        let self_loop_nodes: BTreeSet<_> = self
            .edges
            .iter()
            .filter(|e| e.from == e.to)
            .map(|e| e.from.clone())
            .collect();
        for (ci, comp) in components.iter().enumerate() {
            let multi = comp.len() >= 2;
            let looped = comp.len() == 1 && self_loop_nodes.contains(&comp[0]);
            if multi || looped {
                cyclic.push(ci);
            }
        }

        Condensation {
            domain: self.domain,
            components,
            dag_edges: dag_edges.into_iter().collect(),
            cyclic_component_ids: cyclic,
        }
    }

    /// Quarantine cyclic SCCs for human / oracle review (does not delete edges).
    pub fn quarantine_cyclic_sccs(&mut self) {
        let condensation = self.condense();
        for &ci in &condensation.cyclic_component_ids {
            let nodes = condensation.components[ci].clone();
            self.quarantine.push(QuarantineEntry {
                reason: QuarantineReason::StronglyConnectedComponent { component_id: ci },
                edge: None,
                nodes,
                notes: vec![
                    "SCC quarantined — cycles are not auto-promoted to BIM/IFC relations".into(),
                ],
            });
        }
    }

    /// Quarantine edges below a minimum evidence tier.
    pub fn quarantine_below_tier(&mut self, required: EvidenceTier) {
        let mut kept = Vec::new();
        for edge in std::mem::take(&mut self.edges) {
            if edge.tier < required {
                self.quarantine.push(QuarantineEntry {
                    reason: QuarantineReason::LowEvidence {
                        tier: edge.tier.as_str().to_string(),
                        required: required.as_str().to_string(),
                    },
                    edge: Some(edge),
                    nodes: Vec::new(),
                    notes: vec!["Below evidence gate — left in quarantine ledger".into()],
                });
            } else {
                kept.push(edge);
            }
        }
        self.edges = kept;
    }
}

/// Condensation of a [`RelationGraph`] (DAG of SCCs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condensation {
    pub domain: RelationDomain,
    pub components: Vec<Vec<RelationNodeId>>,
    /// Directed edges between component indices (`from_comp`, `to_comp`).
    pub dag_edges: Vec<(usize, usize)>,
    /// Component indices that contain a cycle (size≥2 or self-loop).
    pub cyclic_component_ids: Vec<usize>,
}

/// Tarjan SCC over an adjacency map.
fn tarjan_scc(adj: &BTreeMap<RelationNodeId, Vec<RelationNodeId>>) -> Vec<Vec<RelationNodeId>> {
    struct Tarjan<'a> {
        adj: &'a BTreeMap<RelationNodeId, Vec<RelationNodeId>>,
        index: usize,
        stack: Vec<RelationNodeId>,
        on_stack: BTreeSet<RelationNodeId>,
        indices: HashMap<RelationNodeId, usize>,
        lowlink: HashMap<RelationNodeId, usize>,
        components: Vec<Vec<RelationNodeId>>,
    }

    impl Tarjan<'_> {
        fn strongconnect(&mut self, v: &RelationNodeId) {
            self.indices.insert(v.clone(), self.index);
            self.lowlink.insert(v.clone(), self.index);
            self.index += 1;
            self.stack.push(v.clone());
            self.on_stack.insert(v.clone());

            if let Some(succs) = self.adj.get(v).cloned() {
                for w in succs {
                    if !self.indices.contains_key(&w) {
                        self.strongconnect(&w);
                        let lw = *self.lowlink.get(&w).expect("lowlink w");
                        let lv = *self.lowlink.get(v).expect("lowlink v");
                        self.lowlink.insert(v.clone(), lv.min(lw));
                    } else if self.on_stack.contains(&w) {
                        let iw = *self.indices.get(&w).expect("index w");
                        let lv = *self.lowlink.get(v).expect("lowlink v");
                        self.lowlink.insert(v.clone(), lv.min(iw));
                    }
                }
            }

            if self.lowlink.get(v) == self.indices.get(v) {
                let mut comp = Vec::new();
                loop {
                    let w = self.stack.pop().expect("stack");
                    self.on_stack.remove(&w);
                    let done = w == *v;
                    comp.push(w);
                    if done {
                        break;
                    }
                }
                self.components.push(comp);
            }
        }
    }

    let mut state = Tarjan {
        adj,
        index: 0,
        stack: Vec::new(),
        on_stack: BTreeSet::new(),
        indices: HashMap::new(),
        lowlink: HashMap::new(),
        components: Vec::new(),
    };

    for v in adj.keys() {
        if !state.indices.contains_key(v) {
            state.strongconnect(v);
        }
    }
    state.components
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::DocumentIdentity;

    fn edge(kind: EdgeKind, from: u32, to: u32, tier: EvidenceTier) -> TypedEdge {
        let doc = DocumentIdentity::from_key("doc");
        TypedEdge {
            kind,
            from: ScopedElementRef::from_element_id(doc.clone(), from),
            to: ScopedElementRef::from_element_id(doc, to),
            tier,
            span: None,
            notes: vec![],
        }
    }

    #[test]
    fn registry_marks_experimental_and_isolates_es_from_elemtable() {
        let reg = RelationDomainRegistry::phase1_stub();
        assert!(reg.experimental);
        let es = reg
            .domains
            .iter()
            .find(|d| d.domain == RelationDomain::EsRefGraph)
            .expect("es");
        assert!(
            es.isolation_peers
                .contains(&RelationDomain::ElemTableOwnership)
        );
        let json = serde_json::to_value(&reg).expect("ser");
        let back: RelationDomainRegistry = serde_json::from_value(json).expect("de");
        assert_eq!(back.domains.len(), 5);
    }

    #[test]
    fn cross_domain_edge_is_quarantined() {
        let mut g = RelationGraph::new(RelationDomain::EsRefGraph);
        g.push_typed(&edge(EdgeKind::ElemTableOwnership, 1, 2, EvidenceTier::E1));
        assert!(g.edges.is_empty());
        assert_eq!(g.quarantine.len(), 1);
        assert!(matches!(
            g.quarantine[0].reason,
            QuarantineReason::CrossDomain
        ));
    }

    #[test]
    fn tarjan_finds_cycle_and_condensation_dag() {
        // 1 → 2 → 3 → 1 (cycle) and 3 → 4 (tail)
        let mut g = RelationGraph::new(RelationDomain::ResearchOther);
        for (a, b) in [(1, 2), (2, 3), (3, 1), (3, 4)] {
            g.push_typed(&edge(EdgeKind::Other("t".into()), a, b, EvidenceTier::E0));
        }
        let comps = g.strongly_connected_components();
        let cyclic = comps.iter().find(|c| c.len() >= 2).expect("cycle scc");
        assert_eq!(cyclic.len(), 3);
        let cond = g.condense();
        assert!(!cond.cyclic_component_ids.is_empty());
        // Exactly one DAG edge: cycle-comp → {4}
        assert_eq!(cond.dag_edges.len(), 1);
    }

    #[test]
    fn quarantine_cyclic_and_low_tier() {
        let mut g = RelationGraph::new(RelationDomain::ResearchOther);
        g.push_typed(&edge(EdgeKind::Other("a".into()), 1, 2, EvidenceTier::E0));
        g.push_typed(&edge(EdgeKind::Other("a".into()), 2, 1, EvidenceTier::E0));
        g.push_typed(&edge(EdgeKind::Other("a".into()), 3, 4, EvidenceTier::E1));
        g.quarantine_cyclic_sccs();
        assert!(g.quarantine.iter().any(|q| matches!(
            q.reason,
            QuarantineReason::StronglyConnectedComponent { .. }
        )));
        g.quarantine_below_tier(EvidenceTier::E2);
        assert!(g.edges.is_empty());
        assert!(
            g.quarantine
                .iter()
                .any(|q| matches!(q.reason, QuarantineReason::LowEvidence { .. }))
        );
    }

    #[test]
    fn condensation_round_trips_json() {
        let mut g = RelationGraph::new(RelationDomain::EsRefGraph);
        g.push_typed(&edge(EdgeKind::EsElementIdRef, 10, 20, EvidenceTier::E0));
        let cond = g.condense();
        let json = serde_json::to_string(&cond).expect("ser");
        let back: Condensation = serde_json::from_str(&json).expect("de");
        assert_eq!(back.domain, RelationDomain::EsRefGraph);
        assert_eq!(back.components.len(), 2);
    }
}
