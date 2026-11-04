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
//!   the elements handed to the export are the elements written plus those set
//!   aside, each for one cause; the default export counts the same as the
//!   export with the log and lists no frame;
//! - on the committed synthetic structural-2023 project (`corpus/tier1`),
//!   whose partitions hold no record: no element is written, and the report
//!   names the stage the records stopped at, the first stage that has none;
//!   `rvt-ifc` prints that stage on a line of its own;
//! - on the committed minimal family `tests/fixtures/families/empty.rfa`, whose
//!   elements come from `Global/Latest`: no report, no `skipped` item and no
//!   line from `rvt-ifc`.
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
use rvt::research::partition_element_records_2023::{
    RecordSpan, enclosing_span, record_marker, record_spans, scan_records_counted,
};
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
        stages.unbalanced().is_empty(),
        "{name}: the counts do not add up at {:?}: {:?}",
        stages.unbalanced(),
        stages
    );
    // A file whose scan could not start has no stream: its streams sum to zero.
    let mut summed = fields(&StageCounts::default());
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
    // The elements handed to the export are written or set aside for one cause
    // each, summed here from the report's own fields.
    let set_aside: usize = stages.not_written_by_class.values().sum();
    assert_eq!(
        stages.total.handed_to_export,
        stages.written_to_model + stages.id_already_held + stages.below_min_confidence + set_aside,
        "{name}: the elements handed to the export against those written and set aside"
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
    // that stopped before its record, and for a frame with no volume its
    // ElementId at the offset or, for a frame inside its element's record (the
    // second prologue, RE-315), that record's.
    let (marker, header_tag) = record_marker(&mut rf).expect("marker");
    let table = rvt::elem_table::parse_records(&mut rf).expect("ElemTable");
    let declared = rvt::elem_table::declared_ids(&table);
    let mut spans_of: BTreeMap<String, Vec<RecordSpan>> = BTreeMap::new();
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
            let opens_with_id = u32_at(buf, offset) == Some(id);
            let in_its_record = !opens_with_id && {
                let spans = spans_of
                    .entry(stream.to_string())
                    .or_insert_with(|| record_spans(buf, header_tag, &declared));
                enclosing_span(spans, offset).is_some_and(|span| span.element_id == id)
            };
            assert!(
                opens_with_id || in_its_record,
                "Einhoven: {stream}:{offset} neither opens with ElementId {id} nor sits in its record"
            );
        }
        assert!(FRAME_STOP_STAGES.contains(&class), "stage {class}");
        read += 1;
    }

    // The default export, asked again of the same file, counts the same and
    // lists no frame: the counts and the `skipped` item are the logged export's.
    let plain = run_export(&mut rf, &ExportOptions::default()).expect("export Einhoven");
    assert!(plain.diagnostics.export_decisions.is_none());
    assert_eq!(
        plain.diagnostics.decoded.element_record_stages.as_ref(),
        Some(&stages),
        "Einhoven: the stage report with and without the log"
    );
    let plain_item = plain
        .diagnostics
        .skipped
        .iter()
        .find(|item| item.reason == FRAME_STOPPED_REASON)
        .map(|item| (item.count, item.classes.clone()));
    assert_eq!(
        plain_item,
        item.map(|item| (item.count, item.classes.clone())),
        "Einhoven: the skipped item with and without the log"
    );
    eprintln!(
        "Einhoven (Revit {layout}): {} markers, {} records, {} handed, {} written ({} held, {} below the floor, {:?} not written by class); {read} stopped frames re-read; stopped by stage {stopped:?}",
        total.markers,
        total.records,
        total.handed_to_export,
        stages.written_to_model,
        stages.id_already_held,
        stages.below_min_confidence,
        stages.not_written_by_class
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
    let (records, scanned) = scan_records_counted(&mut rf, layout, false);
    assert!(records.is_empty(), "records in an empty fixture");
    assert!(scanned.handed.is_empty());
    let markers: usize = scanned
        .streams
        .iter()
        .map(|(_, counts)| counts.markers)
        .sum();
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
        "stream_not_read",
        "no_markers",
    ]
    .into_iter()
    .collect();
    assert!(
        before_any_marker.contains(stopped_at),
        "the records of a file with no marker stopped at {stopped_at}"
    );
    // The stage and the scan's own notes agree: a setup note is the stage when
    // the scan could not start, and a stream that did not inflate is counted.
    assert_eq!(
        scanned.setup.is_some(),
        matches!(
            stopped_at,
            "elem_table_not_read" | "schema_has_no_element_marker"
        ),
        "the scan's setup note and the stage disagree"
    );
    assert_eq!(
        stages.total.stream_not_read > 0,
        stopped_at == "stream_not_read",
        "the streams not read and the stage disagree"
    );
}

/// The `rvt-ifc` binary on `path`: its standard error, with the diagnostics it
/// wrote.
fn run_cli(path: &std::path::Path, name: &str) -> (String, serde_json::Value) {
    let dir = std::env::temp_dir().join(format!("rvt-rs-stages-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create a temp dir");
    let diagnostics_path = dir.join("diagnostics.json");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .arg(path)
        .arg("-o")
        .arg(dir.join("model.ifc"))
        .arg("--diagnostics")
        .arg(&diagnostics_path)
        .output()
        .expect("run rvt-ifc");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "rvt-ifc failed:\n{stderr}");
    let diagnostics = serde_json::from_slice(&std::fs::read(&diagnostics_path).expect("read"))
        .expect("diagnostics json");
    let _ = std::fs::remove_dir_all(&dir);
    (stderr, diagnostics)
}

#[test]
fn rvt_ifc_says_where_the_records_stopped_on_a_project_and_not_on_a_family() {
    // A project none of whose records was written: a line of its own, and the
    // stage in the diagnostics it points at.
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus/tier1/structural-2023/structural-2023.rvt");
    let (stderr, diagnostics) = run_cli(&project, "project");
    let stopped_at = diagnostics["decoded"]["element_record_stages"]["stopped_at"]
        .as_str()
        .expect("the diagnostics name the stage");
    let line = stderr
        .lines()
        .find(|line| line.starts_with("rvt-ifc: no element record of this Revit"))
        .unwrap_or_else(|| panic!("no line about the records in:\n{stderr}"));
    assert!(
        line.contains(&format!("stopped at {stopped_at} ")),
        "{line} does not name {stopped_at}"
    );

    // A family reads its elements from Global/Latest: no report, no line.
    let family =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/families/empty.rfa");
    let (stderr, diagnostics) = run_cli(&family, "family");
    assert!(
        diagnostics["decoded"]
            .get("element_record_stages")
            .is_none(),
        "a family file has the stage report"
    );
    assert!(
        diagnostics["skipped"]
            .as_array()
            .into_iter()
            .flatten()
            .all(|item| item["reason"] != FRAME_STOPPED_REASON),
        "a family file has the skipped item"
    );
    assert!(
        !stderr.contains("no element record"),
        "a family file prints the record line:\n{stderr}"
    );
}
