//! #1694: a compact graph value holds the same graphs in a fraction of the heap.
//!
//! FACT under test: `rvt::research::graph_value::Fv` holds the fields of every
//! object of a decoded graph, whatever the file, in at most a fifth of the heap
//! bytes that `serde_json::Value` takes, and is exact: `Fv::to_json(Fv::from_json(v))`
//! equals `v` on every object of every graph the file's native records decode
//! to. What it does not test is whether the readers can be moved onto it; the
//! pull request that does that measures the exports.
//!
//! A counting global allocator keeps the heap bytes live. Each record's graph
//! is decoded (the live bytes after over before are what the graph holds
//! today), then its objects' fields are cloned (the live bytes of the clone
//! are a floor for today's: a clone is built whole, while the decoder grows
//! each map by inserting) and converted to `Fv` (the live bytes of the
//! conversion, with the lists of names the objects share added to the table
//! the first time they are seen). The graph, the clones and the conversion are
//! dropped before the next record, so the probe holds one record at a time.
//! The conversion is turned back into JSON and compared with the original
//! fields after its bytes are counted.
//!
//! Output, one line each:
//!   file <path> release <release>
//!   graphs <n> objects <n> values <n> refused <n> stopped <bool> seconds <n>
//!   heap decoded <bytes> clone <bytes> compact <bytes> name-lists <n>
//!   ratio compact/clone <x> compact/decoded <x> bytes-per-value clone <x> compact <x>
//!   roundtrip compared <n> mismatches <n>
//!
//! The exit status is 1 where the round trip differs anywhere.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1694_compact_graph -- FILE

use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::time::{Duration, Instant};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::{Fv, NameTable};
use rvt::research::native_document::{self, Options};
use rvt::research::native_es_catalog;
use rvt::research::native_parameters::{GraphLimits, decode_graph_with_catalog_usage};
use rvt::research::schema_registry;

const BUDGET: Duration = Duration::from_secs(10 * 60);

struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            LIVE.fetch_add(layout.size(), Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let moved = unsafe { System.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            LIVE.fetch_sub(layout.size(), Relaxed);
            LIVE.fetch_add(new_size, Relaxed);
        }
        moved
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn live() -> usize {
    LIVE.load(Relaxed)
}

fn ratio(part: usize, whole: usize) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1694_compact_graph FILE"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");
    if !native_document::NATIVE_RELEASES.contains(&release) {
        println!("native: release {release} is outside the native readers' range");
        return Ok(());
    }
    let formats = native_document::read_single(&mut rf, "Formats/Latest")?;
    let registry = schema_registry::parse(&formats)?;
    let catalog = native_document::read_single(&mut rf, "Global/Latest")
        .and_then(|bytes| native_es_catalog::decode(&bytes, &registry))
        .ok()
        .map(|(catalog, _)| catalog);
    let limits = GraphLimits {
        max_values: 100_000,
        max_objects: 100_000,
        ..GraphLimits::default()
    };
    // Every record is handed over with its body and no graph: this probe
    // decodes them itself, to count what each graph holds.
    let options = Options {
        releases: native_document::NATIVE_RELEASES,
        graph_classes: Some(BTreeSet::new()),
        keep_body: true,
        ..Options::default()
    };
    let started = Instant::now();
    let mut table = NameTable::default();
    let mut graphs = 0usize;
    let mut refused = 0usize;
    let mut objects = 0usize;
    let mut values = 0usize;
    let mut decoded_bytes = 0usize;
    let mut clone_bytes = 0usize;
    let mut compact_bytes = 0usize;
    let mut compared = 0usize;
    let mut mismatches = 0usize;
    let mut stopped = false;
    let result = native_document::extract_graphs(&mut rf, &options, |record| {
        if started.elapsed() > BUDGET {
            stopped = true;
            anyhow::bail!("time budget after {graphs} graphs");
        }
        let Some(body) = &record.body else {
            return Ok(());
        };
        let before = live();
        let decoded = decode_graph_with_catalog_usage(body, &registry, &limits, catalog.as_ref());
        let after = live();
        let Ok((graph, _usage)) = decoded else {
            refused += 1;
            return Ok(());
        };
        graphs += 1;
        objects += graph.objects.len();
        decoded_bytes += after.saturating_sub(before);

        let before = live();
        let copies: Vec<serde_json::Value> = graph
            .objects
            .iter()
            .map(|object| object.fields.clone())
            .collect();
        clone_bytes += live().saturating_sub(before);
        drop(copies);

        let before = live();
        let compact: Vec<Fv> = graph
            .objects
            .iter()
            .map(|object| Fv::from_json(&object.fields, &mut table))
            .collect();
        compact_bytes += live().saturating_sub(before);

        for (fv, object) in compact.iter().zip(&graph.objects) {
            values += fv.node_count();
            compared += 1;
            mismatches += usize::from(fv.to_json() != object.fields);
        }
        Ok(())
    });
    if let Err(error) = result {
        if !stopped {
            return Err(error);
        }
    }
    println!(
        "graphs {graphs} objects {objects} values {values} refused {refused} stopped {stopped} seconds {}",
        started.elapsed().as_secs()
    );
    println!(
        "heap decoded {decoded_bytes} clone {clone_bytes} compact {compact_bytes} name-lists {}",
        table.len()
    );
    println!(
        "ratio compact/clone {:.3} compact/decoded {:.3} bytes-per-value clone {:.1} compact {:.1}",
        ratio(compact_bytes, clone_bytes),
        ratio(compact_bytes, decoded_bytes),
        ratio(clone_bytes, values),
        ratio(compact_bytes, values),
    );
    println!("roundtrip compared {compared} mismatches {mismatches}");
    if mismatches > 0 {
        std::process::exit(1);
    }
    Ok(())
}
