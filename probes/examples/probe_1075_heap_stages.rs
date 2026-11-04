//! #1075: where `rvt-ifc`'s memory goes, in exact heap bytes.
//!
//! FACT under test: the export's peak is not one structure. The inflate that
//! builds the handle's cache holds every member's output beside the joined
//! copy it makes; every native pass reads and strips a copy of each stored
//! partition and joins each continuation group into a copy of its own; the
//! saved-graphics pass keeps every owner's decoded graph until the last one is
//! meshed. This probe sizes each, by the number of heap bytes live at the time,
//! which a resident-set size cannot give: the allocator keeps freed pages, so
//! after a drop the process's resident size does not fall.
//!
//! A counting global allocator keeps the live bytes, the high-water mark of
//! each stage, and the live bytes by block size. A sampler thread copies the
//! by-size table whenever the live bytes pass their last high mark, so a
//! stage's `at_peak` line says which block sizes held its peak. The resident
//! set (`VmRSS`) and its high-water mark (`VmHWM`, reset before each stage
//! through `/proc/self/clear_refs`) are printed beside them; both are 0
//! outside Linux.
//!
//! Stages, in the order `rvt-ifc` takes them (open, export, diagnostics, saved
//! graphics, the STEP text), each preceded by what it would otherwise charge
//! to the export:
//!
//! - `inflate <stream>`: one stream's `RevitFile::inflated_partition`, alone.
//!   Its peak over its start is what building the cache costs at the most.
//! - `read+strip <stream>`: the copy of the stored stream and the copy with
//!   the checksum pages stripped that each native pass makes of every
//!   partition it reads.
//! - `walk <stream>`: `native_segments::walk_inflated` over the stream with a
//!   callback that keeps nothing. Its peak over its start is what the
//!   continuation groups it joins cost, and its `multi_mib` is the bytes a
//!   pass copies to join them; `max_group_mib` is the largest group.
//! - `prewarm_*`: the handle's memoised readers, one at a time, so what each
//!   keeps is its `end` over its `start`.
//! - `export`, `diagnostics`, `saved_meshes`, `drop_file`, `step_stream`,
//!   `drop_model`. The export and the saved meshes also print the progress
//!   events of the walk (`event`): the first and last live bytes of each stage
//!   the walker reports, and the most seen in it.
//!
//! The `export` stage follows the prewarms, which fill caches the export would
//! fill itself (every partition's data objects, for one); its `end` over its
//! `start` is therefore what the export adds to them.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1075_heap_stages -- MODEL.rvt [--large-model]

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

use rvt::RevitFile;
use rvt::control::{Stage, WalkerControl};
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::step_writer::{StepOptions, write_step_to};
use rvt::ifc::{ExportDiagnosticsMode, ExportQualityMode, IfcModel, RvtDocExporter};
use rvt::research::native_document::{self, Record};
use rvt::research::{native_segments, schema_registry};

const CLASSES: usize = 40;
const BIG_STREAM_BYTES: usize = 8 << 20;
const AT_PEAK_FLOOR_BYTES: usize = 16 << 20;

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CLASS_BYTES: [AtomicUsize; CLASSES] = [const { AtomicUsize::new(0) }; CLASSES];
static SNAPSHOT: [AtomicUsize; CLASSES] = [const { AtomicUsize::new(0) }; CLASSES];
static SNAPSHOT_LIVE: AtomicUsize = AtomicUsize::new(0);
static SAMPLING: AtomicBool = AtomicBool::new(true);

fn class_of(size: usize) -> usize {
    ((usize::BITS - 1 - size.max(1).leading_zeros()) as usize).min(CLASSES - 1)
}

fn added(size: usize) {
    let now = LIVE.fetch_add(size, Relaxed) + size;
    PEAK.fetch_max(now, Relaxed);
    CLASS_BYTES[class_of(size)].fetch_add(size, Relaxed);
}

