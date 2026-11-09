//! #1326: every skipped item and geometry gap the diagnostics report has
//! matching per-element decisions, and the counts are equal.
//!
//! `rvt-ifc --export-decisions` (`ExportOptions::with_export_decisions`) adds
//! `export_decisions` to the diagnostics: one entry per element for each
//! fail-closed decision, bounded per reason. The `skipped` section is counted
//! from the same decisions. This test exports each model with the log off
//! and then on, and checks, without the code that made the log:
//!
//! - with the log off the diagnostics have no `export_decisions`, and with it
//!   on they are the same JSON but for that field;
//! - for every reason of `skipped` (the stream item apart, which counts
//!   streams, not elements) the log's `total` is the sum of the items' counts,
//!   `recorded` plus `dropped` is `total`, and, for a reason that dropped
//!   none, the entries by class (by detail for an Area, whose items count
//!   reasons; by phase for a room) are the items' `classes`;
//! - the decisions behind `exported.bounding_box_bodies`,
//!   `bounding_box_spaces`, `area_body_refusals`, `slab_shape_refusals`,
//!   `wall_base_refusals` and `wall_end_refusals` equal those counts;
//! - the log has no reason that is in neither;
//! - on Revit 2024 and later, the bytes at an entry's stream and offset are
//!   the record it says: a frame with no ElementId carries none, and an
//!   element's frame starts with its id or sits inside its record;
//! - where the log took an entry's stream and offset from the provenance
//!   table, the table has the same;
//! - an empty log cannot pass: each model names reasons its own diagnostics
//!   count (measured on the model, in `CASES`), and each must have entries, and
//!   for the office and for RE1 Architecture at its first phase the frames of
//!   Revit 2024 and later must have been re-read (`frames_checked > 0`).
//!   Core Interior names none, because it has nothing left out: its `skipped`
//!   section is empty and its box body, box room and Area refusal counts are
//!   0, so its log is empty and equal to that.
//!
//! Per model, the counts by reason go to `export-decisions/<model>.json` and
//! `summary.txt` under `CARGO_TARGET_TMPDIR`, which CI prints and uploads.
//!
//! Runs on every `.rvt` under `RVT_PROJECT_CORPUS_DIR` (Core Interior and the
//! RE1 models in CI) and on the BIM4LCA as-built office under
//! `RVT_NORDIC_LCA_DIR`; each is skipped when its variable is unset.

use crate::element_parameters::nordic_dir;
use rvt::RevitFile;
use rvt::ifc::element_provenance;
use rvt::ifc::export_decisions::ExportDecision;
use rvt::ifc::{ExportOptions, UNREAD_STREAMS_REASON, run_export};
use rvt::research::partition_element_records::carries_no_element_id;
use rvt::walker::WalkerLimits;
use serde_json::json;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The reasons whose entries are frames of the element-record partitions.
const FRAME_REASONS: [&str; 8] = [
    "element_record_without_element_id",
    "element_record_without_volume",
    "element_record_in_non_primary_design_option",
    "element_record_not_in_export_phase",
    "room_not_in_export_phase",
    "element_record_empty_curtain_panel",
    "element_record_family_space_without_body",
    "element_record_family_type_draws_nothing",
];

/// One export of a model, and what its log must show.
struct Case {
    model: &'static str,
    /// The `--phase` to export, or the project's last.
    phase: Option<&'static str>,
    /// Reasons its diagnostics count, which the log must hold entries for, as
    /// measured on `rvt-ifc` with the switch off.
    reasons: &'static [&'static str],
    /// Whether frames of Revit 2024 or later must have been re-read, so that
    /// the evidence check cannot pass having read nothing.
    frames: bool,
}

/// What each model's log must show. Core Interior has nothing left out, so it
/// names nothing and its log must be empty. RE1 Architecture has no frame
/// reason in its default export; exported at its first phase it leaves out its
/// 11 rooms, which are frames to re-read. A model of the corpus that is not
/// named here is checked against its diagnostics alone.
const CASES: [Case; 5] = [
    Case {
        model: "2024_Core_Interior",
        phase: None,
        reasons: &[],
        frames: false,
    },
    Case {
        model: "RE1-Architecture",
        phase: None,
        reasons: &[
            "body_is_record_box",
            "low_confidence_schema_scan_candidate",
            "unsupported_geometry_missing_dimensions",
            "unsupported_geometry_profile",
            "unsupported_geometry_unresolved_host",
        ],
        frames: false,
    },
    Case {
        model: "RE1-Architecture",
        phase: Some("Existing"),
        reasons: &["room_not_in_export_phase"],
        frames: true,
    },
    Case {
        model: "Revit_IFC5_Einhoven",
        phase: None,
        reasons: &[
            "body_is_record_box",
            "low_confidence_schema_scan_candidate",
            "unsupported_geometry_missing_level",
        ],
        frames: false,
    },
    Case {
        model: "nordic-office-as-built",
        phase: None,
        reasons: &[
            "body_is_record_box",
            "element_record_empty_curtain_panel",
            "element_record_without_volume",
            "space_outline_is_record_box",
            "unsupported_geometry_unresolved_host",
        ],
        frames: true,
    },
];

