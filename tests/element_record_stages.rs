//! #1389: where the element records of a Revit 2008 to 2023 file stop.
//!
//! `decoded.element_record_stages` counts, per partition stream and in total,
//! what became of the frames at the file's element-record markers, so a file
//! that exports no element can say at which stage its records stopped. This
//! test exports two files that take the 2008 to 2023 record layout and checks,
//! without the code that made the counts:
//!
//! - on Revit_IFC5_Einhoven (Revit 2023, `RVT_PROJECT_CORPUS_DIR`): the markers
//!   and the markers with a declared ElementId are counted again from the
//!   partitions' bytes, stream by stream, and equal the report's; every later
//!   step adds up, per stream and in total (a step's count is the sum of what
//!   the next step took and left), the streams' counts sum to the total's, the
//!   elements written to the model are no more than the elements handed to the
//!   export and no more than the model's building elements; the `skipped` item
//!   `element_record_frame_stopped` counts the frames that stopped by stage, and
//!   so does the log of decisions, whose entries each point at a marker (or, for
//!   a frame with no volume, at the ElementId its record opens with);
//! - on the committed synthetic structural-2023 project (`corpus/tier1`),
//!   whose partitions hold no record: no element is written, and the report
//!   names the stage the records stopped at, the first stage that has none.
//!
//! The public 2015 to 2023 sample projects are not in CI: Measure scores them
//! (`tools/re/element_stage_counts.py`, run on each sample's diagnostics).
//!
//! Skips what is absent: with `RVT_PROJECT_CORPUS_DIR` set, Einhoven must be
//! there.

use rvt::RevitFile;
use rvt::ifc::element_record_stages::{
    ElementRecordStages, FRAME_STOP_STAGES, FRAME_STOPPED_REASON, StageCounts,
};
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::partition_element_records_2023::{record_marker, scan_records_counted};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn einhoven() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping Einhoven: RVT_PROJECT_CORPUS_DIR is not set");
        return None;
    };
    let path = dir.join("Revit_IFC5_Einhoven.rvt");
    assert!(
        path.is_file(),
        "{} is not in the corpus, and RVT_PROJECT_CORPUS_DIR is set",
        path.display()
    );
    Some(path)
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    let bytes = buf.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(bytes.try_into().ok()?))
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    let bytes = buf.get(at..at.checked_add(2)?)?;
    Some(u16::from_le_bytes(bytes.try_into().ok()?))
}

/// The markers of each partition stream and, of them, those with a declared
/// `u32` ElementId 52 bytes before and the tag of `ElementHeader` 8 bytes after
/// it, counted from the inflated bytes.
fn recount(rf: &mut RevitFile) -> BTreeMap<String, (usize, usize)> {
    let (marker, header_tag) = record_marker(rf).expect("the schema gives the marker");
    let table = rvt::elem_table::parse_records(rf).expect("ElemTable");
    let declared = rvt::elem_table::declared_ids(&table);
    let mut counts = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf
            .inflated_partition(&stream)
            .unwrap_or_else(|error| panic!("{stream}: {error}"));
        let buf = inflated.bytes();
        let (mut markers, mut with_id) = (0usize, 0usize);
        for at in memchr::memmem::find_iter(buf, &marker) {
            markers += 1;
            let declared_here = at.checked_sub(52).is_some_and(|id_at| {
                u32_at(buf, id_at).is_some_and(|id| declared.contains(&id))
                    && u16_at(buf, id_at + 8) == Some(header_tag)
            });
            with_id += usize::from(declared_here);
        }
        counts.insert(stream, (markers, with_id));
    }
    counts
}

/// `counts` as a JSON object of integers.
fn fields(counts: &StageCounts) -> BTreeMap<String, u64> {
    serde_json::to_value(counts)
        .expect("counts json")
        .as_object()
        .expect("counts object")
        .iter()
        .map(|(name, value)| (name.clone(), value.as_u64().expect("an integer count")))
        .collect()
}

