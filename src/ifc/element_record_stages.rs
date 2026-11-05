//! Where the element records of a Revit 2008 to 2023 file stop, stage by stage
//! (#1389).
//!
//! On a 2024 or later file the `skipped` section counts the frames that were
//! rejected (`element_record_without_element_id`, `element_record_without_volume`).
//! A 2008 to 2023 file takes another record layout, whose scan decided without
//! counting, so a file that exported no element showed only
//! `low_confidence_schema_scan_candidate` and nobody could tell which stage lost
//! its elements. [`ElementRecordStages`] is the count, made where the scan and the
//! recovery already decide, per partition stream and in total:
//!
//! 1. `markers`: places that hold the file's element-record marker, the tags of
//!    `Outline`, four `ff` bytes and `ElementParents`.
//! 2. `declared_id`: of the markers, those with a `u32` ElementId that
//!    `Global/ElemTable` declares 52 bytes before them (48 on Revit 2008) and the tag
//!    of `ElementHeader` 8 bytes after it. Not a step the markers must pass: a
//!    placed instance whose frame lies inside its element's record gets that
//!    record's id instead (`id_from_enclosing_record`, RE-315).
//! 3. A marker that gives no record stops at one of `frame_outside_stream` (a field
//!    of its frame lies outside the stream), `category_out_of_range` (its
//!    `BuiltInCategory`), `box_not_valid` (its bounding box is not finite or ends
//!    before it starts) or `no_declared_id` (no id of its own and none from the
//!    record around it). The rest are `records`. A frame is counted once, at the
//!    first of these it fails, and the checks run in the order of the scan: the
//!    category before the box, and the box before the id, so a frame with an
//!    out-of-range category and no declared id is a `category_out_of_range` stop.
//!    A partition stream that does not inflate has no markers to count and is
//!    counted in `stream_not_read`.
//! 4. Of the records, `placed_instances` are those that pass the RE-21 rule (no
//!    container reference, the placed-instance kind); the others
//!    (`not_placed_instances`) are container members and type symbols.
//! 5. Of the placed instances, `with_volume` have a bounding box with volume; the
//!    `without_volume` frames have no 3D body, and Revit's export leaves them out
//!    (#309).
//! 6. Of the frames with volume, one is kept for each ElementId, the newest
//!    (`selected`); the others are `superseded` by it, or are `nested_components`
//!    that the export leaves out (#96).
//! 7. Of the selected frames, `handed_to_export` become elements; the others are
//!    `in_non_primary_design_option`, `not_in_export_phase`,
//!    `nested_in_door_or_window` or `category_not_exported` (no class of the
//!    export has the frame's category).
//! 8. [`ElementRecordStages::written_to_model`] counts, in the IFC model, the
//!    elements of the last step. The others were set aside after the recovery,
//!    and each is counted once: [`ElementRecordStages::id_already_held`] (an
//!    element of an earlier reader already held the ElementId),
//!    [`ElementRecordStages::below_min_confidence`], or, for an element the
//!    exporter writes nothing for, by its Revit class in
//!    [`ElementRecordStages::not_written_by_class`].
//!
//! Each step counts exactly what the one before it passed on, so the counts of a
//! stream add up ([`StageCounts::unbalanced`]), and the elements handed to the
//! export add up to the elements written and the elements set aside
//! ([`ElementRecordStages::unbalanced`]). When no record of the file is written,
//! [`ElementRecordStages::stopped_at`] names the stage they stopped at.
//!
//! The counts are integers, added where the scan already decides, for every
//! export of a project file of these releases: the default export lists no frame
//! and allocates no name for one. The diagnostics only; the IFC is not changed.
//! The frames that stopped at steps 3 and 5 are also in `skipped` as
//! `element_record_frame_stopped`, by stage, counted from those integers, and,
//! when the log of decisions is asked for, listed in it with the stream and the
//! offset of the ElementId's place. Every export of a file of these releases has
//! the report, even when each count is zero (that is what says where the records
//! stopped), but a placeholder export, which writes none of what was decoded. A
//! family file (`.rfa`, `.rft`) reads its elements from `Global/Latest`, so its
//! zero records mean nothing, and `rvt-ifc` prints no line for one; the file name
//! tells, as the streams do not (the synthetic project fixtures carry a `PartAtom`
//! stream as a family does).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::export_decisions::{DecisionStage, ExportDecision};
use super::{IfcModel, SkippedExportItem, element_tag, entities::IfcEntity};
use crate::partition_element_records::LeftOutElement;
use crate::partition_element_records_2023::RecordStages;

