# ADR-001 — Split `pyo3` bindings into the `rvt-py` workspace member

- **Status**: Accepted (2026-04-20)
- **Tickets**: SEC-12, SEC-13
- **Author**: Griffin Long

## Context

The root `rvt` crate wants `#![forbid(unsafe_code)]` so every
committed file is mechanically prevented from introducing a raw-pointer
parser vulnerability. Revit files come from untrusted sources — every
`unsafe` block is a potential attack surface.

The Python bindings use `pyo3`, whose `#[pyclass]` / `#[pymethods]` /
`#[pyfunction]` macros expand into `unsafe impl` / `unsafe fn`
blocks. That's fundamental to how pyo3 bridges the Python C ABI, not
something an upstream fix will remove.

SEC-11 shipped the compromise:

```rust
#![cfg_attr(not(feature = "python"), forbid(unsafe_code))]
```

This gave us the strict posture on the default `cargo add rvt`
surface, but flipped to `allow` when `maturin build --features
python` ran. That soft boundary was insufficient: a future commit
could introduce `unsafe` inside the Python build path and it would
ship to PyPI without the default-build lints catching it.

## Decision

Split the repo into a two-member Cargo workspace:

```
rvt-rs/
├── Cargo.toml            ← workspace root + `rvt` package
├── src/                  ← core library (unconditional forbid)
├── rvt-py/
│   ├── Cargo.toml        ← new member, `rvt-py` package
│   └── src/lib.rs        ← pyo3 bindings (was src/python.rs)
└── pyproject.toml        ← maturin manifest-path → rvt-py/Cargo.toml
```

The root `rvt` crate is now unconditionally
`#![forbid(unsafe_code)]`. No feature flag, cfg, or build profile
can turn it off. The `rvt-py` member holds every `unsafe`
allowance pyo3 needs — and only pyo3's macro expansions, since
`rvt-py/src/lib.rs` itself contains no hand-written `unsafe`.

The wheel on PyPI remains named `rvt`. Users running `pip install
rvt` are unaffected. Users running `cargo add rvt` get the core
library with no pyo3 in the dependency tree.

## Consequences

### Positive

- Hard security boundary. The `#![forbid(unsafe_code)]` in
  `src/lib.rs` is genuinely unconditional — CI cannot build the
  core crate at all if anyone introduces `unsafe`, regardless of
  feature flags.
- `cargo doc`, `cargo test`, and `cargo clippy` on the core crate
  never pull pyo3 or link the cdylib for the extension.
- `cargo publish -p rvt` for crates.io is cleaner: the
  non-publishable rvt-py member is skipped. `rvt-py` is marked
  `publish = false` — the wheel is the distribution channel, not
  crates.io.
- Contributors grepping for `unsafe` in the core crate will find
  zero hits.

### Negative / mitigated

- **Two `Cargo.toml` files to keep in version lock-step.** The
  `rvt-py/Cargo.toml` has `rvt = { path = "..", version =
  "=0.1.2" }`, so updates to `rvt`'s version require a matching
  bump in `rvt-py`. CI's version-match check in `publish.yml`
  covers this when pushing tags; day-to-day development catches
  mismatches at `cargo check` time.
- **Root crate kept `cdylib` in `crate-type`.** wasm-pack requires
  `cdylib` to compile to `wasm32-unknown-unknown`, so we ship
  `crate-type = ["rlib", "cdylib"]`. This doesn't weaken the
  security posture — `cdylib` is a linker configuration, not a
  source-level allowance. The `#![forbid(unsafe_code)]` still
  applies under both output shapes.
- **Workflow files needed updates.** `publish.yml`, `ci.yml`,
  `pyproject.toml`, `README.md`, and `docs/python.md` all had to
  drop `--features python` in favour of `--manifest-path
  rvt-py/Cargo.toml`. One-time cost.

### Alternatives considered

1. **Keep SEC-11's `cfg_attr` approach.** Rejected because the
   security boundary is soft — a future commit under the python
   feature can legally use `unsafe`, and the lints catch it only
   on the default-build CI path.
2. **Feature-gate `unsafe_op_in_unsafe_fn` to rvt-py only.**
   Similar softness problem. The lint can be silenced inside the
   core crate at any time.
3. **Split into more members (`rvt-core`, `rvt-cli`, `rvt-py`,
   `rvt-ifc`).** Deferred. The CLI binaries are pure-safe Rust
   that consume the core library — they don't need their own
   crate. The IFC exporter is a module of the core, not a separate
   concern. Splitting for its own sake creates maintenance surface
   without a matching security or release-engineering benefit. The
   SEC-12/13 minimum split is `rvt` + `rvt-py`; further splits can
   happen later if a concrete reason emerges.

## Verification

After the split:

- `grep -r 'unsafe' src/` → zero results in source files.
- `cargo check --lib` (default features) → compiles, no pyo3 in
  the dependency tree.
- `cargo check --lib --features wasm --no-default-features` →
  compiles, builds the wasm-pack cdylib.
- `cargo check --package rvt-py` → compiles, pyo3 is the only
  source of `unsafe`.
- `cargo test --lib --quiet` → 697/697 pass.
- `maturin build --manifest-path rvt-py/Cargo.toml` → produces
  the abi3 wheel.
