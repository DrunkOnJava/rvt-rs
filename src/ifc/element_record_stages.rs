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
//!    record around it). The rest are `records`.
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
//!    elements of the last step; the walk and the export can set one aside after
//!    the recovery (a lower confidence, an ElementId an earlier reader took).
//!
//! Each step counts exactly what the one before it passed on, so the counts of a
//! stream add up ([`StageCounts::unbalanced`]). When no record of the file is
//! written, [`ElementRecordStages::stopped_at`] names the stage they stopped at.
//!
//! The counts are made for every export of a file of these releases, in the
//! diagnostics only; the IFC is not changed. The frames that stopped at steps 3
//! and 5 are also in `skipped` as `element_record_frame_stopped`, by stage, and
//! in the log of decisions with the stream and the offset of the ElementId's
//! place.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{IfcModel, element_tag, entities::IfcEntity};
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
}

impl StageCounts {
    fn add(&mut self, other: &StageCounts) {
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
/// [`StageCounts`] for each partition stream and for the file, and the count of
/// the elements in the IFC model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ElementRecordStages {
    /// The release whose record layout the file takes.
    pub layout_release: u32,
    /// The counts of all the streams.
    pub total: StageCounts,
    /// The counts of each `Partitions/NN` stream the scan read, by name.
    pub streams: BTreeMap<String, StageCounts>,
    /// Of `total.handed_to_export`, the elements the IFC model holds: one for each
    /// ElementId, counted from the model. The rest were set aside after the
    /// recovery (below the minimum confidence, or an ElementId that another
    /// reader's element already held).
    pub written_to_model: usize,
    /// Present only when `written_to_model` is 0: the stage the records of the
    /// file stopped at. One of `elem_table_not_read`, `schema_has_no_element_marker`,
    /// `no_markers`, the stages of [`FRAME_STOP_STAGES`] the markers stopped at (the
    /// one that stopped most), `no_placed_instances`, `without_volume`,
    /// `nested_components`, a count of the selected frames
    /// (`in_non_primary_design_option`, `not_in_export_phase`,
    /// `nested_in_door_or_window`, `category_not_exported`: the one that stopped
    /// most) or `not_written_to_model`.
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

/// The stage at which a file whose elements are not written stopped.
fn empty_stage(setup: Option<&'static str>, total: &StageCounts) -> &'static str {
    if let Some(setup) = setup {
        setup
    } else if total.markers == 0 {
        "no_markers"
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
        "not_written_to_model"
    }
}

impl ElementRecordStages {
    /// The report of `stages`, with the elements of `model` that came from it.
    pub(crate) fn build(stages: &RecordStages, model: &IfcModel) -> Self {
        let mut total = StageCounts::default();
        for counts in stages.streams.values() {
            total.add(counts);
        }
        let written: BTreeSet<u32> = model
            .entities
            .iter()
            .filter_map(|entity| match entity {
                IfcEntity::BuildingElement { type_guid, .. } => {
                    element_tag(type_guid).filter(|id| stages.handed.contains(id))
                }
                _ => None,
            })
            .collect();
        let stopped_at = written
            .is_empty()
            .then(|| empty_stage(stages.setup, &total).to_string());
        Self {
            layout_release: stages.layout_release,
            total,
            streams: stages.streams.clone(),
            written_to_model: written.len(),
            stopped_at,
        }
    }
}
