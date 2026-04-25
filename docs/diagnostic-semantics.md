# Diagnostic Semantics

This page defines the terms used by `rvt-inspect`, `rvt-ifc`,
Python diagnostics, and the browser viewer. Use it when deciding
whether an output is usable, partial, or only evidence for a bug
report.

## Strict And Lossy Modes

Strict APIs fail when the requested result cannot be produced with the
required confidence:

| Surface | Strict path | Failure meaning |
|---|---|---|
| Rust summary | `summarize_strict()` | Required file metadata could not be decoded. |
| Rust walker | `read_adocument_strict()` | The root document instance could not be decoded completely. |
| CLI IFC | `rvt-ifc --mode strict` | The export lacks required metadata, typed elements, geometry, or has warnings/unsupported features. |
| Python IFC | `write_ifc(mode="strict")` / `rvt_to_ifc(path, mode="strict")` | Same quality gate as the CLI. |

Lossy or best-effort APIs return the data that could be recovered and
attach diagnostics for everything that was skipped, partial, or
unsupported:

| Surface | Lossy or best-effort path | Success meaning |
|---|---|---|
| Rust summary | `summarize_lossy()` | The returned summary may be incomplete; inspect `diagnostics`. |
| Rust walker | `read_adocument_lossy()` | The returned root document may have partial fields. |
| CLI IFC | `rvt-ifc --mode scaffold` | A spec-valid IFC scaffold may be written even when no real model elements were decoded. |
| Python IFC | `write_ifc(mode="scaffold")` | Same scaffold behavior as the CLI. |
| Browser viewer | Drop/open file | The viewer shows what was decoded and labels missing model data before export. |

A lossy success is a successful inspection result. It is not always a
successful model conversion.

## Core Terms

| Term | Meaning | User interpretation |
|---|---|---|
| Empty model | The file opened, but no validated building elements were recovered. | Useful for metadata/schema inspection; not useful as a BIM conversion. |
| Scaffold | A valid IFC framework such as project/site/building/storey records, with no validated building elements. | Valid file envelope, not a converted model. |
| Typed element | A recovered Revit object accepted as a known element class by the conservative production path. | Usable as model data, subject to geometry and property coverage. |
| Geometry-free element | A typed element without enough placement/body information for geometry. | Useful for counts/metadata; not enough for visual or spatial coordination. |
| Proxy | A generic or low-confidence element representation. | Treat as diagnostic evidence unless documentation says that class is supported. |
| Diagnostic candidate | A record that looks relevant during research scans but did not meet the production confidence bar. | Attach diagnostics to an issue; do not rely on it for export counts. |
| Unsupported record layout | The file contains a record shape the current decoder does not understand for that version/profile. | The file may be valid Revit; rvt-rs needs more corpus evidence or decoder work. |
| Partial decode | Some file sections or fields decoded and others did not. | Inspect the warnings before using output downstream. |

## Export Quality Levels

| Level | Meaning | Typical next step |
|---|---|---|
| `scaffold` | IFC has framework entities but zero validated building elements. | Use `rvt-inspect --json` or download viewer diagnostics before filing an issue. |
| `typed_no_geometry` | At least one typed building element exported, but no element geometry. | Good for evidence and counts; not ready for geometry workflows. |
| `geometry` | At least one building element includes decoded geometry. | Validate the output in downstream BIM tooling before relying on it. |
| `diagnostic_partial` | Explicit diagnostic export includes low-confidence proxy evidence. | Research/support only. |
| `proxy_only` | Output contains proxy elements without validated typed elements. | Research/support only. |

## Success Versus Partial Failure

| Scenario | Inspect command | IFC export | Interpretation |
|---|---|---|---|
| File opens and metadata/schema decode | Success | May still be scaffold-only | Inspection success. |
| No validated elements | Success with warning | `scaffold` succeeds; `strict` fails | Conversion partial failure. |
| Typed elements without geometry | Success with warning | `typed-no-geometry` succeeds; `geometry` and `strict` fail | Counts may be useful; model geometry is incomplete. |
| Geometry-bearing typed elements | Success | `geometry` may succeed; `strict` still checks metadata, units, storeys, warnings, and unsupported features | Candidate supported output. |
| Corrupt or non-CFB file | Fails | Fails | Not a Revit container rvt-rs can read. |
| Unsupported record layout | Success with unsupported-feature diagnostics or fails at strict quality gate | Depends on requested mode | Valid bug report input if diagnostics are attached. |

## User-Facing Failure Modes

`rvt-inspect` reports `failure_mode.kind` in JSON, and the viewer shows the same
classification in its file-status panel.

| Kind | Meaning |
|---|---|
| `supported_profile` | The decoded output meets the current export profile. |
| `unsupported_revit_version` | The file opened, but its Revit version is outside the verified support range. |
| `unsupported_model_layout` | The container/version is acceptable, but model records did not meet the production confidence bar. |
| `corrupt_file` | The input could not be opened as a readable Revit OLE/CFB container. |
| `partial_decode` | Some model data was recovered, but warnings, unsupported features, or missing geometry remain. |
| `scaffold_only_export` | IFC can be written as a framework, but no validated building elements were decoded. |
| `parser_bug_please_report` | The file opened, but diagnostics could not classify export readiness. |

## Warning Examples

Users should expect warnings like these when output is incomplete:

```text
No building elements were exported; output is scaffold-only.
Diagnostic proxy elements are low-confidence scan candidates, not validated model elements.
No real-file element geometry decoded.
Required schema/model stream missing.
```

Warnings are part of the contract. Do not suppress them in user-facing
tools, release notes, or screenshots.

## Reporting A File

When a file falls outside the supported profile:

1. Run `rvt-inspect model.rvt --json > model.inspect.json`, or use
   the viewer diagnostics download.
2. If IFC export is involved, run
   `rvt-ifc model.rvt -o model.ifc --mode strict --diagnostics model.diagnostics.json`.
3. Attach the JSON diagnostics to a GitHub issue. Do not attach
   proprietary model bytes unless you have permission to publish them.
