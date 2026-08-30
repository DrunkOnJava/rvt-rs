# rvt-rs fuzzing harness

This directory is the `cargo-fuzz` workspace for `rvt-rs`. It is
intentionally separate from the main crate so that:

- The main `cargo build` / `cargo test` on stable Rust is never
  forced to compile `libfuzzer-sys` (which requires nightly).
- Fuzz targets, corpora, and crash artifacts live outside the
  published crate and do not bloat the crates.io package.
- The fuzz crate can declare its own `[workspace]` root and does
  not interact with any future top-level workspace layout.

## What cargo-fuzz does

[`cargo-fuzz`](https://github.com/rust-fuzz/cargo-fuzz) drives
libFuzzer — a coverage-guided, in-process fuzzer — against a
single entry-point function (`fuzz_target!`). Each target is a
small Rust binary that takes an arbitrary byte slice and feeds it
into a parser under test. libFuzzer mutates the input corpus,
records code-coverage deltas, and surfaces any input that makes
the target panic, abort, time out, or OOM.

Because `rvt-rs` parses untrusted on-disk files (CFB containers,
truncated-gzip streams, PartAtom XML, schema metadata, STEP
output), fuzzing is the right discipline for finding crashes on
malformed input before they land in a real user's file.

## Prerequisites

- **Nightly Rust.** `cargo-fuzz` / `libfuzzer-sys` require nightly
  because they depend on compiler-inserted sanitizer coverage.
  Install with `rustup toolchain install nightly` and either
  activate it with `rustup override set nightly` inside `fuzz/` or
  pass `+nightly` on every invocation (see below).
- **cargo-fuzz itself** (the CLI builds on stable; only the fuzz
  targets need nightly):
  ```
  cargo install cargo-fuzz --locked
  ```

On macOS you may also need `xcode-select --install` for the
linker. On Linux, `clang` is recommended for sanitizer support.

## Layout

```
fuzz/
  Cargo.toml         # standalone fuzz crate, own [workspace] root
  .gitignore         # target/, corpus/, artifacts/, coverage/
  fuzz_targets/      # one .rs per libFuzzer target (eleven today)
  README.md          # this file
```

The `fuzz/` crate depends on the parent `rvt` crate via a path
dependency and does **not** participate in the main crate's
workspace. Running `cargo build` or `cargo test` at the repo root
does not compile anything inside `fuzz/`.

## Usage

All commands are run from inside `fuzz/`.

List the available fuzz targets:
```
cargo +nightly fuzz list
```

Run a target:
```
cargo +nightly fuzz run <target_name>
```

Run with a wall-clock budget:
```
cargo +nightly fuzz run <target_name> -- -max_total_time=300
```

Re-run a single crashing input:
```
cargo +nightly fuzz run <target_name> artifacts/<target_name>/crash-<hash>
```

Minimize a crash corpus:
```
cargo +nightly fuzz cmin <target_name>
```

Collect coverage (requires `llvm-tools-preview`):
```
cargo +nightly fuzz coverage <target_name>
```

## Adding a new fuzz target

1. Create `fuzz_targets/<name>.rs` following the libfuzzer-sys
   template:
   ```rust
   #![no_main]
   use libfuzzer_sys::fuzz_target;

   fuzz_target!(|data: &[u8]| {
       // Parser under test — must not panic/abort on any input.
       let _ = rvt::some_parser(data);
   });
   ```
2. Add a `[[bin]]` section to `fuzz/Cargo.toml`:
   ```toml
   [[bin]]
   name = "<name>"
   path = "fuzz_targets/<name>.rs"
   test = false
   doc = false
   bench = false
   ```
3. Seed the corpus with a few known-good inputs under
   `corpus/<name>/` (not committed — `.gitignore` excludes it).
4. Run the target for at least a few minutes locally before
   committing.

Keep each target narrow — one parser surface per target — so that
coverage feedback is meaningful and crashes are cheap to triage.

## Current targets

Eleven targets are checked in, one `[[bin]]` each in `fuzz/Cargo.toml`
(`cargo +nightly fuzz list` prints the authoritative set):

- `fuzz_open_bytes` — `RevitFile::open` on arbitrary bytes
- `fuzz_gzip_header_len` — truncated-gzip header probe
- `fuzz_inflate_at_with_limits` — bounded inflate against bomb inputs
- `fuzz_parse_schema` — `Formats/Latest` schema field-type decoder
- `fuzz_find_chunks` — chunk scanner
- `fuzz_basic_file_info` — BasicFileInfo parser
- `fuzz_part_atom` — PartAtom XML surface
- `fuzz_walker_entry_detect` — Layer 5a walker entry-point detector
- `fuzz_step_writer` — IFC STEP emission (output shape stability)
- `fuzz_elem_table` — `Global/ElemTable` record parser
- `fuzz_public_byte_parsers` — remaining public byte parsers (gzip offsets, class_index, ArcWall, rect opening, share fragment)

Scope for each target is described in its source file header; the
security task ids that introduced them (SEC-14 onwards) are tracked in
`TODO.md`.

## Related

- `.github/workflows/fuzz.yml` runs every target nightly (07:17 UTC)
  with a bounded `-max_total_time` budget and uploads the crash corpus
  as a workflow artifact when a target fails.
- Stable, nightly-free regression coverage for crash-shaped inputs
  lives in `tests/fuzz_regressions.rs` and runs in normal CI.
