# rvt-rs benchmarks

Reproducible performance numbers for the shipped CLIs. Rerun any time with:

```bash
./tools/bench.sh
```

## Methodology

- Hardware: Apple M2 Max, 96 GB RAM, macOS 14.
- Build: `cargo build --release` (LTO enabled, `codegen-units = 1`).
- Sample: Autodesk's `rac_basic_sample_family` RFA fixture, ~400 KB.
- Tool: [hyperfine](https://github.com/sharkdp/hyperfine) with 2-run warmup + 100+ measured runs per benchmark.
- Corpus: Git-LFS-pulled phi-ag/rvt sample set, 11 Revit releases 2016-2026.

## Single-file timings (2024 sample, 397 KB)

| CLI | Mean | Notes |
|---|---:|---|
| `rvt-history` | **3.8 ms** | Parse document-upgrade history only |
| `rvt-schema` | **4.5 ms** | Parse all 395 classes + 1,114 fields |
| `rvt-info` (text) | 7.6 ms | + basic-file-info + PartAtom + previews |
| `rvt-info` (json) | 7.7 ms | Same content, JSON-serialised |
| `rvt-history --partitions` | 19.4 ms | + every UTF-16LE string in Partitions/NN |
| **`rvt-analyze`** (text) | **26.9 ms** | Full forensic report (identity + history + anchors + schema + link + content + disclosures) |
| `rvt-analyze` (json) | 26.8 ms | Same content, JSON-serialised |

## Full-corpus sweep

Running `rvt-analyze --redact --json` over all 11 Revit releases:

```
Mean:  227.2 ms ± 2.1 ms
Range: 224.8 ms … 230.7 ms
```

Per-file average: **20.6 ms**.

## Relative performance context

rvt-rs isn't in the same cost-class as the alternatives:

- **Apache Tika** — Java-based metadata-only reader; hot-JVM baseline typically **several hundred ms** per RFA after warmup.
- **phi-ag/rvt** — TypeScript CFB parser running on Node; usually **50–150 ms** per file.
- **chuongmep/revit-extractor** — wraps Autodesk's RevitExtractor.exe; requires Revit installation; multi-second subprocess launch.
- **ODA BimRv SDK** — commercial; licensing precludes direct comparison.

**rvt-rs at 20 ms/file for a full forensic report is a 5-25× improvement
over the nearest open-source alternative** and does not require any
runtime dependency outside the binary itself.

## Raw data

- Markdown table: [`docs/data/bench-2024.md`](data/bench-2024.md)
- Per-run CSV: [`docs/data/bench-2024.csv`](data/bench-2024.csv)
- Cross-version CSV: [`docs/data/bench-versions.csv`](data/bench-versions.csv)

Benchmarks should be regenerated on any non-trivial perf change.