/// The reasons made from the model, whose evidence the log reads from the
/// provenance table.
const MODEL_REASONS: [&str; 13] = [
    "unsupported_geometry_curve",
    "unsupported_geometry_profile",
    "unsupported_geometry_unresolved_host",
    "unsupported_geometry_missing_level",
    "unsupported_geometry_missing_dimensions",
    "floor_boundary_annotation_only",
    "opening_index_without_host_geometry",
    "body_is_record_box",
    "space_outline_is_record_box",
    "area_body_refused",
    "slab_shape_refused",
    "wall_base_cut_refused",
    "wall_end_extension_refused",
];

fn project_models() -> Vec<(String, PathBuf)> {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        eprintln!("skipping the project corpus: RVT_PROJECT_CORPUS_DIR is not set");
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut models: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvt"))
        .filter_map(|path| {
            let name = path.file_stem()?.to_string_lossy().into_owned();
            Some((name, path))
        })
        .collect();
    models.sort();
    models
}

fn office_model() -> Vec<(String, PathBuf)> {
    let Some(dir) = nordic_dir() else {
        return Vec::new();
    };
    let path = dir.join("ARK_NordicLCA_Office_Concrete_As-Built.rvt");
    if !path.is_file() {
        eprintln!("skipping the office: the as-built model is absent");
        return Vec::new();
    }
    vec![("nordic-office-as-built".to_string(), path)]
}

