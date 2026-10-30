# rvt

Read Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) in Python, without Revit: a file's metadata, previews and embedded schema and, on the Revit releases it supports, the building's elements with their names, types, storeys and materials, exported as IFC4 or as CSV schedules. `rvt` is the Python package of [rvt-rs](https://github.com/DrunkOnJava/rvt-rs), an Apache-2.0 clean-room reader written in Rust.

```bash
pip install rvt
```

One wheel per platform covers Python 3.8 and later on Linux (x86-64 and ARM64, glibc and musl), macOS (Apple silicon and Intel) and 64-bit Windows, with no Rust toolchain needed. On any other platform pip builds the package from source, which needs a Rust toolchain.

## Example

```python
import rvt

# Identity only (release, last save, worksharing): reads two small streams,
# so it is the fast path for an inventory of many files.
meta = rvt.read_metadata("model.rvt")
print(meta["revit_version"], meta["last_saved"])

f = rvt.RevitFile("model.rvt")
counts = f.element_counts()
print(counts["total"], counts["by_class"])

# write_ifc() raises ValueError when part of the file was over the reader's
# resource limits (f.unread_streams() names it): open the file again with a
# larger max_stream_bytes / max_inflate_bytes, or pass allow_partial=True.
with open("model.ifc", "w", encoding="utf-8") as out:
    out.write(f.write_ifc())
# The CSV already ends its rows with \r\n: write it with newline="".
with open("elements.csv", "w", encoding="utf-8", newline="") as out:
    out.write(f.schedule_csv("elements"))

# What the export approximated or left out, as `rvt-ifc --diagnostics`
# writes it: read it before relying on an export.
diagnostics = f.export_diagnostics()
```

## What to expect

Support depends on the Revit release, the file type and the feature. Opening a file and reading its metadata does not mean its elements or geometry can be recovered. `RevitFile.decoded_elements()` lists each element's saved parameters on Revit 2023 to 2027 files, and experimentally on 2015 to 2022 ones; the CLI `rvt-ifc --revit-property-sets` writes them into IFC exports as Revit's property sets, and a default export leaves most of them out. What each release gives is in the [support table](https://github.com/DrunkOnJava/rvt-rs#what-you-can-rely-on-by-input), and every export comes with the diagnostics above.

## Documentation

- [Python guide](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/python.md): every function and method, the errors they raise, and the parse-safety limits for untrusted files.
- [Export diagnostics](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/export-diagnostics.md): the fields of the diagnostics and what they mean.
- [Changelog](https://github.com/DrunkOnJava/rvt-rs/blob/main/CHANGELOG.md): what each release changed.
- [Discussions](https://github.com/DrunkOnJava/rvt-rs/discussions) for questions, and [issues](https://github.com/DrunkOnJava/rvt-rs/issues) for problems.

## Why the bindings are a crate of their own

This directory, `rvt-py`, is the pyo3 extension the `rvt` wheel is built from. The core [`rvt`](https://crates.io/crates/rvt) crate is unconditionally `#![forbid(unsafe_code)]`: Revit files come from untrusted sources, and every `unsafe` block is a potential parser vulnerability. pyo3's macros expand into `unsafe` code, which `forbid` does not allow, so the bindings live here and the core's boundary stays hard ([ADR-001](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/decisions/ADR-001-workspace-split-for-pyo3.md)).

## Building from source

```bash
# Debug build into the active virtualenv, for day-to-day work.
maturin develop --manifest-path rvt-py/Cargo.toml

# The release wheel CI publishes to PyPI.
maturin build --release --manifest-path rvt-py/Cargo.toml --out dist
```

## Licence and trademarks

Apache-2.0, the same licence as the core crate ([LICENSE](https://github.com/DrunkOnJava/rvt-rs/blob/main/LICENSE), [NOTICE](https://github.com/DrunkOnJava/rvt-rs/blob/main/NOTICE)). "Autodesk" and "Revit" are registered trademarks of Autodesk, Inc. rvt-rs is an independent project, not affiliated with, endorsed by or sponsored by Autodesk; it uses the names only to identify the file format it reads, and contains no Autodesk source code ([CLEANROOM.md](https://github.com/DrunkOnJava/rvt-rs/blob/main/CLEANROOM.md)).
