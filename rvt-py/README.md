# rvt-py

Python bindings for the [`rvt`](https://crates.io/crates/rvt) crate,
via [`pyo3`](https://github.com/PyO3/pyo3) and
[`maturin`](https://github.com/PyO3/maturin).

## Users

You don't install this crate directly. The PyPI wheel is named
`rvt`, not `rvt-py`:

```bash
pip install rvt
```

Python usage docs: [`docs/python.md`](../docs/python.md).

## Why it's a separate crate

The core [`rvt`](https://crates.io/crates/rvt) library is
unconditionally `#![forbid(unsafe_code)]` — Revit files come from
untrusted sources, and every `unsafe` block is a potential parser
vulnerability. pyo3's macros unavoidably expand into `unsafe impl`
/ `unsafe fn` blocks, which is incompatible with `forbid`.

Splitting the Python bindings into this member crate keeps the
core's security boundary hard. See
[`docs/decisions/ADR-001-workspace-split-for-pyo3.md`](../docs/decisions/ADR-001-workspace-split-for-pyo3.md).

## Building from source

```bash
# Debug build — fast iteration, installs into the active venv.
maturin develop --manifest-path rvt-py/Cargo.toml

# Release wheel — what CI publishes to PyPI.
maturin build --release --manifest-path rvt-py/Cargo.toml --out dist
```

Requires Python ≥ 3.8. The abi3 wheel from `--release` covers
every minor version on that platform with a single file.

## Licence

Apache-2.0. Same licence as the core `rvt` crate. See
[`LICENSE`](../LICENSE) at the repo root.