/// `entries` counted by `key`.
fn counted<'a>(
    entries: &[&'a ExportDecision],
    key: impl Fn(&'a ExportDecision) -> Option<&'a String>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for entry in entries.iter().copied() {
        if let Some(key) = key(entry) {
            *counts.entry(key.clone()).or_insert(0) += 1;
        }
    }
    counts
}

fn check_model(model: &str, path: &Path, case: Option<&Case>) {
    let phase = case.and_then(|case| case.phase);
    let required = case.map_or(&[][..], |case| case.reasons);
    let label = match phase {
        Some(phase) => format!("{model}-{phase}"),
        None => model.to_string(),
    };
    let name = label.as_str();
    let options = ExportOptions::default().with_phase(phase.map(str::to_string));
    let mut rf = RevitFile::open(path).expect("open the model");
    let off = run_export(&mut rf, &options).expect("export the model");
    assert!(
        off.diagnostics.export_decisions.is_none(),
        "{name}: the log is made without the switch"
    );
    let off_json = serde_json::to_value(&off.diagnostics).expect("diagnostics json");
    assert!(
        off_json.get("export_decisions").is_none(),
        "{name}: the diagnostics name the log without the switch"
    );

    let mut on = run_export(&mut rf, &options.with_export_decisions(true))
        .expect("export the model with the log");
    let log = on
        .diagnostics
        .export_decisions
        .take()
        .expect("the log when asked for");
    let on_json = serde_json::to_value(&on.diagnostics).expect("diagnostics json");
    assert!(
        on_json == off_json,
        "{name}: the diagnostics differ beyond the log"
    );

    // The aggregate, by reason.
    let mut aggregate: BTreeMap<String, usize> = BTreeMap::new();
    let mut classes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut phases: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for item in &on.diagnostics.skipped {
        if item.reason == UNREAD_STREAMS_REASON {
            continue;
        }
        *aggregate.entry(item.reason.clone()).or_default() += item.count;
        for (class, count) in &item.classes {
            *classes
                .entry(item.reason.clone())
                .or_default()
                .entry(class.clone())
                .or_default() += count;
        }
        if let Some(phase) = &item.phase {
            *phases
                .entry(item.reason.clone())
                .or_default()
                .entry(phase.clone())
                .or_default() += item.count;
        }
    }
    // The counts of `exported` the log also explains.
    let exported = &on.diagnostics.exported;
    aggregate.insert(
        "body_is_record_box".to_string(),
        exported.bounding_box_bodies,
    );
    aggregate.insert(
        "space_outline_is_record_box".to_string(),
        exported.bounding_box_spaces,
    );
    aggregate.insert(
        "area_body_refused".to_string(),
        exported.area_body_refusals.values().sum(),
    );
    classes.insert(
        "area_body_refused".to_string(),
        exported.area_body_refusals.clone(),
    );
    for (reason, refusals) in [
        ("slab_shape_refused", &exported.slab_shape_refusals),
        ("wall_base_cut_refused", &exported.wall_base_refusals),
        ("wall_end_extension_refused", &exported.wall_end_refusals),
        (
            "wall_profile_extension_refused",
            &exported.wall_profile_refusals,
        ),
    ] {
        aggregate.insert(reason.to_string(), refusals.values().sum());
        classes.insert(reason.to_string(), refusals.clone());
    }

    let entries_of = |reason: &str| -> Vec<&ExportDecision> {
        log.entries
            .iter()
            .filter(|entry| entry.reason == reason)
            .collect()
    };
    let recorded: usize = log.reasons.values().map(|reason| reason.recorded).sum();
    assert_eq!(log.entries.len(), recorded, "{name}: entries recorded");
    let dropped: usize = log.reasons.values().map(|reason| reason.dropped).sum();
    assert_eq!(log.dropped, dropped, "{name}: dropped");
    for (reason, count) in &aggregate {
        let total = log.reasons.get(reason).map_or(0, |held| held.total);
        assert_eq!(
            total, *count,
            "{name}: {reason}: log total against the aggregate"
        );
    }
    for (reason, held) in &log.reasons {
        assert!(
            aggregate.contains_key(reason),
            "{name}: the log has {reason}, which no aggregate counts"
        );
        assert_eq!(held.recorded + held.dropped, held.total, "{name}: {reason}");
        let entries = entries_of(reason);
        assert_eq!(entries.len(), held.recorded, "{name}: {reason} entries");
        assert!(
            entries.iter().all(|entry| entry.stage == held.stage),
            "{name}: {reason}: stages differ"
        );
        if held.dropped > 0 {
            continue;
        }
        if reason == "area_without_proven_body" {
            // That item's classes are the reasons the body is not proven.
            if let Some(expected) = classes.get(reason) {
                let found = counted(&entries, |entry| entry.detail.as_ref());
                assert_eq!(&found, expected, "{name}: {reason} by detail");
            }
        } else if matches!(
            reason.as_str(),
            "area_body_refused"
                | "slab_shape_refused"
                | "wall_base_cut_refused"
                | "wall_end_extension_refused"
                | "wall_profile_extension_refused"
        ) {
            let found = counted(&entries, |entry| entry.detail.as_ref());
            assert_eq!(&found, &classes[reason], "{name}: {reason} by detail");
        } else if let Some(expected) = classes.get(reason) {
            let found = counted(&entries, |entry| entry.class.as_ref());
            assert_eq!(&found, expected, "{name}: {reason} by class");
        }
        if let Some(expected) = phases.get(reason) {
            let found = counted(&entries, |entry| entry.detail.as_ref());
            assert_eq!(&found, expected, "{name}: {reason} by phase");
        }
    }

    // The evidence: bytes for the frames, the table for the rest.
    let version = rf.layout_release().expect("layout release");
    let table = element_provenance::build(&mut rf, WalkerLimits::default());
    let (mut frames_checked, mut frames_first, mut frames_second, mut frames_unlocated) =
        (0usize, 0usize, 0usize, 0usize);
    let (mut from_table, mut table_unlocated) = (0usize, 0usize);
    for entry in &log.entries {
        let reason = entry.reason.as_str();
        if FRAME_REASONS.contains(&reason) {
            let (Some(stream), Some(offset)) = (entry.stream.as_deref(), entry.offset) else {
                frames_unlocated += 1;
                continue;
            };
            if version < 2024 {
                continue;
            }
            let inflated = rf
                .inflated_partition(stream)
                .unwrap_or_else(|error| panic!("{name}: {stream}: {error}"));
            let first = u64_at(inflated.bytes(), offset);
            frames_checked += 1;
            match entry.element_id {
                None => {
                    assert!(
                        first.is_some_and(carries_no_element_id),
                        "{name}: {reason} at {stream}:{offset} carries an id: {first:?}"
                    );
                }
                Some(id) if first == Some(u64::from(id)) => frames_first += 1,
                Some(id) => {
                    assert!(
                        first.is_some_and(carries_no_element_id),
                        "{name}: {reason} {id} at {stream}:{offset} starts with {first:?}"
                    );
                    frames_second += 1;
                }
            }
        } else if MODEL_REASONS.contains(&reason) {
            let Some(id) = entry.element_id else {
                continue;
            };
            match (table.get(id), entry.stream.as_deref(), entry.offset) {
                (Some(located), Some(stream), Some(offset)) => {
                    assert_eq!(
                        (located.stream.as_deref(), located.offset),
                        (Some(stream), Some(offset)),
                        "{name}: {reason} {id}: evidence against the provenance table"
                    );
                    from_table += 1;
                }
                (Some(located), None, None) => {
                    assert!(
                        !located.is_located(),
                        "{name}: {reason} {id}: the table places the element, the log does not"
                    );
                    table_unlocated += 1;
                }
                (None, None, None) => table_unlocated += 1,
                other => panic!("{name}: {reason} {id}: evidence {other:?}"),
            }
        }
    }

    if case.is_some_and(|case| case.frames) {
        assert!(
            frames_checked > 0,
            "{name}: no frame was re-read ({frames_unlocated} entries without a place)"
        );
    }
    for reason in required {
        assert!(
            log.reasons
                .get(*reason)
                .is_some_and(|held| held.recorded > 0),
            "{name}: the log holds no decision for {reason}, which its diagnostics count"
        );
    }

    // The report.
    let mut reasons = serde_json::Map::new();
    let mut line = format!(
        "{name}: Revit {version}, {} decisions in {} reasons, {} dropped; log {} bytes of JSON\n",
        log.entries.len(),
        log.reasons.len(),
        log.dropped,
        serde_json::to_vec(&log).expect("log json").len()
    );
    for (reason, held) in &log.reasons {
        reasons.insert(
            reason.clone(),
            json!({
                "stage": held.stage.as_str(),
                "total": held.total,
                "recorded": held.recorded,
                "dropped": held.dropped,
                "aggregate": aggregate.get(reason),
            }),
        );
        line.push_str(&format!(
            "  {reason} ({}): {} total, {} recorded, {} dropped; aggregate {}\n",
            held.stage.as_str(),
            held.total,
            held.recorded,
            held.dropped,
            aggregate.get(reason).copied().unwrap_or(0)
        ));
    }
    line.push_str(&format!(
        "  frames: {frames_checked} re-read ({frames_first} start with the id, {frames_second} sit in their record), {frames_unlocated} without a place; model decisions: {from_table} placed by the table, {table_unlocated} unplaced\n"
    ));
    let report = json!({
        "model": name,
        "layout_release": version,
        "decisions": log.entries.len(),
        "dropped": log.dropped,
        "cap_per_reason": log.cap_per_reason,
        "reasons": reasons,
        "frames_re_read": frames_checked,
        "frames_start_with_the_id": frames_first,
        "frames_inside_their_record": frames_second,
        "frames_without_a_place": frames_unlocated,
        "model_decisions_placed_by_the_table": from_table,
        "model_decisions_unplaced": table_unlocated,
        "log_json_bytes": serde_json::to_vec(&log).expect("log json").len(),
        "diagnostics_json_bytes_without_log": serde_json::to_vec(&off_json).expect("json").len(),
    });
    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("export-decisions");
    std::fs::create_dir_all(&out).expect("report folder");
    std::fs::write(
        out.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&report).expect("report json"),
    )
    .expect("write the report");
    eprint!("{line}");
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join("summary.txt"))
        .and_then(|mut summary| summary.write_all(line.as_bytes()))
        .expect("write the summary");
}

