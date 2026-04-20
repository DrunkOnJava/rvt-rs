# rvt-rs benchmarks

Performance measurement methodology, currently-measured CLI timings, and
the planned library-level benchmark suites that do **not yet exist**. The
project does not yet have a formal `criterion`-based microbench harness;
the numbers that do exist are `hyperfine`-driven end-to-end CLI timings.
This page documents both: what is measured today, and what is still
placeholder.

## Methodology

- **Hardware:** Apple M2 Max, 96 GB RAM, macOS 14 (arm64).
- **Rust version:** whatever `rustup default` resolves stable to at bench
  time. The crate declares `edition = "2024"` and `rust-version = "1.85"`
  (MSRV) in [`Cargo.toml`](../Cargo.toml); there is no pinned
  `rust-toolchain.toml`.
- **Build profile:** `cargo build --release` — `[profile.release]` sets
  `lto = true`, `codegen-units = 1`, `strip = true`.
- **Wall-clock tool:** [`hyperfine`](https://github.com/sharkdp/hyperfine)
  with 1-2 warmup runs and 100+ measured runs per command. Reports mean
  ± stddev, min, max. Used for CLI-level (end-to-end) timings.
- **Future microbench tool:** `criterion` (not yet wired up — see
  [Benchmark suites planned](#benchmark-suites-planned)).
- **Fixture size categories:**
  - **small** — under 1 MB. Reference: `racbasicsamplefamily-2024.rfa`
    at ~400 KB. Currently the only corpus measured.
  - **medium** — 1–50 MB. No fixture in corpus yet.
  - **large** — 50 MB and up. No fixture in corpus yet.
- **Corpus:** git-LFS-pulled [`phi-ag/rvt`](https://github.com/phi-ag/rvt)
  sample set, 11 Revit releases 2016-2026. All small-category files.

## Benchmark suites planned

Each of these is a separate `criterion` suite intended to live under
`benches/` once wired up. None exist yet; treat as design spec.

| Suite | What it measures | Status |
|---|---|---|
| `bench_identify` | File identification: OLE/CFB magic check + CFB header scan, without decompressing any stream. Isolates `reader::open` + `streams::has_stream` cost. | Not yet written — see [issue #TBD](https://github.com/DrunkOnJava/rvt-rs/issues) |
| `bench_schema` | Full class-schema enumeration: `Formats/Latest` decompress + tagged-record walk + all 395 classes × 13,570 fields typed. | Not yet written — see [issue #TBD](https://github.com/DrunkOnJava/rvt-rs/issues) |
| `bench_decode_class` | Element decode, **per class, per 1000 instances**, using the 29 shipped Layer 5b decoders (Level, Wall, Floor, Roof, Door, Window, Column, Beam, …). Fixtures are synthesized schema+bytes (same harness as the existing class-decoder unit tests) rather than real-file corpora. | Not yet written — see [issue #TBD](https://github.com/DrunkOnJava/rvt-rs/issues) |
| `bench_ifc_emit` | End-to-end project-to-STEP IFC4 emission: Revit file in → `IfcProject` + `IfcSite` + `IfcBuilding` + storeys + per-element entities out. Measures the full `rvt-ifc` pipeline. | Not yet written — see [issue #TBD](https://github.com/DrunkOnJava/rvt-rs/issues) |

### Results table (planned)

Populated once the `criterion` suites exist. Every cell is `TBD` until
real numbers land.

| Operation | Fixture size | Time (median) | Allocations | Comparison |
|---|---|---|---|---|
| `bench_identify` | small (~400 KB) | TBD | TBD | TBD |
| `bench_identify` | medium | TBD | TBD | TBD |
| `bench_identify` | large | TBD | TBD | TBD |
| `bench_schema` | small (~400 KB) | TBD | TBD | TBD |
| `bench_schema` | medium | TBD | TBD | TBD |
| `bench_schema` | large | TBD | TBD | TBD |
| `bench_decode_class` / Wall | 1000 synthesized instances | TBD | TBD | TBD |
| `bench_decode_class` / Door | 1000 synthesized instances | TBD | TBD | TBD |
| `bench_decode_class` / Column | 1000 synthesized instances | TBD | TBD | TBD |
| `bench_ifc_emit` | small (~400 KB) | TBD | TBD | TBD |
| `bench_ifc_emit` | medium | TBD | TBD | TBD |
| `bench_ifc_emit` | large | TBD | TBD | TBD |

## Current measurements (hyperfine, CLI-level)

These numbers **do exist today** and are reproducible with
[`tools/bench.sh`](../tools/bench.sh). They measure the whole binary —
process startup, argument parsing, file open, decode, format, write to
`/dev/null` — not isolated library operations. Treat them as the upper
bound on per-operation cost, not as microbenchmarks.

### Single-file timings (2024 sample, ~397 KB)

| CLI | Mean | Notes |
|---|---:|---|
| `rvt-history` | **3.8 ms** | Parse document-upgrade history only |
| `rvt-schema` | **4.5 ms** | Parse all 395 classes + 1,114 fields |
| `rvt-info` (text) | 7.6 ms | + basic-file-info + PartAtom + preview |
| `rvt-info` (json) | 7.7 ms | Same content, JSON-serialised |
| `rvt-history --partitions` | 19.4 ms | + every UTF-16LE string in `Partitions/NN` |
| `rvt-analyze` (text) | 26.9 ms | Full forensic report (identity + history + anchors + schema + link + content + disclosures) |
| `rvt-analyze` (json) | 26.8 ms | Same content, JSON-serialised |

Raw hyperfine export: [`docs/data/bench-2024.md`](data/bench-2024.md),
[`docs/data/bench-2024.csv`](data/bench-2024.csv).

### Full-corpus sweep

`rvt-analyze --redact --json` over all 11 Revit releases:

```
Mean:  227.2 ms ± 2.1 ms
Range: 224.8 ms … 230.7 ms
```

Per-file average: **20.6 ms**. Raw data:
[`docs/data/bench-versions.csv`](data/bench-versions.csv).

## Comparison targets

Honest comparison is hard because the nearest functional equivalent is
closed-source.

- **Autodesk's first-party path** — the [`revit-ifc`](https://github.com/Autodesk/revit-ifc)
  IFC exporter add-in runs **inside** Revit using the closed-source C#
  Revit API. It cannot be benchmarked head-to-head against rvt-rs
  without running the full Revit desktop application, which is
  proprietary, licensed, and Windows-only. Wall-clock parity numbers
  against that path are therefore not published here.
- **What is measurable** — file read throughput against a generic
  gzip/DEFLATE baseline. Revit streams are truncated-gzip (10-byte
  header + raw DEFLATE, no trailing CRC/ISIZE); `flate2::read::DeflateDecoder`
  handles the body. A `bench_identify` + `bench_schema` pair against a
  same-size generic `.gz` payload gives an apples-to-apples decode
  comparison. Not yet wired up — see planned suites above.
- **Open-source cross-reads** (mentioned for context; not yet
  benchmarked side-by-side in this repo):
  - [`phi-ag/rvt`](https://github.com/phi-ag/rvt) — TypeScript CFB parser on Node, metadata-only.
  - [Apache Tika](https://tika.apache.org/) — Java metadata reader; JVM warmup dominates cost.
  - [`chuongmep/revit-extractor`](https://github.com/chuongmep/revit-extractor) — subprocess wrapper around Autodesk's extractor; requires Revit install.
  - ODA BimRv SDK — commercial, licensing precludes redistribution of timings.

## How to run

### Today (hyperfine, CLI-level)

```bash
# From crate root
cargo build --release
./tools/bench.sh
```

The script expects the phi-ag corpus at
`../../samples/_phiag/examples/Autodesk/`. Override with
`RVT_SAMPLES_DIR=/path/to/samples ./tools/bench.sh`.

### Planned (criterion, library-level)

Once the `benches/` suites are wired up:

```bash
# Run all planned benchmark suites
cargo bench --all

# Run a single suite
cargo bench --bench bench_schema
```

The `benches/` directory does not yet exist — it will be created when
the first suite lands. See [Benchmark suites planned](#benchmark-suites-planned).

## Honest limitations

- **No large-scale real-project corpus.** Every number on this page
  comes from Autodesk's ~400 KB `rac_basic_sample_family` RFA — a
  family fixture, not a full building model. A Revit project with
  hundreds of thousands of elements will stress the decoder
  differently, and none of the published timings extrapolate linearly.
  Medium (1-50 MB) and large (50 MB+) fixtures are not in the corpus
  yet.
- **Synthetic fixtures for per-class decode.** The 29 Layer 5b decoders
  are validated against synthesized schema+bytes fixtures (same harness
  as the class-decoder unit tests). Real-file corpus validation is open
  work — see Q-01 in the TODO. `bench_decode_class` numbers will carry
  the same caveat.
- **Timings will change as decoder coverage expands.** The current
  CLI-level numbers include no geometry extraction (Phase 5), no
  per-element materials, and no `IfcShapeRepresentation` emission.
  Adding those will raise per-file cost. Treat the shipped numbers as
  measurements of the current library surface, not as a stable floor.
- **Cold vs hot cache.** hyperfine defaults to warm-cache runs after
  1-2 warmups. First-open-from-disk cost is not separately reported.
- **No allocation profiling.** The `Allocations` column in the planned
  results table will require `criterion-perf-events` or a bespoke
  `GlobalAlloc` wrapper. Not yet wired up.
- **No cross-platform numbers.** Only Apple Silicon (M2 Max) is
  measured today. x86_64 Linux and Windows timings are expected to
  differ.

## Raw data

- Markdown table: [`docs/data/bench-2024.md`](data/bench-2024.md)
- Per-run CSV: [`docs/data/bench-2024.csv`](data/bench-2024.csv)
- Cross-version CSV: [`docs/data/bench-versions.csv`](data/bench-versions.csv)

Regenerate on any non-trivial perf change.
