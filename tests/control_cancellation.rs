//! Cooperative cancellation + progress on the checked-in tier-1 fixture.
//!
//! No corpus files needed: `corpus/tier1/architectural-2024` is a
//! license-free synthetic project small enough for CI.

use rvt::RevitFile;
use rvt::control::{CancelToken, Stage, WalkerControl};
use rvt::partition_scanner::{ScanOptions, scan_partitions_with_control};
use rvt::walker::{PRODUCTION_ELEMENT_MIN_SCORE, WalkerLimits, iter_elements_with_control};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("corpus/tier1/architectural-2024/architectural-2024.rvt")
}

fn open() -> RevitFile {
    RevitFile::open(fixture()).expect("open tier-1 fixture")
}

#[test]
fn pre_cancelled_token_stops_element_iteration_before_any_work() {
    let mut rf = open();
    let token = CancelToken::new();
    token.cancel();
    let control = WalkerControl::new().with_cancel(token);
    let result = iter_elements_with_control(
        &mut rf,
        PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
        &control,
    );
    assert!(
        matches!(result, Err(rvt::Error::Cancelled)),
        "expected Error::Cancelled, got {:?}",
        result.map(|it| it.count())
    );
}

#[test]
fn progress_events_cover_every_stage_in_order_and_reach_their_totals() {
    let mut rf = open();
    let seen: Arc<Mutex<Vec<rvt::control::ProgressEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let control = WalkerControl::new().with_progress(move |event| sink.lock().unwrap().push(event));

    let elements: Vec<_> = iter_elements_with_control(
        &mut rf,
        PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
        &control,
    )
    .expect("uncancelled iteration succeeds")
    .collect();
    // Same result as the plain entry point — control must not change output.
    let plain = rvt::walker::iter_elements(&mut open()).unwrap().count();
    assert_eq!(elements.len(), plain);

    let events = seen.lock().unwrap().clone();
    assert!(!events.is_empty());
    let stages: Vec<Stage> = events.iter().map(|e| e.stage).collect();
    let first_index = |stage: Stage| {
        stages
            .iter()
            .position(|s| *s == stage)
            .expect("stage reported")
    };
    assert!(first_index(Stage::SchemaParse) < first_index(Stage::CandidateScan));
    assert!(first_index(Stage::CandidateScan) < first_index(Stage::ElementDecode));
    assert!(first_index(Stage::ElementDecode) < first_index(Stage::PartitionScan));

    for stage in [
        Stage::SchemaParse,
        Stage::CandidateScan,
        Stage::ElementDecode,
        Stage::PartitionScan,
    ] {
        let of_stage: Vec<_> = events.iter().filter(|e| e.stage == stage).collect();
        assert!(
            of_stage.windows(2).all(|w| w[0].done <= w[1].done),
            "{stage:?} progress must be monotone: {of_stage:?}"
        );
        let last = of_stage.last().unwrap();
        assert_eq!(
            last.total,
            Some(last.done),
            "{stage:?} must end at done == total: {last:?}"
        );
    }
}

#[test]
fn cancelling_from_inside_the_progress_callback_is_honoured() {
    let mut rf = open();
    let token = CancelToken::new();
    let trip = token.clone();
    let control = WalkerControl::new()
        .with_cancel(token)
        .with_progress(move |event| {
            if event.stage == Stage::CandidateScan {
                trip.cancel();
            }
        });
    let result = iter_elements_with_control(
        &mut rf,
        PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
        &control,
    );
    assert!(matches!(result, Err(rvt::Error::Cancelled)));
}

#[test]
fn partition_scan_observes_the_token() {
    let mut rf = open();
    let version = rf.basic_file_info().unwrap().version;
    let token = CancelToken::new();
    token.cancel();
    let control = WalkerControl::new().with_cancel(token);
    let result = scan_partitions_with_control(&mut rf, version, &ScanOptions::default(), &control);
    assert!(matches!(result, Err(rvt::Error::Cancelled)));

    let seen = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    let control = WalkerControl::new().with_progress(move |event| sink.lock().unwrap().push(event));
    let scan =
        scan_partitions_with_control(&mut open(), version, &ScanOptions::default(), &control)
            .expect("uncancelled partition scan");
    let plain =
        rvt::partition_scanner::scan_partitions(&mut open(), version, &ScanOptions::default())
            .unwrap();
    assert_eq!(scan.candidates.len(), plain.candidates.len());
    let events = seen.lock().unwrap();
    assert!(events.iter().all(|e| e.stage == Stage::PartitionScan));
    let last = events.last().expect("at least the final report");
    assert_eq!(last.total, Some(last.done));
}
