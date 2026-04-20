# rvt-rs — Python bindings

`rvt` is a Python package built on top of the Rust `rvt` crate. It
exposes the same schema decoder, ADocument walker, and IFC exporter
that the `rvt-analyze` / `rvt-info` / `rvt-schema` / `rvt-doc` /
`rvt-ifc` CLIs ship, but available from a normal Python import.

The Rust core stays the source of truth — Python is a thin wrapper.
That's the same pattern Polars, Pydantic, and Ruff use, and it means
Python users get native-code speed without touching the Rust
toolchain.

## Install

### From PyPI (pending #52e publish workflow)

```bash
pip install rvt
```

Wheels ship for Python ≥ 3.8 on Linux (x86_64), macOS (arm64 +
x86_64), and Windows (x86_64). A single wheel per OS covers every
supported Python minor (pyo3's `abi3-py38` feature).

### From source

```bash
# Needs a Rust toolchain (>= 1.85) and maturin.
pip install maturin
git clone https://github.com/DrunkOnJava/rvt-rs
cd rvt-rs
maturin build --release --features python
pip install target/wheels/rvt-*.whl
```

For day-to-day development, `maturin develop --features python`
installs the wheel in editable mode in your current virtualenv.

## Quick start

```python
import rvt

f = rvt.RevitFile("my-project.rfa")

# File-level metadata (instant, no schema walk).
print(f.version)              # 2024
print(f.part_atom_title)      # "0610 x 0915mm"
print(f.build)                # "20230308_1635(x64)"
print(f.original_path)        # Windows path from the creator's machine
print(f.stream_names())       # 13 CFB streams

# Schema inventory — what classes and fields exist in this file.
summary = f.schema_summary()
print(summary["classes"])     # 395
print(summary["fields"])      # 1114

# Walker — read ADocument's instance data.
doc = f.read_adocument()
if doc is not None:
    for field in doc["fields"][-3:]:
        print(field["name"], "→", field["kind"], field.get("id"))
    # m_ownerFamilyId                 → element_id 27
    # m_ownerFamilyContainingGroupId  → element_id 31
    # m_devBranchInfo                 → element_id 35

# IFC export — produce a spec-valid IFC4 STEP file.
with open("my-project.ifc", "w") as out:
    out.write(f.write_ifc())
```

## API

All methods below are on the `rvt.RevitFile` class unless marked as
module-level.

### Constructor

```python
rvt.RevitFile(path: str) -> RevitFile
```

Opens a file. Raises `IOError` on missing file, non-CFB input, or
read errors. No size limit; the full file is kept in memory.

### Properties (read-only)

| Property | Type | Description |
|---|---|---|
| `version` | `int \| None` | Revit release year (e.g. `2024`) |
| `original_path` | `str \| None` | Save-time path from the creator's machine |
| `build` | `str \| None` | Revit build tag like `"20230308_1635(x64)"` |
| `guid` | `str \| None` | Document GUID from BasicFileInfo |
| `part_atom_title` | `str \| None` | Family document title from PartAtom XML |

### Methods

| Method | Returns | Description |
|---|---|---|
| `stream_names()` | `list[str]` | All OLE stream paths (sorted, `/`-separated) |
| `missing_required_streams()` | `list[str]` | Empty list if the file has every required stream |
| `schema_summary()` | `dict` | `{"classes": int, "fields": int, "cpp_types": int}` |
| `read_adocument()` | `dict \| None` | ADocument's instance via the Layer-5a walker |
| `write_ifc()` | `str` | IFC4 STEP text |

### `read_adocument()` return shape

```python
{
    "entry_offset": int,     # byte offset in decompressed Global/Latest
    "version": int,          # Revit release year
    "fields": [
        {"name": "m_elemTable",  "kind": "pointer",       "slot_a": 0, "slot_b": 0},
        {"name": "m_appInfoArr", "kind": "ref_container", "count": 12, "col_a": [...], "col_b": [...]},
        {"name": "m_ownerFamilyId", "kind": "element_id", "tag": 0, "id": 27},
        # ... 10 more fields
    ],
}
```

Kinds:

- `pointer` — `{slot_a: u32, slot_b: u32}`
- `element_id` — `{tag: u32, id: u32}` (`id` is the runtime ElementId)
- `ref_container` — `{count: int, col_a: [u16, ...], col_b: [u16, ...]}`
- `bytes` — `{len: int}` — raw bytes, for field types not yet decoded

**The walker is reliable on Revit 2024–2026 today.** Older releases
(2016–2023) need further entry-point detection work — identified
candidate bands but not yet validated. Tracked as `L5B-11` in
[TODO-BLINDSIDE.md](../../../TODO-BLINDSIDE.md). The hybrid
entry-point detector (heuristic first, scoring-based brute-force
fallback) is the starting point; reaching older releases is
expected to require per-version heuristics. See the recon report
§Q6.5 for the current state.

### Module-level helpers

```python
rvt.__version__       # same as the Rust crate version
rvt.rvt_to_ifc(path)  # one-shot: open + export + return IFC4 STEP text
```

### Error handling

| Raised | When |
|---|---|
| `IOError` | File missing, I/O error, non-CFB input |
| `ValueError` | File parsed as CFB but IFC export couldn't build a model |

## What the bindings don't yet cover

- **Per-element extraction** (walls, floors, families as typed Python
  objects). The walker currently reads ADocument (the root document).
  Building-element classes are walker-expansion work.
- **Streaming large files**. The full file is read into memory.
- **Writing Revit files**. The Rust crate has a byte-preserving
  round-trip writer; it's not yet exposed to Python.

## Troubleshooting

### `ImportError: cannot import name 'rvt'`

You likely installed an older / different `rvt` package. Run
`pip uninstall rvt` then reinstall.

### `IOError: NotACfbFile`

The file doesn't start with OLE2 magic bytes (`D0 CF 11 E0 A1 B1 1A
E1`). Either it's not a Revit file, it's a zero-byte placeholder
(common with Git LFS pointer files), or it's been corrupted by a
transfer layer that changed line endings.

### `read_adocument()` returns `None`

The entry-point detector couldn't locate an ADocument record in the
stream. This is unexpected on the 11-release reference corpus — all
known samples resolve — but may happen on exotic file layouts we
haven't tested. Open an issue with the file's Revit release year
and the output of `f.stream_names()`.

### The wheel won't build

`maturin build` needs:

- Rust ≥ 1.85
- A Python dev install (`python3-dev` on Linux, Xcode command-line
  tools on macOS, Python from python.org on Windows).

On Apple Silicon macOS, if you see linker errors, make sure
`pip install maturin` picked up the universal2 wheel rather than a
stale x86_64 one.

## Contributing

See [../CONTRIBUTING.md](../CONTRIBUTING.md). Python-specific
changes land under `tests/python/` for pytest coverage and
`src/python.rs` for the binding layer. The Rust crate's public
surface changes independently; if you need to expose something new
to Python, wire it through `src/python.rs` — don't add Rust code
just for Python.
