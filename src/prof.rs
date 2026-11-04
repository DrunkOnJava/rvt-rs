use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, HashSet};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();
static LAST: Mutex<Option<Instant>> = Mutex::new(None);

pub fn start() -> Instant {
    *START.get_or_init(Instant::now)
}

pub fn at() -> u128 {
    start().elapsed().as_millis()
}

static COMPARED: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static DIFFS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
static FALLBACKS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

pub fn compared(fast: &str, slow: &str) {
    use std::sync::atomic::Ordering::Relaxed;
    COMPARED.fetch_add(1, Relaxed);
    if fast != slow && DIFFS.fetch_add(1, Relaxed) < 3 {
        eprintln!("BASEDIFF fast={fast} slow={slow}");
    }
}

pub fn fallback() {
    FALLBACKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
}

pub fn lap(name: &str) {
    use std::sync::atomic::Ordering::Relaxed;
    let now = Instant::now();
    let mut last = LAST.lock().unwrap();
    let previous = last.unwrap_or_else(start);
    *last = Some(now);
    eprintln!(
        "LAP {name} +{} ms @{} ms base_compared={} base_diffs={} base_fallbacks={}",
        now.duration_since(previous).as_millis(),
        at(),
        COMPARED.load(Relaxed),
        DIFFS.load(Relaxed),
        FALLBACKS.load(Relaxed)
    );
}

pub struct Needles {
    bloom: Vec<bool>,
    set: HashSet<u32>,
}

thread_local! {
    static NEEDLES: RefCell<Option<Needles>> = const { RefCell::new(None) };
    static HIT: Cell<bool> = const { Cell::new(false) };
    static STATS: RefCell<BTreeMap<String, [usize; 4]>> = const { RefCell::new(BTreeMap::new()) };
}

pub fn set_needles(ids: &[i64]) {
    let set: HashSet<u32> = ids.iter().map(|&id| id as u32).collect();
    let mut bloom = vec![false; 65536];
    for &n in &set {
        bloom[(n & 0xffff) as usize] = true;
    }
    NEEDLES.with(|n| *n.borrow_mut() = Some(Needles { bloom, set }));
}

pub fn clear_needles() {
    NEEDLES.with(|n| *n.borrow_mut() = None);
}

pub fn scan(body: &[u8]) -> Option<bool> {
    NEEDLES.with(|n| {
        let n = n.borrow();
        let n = n.as_ref()?;
        let hit = body.windows(4).any(|w| {
            let v = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            n.bloom[(v & 0xffff) as usize] && n.set.contains(&v)
        });
        HIT.with(|h| h.set(hit));
        Some(hit)
    })
}

pub fn last_hit() -> bool {
    HIT.with(|h| h.get())
}

pub fn stat(key: &str, hit: bool, contrib: bool) {
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let e = s.entry(key.to_string()).or_default();
        e[0] += 1;
        e[1] += usize::from(hit);
        e[2] += usize::from(contrib);
        e[3] += usize::from(contrib && !hit);
    });
}

pub fn dump_stats(label: &str) {
    STATS.with(|s| {
        let mut s = s.borrow_mut();
        let text: Vec<String> = s
            .iter()
            .map(|(k, e)| {
                format!(
                    "{k}=[records {} hit {} contrib {} violations {}]",
                    e[0], e[1], e[2], e[3]
                )
            })
            .collect();
        eprintln!("STATS {label} {}", text.join(" "));
        s.clear();
    });
}