/// The reason of the `skipped` item, and of the decisions, for the frames that
/// stopped before they became an element (#1389). Its classes are the stages:
/// [`FRAME_STOP_STAGES`].
pub const FRAME_STOPPED_REASON: &str = "element_record_frame_stopped";

/// The stages [`FRAME_STOPPED_REASON`] counts frames at, in the order of the
/// scan.
pub const FRAME_STOP_STAGES: [&str; 5] = [
    "frame_outside_stream",
    "category_out_of_range",
    "box_not_valid",
    "no_declared_id",
    "without_volume",
];

/// The counts of one partition stream, or of the whole file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StageCounts {
    /// Places that hold the file's element-record marker.
    #[serde(default)]
    pub markers: usize,
    /// Of the markers, those with a declared `u32` ElementId 52 bytes before
    /// them and the tag of `ElementHeader` 8 bytes after it. A marker without one
    /// can still become a record through the record around it.
    #[serde(default)]
    pub declared_id: usize,
    /// Markers stopped because a field of their frame lies outside the stream.
    #[serde(default)]
    pub frame_outside_stream: usize,
    /// Markers stopped because their `BuiltInCategory` is outside the range of
    /// Revit's categories.
    #[serde(default)]
    pub category_out_of_range: usize,
    /// Markers stopped because their bounding box is not finite or ends before it
    /// starts.
    #[serde(default)]
    pub box_not_valid: usize,
    /// Markers stopped because the frame has no declared ElementId of its own and
    /// none from a record around it. It can include frames inside the documents
    /// of loaded families (RE-135), whose ids no ElemTable record declares: a
    /// count here is not a count of lost elements.
    #[serde(default)]
    pub no_declared_id: usize,
    /// Markers that gave a record.
    #[serde(default)]
    pub records: usize,
    /// Of the records, those whose ElementId is the one of the record around the
    /// frame (the second prologue, RE-315).
    #[serde(default)]
    pub id_from_enclosing_record: usize,
    /// Records that are not placed instances: container members and type symbols.
    #[serde(default)]
    pub not_placed_instances: usize,
    /// Records that are placed instances.
    #[serde(default)]
    pub placed_instances: usize,
    /// Placed instances whose bounding box has no volume.
    #[serde(default)]
    pub without_volume: usize,
    /// Placed instances whose bounding box has volume.
    #[serde(default)]
    pub with_volume: usize,
    /// Frames with volume that another, newer frame of the same ElementId
    /// replaces.
    #[serde(default)]
    pub superseded: usize,
    /// Frames with volume that are nested MEP components, left out of the export.
    #[serde(default)]
    pub nested_components: usize,
    /// The newest frame of each ElementId, without the nested components.
    #[serde(default)]
    pub selected: usize,
    /// Selected frames in a design option other than the primary one.
    #[serde(default)]
    pub in_non_primary_design_option: usize,
    /// Selected frames of an element that does not exist in the export phase.
    #[serde(default)]
    pub not_in_export_phase: usize,
    /// Selected frames of a component nested in a door or window.
    #[serde(default)]
    pub nested_in_door_or_window: usize,
    /// Selected frames whose category no class of the export has.
    #[serde(default)]
    pub category_not_exported: usize,
    /// Selected frames that became elements handed to the export.
    #[serde(default)]
    pub handed_to_export: usize,
    /// Partition streams that did not inflate, so that their markers are not
    /// counted: 1 on the entry of such a stream, 0 on the others. It enters no
    /// step.
    #[serde(default)]
    pub stream_not_read: usize,
}

impl StageCounts {
    /// The frames that stopped before they became an element, by the stage:
    /// the entries of [`FRAME_STOP_STAGES`] that are not 0.
    pub(crate) fn frame_stops(&self) -> BTreeMap<String, usize> {
        [
            ("frame_outside_stream", self.frame_outside_stream),
            ("category_out_of_range", self.category_out_of_range),
            ("box_not_valid", self.box_not_valid),
            ("no_declared_id", self.no_declared_id),
            ("without_volume", self.without_volume),
        ]
        .into_iter()
        .filter(|(_, count)| *count > 0)
        .map(|(stage, count)| (stage.to_string(), count))
        .collect()
    }

