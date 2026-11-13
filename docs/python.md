# rvt-rs — Python bindings (v0.21.0)

`rvt` is a Python package built on top of the Rust `rvt` crate. It
exposes the file reader, metadata, schema introspection, the
`Global/ElemTable` index, the IFC4 exporter with its diagnostics, and
element and room schedules from the Rust core, without requiring a Rust
toolchain at install time. The Python surface is narrower than the Rust
surface: one class (`RevitFile`) and three module-level functions
(`rvt_to_ifc`, `rvt_to_ifc_diagnostics`, `read_metadata`), all documented
below.

Single source of truth for the runtime surface is
[`rvt-py/src/lib.rs`](../rvt-py/src/lib.rs); the hand-maintained type stubs
at [`python/rvt/__init__.pyi`](../python/rvt/__init__.pyi) mirror
it for mypy / pyright.

## Install

For all installation paths, including release smoke tests, see
[`install.md`](install.md).

### From PyPI

```bash
pip install rvt
```

**PyPI `rvt` 0.21.0 is the current release.** Wheels target Python ≥ 3.8
via pyo3's `abi3-py38` feature, so one wheel per OS/architecture covers
every supported Python minor. The matching Rust crate is on crates.io
(`cargo add rvt`); platforms and the other install channels are in
[`install.md`](install.md). This page describes `main`, which can be ahead
of the latest release: what has landed since 0.21.0 is in the fragments
under [`changelog.d/`](../changelog.d/README.md).

### From source

```bash
# Needs a Rust toolchain (>= 1.87) and maturin.
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
# stream, re-read and re-parsed on every call.
print(f.version)              # 2024
print(f.build)                # "20230308_1635(x64)"
print(f.guid)                 # "7f3c..." or None
print(f.original_path)        # save-time path from creator's machine
print(f.part_atom_title)      # e.g. "0610 x 0915mm" on family files

# Stream enumeration.
print(f.stream_names())       # 13 entries on the reference corpus
print(f.missing_required_streams())  # [] on the family file

# Schema counts (cheap).
summary = f.schema_summary()
print(summary["classes"], summary["fields"], summary["cpp_types"])
# 4126 11562 ...  on a Revit 2024 file

# Full schema as JSON (large: every class and field).
import json
schema = json.loads(f.schema_json())
adoc = next((c for c in schema["classes"] if c["name"] == "ADocument"), None)

# Layer-5a walker — ADocument instance fields. Returns None when
# the entry-point detector can't confidently locate ADocument. A file
# whose schema declares no ADocument (a synthetic one) raises instead,
# so ask only when `adoc` above found the class.
doc = f.read_adocument() if adoc is not None else None
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
[`rvt-py/src/lib.rs`](../rvt-py/src/lib.rs) on `main` (0.21.0).

### Module-level

```text
rvt.__version__  # str — same as the Rust crate version (Cargo.toml)

rvt.rvt_to_ifc(path: str, mode: str = "scaffold", allow_partial: bool = False, saved_meshes: bool = True, phase: str | int | None = None) -> str
rvt.rvt_to_ifc_diagnostics(path: str, saved_meshes: bool = True, phase: str | int | None = None, element_provenance: bool = False, export_decisions: bool = False) -> str
rvt.read_metadata(path: str, redact: bool = False) -> dict
```

`read_metadata(path)` returns the same document-identity dict as
`RevitFile(path).metadata()` (release, worksharing, central model path,
last saved time and user, document GUID, save counter) but reads only the
file's two identity streams, so it is the fast path for inventories:

```python
import pathlib, rvt

for path in pathlib.Path("projects").rglob("*.rvt"):
    m = rvt.read_metadata(str(path))
    print(m["revit_version"], m["worksharing"], m["last_saved"], path)
