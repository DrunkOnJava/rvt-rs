# rvt-inspect

`rvt-inspect` is the support-facing command for a quick file health and export
readiness answer. It avoids reverse-engineering terminology in the default text
output and writes a stable JSON report with `--json`.

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

## JSON Schema

`schema_version` is currently `1`. Additive fields may be introduced in minor
releases. Incompatible changes must increment this number.

By default, paths are redacted so the output is safer to share in bug reports.
Use `--no-redact` only for private/local diagnostics where original paths are
needed.