    fn add(&mut self, other: &StageCounts) {
        self.stream_not_read += other.stream_not_read;
        self.markers += other.markers;
        self.declared_id += other.declared_id;
        self.frame_outside_stream += other.frame_outside_stream;
        self.category_out_of_range += other.category_out_of_range;
        self.box_not_valid += other.box_not_valid;
        self.no_declared_id += other.no_declared_id;
        self.records += other.records;
        self.id_from_enclosing_record += other.id_from_enclosing_record;
        self.not_placed_instances += other.not_placed_instances;
        self.placed_instances += other.placed_instances;
        self.without_volume += other.without_volume;
        self.with_volume += other.with_volume;
        self.superseded += other.superseded;
        self.nested_components += other.nested_components;
        self.selected += other.selected;
        self.in_non_primary_design_option += other.in_non_primary_design_option;
        self.not_in_export_phase += other.not_in_export_phase;
        self.nested_in_door_or_window += other.nested_in_door_or_window;
        self.category_not_exported += other.category_not_exported;
        self.handed_to_export += other.handed_to_export;
    }

    /// The steps whose counts do not add up, by the name of the count each
    /// step passes on; empty when they all do.
    pub fn unbalanced(&self) -> Vec<&'static str> {
        let mut broken = Vec::new();
        if self.markers
            != self.frame_outside_stream
                + self.category_out_of_range
                + self.box_not_valid
                + self.no_declared_id
                + self.records
        {
            broken.push("markers");
        }
        if self.records != self.not_placed_instances + self.placed_instances {
            broken.push("records");
        }
        if self.placed_instances != self.without_volume + self.with_volume {
            broken.push("placed_instances");
        }
        if self.with_volume != self.superseded + self.nested_components + self.selected {
            broken.push("with_volume");
        }
        if self.selected
            != self.in_non_primary_design_option
                + self.not_in_export_phase
                + self.nested_in_door_or_window
                + self.category_not_exported
                + self.handed_to_export
        {
            broken.push("selected");
        }
        if self.declared_id > self.markers {
            broken.push("declared_id");
        }
        if self.id_from_enclosing_record > self.records {
            broken.push("id_from_enclosing_record");
        }
        broken
    }
}

/// Where the element records of a Revit 2008 to 2023 file stopped (#1389):
/// [`StageCounts`] for each partition stream and for the file, and what became of
/// the elements handed to the export.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ElementRecordStages {
    /// The release whose record layout the file takes.
    pub layout_release: u32,
    /// The counts of all the streams.
    pub total: StageCounts,
    /// The counts of each `Partitions/NN` stream the scan read or failed to
    /// inflate, by name.
    pub streams: BTreeMap<String, StageCounts>,
    /// Of `total.handed_to_export`, the elements the IFC model holds: one for each
    /// ElementId, counted from the model.
    pub written_to_model: usize,
    /// Of `total.handed_to_export`, the elements the walk dropped because an
    /// element of an earlier reader (an ArcWall, a `Global/Latest` instance)
    /// already held the ElementId.
    #[serde(default)]
    pub id_already_held: usize,
    /// Of `total.handed_to_export`, the elements below the minimum confidence,
    /// which the default export hides.
    #[serde(default)]
    pub below_min_confidence: usize,
    /// The rest of `total.handed_to_export`, by the Revit class they were handed
    /// as: elements the exporter writes nothing for. Not attributed to a cause
    /// beyond the class.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub not_written_by_class: BTreeMap<String, usize>,
    /// Present only when `written_to_model` is 0: the stage the records of the
    /// file stopped at. One of `elem_table_not_read`, `schema_has_no_element_marker`,
    /// `stream_not_read`, `no_markers`, the stages of [`FRAME_STOP_STAGES`] the
    /// markers stopped at (the one that stopped most), `no_placed_instances`,
    /// `without_volume`, `nested_components`, the way the selected frames were left
    /// out that left out most (`in_non_primary_design_option`,
    /// `not_in_export_phase`, `nested_in_door_or_window`, `category_not_exported`),
    /// or, of the elements handed to the export, the cause that set aside most
    /// (`id_already_held`, `below_min_confidence`, `class_not_exported`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_at: Option<String>,
}

