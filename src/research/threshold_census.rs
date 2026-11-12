//! How close the value each near-zero geometric test compares comes to its
//! threshold, counted by decade (#1825).
//!
//! RE-447 found a 1e-9 test deciding a wall's axis on coordinates that drift
//! by about 5e-8 ft. #1825 lists eleven more tests at 1e-9 or 1e-12. Each
//! notes the value it compares here, through [`note`], which returns that
//! value unchanged, so the test decides exactly as before. Nothing is counted
//! until a probe calls [`record_thresholds`]; until then a note is one relaxed
//! atomic load.
//!
//! The counts are kept across threads: the saved meshes are built on worker
//! threads (`native_saved_scene`), and a thread-local count would read zero
//! there.
//!
//! Each site is keyed by the file and line of its `note` call. A value falls
//! in one of [`BUCKETS`] buckets by its sign and the decade of its magnitude,
//! with exact zeros apart: a test whose value is exactly zero (a ring that
//! repeats its closing point, two boxes of the same bits) cannot tip, while
//! one whose value lies between 1e-12 and 1e-6 sits where a coordinate's
//! drift can move it across.

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// The number of buckets a site's values fall into.
pub const BUCKETS: usize = 14;

/// Each bucket's label, in order: negative values from the largest magnitude
/// down, exact zero, positive values from the smallest up, then NaN.
pub const BUCKET_LABELS: [&str; BUCKETS] = [
    "<-1e-6", "-1e-6", "-1e-7", "-1e-8", "-1e-9", "-1e-12", "0", "+1e-12", "+1e-9", "+1e-8",
    "+1e-7", "+1e-6", ">1e-6", "nan",
];

// A magnitude falls in the first decade whose edge it does not pass.
const EDGES: [f64; 5] = [1e-12, 1e-9, 1e-8, 1e-7, 1e-6];

static ON: AtomicBool = AtomicBool::new(false);
static COUNTS: Mutex<BTreeMap<(&'static str, u32), [u64; BUCKETS]>> = Mutex::new(BTreeMap::new());

/// The values one site compared while the census was on.
#[derive(Debug, Clone)]
pub struct SiteCount {
    /// The source file of the site's `note` call.
    pub file: &'static str,
    /// Its line.
    pub line: u32,
    /// How many values fell in each bucket of [`BUCKET_LABELS`].
    pub buckets: [u64; BUCKETS],
}

/// Starts counting, from nothing.
pub fn record_thresholds() {
    if let Ok(mut counts) = COUNTS.lock() {
        counts.clear();
    }
    ON.store(true, Ordering::SeqCst);
}

/// Stops counting and returns every site's counts, by file and line.
pub fn take_thresholds() -> Vec<SiteCount> {
    ON.store(false, Ordering::SeqCst);
    let taken = COUNTS
        .lock()
        .map(|mut counts| std::mem::take(&mut *counts))
        .unwrap_or_default();
    taken
        .into_iter()
        .map(|((file, line), buckets)| SiteCount {
            file,
            line,
            buckets,
        })
        .collect()
}

/// Counts `value`, the quantity a near-zero test compares, against the line
/// that calls this, and returns it unchanged.
#[track_caller]
#[inline]
pub(crate) fn note(value: f64) -> f64 {
    if ON.load(Ordering::Relaxed) {
        let at = std::panic::Location::caller();
        if let Ok(mut counts) = COUNTS.lock() {
            counts.entry((at.file(), at.line())).or_insert([0; BUCKETS])[bucket(value)] += 1;
        }
    }
    value
}

fn bucket(value: f64) -> usize {
    if value.is_nan() {
        return BUCKETS - 1;
    }
    if value == 0.0 {
        return 6;
    }
    let decade = EDGES
        .iter()
        .position(|&edge| value.abs() <= edge)
        .unwrap_or(EDGES.len());
    if value < 0.0 { 5 - decade } else { 7 + decade }
}
