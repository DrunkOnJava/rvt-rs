use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static LAST: Mutex<Option<Instant>> = Mutex::new(None);
static CLASSES: Mutex<BTreeMap<String, (usize, u128, u128)>> = Mutex::new(BTreeMap::new());

pub fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

pub fn at() -> u128 {
    start().elapsed().as_millis()
}

pub fn lap(name: &str) {
    let now = Instant::now();
    let mut last = LAST.lock().unwrap();
    let previous = last.unwrap_or_else(start);
    *last = Some(now);
    eprintln!(
        "LAP {name} +{} ms @{} ms",
        now.duration_since(previous).as_millis(),
        at()
    );
}

pub fn class(label: &str, name: &str, decode_ns: u128, drop_ns: u128) {
    let mut classes = CLASSES.lock().unwrap();
    let entry = classes.entry(format!("{label}/{name}")).or_default();
    entry.0 += 1;
    entry.1 += decode_ns;
    entry.2 += drop_ns;
}

pub fn dump_classes(label: &str) {
    let mut classes = CLASSES.lock().unwrap();
    let mut rows: Vec<(String, (usize, u128, u128))> = classes
        .iter()
        .filter(|(key, _)| key.starts_with(&format!("{label}/")))
        .map(|(key, value)| (key.clone(), *value))
        .collect();
    rows.sort_by_key(|(_, (_, decode, drop))| std::cmp::Reverse(*decode + *drop));
    for (key, (n, decode, drop)) in rows.iter().take(6) {
        eprintln!(
            "CLASS {key} n={n} decode={}us drop={}us",
            decode / 1000,
            drop / 1000
        );
    }
    classes.retain(|key, _| !key.starts_with(&format!("{label}/")));
}

static PLAN_NS: AtomicU64 = AtomicU64::new(0);
static PLAN_COUNT: AtomicU64 = AtomicU64::new(0);

pub fn plan_built(ns: u64) {
    PLAN_NS.fetch_add(ns, Ordering::Relaxed);
    PLAN_COUNT.fetch_add(1, Ordering::Relaxed);
}

pub fn plan_totals() -> (u64, u64) {
    (
        PLAN_NS.load(Ordering::Relaxed) / 1000,
        PLAN_COUNT.load(Ordering::Relaxed),
    )
}