```

It raises `FileNotFoundError` / `OSError` on I/O failure and `ValueError`
when the file is not a readable Revit file. `redact=True` scrubs the user
name and Windows user folders.

`rvt_to_ifc(path, mode="scaffold", allow_partial=False, saved_meshes=True, phase=None)`
opens the file, runs the document-level IFC4 exporter
(`ifc::RvtDocExporter`), and returns the IFC4 STEP text, the same file
`rvt-ifc` writes. Equivalent to
`rvt.RevitFile(path).write_ifc(mode=mode, allow_partial=allow_partial, saved_meshes=saved_meshes, phase=phase)`.
Raises `IOError` on open failure, `ValueError` if the file parses as CFB
but the exporter can't build a model, the requested export mode cannot be
satisfied, or a resource limit kept part of the file from being read and
`allow_partial` is false. A reader that stopped at its work budget leaves out
what it would have read, and the IFC is still returned, with a
`RuntimeWarning` naming it.

`rvt_to_ifc_diagnostics(path, saved_meshes=True, phase=None, element_provenance=False, export_decisions=False)` returns the JSON
diagnostics sidecar for the same export path. The schema matches `rvt-ifc --diagnostics`
and is documented in [`docs/export-diagnostics.md`](export-diagnostics.md);
the machine-readable contract is
[`docs/schemas/export-diagnostics.schema.json`](schemas/export-diagnostics.schema.json).
The terms used in those diagnostics are defined in
[`docs/diagnostic-semantics.md`](diagnostic-semantics.md).

### `rvt.RevitFile`

#### Constructor

```text
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
| `max_inflate_bytes` | 256 MiB (`256 * 1024²`) | Decompressed output cap per inflate call; a multi-member stream such as `Partitions/NN` may inflate in total to this or, whichever is larger, 16 times its stored size, at least 1 GiB and at most 4 GiB (1 GiB on a 32-bit build) |
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

```text
stream_names(self) -> list[str]
```
All OLE stream paths in the file, sorted, `/`-separated
regardless of host OS. The reference corpus has 13 entries.

