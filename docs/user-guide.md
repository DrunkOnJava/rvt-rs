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

## Open A File In The Browser

1. Open <https://drunkonjava.github.io/rvt-rs/>.
2. Drop a `.rvt`, `.rfa`, `.rte`, or `.rft` file onto the page, or choose a file
   with the file picker.
3. Read the File status panel before exporting anything.
4. Use Diagnostics details or Download diagnostics when the status panel reports
   warnings, partial decode, unsupported model layout, or scaffold-only export.

The viewer can show a scene, categories, element info, schedule summary, export
quality, and a supported-profile matrix. Geometry shown in the viewer is limited
to what rvt-rs actually decoded.

## Inspect A File From The Command Line

Use `rvt-inspect` when you want a shareable support report:

```bash
rvt-inspect model.rvt
rvt-inspect model.rvt --json > model.inspect.json
```

The text output is intended for quick triage. The JSON output is intended for
automation and GitHub issues. By default, paths are redacted so the report is
safer to share.

## Write Or Patch A File

rvt-rs has a stream-level writer, not a semantic Revit editor.

| Operation | What it means | Use when |
|---|---|---|
| Byte-preserving copy | Copy the CFB container without changing any stream bytes. | You need a local safety copy or a write-path smoke test. |
| Stream patching | Replace the complete bytes of a named OLE stream, with explicit framing for raw/truncated-gzip streams. | You are building controlled tooling around known stream payloads. |
| Semantic editing | Change a Revit concept such as wall height, room name, level elevation, or parameter value. | Not supported yet. This needs field-level encoders and Revit semantic validation; see [ADR-002](decisions/ADR-002-semantic-write-api-gate.md). |

`rvt-write` applies JSON patch manifests atomically: it validates every target
stream name before writing, writes through a sibling temp file, verifies patched
streams after write, and preserves unpatched streams. Corpus tests cover
identity, grow, shrink, and multi-stream patches on family files and a real
project fixture; GUID and history preservation are checked for project patches.

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
