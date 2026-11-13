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
//! `exported.bounding_box_spaces`, `exported.area_body_refusals`,
//! `exported.slab_shape_refusals`, `exported.wall_base_refusals`,
//! `exported.wall_end_refusals`, `exported.wall_profile_refusals` and
//! `exported.saved_mesh_refusals` are here too (`body_is_record_box`,
//! `space_outline_is_record_box`, `area_body_refused`, `slab_shape_refused`,
//! `wall_base_cut_refused`, `wall_end_extension_refused`,
//! `wall_profile_extension_refused`, `saved_mesh_not_used`).
//!
//! The log is off unless asked for (`ExportOptions::with_export_decisions`,
//! `rvt-ifc --export-decisions`) and bounded: it keeps at most
//! [`MAX_DECISIONS_PER_REASON`] entries of a reason, in ElementId order, and
//! counts the rest as dropped. It goes in the diagnostics and the decoded
//! model, never in the IFC. `rvt-ifc --anonymize` scrubs it as it scrubs the
//! `skipped` items ([`ExportDecisionLog::anonymize`]).
//!
//! Where an entry's stream and offset come from:
//!
//! - The reasons made from the element-record frames
//!   (`element_record_without_element_id`, `element_record_without_volume`,
//!   `element_record_in_non_primary_design_option`,
//!   `element_record_not_in_export_phase`, `room_not_in_export_phase`,
//!   `element_record_empty_curtain_panel`) name the frame their scan read. A
//!   scan that finds an element framed in several partitions keeps the last
//!   frame in scan order, partition number and then offset, with no test of
//!   vertical extent. The frame the provenance table names for an exported
//!   element ([`super::element_provenance`], `select_instance_records`) is
//!   the newest and, within one partition, the one with the greatest extent,
//!   so the two can name different frames of the same element.
//! - `element_record_frame_stopped` is made on Revit 2008 to 2023 files, from the
//!   frames of their marker scan that did not become an element: its `class` is
//!   the stage the frame stopped at (`frame_outside_stream`,
//!   `category_out_of_range`, `box_not_valid`, `no_declared_id` or
//!   `without_volume`, [`super::element_record_stages`]). Its `offset` is where the
//!   frame's ElementId is stored, 52 bytes before the marker (48 on Revit 2008),
//!   in the stream inflated; a frame stopped before its record has no ElementId,
//!   and one with no volume has its record's.
//! - The reasons made from the model (the geometry gaps, `body_is_record_box`,
//!   `space_outline_is_record_box`, `area_body_refused`) take the table's
//!   stream and offset for the element.
//! - Only the offsets of the element-record frames on Revit 2024 and later have
//!   been re-read from the bytes (`tests/export_decisions_counts.rs`). Those
//!   of Revit 2015 to 2023 files and of `Global/Latest` are not verified.
//!
//! Not recorded: [`DecisionStage::ValueLeftOutExportersDisagree`]. The values
//! left out because Revit's exporters disagree are left out by a rule for a
//! whole class of element, in the reader that would write them; no step
//! decides it for one element, and no aggregate counts them, so there is
//! nothing to log until a reader does.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::element_provenance::ElementProvenanceTable;
use super::element_record_stages::{FRAME_STOP_STAGES, FRAME_STOPPED_REASON};
use crate::partition_room_parameters::partition_number;

/// The most entries the log keeps of one reason. The rest are counted in
/// [`ExportDecisionReason::dropped`]. The reason `element_record_frame_stopped`
/// holds five stages, so its cap is shared out: it keeps at most this many
/// divided by the number of stages for each stage, in ElementId order, the
/// frames with no ElementId first, and the stage that fills its share does not
/// crowd the others out (#1389).
pub const MAX_DECISIONS_PER_REASON: usize = 5_000;

