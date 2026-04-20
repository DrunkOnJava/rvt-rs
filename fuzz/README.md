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
- **cargo-fuzz itself:**
  ```
  cargo install cargo-fuzz
  ```

On macOS you may also need `xcode-select --install` for the
linker. On Linux, `clang` is recommended for sanitizer support.

## Layout

```
fuzz/
  Cargo.toml         # standalone fuzz crate, own [workspace] root
  .gitignore         # target/, corpus/, artifacts/, coverage/
  src/lib.rs         # empty placeholder — Cargo refuses to parse a
                     #   manifest with zero targets; delete when the
                     #   first fuzz_targets/<name>.rs + [[bin]] lands
  fuzz_targets/      # one .rs per libFuzzer target (currently empty)
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
(With no targets defined yet, this prints nothing — that is the
expected state while the scaffold lands.)

Run a target (once targets are added):
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

## Planned targets (SEC-15 through SEC-23)

The fuzz targets themselves are tracked as separate tasks. This
scaffold is SEC-14; the per-target implementations are SEC-15..23
covering (among others):

- `fuzz_open_bytes` — `RevitFile::open` on arbitrary bytes
- `fuzz_gzip_header_len` — truncated-gzip header probe
- `fuzz_inflate_at_with_limits` — bounded inflate against bomb inputs
- `fuzz_find_chunks` — chunk scanner
- `fuzz_basic_file_info` — BasicFileInfo parser
- `fuzz_part_atom` — PartAtom XML surface
- `fuzz_walker_entry_detect` — Layer 5a walker entry-point detector
- `fuzz_parse_schema` — schema field-type decoder
- `fuzz_step_writer` — IFC STEP emission (output shape stability)

See the `TODO-BLINDSIDE.md` / `ROADMAP.md` trail and each SEC-NN
task description for scope of the individual targets.

## Related

- Crash corpora accumulated from CI will eventually be committed
  as regression inputs under `fuzz/corpus/<target>/` — see Q-04 in
  the quality-bar task list for that follow-up.
- A nightly `cargo-fuzz` GitHub Action that runs each target for a
  bounded budget is tracked under SEC-25.
