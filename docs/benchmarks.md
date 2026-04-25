# rvt-rs benchmarks

Performance measurement methodology, `hyperfine` CLI-level timings, and
four `criterion` library-level suites. The suites cover the inner-loop
paths every CLI hits (compression, BasicFileInfo, schema) plus a
multi-megabyte project-file suite (Q-07) that times open/summarize/
schema/elem_table/walker against 913 KB and 34 MB real `.rvt` files.

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
- **Library microbench tool:** `criterion`, with compile coverage in
  normal CI and runtime budget enforcement in the scheduled
  performance workflow.
- **Fixture size categories:**
  - **small** — under 1 MB. Reference: `racbasicsamplefamily-2024.rfa`
    at ~400 KB. Currently the only corpus measured.
  - **medium** — 1–50 MB. No fixture in corpus yet.
  - **large** — 50 MB and up. No fixture in corpus yet.
- **Corpus:** git-LFS-pulled [`phi-ag/rvt`](https://github.com/phi-ag/rvt)
  sample set, 11 Revit releases 2016-2026. All small-category files.

## Criterion suites

Four `criterion` suites under `benches/`. Run with `cargo bench --bench
<name>`; overall `cargo bench` runs all four.

| Suite | What it measures | Fixture |
|---|---|---|
| `compression` | Raw inflate throughput on synthetic gzip chunks. Isolates `compression::inflate_at` from any surrounding CFB / schema work. | In-memory |
| `basic_file_info` | UTF-16LE key-value parse on a realistic BasicFileInfo buffer. Hot path for `rvt-info` / `rvt-history`. | In-memory synthesized |
| `schema` | Floor-time timing for `formats::parse_schema` on an empty input. Richer real-schema timings are in the `project_file` suite below. | In-memory |
| `project_file` | Open / summarize / schema-parse / elem-table records / ADocument walker on 913 KB and 34 MB real `.rvt` project files. Skips when the corpus isn't present. | magnetar-io/revit-test-datasets (LFS) |

## Runtime Budget Gate

`tools/perf_budget.py` is the machine-readable runtime and memory
budget gate. It builds `examples/perf_budget_op.rs`, runs each operation
in a fresh process, records wall time plus peak RSS when the platform
exposes it through `/usr/bin/time`, and writes JSON suitable for CI
artifacts or trend ingestion.

```bash
python3 tools/perf_budget.py \
  --enforce \
  --require-category small \
  --json-out target/perf-budgets/local.json
```

Budget targets live in [`tools/perf-budgets.json`](../tools/perf-budgets.json).
The harness tracks:

| Operation | What runs |
|---|---|
| `open` | `RevitFile::open` and stream-directory enumeration |
| `summarize` | Strict summary path: metadata, stream inventory, and schema status |
| `schema_parse` | `Formats/Latest` read, inflate, and `formats::parse_schema` |
| `element_decode` | Production `walker::iter_elements` path |
| `ifc_export` | `RvtDocExporter` plus STEP serialization |
| `viewer_parse_render` | Native proxy for the viewer worker: export, scene graph, schedule, and glTF bytes |

Fixture categories:

| Category | Default source | CI status |
|---|---|---|
| `small` | phi-ag/rvt `racbasicsamplefamily-2024.rfa` | Required in the scheduled workflow |
| `medium` | magnetar-io/revit-test-datasets `2024_Core_Interior.rvt` | Required in the scheduled workflow |
| `large` | `RVT_PERF_LARGE_FILE` | Tracked in config; skipped until a redistributable >=50 MiB fixture is available |

The scheduled/manual `Performance Budgets` workflow enforces small and
medium categories weekly and uploads `target/perf-budgets/perf-budget.json`.
It is intentionally not a push-required check: shared GitHub runners are
noisy, so wall-clock regression gates are useful as a trend signal and
nightly guard, while ordinary CI keeps the cheaper `cargo bench --no-run`
compile gate on every code push.

### Multi-megabyte results (Q-07)

`cargo bench --bench project_file -- --quick --warm-up-time 1
--measurement-time 3` on Apple M2 Max, Rust release profile (`lto =
true, codegen-units = 1`), macOS 14:

| Operation | 913 KB (2023 project) | 34 MB (2024 project) | Scaling |
|---|---:|---:|---:|
| `RevitFile::open` | 67.9 µs | 3.65 ms | 53× (matches file-size ratio of ~37×, I/O-bound) |
| `summarize_strict` | 5.08 ms | 8.03 ms | 1.58× (sub-linear — schema is invariant) |
| `parse_schema` | 287 µs | 275 µs | ≈ constant (schema size doesn't scale) |
| `elem_table::parse_records` | 232 µs | 5.61 ms | 24× (tracks record count 2614 → 26,425) |
| `walker::read_adocument_lossy` | 22.9 ms | 31.0 ms | 1.35× (ADocument entry is small region) |

Takeaways:

- **Full summary in 8 ms on a 34 MB project** — rvt-rs is viable for
  interactive/IDE-speed workflows on realistic project files, not just
  family samples.
- **Schema parse is constant** across the two files because Revit ships
  the same schema bytes for a given release (see the "17,266 bytes
  byte-identical across family/project" finding in
  [`docs/project-file-corpus-probe-2026-04-21.md`](project-file-corpus-probe-2026-04-21.md)).
- **ElemTable enumeration is the sub-linear bottleneck** — 5.6 ms for
  26,425 records ≈ 212 ns per record, dominated by bounds checks and
  vector allocation. Fine for batch work; would need a streaming iterator
  if someone wanted to ship real-time parsing of million-element files.
- **Walker read_adocument_lossy ≈ 30 ms** holds flat because the ADocument
  record is the same 13-field shape regardless of project size —
  walking to it takes the same time once the stream directory is loaded.

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

### Criterion (library-level)

```bash
# Run every criterion suite
cargo bench

# Run a single suite
cargo bench --bench compression
cargo bench --bench basic_file_info
cargo bench --bench schema

# Q-07 multi-MB project file suite (skips if corpus absent;
# override path with RVT_PROJECT_CORPUS_DIR)
cargo bench --bench project_file
```

The `project_file` suite requires the magnetar-io/revit-test-datasets
corpus (MIT, git-LFS); without it, each sub-benchmark emits a
`skipping: … not present` message and exits green.

## Honest limitations

- **Medium-corpus coverage but no large-corpus coverage yet.** The
  project_file suite covers 913 KB and 34 MB project files. 50 MB+ fixtures
  remain open; contributions welcome. See Q-01 in the TODO.
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
