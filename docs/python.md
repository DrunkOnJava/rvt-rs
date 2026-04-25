# rvt-rs — Python bindings (v0.1.2)

`rvt` is a Python package built on top of the Rust `rvt` crate. It
exposes the file reader, metadata getters, schema introspection,
Layer-5a walker, and document-level IFC4 STEP exporter from the
Rust core, without requiring a Rust toolchain at install time. The
Python surface is narrower than the Rust surface — there is one
class (`RevitFile`) and one module-level helper (`rvt_to_ifc`),
both documented in full below.

Single source of truth for the runtime surface is
[`src/python.rs`](../src/python.rs); the hand-maintained type stubs
at [`python/rvt/__init__.pyi`](../python/rvt/__init__.pyi) mirror
it for mypy / pyright.

## Install

For all installation paths, including release smoke tests, see
[`install.md`](install.md).

### From PyPI

```bash
pip install rvt
```

Wheels target Python ≥ 3.8 via pyo3's `abi3-py38` feature, so one
wheel per OS/architecture covers every supported Python minor. The
PyPI release workflow (issue #89) publishes wheels on tag.

### From source

```bash
# Needs a Rust toolchain (>= 1.85) and maturin.
pip install maturin
git clone https://github.com/DrunkOnJava/rvt-rs
cd rvt-rs
maturin build --release --manifest-path rvt-py/Cargo.toml
pip install target/wheels/rvt-*.whl
```

For day-to-day development, `maturin develop --manifest-path rvt-py/Cargo.toml`
installs the extension in editable mode into the active virtualenv.

## Quick start

```python
import rvt

f = rvt.RevitFile("my-family.rfa")

# BasicFileInfo getters — each a cheap parse of the BasicFileInfo
# stream, cached by the Rust reader.
print(f.version)              # 2024
print(f.build)                # "20230308_1635(x64)"
print(f.guid)                 # "7f3c..." or None
print(f.original_path)        # save-time path from creator's machine
print(f.part_atom_title)      # e.g. "0610 x 0915mm" on family files

# Stream enumeration.
print(f.stream_names())       # 13 entries on the reference corpus
print(f.missing_required_streams())  # [] on a valid Revit file

# Schema counts (cheap).
summary = f.schema_summary()
print(summary["classes"], summary["fields"], summary["cpp_types"])
# 395 13570 ...   on the 2024 sample

# Full schema as JSON (~1-2 MB on typical families).
import json
schema = json.loads(f.schema_json())
adoc = next(c for c in schema["classes"] if c["name"] == "ADocument")

# Layer-5a walker — ADocument instance fields. Returns None when
# the entry-point detector can't confidently locate ADocument.
doc = f.read_adocument()
if doc is not None:
    for field in doc["fields"][-3:]:
        print(field["name"], "→", field["kind"], field.get("id"))
    # m_ownerFamilyId                 → element_id 27
    # m_ownerFamilyContainingGroupId  → element_id 31
    # m_devBranchInfo                 → element_id 35

# Document-level IFC4 STEP export.
with open("my-family.ifc", "w") as out:
    out.write(f.write_ifc())
```

## API reference

Every signature below is verified against
[`src/python.rs`](../src/python.rs) at v0.1.2.

### Module-level

```python
rvt.__version__  # str — same as the Rust crate version (Cargo.toml)

rvt.rvt_to_ifc(path: str, mode: str = "scaffold") -> str
rvt.rvt_to_ifc_diagnostics(path: str) -> str
```

`rvt_to_ifc(path, mode="scaffold")` opens the file, runs the
document-level IFC4 exporter (`ifc::RvtDocExporter`), and returns the
IFC4 STEP text. Equivalent to
`rvt.RevitFile(path).write_ifc(mode=mode)`. Raises `IOError` on open
failure, `ValueError` if the file parses as CFB but the exporter
can't build a model or the requested export mode cannot be satisfied.

`rvt_to_ifc_diagnostics(path)` returns the JSON diagnostics sidecar
for the same export path. The schema matches `rvt-ifc --diagnostics`
and is documented in [`docs/export-diagnostics.md`](export-diagnostics.md).
The terms used in those diagnostics are defined in
[`docs/diagnostic-semantics.md`](diagnostic-semantics.md).

### `rvt.RevitFile`

#### Constructor

```python
rvt.RevitFile(
    path: str,
    max_file_bytes: int | None = None,
    max_stream_bytes: int | None = None,
    max_inflate_bytes: int | None = None,
    max_walker_scan_bytes: int | None = None,
    max_walker_candidates: int | None = None,
    max_walker_trial_offsets: int | None = None,
    max_walker_record_decode_bytes: int | None = None,
    max_walker_container_records: int | None = None,
) -> RevitFile
```

Opens a Revit file (`.rvt`, `.rfa`, `.rte`, or `.rft` — all four
share the same CFB container). Dispatch is purely on the CFB magic
`D0 CF 11 E0 A1 B1 1A E1`, so the extension is advisory.

Resource limits map directly to the Rust `OpenLimits` /
`InflateLimits` structs:

| Parameter | Default | Meaning |
|---|---|---|
| `max_file_bytes` | 2 GiB (`2 * 1024³`) | File size cap, enforced before reading |
| `max_stream_bytes` | 256 MiB (`256 * 1024²`) | Per-stream size cap in `read_stream` |
| `max_inflate_bytes` | 256 MiB (`256 * 1024²`) | Decompressed output cap per inflate call |
| `max_walker_scan_bytes` | 128 MiB (`128 * 1024²`) | Decompressed `Global/Latest` bytes scanned by walker fallback paths |
| `max_walker_candidates` | 100,000 | Maximum schema-scan candidates retained |
| `max_walker_trial_offsets` | 16,000,000 | Maximum trial decodes attempted by walker scans |
| `max_walker_record_decode_bytes` | 1 MiB (`1024²`) | Maximum bytes inspected while decoding one walker candidate |
| `max_walker_container_records` | 1,000 | Maximum reference-container records accepted by walker decoders |

Passing `None` for any parameter keeps the Rust-side default.
Hostile input that would otherwise force multi-GB allocations is
rejected up-front.

Raises `IOError` on missing files, non-CFB input, file-size over
`max_file_bytes`, or read errors.

#### Properties (read-only)

| Property | Type | Description |
|---|---|---|
| `version` | `int \| None` | Revit release year (e.g. `2024`), from `BasicFileInfo` |
| `original_path` | `str \| None` | Save-time path on the creator's machine |
| `build` | `str \| None` | Revit build tag, e.g. `"20230308_1635(x64)"` |
| `guid` | `str \| None` | Document GUID from `BasicFileInfo`, if present |
| `part_atom_title` | `str \| None` | Family document title from `PartAtom` XML |

Each getter calls into the Rust parser on access. All four
`BasicFileInfo`-backed getters return `None` if the
`BasicFileInfo` stream fails to parse; `part_atom_title` returns
`None` when the file carries no `PartAtom` stream (common on
project `.rvt` files).

#### Methods

```python
stream_names(self) -> list[str]
```
All OLE stream paths in the file, sorted, `/`-separated
regardless of host OS. The reference corpus has 13 entries.

```python
read_stream(self, name: str) -> bytes
```
Raw bytes of the named OLE stream. Accepts either leading-slash
(`"/Formats/Latest"`) or bare (`"Formats/Latest"`) forms — both
resolve the same way. Returns *compressed* bytes on streams that
use truncated-gzip framing (most Revit streams do); callers who
want decompressed content need the Rust-side
`compression::inflate_at` equivalent, which is not yet exposed to
Python. Subject to `max_stream_bytes`. Raises `IOError` on
unknown stream names.

```python
missing_required_streams(self) -> list[str]
```
Required Revit stream names that are absent. Empty list on a
valid Revit file. Useful for pre-validating an input before
running heavy extractors.

```python
basic_file_info_json(self) -> str | None
```
Full `BasicFileInfo` as a JSON string (parseable via
`json.loads`). Single-call equivalent of the four individual
getters plus any future fields added to the Rust `BasicFileInfo`
struct. Returns `None` if the `BasicFileInfo` stream can't be
parsed.

```python
part_atom_json(self) -> str | None
```
Full `PartAtom` as a JSON string. Superset of the
`part_atom_title` getter — also includes `id`, `updated`,
`taxonomies`, `categories`, `omniclass`, and `raw_xml` (the
original XML bytes for lossless downstream reuse). Returns `None`
if the file has no `PartAtom` stream.

```python
schema_summary(self) -> dict[str, int]
```
Decoded schema counts. Cheap. Keys: `classes`, `fields`,
`cpp_types` (all `int`).

```python
schema_json(self) -> str
```
Full schema as a JSON string. The Rust-side `SchemaTable` type
derives `Serialize`, so the call is effectively zero-copy
relative to the in-memory schema. Parse with `json.loads()` to
get a structured dict.

Return shape after `json.loads`:

```python
{
    "classes": [
        {
            "name": "ADocument",
            "offset": int,
            "fields": [
                {"name": "m_elemTable", "cpp_type": "...",
                 "field_type": {"ElementId": null}},
                ...
            ],
            "tag": 4,
            "parent": None,
            "declared_field_count": 13,
            "was_parent_only": False,
            "ancestor_tag": None,
        },
        ...
    ],
    "cpp_types": ["ElementId", "std::pair< ElementId, double >", ...],
    "skipped_records": 0,
}
```

Typical reference-corpus schema is ~395 classes / ~13,570 fields
— the JSON string is on the order of 1-2 MB. For counts only,
prefer `schema_summary()`.

```python
read_adocument(self) -> dict | None
```
Run the Layer-5a walker and return `ADocument`'s instance fields
as a dict, or `None` if the entry-point detector can't
confidently locate an `ADocument` record in this file.

Return shape when present:

```python
{
    "entry_offset": int,   # byte offset in decompressed Global/Latest
    "version": int,        # Revit release year
    "fields": [
        {"name": "m_elemTable", "kind": "pointer",
         "slot_a": 0, "slot_b": 0},
        {"name": "m_appInfoArr", "kind": "ref_container",
         "count": 12, "col_a": [...], "col_b": [...]},
        {"name": "m_ownerFamilyId", "kind": "element_id",
         "tag": 0, "id": 27},
        # ... 10 more fields
    ],
}
```

Field kinds:

| `kind` | Extra keys | Meaning |
|---|---|---|
| `pointer` | `slot_a: int`, `slot_b: int` | Two u32 slots (raw container pointer) |
| `element_id` | `tag: int`, `id: int` | Tagged ElementId; `id` is the runtime value |
| `ref_container` | `count: int`, `col_a: list[int]`, `col_b: list[int]` | Two-column reference container (u16 per cell) |
| `bytes` | `len: int` | Raw bytes; field type not yet decoded |

```python
write_ifc(self, mode: str = "scaffold") -> str
export_diagnostics_json(self) -> str
```
Produce an IFC4 STEP string via `ifc::RvtDocExporter`. This is
document-level export: project name, description, units,
classifications — not per-element geometry. Raises `ValueError`
if the file can't be parsed far enough to build a model.

`mode` is one of `scaffold`, `typed-no-geometry`, `geometry`, or
`strict`. `scaffold` accepts the historical spec-valid framework
output; stronger modes fail loudly when the recovered Revit data does
not meet that export quality.

`scaffold` means the export envelope succeeded. It can still be a partial
conversion failure if `export_diagnostics_json()` reports zero validated
building elements or zero geometry elements. Use `mode="strict"` in automation
when an incomplete real-model export should raise `ValueError` instead of
returning IFC text.

`export_diagnostics_json()` returns the JSON diagnostics sidecar
for the default IFC export without writing files.

```python
__repr__(self) -> str
```
Returns `"RevitFile(version=2024)"` or `"RevitFile(version=?)"`
if `BasicFileInfo` can't be parsed.

### Errors

| Exception | Raised when |
|---|---|
| `IOError` | File missing, non-CFB input, file-size over `max_file_bytes`, unknown stream name, read error |
| `ValueError` | File parsed as CFB but IFC export couldn't build a model |

`IOError` is the Python alias of `OSError` — both are caught by
`except OSError` or `except IOError`.

## Parse-safety controls

The constructor accepts optional `Option<u64/usize>` caps
that pass through to the Rust `OpenLimits` / `InflateLimits`
structs defined in [`src/reader.rs`](../src/reader.rs) and
[`src/compression.rs`](../src/compression.rs), plus walker scan
caps from [`src/walker.rs`](../src/walker.rs):

- `max_file_bytes` (default 2 GiB) — enforced with a `stat`
  pre-check; over-sized files error before any read.
- `max_stream_bytes` (default 256 MiB) — enforced per
  `read_stream` call.
- `max_inflate_bytes` (default 256 MiB) — enforced per inflate
  call in the truncated-gzip decompressor; protects against
  compressed-bomb input.
- `max_walker_scan_bytes`, `max_walker_candidates`,
  `max_walker_trial_offsets`, `max_walker_record_decode_bytes`,
  and `max_walker_container_records` — bound schema-directed
  fallback scans and return warnings in lossy diagnostics when hit.

All three defaults are conservative for desktop use. Tighten them
for untrusted input:

```python
# Reject anything over 100 MiB; cap inflate at 64 MiB.
f = rvt.RevitFile(
    untrusted_path,
    max_file_bytes=100 * 1024 * 1024,
    max_stream_bytes=100 * 1024 * 1024,
    max_inflate_bytes=64 * 1024 * 1024,
    max_walker_scan_bytes=16 * 1024 * 1024,
)
```

A lower-level `InflateLimits` binding is not yet exposed — if you
call `read_stream` and want to decompress the returned bytes, the
only caps applied are the constructor-level ones. Issue #152 /
#149 track tightening the Python-side limit surface.

## Version support

The Python bindings inherit the Rust core's version support
matrix verbatim:

| Revit release | Open + metadata + schema | Walker (ADocument) | IFC4 document export |
|---|---|---|---|
| 2016 – 2023 | yes | `None` (entry-point detector: partial) | yes |
| 2024 – 2026 | yes | full | yes |

Full matrix and per-column definitions are in
[`docs/compatibility.md`](./compatibility.md). The "walker
partial" rows are tracked as L5B-11 in
[`TODO-BLINDSIDE.md`](../../../TODO-BLINDSIDE.md); expanding to
2016–2023 requires per-version entry-point heuristics and is
active work, not shipped.

File types: `.rvt`, `.rfa`, `.rte`, `.rft` all read through the
same code path (CFB magic dispatch). The reference corpus is
entirely `.rfa`; project/template fixtures are pending (Q-01).

## What isn't exposed yet

The Rust crate is broader than these bindings. The Python surface
intentionally stays minimal; if you need more, either file an
issue or drop to the Rust CLIs (`rvt-info`, `rvt-analyze`,
`rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`,
`rvt-doc`, `rvt-ifc`).

Not in Python today:

- **Per-element decoders.** The walker reads `ADocument`; the 54
  Layer-5b element decoders (`Wall`, `Floor`, `Door`, etc. — see
  `compatibility.md` §3) aren't yet exposed to Python. `rvt-doc`
  and the Rust `elements::all_decoders()` API cover this.
- **Per-element IFC export.** The `write_ifc()` method uses
  `RvtDocExporter` — document-level only (project, units,
  classifications). The Rust crate's per-element mappings
  (`IfcWall` / `IfcDoor` / `IfcSlab` / etc., also in
  `compatibility.md` §4) are driven by internal exporter types
  not yet surfaced through pyo3. The `rvt-ifc` CLI runs the full
  pipeline.
- **Decompression helper.** `read_stream` returns compressed
  bytes on truncated-gzip streams. The Rust
  `compression::inflate_at_with_limits` function isn't bound yet.
- **History / diff.** The `rvt-history` and `rvt-diff` CLIs have
  no Python equivalent.
- **Writing Revit files.** The Rust crate's byte-preserving
  writer (`writer.rs`) is not exposed. Open an issue if you have
  a round-trip use case.
- **Streaming large files.** The entire file is read into memory.
  There is no chunked / streaming reader.
- **Strict-mode diagnostics.** `basic_file_info_json`,
  `part_atom_json`, `read_adocument`, and `schema_json` all fall
  back silently on parse errors (returning `None` or erroring
  respectively). A `*_strict` variant surface that accumulates
  per-field diagnostics is tracked in API-14 / API-15 / API-16.

## Troubleshooting

### `ImportError: cannot import name 'rvt'`

Likely an older / different `rvt` package in the environment. Run
`pip uninstall rvt` and reinstall.

### `IOError: NotACfbFile`

The file doesn't start with OLE2 magic bytes
(`D0 CF 11 E0 A1 B1 1A E1`). Likely causes: it's not a Revit file,
it's a zero-byte Git LFS placeholder, or a transfer layer mangled
the bytes (e.g. line-ending conversion).

### `IOError: FileTooLarge`

The file exceeds `max_file_bytes` (default 2 GiB). Either pass a
larger cap explicitly in the constructor or verify the file isn't
corrupt.

### `read_adocument()` returns `None`

The Layer-5a walker's entry-point detector didn't find an
`ADocument` record. Expected on Revit 2016–2023 files (see version
support matrix). For 2024–2026 files this is unexpected on the
11-release reference corpus — all samples resolve — but may occur
on exotic layouts. File an issue with the Revit release year and
`f.stream_names()` output.

### Wheel won't build from source

`maturin build --manifest-path rvt-py/Cargo.toml` needs:

- Rust ≥ 1.85
- Python development headers (`python3-dev` on Linux, Xcode
  command-line tools on macOS, python.org Python on Windows)

On Apple Silicon, verify `pip install maturin` resolved a
`universal2` or `arm64` wheel, not a stale `x86_64` one.

## Contributing

See [`CONTRIBUTING.md`](../CONTRIBUTING.md). Python-specific
changes land in:

- [`src/python.rs`](../src/python.rs) — the pyo3 binding layer
- [`python/rvt/__init__.pyi`](../python/rvt/__init__.pyi) — type
  stubs (keep in sync)
- [`tests/python/test_rvt.py`](../tests/python/test_rvt.py) —
  pytest coverage against the reference corpus

Do not add Rust code solely to expose something to Python — wire
through the existing Rust public API where possible.
