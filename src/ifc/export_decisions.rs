//! Why an element, body, opening or outline was left out, one entry per
//! element (#1326).
//!
//! The diagnostics' `skipped` section counts what the exporter left out by
//! reason and class, with sample names. That cannot answer "why does this door
//! have no opening?" or "why is this room a box?". An [`ExportDecisionLog`]
//! lists each fail-closed decision as an [`ExportDecision`]: the element's
//! ElementId, the [`DecisionStage`], a reason code and, where known, the
//! stream and byte offset its record was read from (the offsets of
//! [`super::element_provenance`]).
//!
//! The `skipped` counts are made from the same decisions, so the two cannot
//! disagree: every item of `skipped` has a reason here whose `total` is its
//! count, and, for a reason that dropped none, whose entries by class are its
//! classes. The decisions behind `exported.bounding_box_bodies`,
//! `exported.bounding_box_spaces` and `exported.area_body_refusals` are here
//! too (`body_is_record_box`, `space_outline_is_record_box`,
//! `area_body_refused`).
//!
//! The log is off unless asked for (`ExportOptions::with_export_decisions`,
//! `rvt-ifc --export-decisions`) and bounded: it keeps at most
//! [`MAX_DECISIONS_PER_REASON`] entries of a reason, in ElementId order, and
//! counts the rest as dropped. It goes in the diagnostics and the decoded
//! model, never in the IFC.
//!
//! Not recorded: [`DecisionStage::ValueLeftOutExportersDisagree`]. The values
//! left out because Revit's exporters disagree are left out by a rule for a
//! whole class of element, in the reader that would write them; no step
//! decides it for one element, and no aggregate counts them, so there is
//! nothing to log until a reader does.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::element_provenance::ElementProvenanceTable;

/// The most entries the log keeps of one reason. The rest are counted in
/// [`ExportDecisionReason::dropped`].
pub const MAX_DECISIONS_PER_REASON: usize = 5_000;

/// Which fail-closed step made a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DecisionStage {
    /// The element is not in the export.
    ElementSkipped,
    /// The element is written with no body: its curve, profile or dimensions
    /// were not read.
    BodyNotDrawn,
    /// The element's body is only its record's bounding box.
    BodyLeftAsBox,
    /// A door or window has no host to cut, or an opening index row has no
    /// host geometry: no opening is cut.
    OpeningNotCut,
    /// A room, an Area or a floor keeps no outline read from the file: its
    /// record's box, a refusal or its boundary annotation alone.
    OutlineNotRead,
    /// The element is written outside any storey.
    StoreyNotBound,
    /// A value is left out because Revit's exporters disagree about it. Not
    /// recorded yet: see the module documentation.
    ValueLeftOutExportersDisagree,
}

impl DecisionStage {
    /// The stage as the JSON diagnostics write it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ElementSkipped => "element_skipped",
            Self::BodyNotDrawn => "body_not_drawn",
            Self::BodyLeftAsBox => "body_left_as_box",
            Self::OpeningNotCut => "opening_not_cut",
            Self::OutlineNotRead => "outline_not_read",
            Self::StoreyNotBound => "storey_not_bound",
            Self::ValueLeftOutExportersDisagree => "value_left_out_exporters_disagree",
        }
    }
}

