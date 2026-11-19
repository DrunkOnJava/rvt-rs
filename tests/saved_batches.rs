//! #1075, #1429, #1808: the saved-graphics read that meshes a batch at a time
//! emits, for every owner, the `SavedElement` the read that decodes every
//! symbol first emits, and reports the same progress events.
//!
//! It runs on `Revit_IFC5_Einhoven.rvt` (magnetar-io's MIT Revit 2023
//! project, 913 KiB) from the project corpus CI fetches, found through
//! `RVT_PROJECT_CORPUS_DIR`: this repository commits no Revit project file. A
//! corpus that is set but lacks the file fails, an unset one skips and says
//! so, as in `tests/native_current_records.rs`. The file holds 2,615 owners
//! with saved graphics, among them an owner that names another owner and a
//! symbol that names one, the shapes #1429 and #1691 need. Reading all of
//! them takes about a third of a second.
//!
//! The file's graphs are small, so the check says nothing about where
//! `OWNER_OBJECT_BUDGET` (20,000 graph objects) or `BATCH_OBJECT_BUDGET` splits
//! a plan; the models for that are the ones the Measure runs read. What it does
//! check is the plan: every owner lands in exactly one of the streamed pass and
//! the batches, each batch reads the closure it needs, the re-reads are the
//! ones an owner or a symbol named, and both reads give the same elements and
//! progress.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use rvt::RevitFile;
use rvt::control::WalkerControl;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene;

/// A progress event as the CLIs print it: stage, done, total.
type Progress = (&'static str, u64, Option<u64>);

/// A control that records every progress event it is given.
fn recording() -> (WalkerControl, Arc<Mutex<Vec<Progress>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&events);
    let control = WalkerControl::new().with_progress(move |event| {
        kept.lock().unwrap_or_else(|e| e.into_inner()).push((
            event.stage.as_str(),
            event.done,
            event.total,
        ));
    });
    (control, events)
}

/// The Einhoven project in the project corpus: `None` when no corpus is
/// configured, and a failure when one is configured without the file.
fn einhoven() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)?;
    let path = dir.join("Revit_IFC5_Einhoven.rvt");
    assert!(
        path.exists(),
        "RVT_PROJECT_CORPUS_DIR is set to {} but Revit_IFC5_Einhoven.rvt is not there",
        dir.display()
    );
    Some(path)
}

#[test]
fn the_batched_read_agrees_with_the_one_shot_read() {
    let Some(path) = einhoven() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut rf = RevitFile::open(&path).unwrap_or_else(|e| panic!("open {}: {e}", path.display()));
    let release = rf.basic_file_info().map(|info| info.version).unwrap_or(0);
    assert!(
        (2023..=2027).contains(&release),
        "{}: Revit {release}, and saved graphics are read on 2023 to 2027",
        path.display()
    );
    // Every channel-103 owner, no selection: the file is small enough to
    // read them all, so the check is not narrowed to doors and windows.
    let options = Options {
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };

    let (one_control, one_progress) = recording();
    let scene =
        native_saved_scene::extract_at_detail_with_control(&mut rf, &options, 3, &one_control)
            .expect("one-shot saved-graphics read");
    let one_shot: BTreeMap<u64, serde_json::Value> = scene
        .elements
        .iter()
        .map(|element| {
            (
                element.id,
                serde_json::to_value(element).expect("element to JSON"),
            )
        })
        .collect();
    drop(scene);

    let (batch_control, batch_progress) = recording();
    let mut batched: BTreeMap<u64, serde_json::Value> = BTreeMap::new();
    native_saved_scene::extract_in_batches_with_control(
        &mut rf,
        &options,
        3,
        &batch_control,
        |element| {
            let id = element.id;
            let previous = batched.insert(id, serde_json::to_value(&element)?);
            assert!(previous.is_none(), "owner {id} was visited twice");
            Ok(())
        },
    )
    .expect("batched saved-graphics read");

    // The file's own count: if the corpus ever serves another file under this
    // name, this fails instead of the check quietly comparing nothing.
    // `Revit_IFC5_Einhoven.rvt` holds 2,615 owners with saved graphics.
    assert_eq!(
        one_shot.len(),
        2_615,
        "{}'s saved-graphics owners changed: update this count",
        path.display()
    );
    let missing: Vec<u64> = one_shot
        .keys()
        .filter(|id| !batched.contains_key(id))
        .copied()
        .collect();
    let extra: Vec<u64> = batched
        .keys()
        .filter(|id| !one_shot.contains_key(id))
        .copied()
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "owners only the one-shot read emits: {missing:?}; only the batched read: {extra:?}"
    );
    let different: Vec<u64> = one_shot
        .iter()
        .filter(|(id, element)| batched.get(id) != Some(element))
        .map(|(id, _)| *id)
        .collect();
    assert!(
        different.is_empty(),
        "owners whose element differs between the two reads: {different:?}"
    );
    assert_eq!(
        *one_progress.lock().unwrap_or_else(|e| e.into_inner()),
        *batch_progress.lock().unwrap_or_else(|e| e.into_inner()),
        "the two reads report different progress events"
    );
}