/// The reasons whose `detail` is a code rvt-rs writes, not text from the
/// project, so [`ExportDecisionLog::anonymize`] keeps it.
const CODE_DETAIL_REASONS: [&str; 8] = [
    "area_without_proven_body",
    "area_body_refused",
    "body_is_record_box",
    "slab_shape_refused",
    "wall_base_cut_refused",
    "wall_end_extension_refused",
    "wall_profile_extension_refused",
    "saved_mesh_not_used",
];

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
    /// The element's body is only its record's bounding box, or keeps its
    /// record's extent where a shape or a cut Revit applies is not read: a
    /// shape-edited floor (`slab_shape_refused`), the base of a wall
    /// (`wall_base_cut_refused`, #1384), the end of a wall
    /// (`wall_end_extension_refused`, #1396), the top of a wall past its
    /// elevation profile (`wall_profile_extension_refused`, #1473) or a box
    /// the mesh Revit saved for the element did not replace
    /// (`saved_mesh_not_used`, #1645).
    BodyLeftAsBox,
    /// A door or window has no host to cut, or an opening index row has no
    /// host geometry: no opening is cut.
    OpeningNotCut,
    /// A room, an Area or a floor keeps no outline read from the file: its
    /// record's box, a refusal or its boundary annotation alone.
    OutlineNotRead,
    /// The element is written outside any storey.
    StoreyNotBound,
    /// A value is left out because Revit's exporters disagree about it. Nothing
    /// is recorded under this stage yet: see the module documentation.
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
    /// or `body_is_record_box`, `space_outline_is_record_box`,
    /// `area_body_refused`, `slab_shape_refused`, `wall_base_cut_refused`,
    /// `wall_end_extension_refused`, `wall_profile_extension_refused` or
    /// `saved_mesh_not_used` for the decisions behind the `exported` counts.
    pub reason: String,
    /// The element's Revit ElementId. Absent for a frame that carries none
    /// (`element_record_without_element_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_id: Option<u32>,
    /// The class the aggregate counts it under: a Revit class for the
    /// element-record reasons, an IFC entity such as `IFCDOOR` for the
    /// geometry gaps and the decisions behind the `exported` counts. For
    /// `element_record_frame_stopped` it is the stage the frame stopped at
    /// (`frame_outside_stream`, `category_out_of_range`, `box_not_valid`,
    /// `no_declared_id` or `without_volume`), not a Revit class.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// The stream the evidence was read from, where known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// The byte offset of the record in that stream inflated, where known (the
    /// bytes `rvt-dump` writes as `Partitions_NN.decomp`). For the
    /// element-record reasons it is the element's last frame in scan order
    /// (partition number, then offset), which can differ from the frame the
    /// provenance table names; see the module documentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// What tells the reason apart: the phase's name for
    /// `room_not_in_export_phase` (text from the project, which
    /// [`ExportDecisionLog::anonymize`] replaces), why an Area's body is not
    /// proven for `area_without_proven_body` and `area_body_refused`, the
    /// `BodySource` for `body_is_record_box` (codes rvt-rs writes).
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

/// The stage a decision of a reason that holds several is counted under, which
/// the log sorts and caps by: the class of an `element_record_frame_stopped`
/// decision, none for any other reason.
fn stage_of(decision: &ExportDecision) -> Option<&String> {
    if decision.reason == FRAME_STOPPED_REASON {
        decision.class.as_ref()
    } else {
        None
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
    /// The kept decisions, by reason and then ElementId, partition number,
    /// stream and offset.
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
        // A partition's number, not its name as text: `Partitions/99` comes
        // before `Partitions/100`.
        let number =
            |decision: &ExportDecision| decision.stream.as_deref().map_or(0, partition_number);
        decisions.sort_by(|a, b| {
            (
                &a.reason,
                stage_of(a),
                a.element_id,
                number(a),
                &a.stream,
                a.offset,
                &a.detail,
            )
                .cmp(&(
                    &b.reason,
                    stage_of(b),
                    b.element_id,
                    number(b),
                    &b.stream,
                    b.offset,
                    &b.detail,
                ))
        });
        let mut log = Self {
            cap_per_reason: MAX_DECISIONS_PER_REASON,
            ..Self::default()
        };
        let share = MAX_DECISIONS_PER_REASON / FRAME_STOP_STAGES.len();
        let mut recorded_by_stage: BTreeMap<String, usize> = BTreeMap::new();
        for mut decision in decisions {
            let stage = stage_of(&decision);
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
            let full = match stage {
                Some(stage) => {
                    let held = recorded_by_stage.entry(stage.clone()).or_default();
                    let full = *held >= share;
                    *held += usize::from(!full);
                    full
                }
                None => reason.recorded >= MAX_DECISIONS_PER_REASON,
            };
            if full {
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

    /// Replace what an entry takes from the project, as
    /// [`super::ExportDiagnostics::anonymize`] does the `skipped` items'
    /// phases: the phase name in the `detail` of a
    /// `room_not_in_export_phase` entry becomes
    /// [`crate::redact::ANONYMIZED`]. A `detail` is kept only for the reasons
    /// whose detail is a code rvt-rs writes (`area_without_proven_body`,
    /// `area_body_refused`, `body_is_record_box`); any other reason's is
    /// replaced, so a reason added later cannot leak a name. The reason codes,
    /// classes (Revit classes and IFC entities), streams, offsets, ElementIds
    /// and counts are the file's structure, not its text, and stay.
    pub fn anonymize(&mut self) {
        for entry in &mut self.entries {
            if entry.detail.is_some() && !CODE_DETAIL_REASONS.contains(&entry.reason.as_str()) {
                entry.detail = Some(crate::redact::ANONYMIZED.to_string());
            }
        }
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