/// The name among `candidates` with the highest count, the first of them on a
/// tie.
fn most(candidates: &[(&'static str, usize)]) -> &'static str {
    candidates
        .iter()
        .rev()
        .max_by_key(|(_, count)| *count)
        .map_or("no_markers", |(name, _)| *name)
}

impl ElementRecordStages {
    /// The steps whose counts do not add up: the file's and each stream's
    /// ([`StageCounts::unbalanced`]), the streams' counts against the file's
    /// (`streams`), and the elements handed to the export against those written
    /// and those set aside (`handed_to_export`); empty when they all do.
    pub fn unbalanced(&self) -> Vec<&'static str> {
        let mut broken = self.total.unbalanced();
        let mut summed = StageCounts::default();
        for counts in self.streams.values() {
            if !counts.unbalanced().is_empty() {
                broken.push("streams");
            }
            summed.add(counts);
        }
        if summed != self.total {
            broken.push("streams_sum");
        }
        let set_aside: usize = self.not_written_by_class.values().sum();
        if self.total.handed_to_export
            != self.written_to_model + self.id_already_held + self.below_min_confidence + set_aside
        {
            broken.push("handed_to_export");
        }
        broken
    }

    /// The stage at which a file whose elements are not written stopped.
    fn empty_stage(&self, setup: Option<&'static str>) -> &'static str {
        let total = &self.total;
        if let Some(setup) = setup {
            setup
        } else if total.markers == 0 {
            if total.stream_not_read > 0 {
                "stream_not_read"
            } else {
                "no_markers"
            }
        } else if total.records == 0 {
            most(&[
                ("frame_outside_stream", total.frame_outside_stream),
                ("category_out_of_range", total.category_out_of_range),
                ("box_not_valid", total.box_not_valid),
                ("no_declared_id", total.no_declared_id),
            ])
        } else if total.placed_instances == 0 {
            "no_placed_instances"
        } else if total.with_volume == 0 {
            "without_volume"
        } else if total.selected == 0 {
            "nested_components"
        } else if total.handed_to_export == 0 {
            most(&[
                (
                    "in_non_primary_design_option",
                    total.in_non_primary_design_option,
                ),
                ("not_in_export_phase", total.not_in_export_phase),
                ("nested_in_door_or_window", total.nested_in_door_or_window),
                ("category_not_exported", total.category_not_exported),
            ])
        } else {
            most(&[
                ("id_already_held", self.id_already_held),
                ("below_min_confidence", self.below_min_confidence),
                (
                    "class_not_exported",
                    self.not_written_by_class.values().sum(),
                ),
            ])
        }
    }

    /// The report of `stages`, with the elements of `model` that came from it.
    /// `below_min` holds the ElementIds the walk found below the minimum
    /// confidence.
    pub(crate) fn build(
        stages: &RecordStages,
        model: &IfcModel,
        below_min: &BTreeSet<u32>,
    ) -> Self {
        let mut total = StageCounts::default();
        let mut streams = BTreeMap::new();
        for (name, counts) in &stages.streams {
            total.add(counts);
            streams.insert(name.clone(), counts.clone());
        }
        let in_model: BTreeSet<u32> = model
            .entities
            .iter()
            .filter_map(|entity| match entity {
                IfcEntity::BuildingElement { type_guid, .. } => {
                    element_tag(type_guid).filter(|id| stages.handed.contains_key(id))
                }
                _ => None,
            })
            .collect();
        // Each handed element is counted once: its ElementId was held by an
        // earlier reader's element (then whatever the model holds under it is
        // not this one), or the model holds it, or it is below the confidence
        // floor, or the exporter writes nothing for its class.
        let (mut written, mut held, mut below) = (0usize, 0usize, 0usize);
        let mut not_written_by_class: BTreeMap<String, usize> = BTreeMap::new();
        for (id, class) in &stages.handed {
            if stages.held.contains(id) {
                held += 1;
            } else if in_model.contains(id) {
                written += 1;
            } else if below_min.contains(id) {
                below += 1;
            } else {
                *not_written_by_class
                    .entry((*class).to_string())
                    .or_default() += 1;
            }
        }
        let mut report = Self {
            layout_release: stages.layout_release,
            total,
            streams,
            written_to_model: written,
            id_already_held: held,
            below_min_confidence: below,
            not_written_by_class,
            stopped_at: None,
        };
        if written == 0 {
            report.stopped_at = Some(report.empty_stage(stages.setup).to_string());
        }
        report
    }

    /// The `skipped` item of the frames that stopped before they became an
    /// element, counted by stage from the file's counts, or `None` where none
    /// did (#1389).
    pub(crate) fn skipped_item(&self) -> Option<SkippedExportItem> {
        let classes = self.total.frame_stops();
        let count: usize = classes.values().sum();
        (count > 0).then(|| SkippedExportItem {
            reason: FRAME_STOPPED_REASON.to_string(),
            count,
            classes,
            sample_names: Vec::new(),
            phase: None,
        })
    }
}

/// The decisions of the frames in `stopped`, one for each, with their stage as
/// the class (#1389). Made only where the log of decisions is asked for.
pub(crate) fn frame_stopped_decisions(stopped: &[LeftOutElement]) -> Vec<ExportDecision> {
    stopped
        .iter()
        .map(|frame| {
            let mut decision =
                ExportDecision::new(DecisionStage::ElementSkipped, FRAME_STOPPED_REASON);
            decision.element_id = frame.element_id;
            decision.class = Some(frame.class.clone());
            decision.stream = frame.stream.clone();
            decision.offset = frame.offset;
            decision
        })
        .collect()
}
