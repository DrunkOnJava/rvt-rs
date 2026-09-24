# User Guide

This guide is for BIM, architecture, engineering, construction, and facilities
users who want to inspect a Revit file without installing Revit or uploading the
file to a cloud service.

rvt-rs is useful today for safe local inspection, metadata extraction, schema
inspection, previews, diagnostics, and scaffold-level IFC checks. It is not yet
a general-purpose Revit-to-IFC replacement for real project models.

## What The Tool Does

Use rvt-rs when you need to:

- Check that a `.rvt`, `.rfa`, `.rte`, or `.rft` file opens.
- See the Revit version, build, stream inventory, project metadata, and schema
  status.
- Preview what model data rvt-rs decoded.
- Export diagnostics that explain why a file is unsupported or partial.
- Produce IFC4 STEP output for support and interoperability testing; geometry
  is limited to the model parts the current decoders recover.

Do not use rvt-rs yet as the only conversion path for production BIM delivery,
coordination, quantity takeoff, fabrication, permitting, or contractual model
exchange.

## What Stays Private

The browser viewer at <https://drunkonjava.github.io/rvt-rs/> parses files
inside your browser tab.

- No model upload.
- No account.
- No telemetry.
- No third-party analytics.
- No network requests after the static viewer files load.

Downloads such as IFC, glTF, SVG, and diagnostics are created in the browser and
saved by you. The privacy posture is documented in
[viewer-privacy-posture.md](viewer-privacy-posture.md).

There is no native desktop app (Tauri/Electron). The supported product path is
the hosted or self-hosted static viewer; see
[ADR-004](decisions/ADR-004-desktop-distribution.md).

## Open A File In The Browser

1. Open <https://drunkonjava.github.io/rvt-rs/>.
2. Drop a `.rvt`, `.rfa`, `.rte`, or `.rft` file onto the page, choose a file
   with the file picker, or open a redistributable entry from the demo gallery.
   The gallery leads with two MIT-licensed real Revit projects:
   `Revit_IFC5_Einhoven.rvt` (2023, 913 KB, opens in about half a second) and
   `2024_Core_Interior.rvt` (2024, 33.7 MB, about 3 seconds to decode —
   measured on the deployed site, about 7 seconds including the download).
   Everything below them is a 20 KB synthetic that decodes to a scaffold only.
3. Read the File status panel and export-quality label before exporting anything.
4. Use Diagnostics details or Download diagnostics when the status panel reports
   warnings, partial decode, unsupported model layout, or scaffold-only export.

### Supported MVP workflow

1. Open locally (nothing uploads).
2. Confirm decode/confidence status.
3. Inspect whatever elements were actually decoded.
4. Export IFC / glTF / plan with the quality label in mind.
5. Attach diagnostics JSON to bug reports for partial exports.

