# rvt-inspect

`rvt-inspect` is the support-facing command for a quick file health and export
readiness answer. It avoids reverse-engineering terminology in the default text
output and writes a stable JSON report with `--json`.

The diagnostic terms used here are defined in
[Diagnostic Semantics](diagnostic-semantics.md). The important distinction is
that opening and inspecting a file can succeed even when model conversion is
partial or scaffold-only.

```bash
rvt-inspect model.rvt
rvt-inspect model.rvt --json
```

The report answers:

| Section | Meaning |
|---|---|
| `file` | File opened, Revit version/build, verified version range, stream count, schema parse status. |
| `decoded` | Class count, validated elements, diagnostic candidates, ArcWall records, geometry count. |
| `export` | IFC readiness level, score, building element counts, unsupported features. |
| `warnings` | User-facing caveats from the export diagnostics sidecar. |
| `next_steps` | Short actions appropriate for the current readiness level. |
| `export_diagnostics` | The full diagnostics payload also used by `rvt-ifc --diagnostics`. |

## Reading The Result

`rvt-inspect` treats metadata/schema inspection as useful work. A report can
therefore be successful while still warning that no validated elements or
geometry were decoded.

| Result | Meaning |
|---|---|
| File/schema fields present, no export warnings | Inspection succeeded and no obvious export caveat was reported. |
| `export.level` is `scaffold` | The IFC path can write a valid framework, but no validated building elements were decoded. |
| `decoded.diagnostic_candidates` is greater than zero | Research scans found low-confidence candidates that are not production model elements. |
| `decoded.geometry_elements` is zero | Geometry workflows are not supported for this file yet. |
| `warnings` is non-empty | Read the warnings before relying on exported IFC/glTF/SVG output. |

## JSON Schema

`schema_version` is currently `1`. Additive fields may be introduced in minor
releases. Incompatible changes must increment this number.

By default, paths are redacted so the output is safer to share in bug reports.
Use `--no-redact` only for private/local diagnostics where original paths are
needed.