fn removed(size: usize) {
    LIVE.fetch_sub(size, Relaxed);
    CLASS_BYTES[class_of(size)].fetch_sub(size, Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            added(layout.size());
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            added(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        removed(layout.size());
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            removed(layout.size());
            added(new_size);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn sample_peaks() {
    while SAMPLING.load(Relaxed) {
        let live = LIVE.load(Relaxed);
        if live > SNAPSHOT_LIVE.load(Relaxed) {
            for (to, from) in SNAPSHOT.iter().zip(CLASS_BYTES.iter()) {
                to.store(from.load(Relaxed), Relaxed);
            }
            SNAPSHOT_LIVE.store(live, Relaxed);
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn status_kib(field: &str) -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status.lines().find_map(|line| {
                line.strip_prefix(field)?
                    .trim()
                    .trim_end_matches("kB")
                    .trim()
                    .parse()
                    .ok()
            })
        })
        .unwrap_or(0)
}

fn mib(bytes: usize) -> f64 {
    bytes as f64 / 1_048_576.0
}

fn mib_kib(kib: u64) -> f64 {
    kib as f64 / 1024.0
}

fn size_label(class: usize) -> String {
    let bytes = 1usize << class;
    if bytes >= 1 << 30 {
        format!("{}G", bytes >> 30)
    } else if bytes >= 1 << 20 {
        format!("{}M", bytes >> 20)
    } else if bytes >= 1 << 10 {
        format!("{}K", bytes >> 10)
    } else {
        format!("{bytes}B")
    }
}

struct Mark {
    started: Instant,
    live_start: usize,
    rss_start: u64,
}

fn begin() -> Mark {
    let _ = std::fs::write("/proc/self/clear_refs", "5");
    PEAK.store(LIVE.load(Relaxed), Relaxed);
    SNAPSHOT_LIVE.store(0, Relaxed);
    Mark {
        started: Instant::now(),
        live_start: LIVE.load(Relaxed),
        rss_start: status_kib("VmRSS:"),
    }
}

struct Sample {
    ms: u128,
    live_start: usize,
    live_end: usize,
    live_peak: usize,
    rss_start: u64,
    rss_end: u64,
    rss_peak: u64,
    at_peak: String,
}

impl Mark {
    fn finish(self) -> Sample {
        let ms = self.started.elapsed().as_millis();
        let live_end = LIVE.load(Relaxed);
        let live_peak = PEAK.load(Relaxed).max(live_end);
        let mut classes: Vec<(usize, usize)> = SNAPSHOT
            .iter()
            .enumerate()
            .map(|(class, bytes)| (class, bytes.load(Relaxed)))
            .filter(|&(_, bytes)| bytes >= 1 << 20)
            .collect();
        classes.sort_by_key(|&(_, bytes)| std::cmp::Reverse(bytes));
        let at_peak = classes
            .iter()
            .take(6)
            .map(|&(class, bytes)| format!("{}:{:.0}", size_label(class), mib(bytes)))
            .collect::<Vec<_>>()
            .join(" ");
        Sample {
            ms,
            live_start: self.live_start,
            live_end,
            live_peak,
            rss_start: self.rss_start,
            rss_end: status_kib("VmRSS:"),
            rss_peak: status_kib("VmHWM:"),
            at_peak,
        }
    }
}

impl Sample {
    fn grew_over_start(&self) -> usize {
        self.live_peak.saturating_sub(self.live_start)
    }

    fn print(&self, label: &str, detail: &str) {
        println!(
            "{label} ms {} live_mib start {:.1} end {:.1} peak {:.1} over_start {:.1} rss_mib start {:.1} end {:.1} peak {:.1} {detail}",
            self.ms,
            mib(self.live_start),
            mib(self.live_end),
            mib(self.live_peak),
            mib(self.grew_over_start()),
            mib_kib(self.rss_start),
            mib_kib(self.rss_end),
            mib_kib(self.rss_peak),
        );
        if self.grew_over_start() >= AT_PEAK_FLOOR_BYTES && !self.at_peak.is_empty() {
            println!("  at_peak by block size (live MiB): {}", self.at_peak);
        }
    }
}

struct Event {
    stage: Stage,
    first_ms: u128,
    last_ms: u128,
    done: u64,
    total: Option<u64>,
    live_first: usize,
    live_last: usize,
    live_max: usize,
}

static EVENTS: Mutex<Vec<Event>> = Mutex::new(Vec::new());

fn recording_control() -> WalkerControl {
    let started = Instant::now();
    EVENTS.lock().unwrap_or_else(|e| e.into_inner()).clear();
    WalkerControl::new().with_progress(move |event| {
        let ms = started.elapsed().as_millis();
        let live = LIVE.load(Relaxed);
        let mut events = EVENTS.lock().unwrap_or_else(|e| e.into_inner());
        match events.last_mut() {
            Some(last) if last.stage == event.stage => {
                last.last_ms = ms;
                last.done = event.done;
                last.total = event.total;
                last.live_last = live;
                last.live_max = last.live_max.max(live);
            }
            _ => events.push(Event {
                stage: event.stage,
                first_ms: ms,
                last_ms: ms,
                done: event.done,
                total: event.total,
                live_first: live,
                live_last: live,
                live_max: live,
            }),
        }
    })
}

fn print_events() {
    for event in EVENTS.lock().unwrap_or_else(|e| e.into_inner()).iter() {
        let total = event
            .total
            .map_or_else(|| "-".to_string(), |t| t.to_string());
        println!(
            "  event {} ms {}..{} done {}/{} live_mib first {:.1} last {:.1} max {:.1}",
            event.stage.as_str(),
            event.first_ms,
            event.last_ms,
            event.done,
            total,
            mib(event.live_first),
            mib(event.live_last),
            mib(event.live_max),
        );
    }
}

#[derive(Default)]
struct Totals {
    streams: usize,
    ms: u128,
    stored: u64,
    bytes: usize,
    widest_growth: usize,
    rss_peak: u64,
}

impl Totals {
    fn add(&mut self, sample: &Sample, stored: u64, bytes: usize) {
        self.streams += 1;
        self.ms += sample.ms;
        self.stored += stored;
        self.bytes += bytes;
        self.widest_growth = self.widest_growth.max(sample.grew_over_start());
        self.rss_peak = self.rss_peak.max(sample.rss_peak);
    }

    fn print(&self, label: &str, detail: &str) {
        println!(
            "{label} streams {} ms {} stored_mib {:.1} inflated_mib {:.1} widest_growth_mib {:.1} rss_peak_mib {:.1} {detail}",
            self.streams,
            self.ms,
            mib(self.stored as usize),
            mib(self.bytes),
            mib(self.widest_growth),
            mib_kib(self.rss_peak),
        );
    }
}

/// A saved identifier as `native_metadata::identifier` reads it.
fn identifier(value: &serde_json::Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

/// The symbols the records' `InstanceInfo` objects name, with how many
/// references name them.
fn symbol_references(records: &BTreeMap<u64, Record>) -> BTreeMap<u64, usize> {
    let mut named = BTreeMap::new();
    for graph in records.values().filter_map(|record| record.graph.as_ref()) {
        for object in &graph.objects {
            if object.class_name != "InstanceInfo" {
                continue;
            }
            let id = identifier(&object.fields["m_symbolId"])
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0);
            if let Some(id) = id {
                *named.entry(id).or_insert(0) += 1;
            }
        }
    }
    named
}

fn graph_objects(records: &BTreeMap<u64, Record>) -> usize {
    records
        .values()
        .filter_map(|record| record.graph.as_ref())
        .map(|graph| graph.objects.len())
        .sum()
}

/// The saved-graphics pass's two reads, one at a time as it makes them:
/// the doors' and windows' own graphs, then the symbols they name. What each
/// holds says whether the instances or the shared symbols are the memory.
fn saved_graphics_split(rf: &mut RevitFile, model: &IfcModel) -> anyhow::Result<()> {
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if matches!(ifc_type.as_str(), "IFCDOOR" | "IFCWINDOW") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut options = native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mark = begin();
    let mut roots: BTreeMap<u64, Record> = BTreeMap::new();
    native_document::extract_graphs(rf, &options, |record| {
        roots.insert(record.identity.element_id, record);
        Ok(())
    })?;
    let sample = mark.finish();
    sample.print(
        "stage saved_roots",
        &format!(
            "records {} graph_objects {}",
            roots.len(),
            graph_objects(&roots)
        ),
    );

    let named = symbol_references(&roots);
    options.selected_ids = named
        .keys()
        .copied()
        .filter(|id| !roots.contains_key(id))
        .collect();
    let mark = begin();
    let mut symbols: BTreeMap<u64, Record> = BTreeMap::new();
    native_document::extract_graphs(rf, &options, |record| {
        symbols.insert(record.identity.element_id, record);
        Ok(())
    })?;
    let sample = mark.finish();
    sample.print(
        "stage saved_symbols",
        &format!(
            "named {} records {} graph_objects {} references {}",
            named.len(),
            symbols.len(),
            graph_objects(&symbols),
            named.values().sum::<usize>()
        ),
    );

    let mark = begin();
    drop(roots);
    drop(symbols);
    mark.finish().print("stage saved_split_drop", "");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args.iter().find(|a| !a.starts_with("--")).ok_or_else(|| {
        anyhow::anyhow!("usage: probe_1075_heap_stages MODEL.rvt [--large-model]")
    })?;
    let large_model = args.iter().any(|a| a == "--large-model");
    let sampler = std::thread::spawn(sample_peaks);

    println!(
        "start live_mib {:.1} rss_mib {:.1}",
        mib(LIVE.load(Relaxed)),
        mib_kib(status_kib("VmRSS:"))
    );

    let mark = begin();
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, large_model))?;
    let release = rf.basic_file_info().map(|b| b.version).unwrap_or(0);
    mark.finish()
        .print("stage open", &format!("release {release}"));

    let mut names = rf.partition_stream_names();
    names.push(rvt::streams::GLOBAL_LATEST.to_string());

    let mut inflate = Totals::default();
    for name in &names {
        let stored = rf.stream_size(name).unwrap_or(0);
        let mark = begin();
        let inflated = rf.inflated_partition(name);
        let sample = mark.finish();
        let (bytes, members) = inflated
            .as_ref()
            .map_or((0, 0), |i| (i.bytes().len(), i.chunk_count()));
        inflate.add(&sample, stored, bytes);
        if bytes >= BIG_STREAM_BYTES {
            sample.print(
                &format!("inflate {name}"),
                &format!(
                    "stored_mib {:.1} inflated_mib {:.1} members {members}",
                    mib(stored as usize),
                    mib(bytes)
                ),
            );
        }
    }
    inflate.print("inflate_all", "");

    let formats = native_document::read_single(&mut rf, rvt::streams::FORMATS_LATEST)?;
    let registry = schema_registry::parse(&formats)?;
    drop(formats);
    let budget = rf
        .limits()
        .inflate_limits
        .max_output_bytes
        .max(native_document::DEFAULT_MAX_GROUP_BYTES);
    let (mut strip, mut walk) = (Totals::default(), Totals::default());
    let (mut groups_all, mut multi_groups_all, mut multi_bytes_all, mut widest_group) =
        (0usize, 0usize, 0usize, 0usize);
    for name in names.iter().filter(|n| n.starts_with("Partitions/")) {
        let Ok(inflated) = rf.inflated_partition(name) else {
            continue;
        };
        let mark = begin();
        let Ok(stored) = rf.read_stream(name) else {
            continue;
        };
        let prepared = rvt::compression::prepare_stream_for_inflate(name, &stored);
        let strip_sample = mark.finish();
        let stored_bytes = stored.len() as u64;
        strip.add(&strip_sample, stored_bytes, prepared.len());

        let (mut groups, mut multi_groups, mut multi_bytes, mut widest) = (0usize, 0, 0usize, 0);
        let mark = begin();
        let walked = native_segments::walk_inflated(
            &prepared,
            Some(&*inflated),
            &registry,
            budget,
            |source, bytes| {
                groups += 1;
                widest = widest.max(bytes.len());
                if source.segment_count > 1 {
                    multi_groups += 1;
                    multi_bytes += bytes.len();
                }
                Ok(())
            },
        );
        let walk_sample = mark.finish();
        walk.add(&walk_sample, stored_bytes, inflated.bytes().len());
        groups_all += groups;
        multi_groups_all += multi_groups;
        multi_bytes_all += multi_bytes;
        widest_group = widest_group.max(widest);
        if inflated.bytes().len() >= BIG_STREAM_BYTES {
            strip_sample.print(
                &format!("read+strip {name}"),
                &format!(
                    "stored_mib {:.1} prepared_mib {:.1}",
                    mib(stored.len()),
                    mib(prepared.len())
                ),
            );
            walk_sample.print(
                &format!("walk {name}"),
                &format!(
                    "groups {groups} multi_groups {multi_groups} multi_mib {:.1} max_group_mib {:.1} ok {}",
                    mib(multi_bytes),
                    mib(widest),
                    walked.is_ok()
                ),
            );
        }
    }
    strip.print("read+strip_all", "");
    walk.print(
        "walk_all",
        &format!(
            "groups {groups_all} multi_groups {multi_groups_all} multi_mib {:.1} max_group_mib {:.1}",
            mib(multi_bytes_all),
            mib(widest_group)
        ),
    );

    let mark = begin();
    let strings = rf.partition_string_records().map_or(0, |r| r.len());
    mark.finish().print(
        "stage prewarm_string_records",
        &format!("records {strings}"),
    );

    let mark = begin();
    let _ = rf.second_prologue_ids();
    mark.finish().print("stage prewarm_second_prologue_ids", "");

    let mark = begin();
    let _ = rf.element_names();
    mark.finish().print("stage prewarm_element_names", "");

    let mark = begin();
    let _ = rf.design_options();
    mark.finish().print("stage prewarm_design_options", "");

    let mark = begin();
    let _ = rf.phase_filter();
    mark.finish().print("stage prewarm_phase_filter", "");

    let mark = begin();
    let mut objects = 0usize;
    for name in names.iter().filter(|n| n.starts_with("Partitions/")) {
        objects += rf.partition_data_objects(name).map_or(0, |o| o.len());
    }
    mark.finish()
        .print("stage prewarm_data_objects", &format!("objects {objects}"));

    let mut walker_limits = rvt::walker::WalkerLimits::default();
    if large_model {
        walker_limits.max_scan_bytes = rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES;
    }

    let control = recording_control();
    let mark = begin();
    let mut model = RvtDocExporter.export_with_mode_limits_and_control(
        &mut rf,
        ExportQualityMode::Scaffold,
        walker_limits,
        &control,
    )?;
    mark.finish().print(
        "stage export",
        &format!("entities {}", model.entities.len()),
    );
    print_events();

    let mark = begin();
    let diagnostics = rvt::ifc::build_export_diagnostics_with_limits(
        &mut rf,
        &model,
        ExportDiagnosticsMode::Default,
        walker_limits,
    );
    mark.finish().print("stage diagnostics", "");
    drop(diagnostics);

    saved_graphics_split(&mut rf, &model)?;

    let control = recording_control();
    let mark = begin();
    let detail = match rvt::ifc::saved_meshes::default_scope(&mut rf) {
        Some(scope) => {
            match rvt::ifc::saved_meshes::attach_scoped_with_control(
                &mut rf, &mut model, scope, &control,
            ) {
                Ok(report) => format!(
                    "replaced {} of {} triangles {}",
                    report.replaced, report.candidates, report.triangles
                ),
                Err(e) => format!("error {e}"),
            }
        }
        None => "none".to_string(),
    };
    mark.finish().print("stage saved_meshes", &detail);
    print_events();

    // The pass frees its decoded graphs on a thread of its own.
    let mark = begin();
    std::thread::sleep(Duration::from_secs(3));
    mark.finish().print("stage settle", "");

    let mark = begin();
    drop(rf);
    mark.finish().print("stage drop_file", "");

    let mark = begin();
    let streamed = write_step_to(
        &model,
        &StepOptions { timestamp: Some(0) },
        &mut std::io::sink(),
    )?;
    mark.finish()
        .print("stage step_stream", &format!("bytes {streamed}"));

    let mark = begin();
    drop(model);
    mark.finish().print("stage drop_model", "");

    SAMPLING.store(false, Relaxed);
    let _ = sampler.join();
    Ok(())
}