Partition MVP recovers Level / Material / Room names on supported project
files, 2023 ArcWall geometry on the research path, and — on Revit 2024 —
Wall / Door / Window / Column / Floor / BuildingPad instances from partition
element records, with a measured slab thickness (#212) and, for all 80
exported slabs, the plan profile their `OST_SketchLines` records close
(#31, RE-25). Wall bodies carry the wall's real thickness and the length
its joins leave it — the element record names the walls a wall is joined
to, which takes 351 of 360 walls to an exact match against Revit's own
export in world coordinates, and every column is the record prism minus
the walls that cut it, exact on 256 of 256 (#215 / #238 / #239, RE-26 and
RE-29). Rooms come out as 116 `IFCSPACE` carrying their real Revit name,
number and storey, with a body that is the record's bounding box — exactly
the reference export's plan envelope and floor-to-ceiling extent on all
116 (#90, RE-29). Those elements land on the Revit Level their own element
record names, or their base constraint where it names two, so all 970
exported elements on Core Interior are contained in the storey Revit's own
export gives them (#219, RE-27, RE-59). An element whose record names no
Level is contained in the building rather than filed under an arbitrary
storey. Every wall also joins to its
`IfcWallType` exactly, 360 of 360, though that join is library-side today
and is not yet written onto the exported wall (#88, RE-28). Still blocked
(RE-19 / RE-20 negative on magnetar corpora): schema-field Walls, typed
Door/Window host binding, room boundary polygons — RE-29 measured that
negative, a room record carries no sketch — door and window bodies, the
profile of the 20 rotated shading plates. Compound layers are read from
each type's own data (RE-53) and written as layer sets with their
material names (RE-58); the reference export is a `ReferenceView_V1.2`
file with no `IfcMaterialLayerSet` in it, so they are scored against the
faces its bodies style with each material. See [status.md](status.md) and
[supported-profile.md](supported-profile.md).

The viewer can show a scene, categories, element info, schedule summary, export
quality, a demo gallery with license/provenance, and a supported-profile matrix.
Geometry shown in the viewer is limited to what rvt-rs actually decoded.

The scene tree lists each storey's elements by category, with a count per
category. A category opens to its elements, sorted by family and type. Each
element row leads with its type name, with the family and ElementId on a
second line, so "Basic Wall:8" Interior Partition 3 Hour:20796" reads as
"8" Interior Partition 3 Hour" over "Basic Wall · 20796". A curtain wall
opens to its panels and mullions, and a stair to its flights. Doors and
windows are listed in their own category; the info panel names the wall
that hosts them. Selecting a category lights up its elements. Picking an
element in the 3-D view opens its category and scrolls to it. The arrow
keys move through the tree: Right opens a row, Left closes it or moves to
its parent, and Home and End go to the first and last rows.

Type in the filter above the tree, or press / to reach it, to keep only the
elements whose name or IFC type contains the text: "M_Single", "IfcDoor" or
an ElementId such as "20796". The storeys and categories that hold them
open, and a count shows under the box. Enter moves to the first match, and
Escape clears the filter.

Selecting an element fills the info panel with typed rows rather than raw
record fields: name, IFC type and predefined type, ElementId, Revit's own
GlobalId where the file yields it (RE-48), the storey it sits
on (clickable — it selects that storey in the tree), placement and extents
in feet, and each property set as labelled rows carrying their units. Host
and hosted rows let you jump from a door or window to its wall and back.
The schedule groups the elements by IFC type with a count per type, and each
row toggles a tint over every mesh of that type in the 3-D scene. Where a
field is simply absent it is omitted; a one-line "Not recovered" note names
a field only where the export diagnostics confirm it as a known decode gap,
so an omission is never mistaken for a failure (#272).

To bring an element into view, double-click it in the tree or the 3-D view,
press F with it selected, or use Zoom to element in the info panel.
Double-clicking a storey frames everything on it, and F with nothing
selected frames the whole model. Dragging in the view orbits without
changing the selection; a click without a drag selects.

### Performance

Opening a large project is fast since #266, which inflates each partition
stream once per file instead of once per consumer. `rvt-ifc --mode geometry`
on the 33.7 MB `2024_Core_Interior.rvt` went from 26.07 s and 2641.4 MiB of
peak RSS to **1.69 s and 490.1 MiB** (Apple Silicon, `/usr/bin/time -l`,
best of three), and the exported IFC is byte-identical. In the browser the
same file decodes in **about 3 seconds**, measured on the deployed site,
where it used to take about 28. Nothing about the decode changed — only how
many times the same bytes were decompressed.

The partition scans that later releases added each searched every partition
once per thing they looked for: once per element category, once per Level,
once per type. They now make one pass for all of them. `rvt-ifc` on Core
Interior went from 5.3 s to **2.75 s**, and on Autodesk's 95 MB Snowdon
Towers sample from 17.9 s to **10.1 s**, with the same peak memory (Apple
Silicon, `/usr/bin/time -l`, both builds measured back to back). The browser
viewer gains more, since WebAssembly searches without SIMD: Snowdon Towers
now opens in **24 seconds** instead of 56. The IFC and the diagnostics are
byte-identical on all 14 local test files.

## Inspect A File From The Command Line

Use `rvt-inspect` when you want a shareable support report:

```bash
rvt-inspect model.rvt
rvt-inspect model.rvt --json > model.inspect.json
```

The text output is intended for quick triage. The JSON output is intended for
automation and GitHub issues. By default, paths are redacted so the report is
safer to share.

## Inventory A Folder Of Revit Files

`rvt-info` answers "which Revit release saved this, is it workshared, and
when was it last saved?" for one file or a whole folder tree:

```bash
rvt-info model.rvt                              # detailed report for one file
rvt-info projects/                              # one row per .rvt/.rfa/.rte/.rft file
rvt-info projects/ -f csv > inventory.csv       # open in Excel / Sheets
rvt-info projects/ -f jsonl                     # one JSON object per line, for scripts
rvt-info projects/ --no-recurse                 # top level only
```

The folder mode reads only the two small identity streams of each file, so
a share full of large projects scans in seconds. Each row carries the Revit
release, build, worksharing state (`Not enabled`, or the role of the saved
copy in a workshared project), central model path, the user and time of the
last save, the document GUID, and Revit's save counter. Revit backup copies
(`name.0001.rvt`) are marked. A file that cannot be read gets a row with the
reason, and the command exits with status 1 so scripts notice.

Pass `--redact` before sharing the output: it replaces the last-saved-by
user name everywhere it appears (Revit names a local copy
`<central>_<user>.rvt`) and scrubs Windows user folders from paths.

## Get A Schedule Into Excel

`rvt-schedule` writes the decoded elements, or the rooms, as a CSV file that
Excel, Google Sheets and LibreOffice open directly:

```bash
rvt-schedule model.rvt                          # model.elements.csv next to the model
rvt-schedule model.rvt --schedule rooms         # model.rooms.csv: number, name, level
rvt-schedule model.rvt --metric --excel         # metres, and a BOM so Excel reads non-ASCII names
rvt-schedule model.rvt -o -                     # print the CSV instead of writing a file
```

The element schedule has one row per decoded building element: Revit
ElementId, IFC type, level and its elevation, material, placement, body
size, the host wall of each door and window, and where the body came from
(`body_source`, `profile_resolved`). A value rvt-rs did not decode is an
empty cell, never a guess. The room schedule has no area column: today a
room's body is its bounding box, and the area of a box is not the area of
a room.

What the schedules contain is exactly what rvt-rs decodes, so check
`rvt-inspect model.rvt` first: Revit 2024 project files with element
records give walls, doors, windows, columns, slabs and rooms; other files
give few or no rows. The browser viewer has the same two schedules as
**Download element schedule (CSV)** and **Download room schedule (CSV)**
under Schedule, built in the tab like every other export.

## Write Or Patch A File

rvt-rs has a stream-level writer, not a semantic Revit editor.

| Operation | What it means | Use when |
|---|---|---|
| Byte-preserving copy | Copy the CFB container without changing any stream bytes. | You need a local safety copy or a write-path smoke test. |
| Stream patching | Replace the complete bytes of a named OLE stream, with explicit framing for raw/truncated-gzip streams. | You are building controlled tooling around known stream payloads. |
| Semantic editing | Change a Revit concept such as wall height, room name, level elevation, or parameter value. | Not supported yet. This needs field-level encoders and Revit semantic validation; see [ADR-002](decisions/ADR-002-semantic-write-api-gate.md). |

`rvt-write` applies JSON patch manifests atomically: it validates every target
stream name before writing, writes through a sibling temp file, verifies patched
streams after write, and preserves unpatched streams. Always-on corpus tests
(`tests/cfb_patch_corpus.rs`) cover identity, grow, shrink, multi-stream, and
missing-stream cases on a `gen-fixture` synthetic project plus the MIT-licensed
`tests/fixtures/families/empty.rfa` family; GUID and history preservation are
checked when those streams are left unpatched. Optional Autodesk-family /
project-corpus coverage in `tests/cfb_roundtrip_delta.rs` still runs when
`RVT_SAMPLES_DIR` / `RVT_PROJECT_CORPUS_DIR` are set.

```bash
rvt-write model.rvt --patches patches.json -o patched.rvt
```

## Export IFC

The default IFC mode writes a valid IFC4 framework when possible:

```bash
rvt-ifc model.rvt -o model.ifc
```

For automation, use strict mode so incomplete real-model exports fail instead of
quietly producing a scaffold:

```bash
rvt-ifc model.rvt -o model.ifc --mode strict --diagnostics model.diagnostics.json
```

Export modes:

| Mode | Use when |
|---|---|
| `scaffold` | You want a valid IFC envelope and diagnostics even if no validated model elements were decoded. |
| `typed-no-geometry` | You require at least one typed building element, but geometry is not required. |
| `geometry` | You require at least one building element with decoded geometry. |
| `strict` | You require typed elements, geometry, project metadata, units, storeys, no warnings, and no unsupported features. |

If strict mode fails, read the diagnostics JSON. The failure may mean the file is
valid Revit but outside the current rvt-rs support profile.

## Understand Warnings

The most important status words are:

| Status | Meaning |
|---|---|
| Supported profile | The decoded output meets the current export profile. |
| Scaffold-only export | IFC can be written as a framework, but no validated building elements were decoded. |
| Partial decode | Some data was recovered, but warnings, unsupported features, or missing geometry remain. |
| Supported file, unsupported model layout | The file opened, but rvt-rs did not recover validated model elements from this layout. |
| Unsupported Revit version | The file opened, but its version is outside the verified support range. |
| Corrupt or unreadable file | The input could not be opened as a readable Revit OLE/CFB container. |
| Parser bug, please report | The file opened, but diagnostics could not classify export readiness. |

For the precise terminology, read
[diagnostic-semantics.md](diagnostic-semantics.md).

## Supported Files

Supported today:

- `.rvt`, `.rfa`, `.rte`, and `.rft` containers that use the standard Revit
  OLE/CFB layout.
- Metadata and schema inspection across the verified 2016-2026 family corpus.
- Browser and CLI diagnostics for support triage.
- IFC4 scaffold output and narrow version-gated typed evidence.

The first real-model conversion target is narrower: Revit 2023/2024
architectural `.rvt` project files with levels, walls, floors, doors, windows,
rooms, materials, and common parameters. See
[supported-profile.md](supported-profile.md).

## Report A Bad File

1. Run:

   ```bash
   rvt-inspect model.rvt --json > model.inspect.json
   rvt-ifc model.rvt -o model.ifc --mode strict --diagnostics model.diagnostics.json
   ```

2. Open a GitHub issue and include:

   - Revit version, if known.
   - File type: `.rvt`, `.rfa`, `.rte`, or `.rft`.
   - What you expected to see.
   - The failure mode shown by `rvt-inspect` or the viewer.
   - The diagnostics JSON.

3. Do not attach proprietary model files unless you have permission to publish
   them. Diagnostics are designed to be useful without raw model bytes.

## Where To Go Next

- Current support boundary: [status.md](status.md)
- Supported MVP profile: [supported-profile.md](supported-profile.md)
- Diagnostic terminology: [diagnostic-semantics.md](diagnostic-semantics.md)
- Python API: [python.md](python.md)