```text
unread_streams(self) -> list[dict]
```
The streams a resource limit kept from being read whole so far (#1075), each
a dict with `name`, `stored_bytes`, `kind` (`"stream_too_large"`,
`"inflated_member_too_large"` or `"inflated_total_too_large"`) and `limit`.
Empty when nothing was cut; see [Parse-safety controls](#parse-safety-controls).

```text
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

```text
missing_required_streams(self) -> list[str]
```
Required Revit stream names that are absent. Empty list on a
family file, whose 13 invariant streams are all present; a project
`.rvt` with no `PartAtom` stream lists `["PartAtom"]`. Useful for
pre-validating an input before running heavy extractors.

```text
basic_file_info_json(self) -> str | None
basic_file_info_json_strict(self) -> str
```
Full `BasicFileInfo` as a JSON string (parseable via
`json.loads`). Single-call equivalent of the four individual
getters plus any future fields added to the Rust `BasicFileInfo`
struct. `basic_file_info_json` returns `None` if the `BasicFileInfo`
stream can't be parsed; `basic_file_info_json_strict` raises
`ValueError` instead.

```text
part_atom_json(self) -> str | None
part_atom_json_strict(self) -> str
```
Full `PartAtom` as a JSON string. Superset of the
`part_atom_title` getter — also includes `id`, `updated`,
`taxonomies`, `categories`, `omniclass`, and `raw_xml` (the
original XML bytes for lossless downstream reuse). `part_atom_json`
returns `None` if the file has no `PartAtom` stream or it does not
parse; `part_atom_json_strict` raises `ValueError` naming which
(`Stream not found: PartAtom` or `Malformed PartAtom XML: ...`).

```text
metadata(self, redact: bool = False) -> dict
```
Document identity, the same dict as `rvt.read_metadata(path)`, with the keys
`revit_version`, `build`, `title`, `last_saved` (ISO 8601 UTC), `worksharing`,
`workshared`, `username`, `central_model_path`, `last_save_path`,
`document_guid`, `document_increments`, `locale`, `single_user_cloud_model` and
`properties` (every `BasicFileInfo` `Key: value` line, as `{"key", "value"}`).
`redact=True` scrubs the user name and user paths before sharing. JSON
contract: [`file-metadata.schema.json`](schemas/file-metadata.schema.json).

```text
schedule_csv(self, kind: str = "elements", metric: bool = False, excel: bool = False, phase: str | int | None = None) -> str
```
The element (`kind="elements"`) or room (`kind="rooms"`) schedule as CSV
text, with the columns of `rvt-schedule`: the elements the IFC export writes,
each on the storey it puts it on (see the [user
guide](user-guide.md#get-a-schedule-into-excel)); the room schedule lists every
`IfcSpace`, and its last column, `space_kind`, says whether each is a `room`,
an `area`, a `mep_space` or a `family_instance`. `metric=True` writes lengths
in metres, `excel=True` prefixes a UTF-8 byte-order mark for Excel on Windows.
Only decoded values are written; unknowns are empty cells. `phase` is
`write_ifc`'s, and `rvt-schedule --phase <name|id>` is its command-line twin:
the schedule is of the model the document exporter builds at that phase. Any
other `kind` raises `ValueError`.

```text
schema_summary(self) -> dict[str, int]
```
Decoded schema counts. Cheap. Keys: `classes`, `fields`,
`cpp_types` (all `int`).

```text
schema_json(self) -> str
```
Full schema as a JSON string. The Rust-side `SchemaTable` type
derives `Serialize`, so the call serialises the whole in-memory
schema into a new string. Parse with `json.loads()` to
get a structured dict.

Return shape after `json.loads`:

```text
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
    "scanned_bytes": int,
    "total_bytes": int,
    "scan_truncated": False,
}
```

The whole schema is read (#410): 4,126 classes and 11,562 fields on a
Revit 2024 file. The JSON string is large, so for counts only prefer
`schema_summary()`.

Stable JSON schemas for summary dictionaries, schema diagnostics,
decoded ADocument records, production decoded elements, element
counts, export diagnostics, and corpus reports are checked in under
[`docs/schemas/`](schemas/). Use those files for automation contracts
instead of scraping prose examples.

```text
read_adocument(self) -> dict | None
```
Run the Layer-5a walker and return `ADocument`'s instance fields
as a dict, or `None` if the entry-point detector can't confidently
locate an `ADocument` record in this file. Raises `ValueError` if the
file's schema declares no `ADocument` class, which no Revit file lacks
(a synthetic file, or a damaged schema), and `IOError` if a stream cannot be
read.

Return shape when present:

```text
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
| `integer` | `value: int`, `signed: bool`, `size: int` | Fixed-size integer; `value` is the widened 64-bit value |
| `float` | `value: float`, `size: int` | 32- or 64-bit IEEE 754 value |
| `bool` | `value: bool` | 1-byte boolean |
| `guid` | `bytes: list[int]` | 16-byte GUID / UUID |
| `string` | `value: str` | UTF-16LE length-prefixed string |
| `vector` | `len: int` | Homogeneous vector; count only |
| `bytes` | `len: int` | Raw bytes; field type not yet decoded |

```text
decoded_elements(self) -> list[dict]
element_counts(self) -> dict
export_diagnostics(self, saved_meshes: bool = True, phase: str | int | None = None, element_provenance: bool = False, export_decisions: bool = False) -> dict
```

`decoded_elements()` returns production-walker elements. Each entry
has `id`, `class_name`, `byte_range`, `fields`, `parameters` (the element's
saved parameters, as `rvt-elements` lists them; a file whose parameters
cannot be read keeps its elements and raises a `UserWarning`), `provenance`
(where the element was read from, and how confidently) and `typed`.
`typed` is the typed projection for
`Level` / `Wall` / `Floor` / `Door` / `Window` / `Room` / `Material`
(also listed as `rvt.MVP_TYPED_CLASSES`), or `None` for other classes.
JSON contract: [`decoded-elements.schema.json`](schemas/decoded-elements.schema.json).
CLI mirror: `rvt-elements file.rvt`.

`element_counts()` returns `{"total": int, "by_class": {class: count}}`.
`total` matches
`export_diagnostics()["decoded"]["production_walker_elements"]`.
JSON contract: [`element-counts.schema.json`](schemas/element-counts.schema.json).
CLI mirror: `rvt-elements file.rvt --counts`.

`export_diagnostics()` / `export_diagnostics_json()` return the IFC
export diagnostics sidecar (same payload as `rvt-ifc --diagnostics`).
`element_provenance=True` adds `decoded.element_provenance`, where each
decoded element was read from (its stream, byte offset in the inflated stream,
class and native record), as `rvt-ifc --element-provenance` does (#1325);
`rvt_to_ifc_diagnostics()` takes it too. It is off by default, as the table is large
([Provenance table size](export-diagnostics.md#provenance-table-size)). An `offset` is the frame the
exporter selected for the element, so it can name another partition than the
native record's `stream`; each element's `value_sources` name the copy a reader
took a value of its body from (#1360)
([Export diagnostics](export-diagnostics.md#cli)).
`export_decisions=True` adds `export_decisions`, one entry per element for each
fail-closed decision (why an element was left out, a body is only its record's
box, an opening was not cut, an outline was not read), bounded at 5,000 entries
a reason with the rest counted as dropped, as `rvt-ifc --export-decisions` does
(#1326). It is off by default and is a switch of its own: the log is bounded
where the provenance table is not.

```text
write_ifc(self, mode: str = "scaffold", allow_partial: bool = False, saved_meshes: bool = True, phase: str | int | None = None) -> str
export_diagnostics_json(self, saved_meshes: bool = True, phase: str | int | None = None, element_provenance: bool = False, export_decisions: bool = False) -> str
```
Produce an IFC4 STEP string via `ifc::RvtDocExporter`, the same
exporter `rvt-ifc` runs. On Revit 2023, 2024 and 2025 projects it writes
typed elements with bodies, storeys, types and materials (on
`2024_Core_Interior.rvt`, 970 elements, every one with a body), and on Revit
2015 to 2022, 2026 and 2027 projects, experimentally, their elements; on
family files, the spatial scaffold. What each release
recovers is in [`status.md`](status.md). Raises `ValueError` if the file
can't be parsed far enough to build a model.

Every element of a Revit 2023 to 2027 file that would be a bounding box
(doors, windows, furniture, fixtures, equipment and the rest) is drawn from
the mesh Revit saved for it where all of a mesh decodes, as `rvt-ifc` draws
them by default; `saved_meshes=False` keeps their bounding boxes, as
`rvt-ifc --no-saved-meshes` does; reading every box's saved graphics takes
1.04 to 1.92 times the time and 1.04 to 1.69 times the peak memory of 0.17.0 on Autodesk's samples of Revit 2023 to 2027 (Measure run 43152271314, #1191, #1694); on Revit 2024 to 2027 files a wall that embeds a curtain wall is drawn from its saved mesh the same way where Revit has raised its base (RE-340), and `saved_meshes=False` keeps its record's body. When the saved graphics cannot be read, or an element's decode only in part (that element keeps its box),
the export keeps the bounding boxes and a `RuntimeWarning` says so.
`export_diagnostics()`, `export_diagnostics_json()` and
`rvt_to_ifc_diagnostics()` take the same `saved_meshes` and describe the
export it gives.

A Revit project can hold several phases, and an export shows one: the last, as
Revit's own does. An element that does not exist in the export phase, and a
room whose own phase is not it, are left out, and the diagnostics count them
(the `room_not_in_export_phase` and `element_record_not_in_export_phase`
skipped items). `phase` exports another, as `rvt-ifc --phase` does: a phase's
name, matched exactly first, or its ElementId, as a `str` or an `int`; `None`,
the default, is the last phase, so a call without it writes the bytes it
always wrote, with one exception (#1359): a project whose last phase holds no
room (Autodesk's `rac_basic_sample_project`) writes the rooms of the latest phase
that holds any, and `write_ifc`, `rvt_to_ifc`, `schedule_csv`, `export_diagnostics`
and `rvt_to_ifc_diagnostics` raise a `RuntimeWarning` naming the phase and
saying to pass `phase=`; `decoded_elements()` and `element_counts()`, which take
no `phase`, warn the same way when their result holds rooms, and say to export
with `phase=`, since the handle keeps it. The diagnostics' `room_phase_fallback`
names the phase and counts the rooms. A phase passed is exact: naming the last
phase writes no room. A phase the file does not have raises `ValueError` listing the
phases it does, and so does a file whose phases are not read (a family, or
Revit 2015 and earlier). `write_ifc`, `schedule_csv`, `export_diagnostics`,
`export_diagnostics_json`, `rvt_to_ifc` and `rvt_to_ifc_diagnostics` take it,
and one `RevitFile` can export different phases in turn:

```text
f.write_ifc(phase="Existing")        # by name
f.write_ifc(phase=12589)             # by ElementId
f.write_ifc()                        # the last phase again
```

The handle keeps the phase an export names: `decoded_elements()` and
`element_counts()` read at that phase until a call without `phase` goes back
to the last.

`mode` is one of `scaffold`, `typed-no-geometry`, `geometry`, or
`strict`. `scaffold` accepts the historical spec-valid framework
output; stronger modes fail loudly when the recovered Revit data does
not meet that export quality.

`scaffold` means the export envelope succeeded. It can still be a partial
conversion failure if `export_diagnostics_json()` reports zero validated
building elements or zero geometry elements. Use `mode="strict"` in automation
when an incomplete real-model export should raise `ValueError` instead of
returning IFC text. On a file with no recovered elements, such as the
synthetic fixtures, only `scaffold` succeeds.

`export_diagnostics_json()` returns the JSON diagnostics sidecar
for the default IFC export without writing files.

```text
read_adocument_strict(self) -> dict
read_adocument_lossy(self) -> dict
```
Variants of `read_adocument()`. `read_adocument_strict` raises
`ValueError` when the `ADocument` record is not located or any of its
fields fell back to raw bytes. `read_adocument_lossy` returns
`{"value", "complete", "partial_fields", "failed_streams", "warnings",
"confidence"}`, and raises `OSError` on stream-level failures and
`ValueError` when the schema declares no `ADocument` class.

```text
schema_diagnostics(self) -> dict
```
Integer counts describing the parsed schema: `class_count`,
`parsed_field_count`, `declared_field_count_sum`,
`field_count_mismatches`, `tagged_class_count`,
`parent_only_class_count`, `ancestor_tag_count`, `skipped_records`,
`cpp_type_count`. Raises `ValueError` if the schema can't be parsed.

```text
elem_table_header(self) -> dict
elem_table_records(self) -> list[dict]
declared_element_ids(self) -> list[int]
```
The `Global/ElemTable` index. `elem_table_header()` returns
`{element_count, record_count, header_flag, decompressed_bytes}`
(`element_count` is a per-release class tag, not a count; see
`rvt-elem-table`). `elem_table_records()` returns one
`{offset, id_primary, id_secondary, owner_id}` dict per record, where
`owner_id` is the element the record's element belongs to, or `None`
(RE-31; experimental). `declared_element_ids()` is the sorted,
deduplicated set of ElementIds the table declares. CLI mirror:
`rvt-elem-table file.rvt --json`.

```text
__repr__(self) -> str
```
Returns `"RevitFile(version=2024)"` or `"RevitFile(version=?)"`
if `BasicFileInfo` can't be parsed.

### Threads

The calls that do the Rust work release the GIL while they run (#1884): opening
a file (`RevitFile(path)`, `rvt_to_ifc`, `rvt_to_ifc_diagnostics`), `write_ifc`,
`schedule_csv`, `export_diagnostics_json`, `export_diagnostics`,
`decoded_elements` and `element_counts`. Another Python thread runs meanwhile,
so a service can export several files at once, or keep answering while a large
model decodes. The result is built into Python objects, and a warning raised,
after the GIL is taken again.

A handle is used by one thread at a time: a second call on the same
`RevitFile`, from another thread, while one runs raises `RuntimeError` (the
object is already borrowed). Before 0.22.0 that call waited for the GIL and then
ran, so a program that shares a handle between threads needs a handle for each
thread, or a lock.

### Errors

| Exception | Raised when |
|---|---|
| `IOError` | File missing, non-CFB input, file-size over `max_file_bytes`, unknown stream name, read error |
| `ValueError` | File parsed as CFB but could not be decoded further: the IFC export cannot build a model, `mode` is not one of the four or is not met, a resource limit left part of the file unread and `allow_partial` is false, a strict variant fails, the schema or `Global/ElemTable` does not parse, `read_adocument` finds no `ADocument` class, or `schedule_csv` gets another `kind` |
| `RuntimeWarning` (a warning, not an exception) | A reader stopped at its work budget: the export is still written, without what that reader would have read, and the warning names the reader (since 0.20.0, #1855) |

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
  compressed-bomb input. The members of one multi-member stream
  (`Partitions/NN`) may together inflate to this or, whichever is
  larger, 16 times the stream's stored size, at least 1 GiB and at most
  4 GiB (1 GiB on a 32-bit build).
- `max_walker_scan_bytes`, `max_walker_candidates`,
  `max_walker_trial_offsets`, `max_walker_record_decode_bytes`,
  and `max_walker_container_records` — bound schema-directed
  fallback scans and return warnings in lossy diagnostics when hit.

A stream these limits stop is not read (or is read only in part),
and every reader goes on without it; `unread_streams()` lists each
one with its stored size, the limit and which limit it was, and
`write_ifc()` and `rvt_to_ifc()` raise `ValueError` rather than
return an incomplete IFC unless `allow_partial=True` (#1075). Real
client models with a 400 MiB to 1 GiB partition need larger limits,
and several GB of memory:

```python
f = rvt.RevitFile(
    "model.rvt",
    max_stream_bytes=2 * 1024**3,
    max_inflate_bytes=8 * 1024**3,
    max_walker_scan_bytes=8 * 1024**3,
)
assert f.unread_streams() == []
ifc_text = f.write_ifc()   # raises ValueError if part of the file is still unread
```

The defaults are conservative for desktop use. Tighten them
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

The Python bindings have the Rust core's version support:

| Revit release | Open + metadata + schema | Walker (ADocument) | IFC4 document export |
|---|---|---|---|
| 2015 | partial: the sample projects open and their schema and `BasicFileInfo` read; `PartAtom` and the preview are not measured | not measured | typed elements from element records, experimental |
| 2016 – 2022 | yes | `None` (entry-point detector: partial) | typed elements from element records, experimental |
| 2023 | yes | `None` (entry-point detector: partial) | typed elements from element records |
| 2024 – 2025 | yes | full | typed elements with measured geometry |
| 2026 | yes | full | typed elements from element records, experimental |
| 2027 | yes | not measured | typed elements from element records, experimental |

Full matrix and per-column definitions are in
[`docs/compatibility.md`](./compatibility.md). Expanding the generic
walker to 2016–2023 requires per-version entry-point heuristics and is
tracked in [`ROADMAP.md`](../ROADMAP.md) — active work, not shipped.

File types: `.rvt`, `.rfa`, `.rte`, `.rft` all read through the
same code path (CFB magic dispatch). The reference models, projects and
families, are listed in [`reference-models.md`](reference-models.md).

## What isn't exposed yet

The Rust crate is broader than these bindings. The Python surface
intentionally stays minimal; if you need more, either file an
issue or drop to the Rust CLIs (`rvt-info`, `rvt-analyze`,
`rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`,
`rvt-doc`, `rvt-ifc`).

Not in Python today:

- **The decoder registry.** `decoded_elements()` lists the production
  walker's elements, with the typed projection of the seven classes in
  `rvt.MVP_TYPED_CLASSES`. The 81 classes registered in
  `elements::all_decoders()` (see [`compatibility.md`](compatibility.md) §3)
  are not exposed as a registry: the walker decodes the other registered
  classes generically, and `rvt-doc` and the Rust API list them.
- **The export model itself.** `write_ifc()` returns the IFC text and
  `export_diagnostics()` its diagnostics, but the in-memory `IfcModel`
  (entities, storeys, materials) is not exposed as Python objects, and
  neither are the glTF and plan-SVG writers (`rvt-gltf`, `rvt-sheet`).
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
- **Strict variants for every getter.** `basic_file_info_json`,
  `part_atom_json` and `read_adocument` have `_strict` (and
  `read_adocument_lossy`) variants. `schema_json` already raises on
  failure, so it has none.

## Troubleshooting

### `ImportError: cannot import name 'rvt'`

Likely an older / different `rvt` package in the environment. Run
`pip uninstall rvt` and reinstall.

### `IOError: Not a Revit file (magic bytes did not match CFB/OLE)`

The file doesn't start with OLE2 magic bytes
(`D0 CF 11 E0 A1 B1 1A E1`). Likely causes: it's not a Revit file,
it's a zero-byte Git LFS placeholder, or a transfer layer mangled
the bytes (e.g. line-ending conversion).

### `IOError: the file is <size> bytes, over the limit of <limit>`

The file exceeds `max_file_bytes` (default 2 GiB). Either pass a
larger cap explicitly in the constructor or verify the file isn't
corrupt.

### `read_adocument()` returns `None`

The Layer-5a walker's entry-point detector didn't find an
`ADocument` record. Expected on Revit 2016–2023 files (see version
support matrix). For 2024–2026 files this is unexpected on the
12-release reference corpus — all samples resolve — but may occur
on exotic layouts. File an issue with the Revit release year and
`f.stream_names()` output.

### Wheel won't build from source

`maturin build --manifest-path rvt-py/Cargo.toml` needs:

- Rust ≥ 1.87
- Python development headers (`python3-dev` on Linux, Xcode
  command-line tools on macOS, python.org Python on Windows)

On Apple Silicon, verify `pip install maturin` resolved a
`universal2` or `arm64` wheel, not a stale `x86_64` one.

## Contributing

See [`CONTRIBUTING.md`](../CONTRIBUTING.md). Python-specific
changes land in:

- [`rvt-py/src/lib.rs`](../rvt-py/src/lib.rs) — the pyo3 binding layer
- [`python/rvt/__init__.pyi`](../python/rvt/__init__.pyi) — type
  stubs (keep in sync)
- [`tests/python/`](../tests/python/) — pytest: `test_rvt.py` against the
  reference corpus, and `test_docs_python.py`, which runs every `python`
  block on this page against the synthetic fixture (a block that shows a
  signature or a result shape is fenced `text` and is not run)

Do not add Rust code solely to expose something to Python — wire
through the existing Rust public API where possible.
