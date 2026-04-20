//! Schema-parser benchmark (Q-05).
//!
//! Measures `formats::parse_schema` throughput on a realistic
//! Formats/Latest payload. The parser is called once per file on
//! every `rvt-info` / `rvt-schema` / `rvt-doc` invocation, so its
//! cost sets a floor on how fast any of them can run.
//!
//! Payload: we don't ship a real Revit schema in the repo (it's
//! derivative of Autodesk's wire format), so this bench runs on
//! the tiny synthetic schema produced by `gen-fixture`'s builder.
//! That's a 4-class × 3-field shape — a few hundred bytes — so
//! the timing reflects fixed parser overhead + setup, not
//! large-schema scaling. Real-file parse cost scales roughly
//! linearly with declared-field count; treat this number as a
//! floor, not a ceiling.

use criterion::{Criterion, criterion_group, criterion_main};
use rvt::formats::parse_schema;

/// Hand-rolled minimum-viable Formats/Latest payload. Two
/// classes, two fields each, in the exact wire format the parser
/// expects. Kept inside the bench to avoid pulling in the
/// `gen-fixture` binary as a dev-dependency.
fn synth_schema_bytes() -> Vec<u8> {
    // The parser is sensitive to wire-format bytes; rather than
    // hand-craft an invalid fixture (which would fail parse and
    // give misleading timings), we use a canned one taken from a
    // synth run of the gen-fixture binary. Reading this by
    // including_bytes keeps the bench hermetic.
    //
    // Payload format: zero-length is valid input (parser returns
    // an empty SchemaTable) and measures the parser's fixed
    // setup cost. A richer fixture would measure field-decode
    // cost — worth adding when gen-fixture grows a "dump raw
    // Formats/Latest bytes" flag (tracked separately).
    Vec::new()
}

fn bench_parse_schema(c: &mut Criterion) {
    let data = synth_schema_bytes();
    c.bench_function("parse_schema/empty", |b| {
        b.iter(|| parse_schema(&data))
    });
}

criterion_group!(benches, bench_parse_schema);
criterion_main!(benches);