fn check_report(name: &str, stages: &ElementRecordStages) {
    assert!(
        stages.total.unbalanced().is_empty(),
        "{name}: the file's counts do not add up at {:?}: {:?}",
        stages.total.unbalanced(),
        stages.total
    );
    let mut summed: BTreeMap<String, u64> = BTreeMap::new();
    for (stream, counts) in &stages.streams {
        assert!(
            counts.unbalanced().is_empty(),
            "{name}: {stream}: the counts do not add up at {:?}: {counts:?}",
            counts.unbalanced()
        );
        for (field, value) in fields(counts) {
            *summed.entry(field).or_default() += value;
        }
    }
    assert_eq!(
        summed,
        fields(&stages.total),
        "{name}: the streams' counts are not the file's"
    );
    assert!(
        stages.written_to_model <= stages.total.handed_to_export,
        "{name}: {} elements written of {} handed to the export",
        stages.written_to_model,
        stages.total.handed_to_export
    );
}

#[test]
fn einhoven_stage_counts_add_up_and_equal_the_bytes() {
    let Some(path) = einhoven() else {
        return;
    };
    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let layout = rf.layout_release().expect("layout release");
    assert!(
        (2008..=2023).contains(&layout),
        "Einhoven takes the layout of Revit {layout}, not one of 2008 to 2023"
    );
    let run = run_export(
        &mut rf,
        &ExportOptions::default().with_export_decisions(true),
    )
    .expect("export Einhoven");
    let mut diagnostics = run.diagnostics;
    let stages = diagnostics
        .decoded
        .element_record_stages
        .clone()
        .expect("the stage counts of a file of this layout");
    assert_eq!(stages.layout_release, layout);
    check_report("Einhoven", &stages);

    // The first two steps, counted again from the bytes.
    let recounted = recount(&mut rf);
    let reported: BTreeMap<String, (usize, usize)> = stages
        .streams
        .iter()
        .map(|(stream, counts)| (stream.clone(), (counts.markers, counts.declared_id)))
        .collect();
    assert_eq!(
        reported, recounted,
        "Einhoven: markers and markers with a declared ElementId, by stream"
    );

    // A file that gives records writes some, and says nothing of a stage.
    assert!(stages.total.markers > 0, "Einhoven has no marker");
    assert!(stages.total.records > 0, "Einhoven has no record");
    assert!(
        stages.written_to_model > 0,
        "Einhoven writes no record-backed element"
    );
    assert_eq!(stages.stopped_at, None);
    assert!(
        stages.written_to_model <= diagnostics.exported.building_elements,
        "Einhoven: {} elements written of {} building elements",
        stages.written_to_model,
        diagnostics.exported.building_elements
    );

    // The `skipped` item and the log count the frames that stopped, by stage.
    let total = &stages.total;
    let stopped: BTreeMap<&str, usize> = [
        ("frame_outside_stream", total.frame_outside_stream),
        ("category_out_of_range", total.category_out_of_range),
        ("box_not_valid", total.box_not_valid),
        ("no_declared_id", total.no_declared_id),
        ("without_volume", total.without_volume),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .collect();
    assert!(
        stopped
            .keys()
            .all(|stage| FRAME_STOP_STAGES.contains(stage)),
        "a stage the skipped item does not name: {stopped:?}"
    );
    let item = diagnostics
        .skipped
        .iter()
        .find(|item| item.reason == FRAME_STOPPED_REASON);
    assert_eq!(
        item.map(|item| item.count),
        (!stopped.is_empty()).then(|| stopped.values().sum::<usize>()),
        "Einhoven: the skipped item's count"
    );
    if let Some(item) = item {
        let classes: BTreeMap<&str, usize> = item
            .classes
            .iter()
            .map(|(stage, count)| (stage.as_str(), *count))
            .collect();
        assert_eq!(classes, stopped, "Einhoven: the skipped item by stage");
    }
    let log = diagnostics
        .export_decisions
        .take()
        .expect("the log when asked for");
    let held = log.reasons.get(FRAME_STOPPED_REASON);
    assert_eq!(
        held.map(|reason| reason.total),
        item.map(|item| item.count),
        "Einhoven: the log's total against the skipped item"
    );

    // Each entry points at the bytes it says: a marker 52 bytes on from a frame
    // that stopped before its record, the record's own ElementId for a frame
    // with no volume.
    let (marker, _) = record_marker(&mut rf).expect("marker");
    let mut read = 0usize;
    for entry in log
        .entries
        .iter()
        .filter(|entry| entry.reason == FRAME_STOPPED_REASON)
    {
        let stream = entry.stream.as_deref().expect("entry stream");
        let offset = entry.offset.expect("entry offset");
        let inflated = rf
            .inflated_partition(stream)
            .unwrap_or_else(|error| panic!("{stream}: {error}"));
        let buf = inflated.bytes();
        let class = entry.class.as_deref().expect("entry class");
        // A frame that begins before the stream has no place 52 bytes before
        // its marker to point at.
        if class != "frame_outside_stream" {
            assert_eq!(
                buf.get(offset + 52..offset + 60),
                Some(&marker[..]),
                "Einhoven: {stream}:{offset} is not 52 bytes before a marker"
            );
        }
        if let Some(id) = entry.element_id {
            assert_eq!(
                u32_at(buf, offset),
                Some(id),
                "Einhoven: {stream}:{offset} does not open with ElementId {id}"
            );
        }
        assert!(FRAME_STOP_STAGES.contains(&class), "stage {class}");
        read += 1;
    }
    eprintln!(
        "Einhoven (Revit {layout}): {} markers, {} records, {} handed, {} written; {read} stopped frames re-read; stopped by stage {stopped:?}",
        total.markers, total.records, total.handed_to_export, stages.written_to_model
    );
}

#[test]
fn a_file_with_no_record_says_where_its_records_stopped() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus/tier1/structural-2023/structural-2023.rvt");
    assert!(path.is_file(), "missing tier1 fixture {}", path.display());
    let mut rf = RevitFile::open(&path).expect("open structural-2023");
    let layout = rf.layout_release().expect("layout release");
    assert!(
        (2008..=2023).contains(&layout),
        "the fixture takes the layout of Revit {layout}"
    );

    // The scan: nothing to count in empty partitions.
    let (records, scanned) = scan_records_counted(&mut rf, layout);
    assert!(records.is_empty(), "records in an empty fixture");
    assert!(scanned.handed.is_empty());
    let markers: usize = scanned.streams.values().map(|counts| counts.markers).sum();
    assert_eq!(markers, 0, "markers in an empty fixture");

    // The export: no element of the records is written, and the diagnostics say
    // at which stage they stopped, in the stage names they document.
    let run = run_export(&mut rf, &ExportOptions::default()).expect("export the fixture");
    let stages = run
        .diagnostics
        .decoded
        .element_record_stages
        .expect("the stage counts of a file of this layout");
    assert_eq!(stages.layout_release, layout);
    assert_eq!(stages.written_to_model, 0);
    assert_eq!(stages.total.markers, 0);
    check_report("structural-2023", &stages);
    let stopped_at = stages
        .stopped_at
        .as_deref()
        .expect("a stage when none is written");
    let before_any_marker: BTreeSet<&str> = [
        "elem_table_not_read",
        "schema_has_no_element_marker",
        "no_markers",
    ]
    .into_iter()
    .collect();
    assert!(
        before_any_marker.contains(stopped_at),
        "the records of a file with no marker stopped at {stopped_at}"
    );
    // The scan found no marker, so nothing it set aside is the reason.
    assert_eq!(
        scanned.setup.is_some(),
        stopped_at != "no_markers",
        "the scan's setup note and the stage disagree"
    );
}