/// Check each model against its cases, or against its diagnostics alone when
/// `CASES` names none. With `dir_variable` set, a model of `must_hold` that the
/// corpus lacks fails the test, whether or not any other model is there, so a
/// corpus that lost a model cannot pass it.
fn check_all(dir_variable: &str, models: Vec<(String, PathBuf)>, must_hold: &[&str]) {
    if std::env::var_os(dir_variable).is_some() {
        for model in must_hold {
            assert!(
                models.iter().any(|(name, _)| name == model),
                "{model} is not in the corpus, and {dir_variable} is set"
            );
        }
    }
    if models.is_empty() {
        eprintln!("skipping: no model to read");
        return;
    }
    for (name, path) in &models {
        let cases: Vec<&Case> = CASES.iter().filter(|case| case.model == name).collect();
        if cases.is_empty() {
            check_model(name, path, None);
        }
        for case in cases {
            check_model(name, path, Some(case));
        }
    }
}

#[test]
fn project_corpus_decisions_match_the_aggregate() {
    check_all(
        "RVT_PROJECT_CORPUS_DIR",
        project_models(),
        &[
            "2024_Core_Interior",
            "RE1-Architecture",
            "Revit_IFC5_Einhoven",
        ],
    );
}

#[test]
fn nordic_office_decisions_match_the_aggregate() {
    check_all(
        "RVT_NORDIC_LCA_DIR",
        office_model(),
        &["nordic-office-as-built"],
    );
}
