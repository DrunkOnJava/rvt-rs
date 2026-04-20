//! Q-08: heap-profile the schema parser on a real Revit file.
//!
//! Build + run (requires the `dhat-heap` feature):
//!
//! ```sh
//! cargo run --release --features dhat-heap \
//!   --example dhat_schema -- path/to/file.rfa
//! ```
//!
//! Produces a `dhat-heap.json` in the current working directory.
//! Open it in the dhat-viewer web app at
//! <https://nnethercote.github.io/dh_view/dh_view.html> — drag the
//! JSON onto the page to get a per-call-site breakdown of total
//! bytes allocated, bytes at peak, and allocation count.
//!
//! The measurement covers:
//!
//! 1. OLE container open (`cfb` crate I/O + internal dir tables)
//! 2. Truncated-gzip decompression of `Formats/Latest`
//! 3. Schema-header + per-class field-table parse (every class,
//!    every field, every tagged enumeration)
//! 4. Conversion into the in-memory `SchemaTable`
//!
//! With the `dhat-heap` feature DISABLED this example binary
//! doesn't compile — the `dhat::Profiler` type is gated behind
//! the feature flag so default builds stay allocator-cost-free.
//!
//! To profile a different code path (walker, IFC export, writer),
//! copy this file and swap the inner `f.formats()` call for the
//! function you want to measure. The `Profiler` guard at function
//! scope ensures everything between `::new_heap()` and the
//! implicit drop lands in the trace.

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(feature = "dhat-heap")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start the profiler BEFORE we allocate anything we care about.
    // The guard's Drop impl flushes the trace to `dhat-heap.json`.
    let _profiler = dhat::Profiler::new_heap();

    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "tests/fixtures/sample.rfa".to_string());

    let mut f = rvt::RevitFile::open(&path)?;
    let schema = f.schema()?;
    println!(
        "parsed {} classes, {} fields total",
        schema.classes.len(),
        schema.classes.iter().map(|c| c.fields.len()).sum::<usize>()
    );
    println!("dhat profile written to dhat-heap.json");
    Ok(())
}

// Without the feature, leave a stub main that compiles but is
// useless — ensures `cargo check` on the default build still
// succeeds, while making the feature-gated intent explicit.
#[cfg(not(feature = "dhat-heap"))]
fn main() {
    eprintln!(
        "This example requires the `dhat-heap` feature.\n\
         Run it as:\n\
         \n  cargo run --release --features dhat-heap --example dhat_schema -- path/to/file.rfa\n"
    );
    std::process::exit(2);
}