/// One fail-closed decision about one element.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExportDecision {
    /// The step that made it.
    pub stage: DecisionStage,
    /// The reason code: the `reason` of the `skipped` item it is counted in,
    /// or `body_is_record_box`, `space_outline_is_record_box` or
    /// `area_body_refused` for the decisions behind the `exported` counts.
    pub reason: String,
    /// The element's Revit ElementId. Absent for a frame that carries none
    /// (`element_record_without_element_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_id: Option<u32>,
    /// The class the aggregate counts it under: a Revit class for the
    /// element-record reasons, an IFC entity such as `IFCDOOR` for the
    /// geometry gaps and the decisions behind the `exported` counts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// The stream the evidence was read from, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// The byte offset of the record in that stream inflated, where known (the
    /// bytes `rvt-dump` writes as `Partitions_NN.decomp`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// What tells the reason apart: the phase's name for
    /// `room_not_in_export_phase`, why an Area's body is not proven for
    /// `area_without_proven_body` and `area_body_refused`, the `BodySource`
    /// for `body_is_record_box`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl ExportDecision {
    /// A decision with no element, class, evidence or detail yet.
    pub(crate) fn new(stage: DecisionStage, reason: &str) -> Self {
        Self {
            stage,
            reason: reason.to_string(),
            element_id: None,
            class: None,
            stream: None,
            offset: None,
            detail: None,
        }
    }
}

/// How many decisions a reason has, and how many the log kept.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExportDecisionReason {
    /// The stage of every decision of this reason.
    pub stage: DecisionStage,
    /// Every decision of this reason, kept or not: the count the `skipped`
    /// item (or the `exported` count) of the reason holds.
    pub total: usize,
    /// How many are in [`ExportDecisionLog::entries`].
    pub recorded: usize,
    /// How many [`MAX_DECISIONS_PER_REASON`] left out: `total` less `recorded`.
    pub dropped: usize,
}

/// The decisions of one export, bounded per reason.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ExportDecisionLog {
    /// The most entries kept of one reason ([`MAX_DECISIONS_PER_REASON`]).
    pub cap_per_reason: usize,
    /// Decisions left out by the cap, over all reasons.
    pub dropped: usize,
    /// Each reason with decisions, with its counts.
    pub reasons: BTreeMap<String, ExportDecisionReason>,
    /// The kept decisions, by reason and then ElementId, stream and offset.
    pub entries: Vec<ExportDecision>,
}

impl ExportDecisionLog {
    /// The log of `decisions`, sorted, capped per reason and given the stream
    /// and offset of the element's record from `provenance` where a decision
    /// has none and the table lists the element.
    pub fn build(
        mut decisions: Vec<ExportDecision>,
        provenance: Option<&ElementProvenanceTable>,
    ) -> Self {
        decisions.sort_by(|a, b| {
            (&a.reason, a.element_id, &a.stream, a.offset, &a.detail).cmp(&(
                &b.reason,
                b.element_id,
                &b.stream,
                b.offset,
                &b.detail,
            ))
        });
        let mut log = Self {
            cap_per_reason: MAX_DECISIONS_PER_REASON,
            ..Self::default()
        };
        for mut decision in decisions {
            let reason = log
                .reasons
                .entry(decision.reason.clone())
                .or_insert_with(|| ExportDecisionReason {
                    stage: decision.stage,
                    total: 0,
                    recorded: 0,
                    dropped: 0,
                });
            reason.total += 1;
            if reason.recorded >= MAX_DECISIONS_PER_REASON {
                reason.dropped += 1;
                log.dropped += 1;
                continue;
            }
            reason.recorded += 1;
            if decision.stream.is_none() {
                let located = decision
                    .element_id
                    .and_then(|id| provenance?.get(id))
                    .filter(|entry| entry.is_located());
                if let Some(entry) = located {
                    decision.stream = entry.stream.clone();
                    decision.offset = entry.offset;
                }
            }
            log.entries.push(decision);
        }
        log
    }

    /// The kept decisions about the element with `element_id`.
    pub fn for_element(&self, element_id: u32) -> impl Iterator<Item = &ExportDecision> {
        self.entries
            .iter()
            .filter(move |decision| decision.element_id == Some(element_id))
    }

    /// The reasons that dropped decisions: an element's decisions of these
    /// may be missing from [`Self::entries`].
    pub fn capped_reasons(&self) -> impl Iterator<Item = &str> {
        self.reasons
            .iter()
            .filter(|(_, reason)| reason.dropped > 0)
            .map(|(name, _)| name.as_str())
    }
}
