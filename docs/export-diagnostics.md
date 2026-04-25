# IFC Export Diagnostics

`rvt-ifc --diagnostics path.json` writes a JSON sidecar next to the IFC
output. The same payload is available to Python via
`RevitFile.export_diagnostics_json()` / `rvt_to_ifc_diagnostics(path)` and to
WASM via `openRvtBytesWithDiagnostics(bytes)`.

The sidecar is meant for non-technical support, bug reports, and automated
export-readiness checks. It does not contain raw model bytes.

Terminology used by this file, `rvt-inspect`, Python, and the browser viewer is
defined in [Diagnostic Semantics](diagnostic-semantics.md).

## CLI

```bash
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json
```

Use `--mode` when automation needs to reject incomplete output instead of
accepting a scaffold IFC:

```bash
rvt-ifc model.rvt -o model.ifc --mode strict --diagnostics model.diagnostics.json
```

Supported quality gates are `scaffold`, `typed-no-geometry`, `geometry`, and
`strict`. `strict` fails before writing IFC when required model data is missing;
when `--diagnostics` is present, the sidecar is still written for triage.
`scaffold` is an inspection/export-envelope success, not proof that Revit model
elements were converted.

Diagnostic proxy mode can also write the sidecar:

```bash
rvt-ifc model.rvt -o diagnostic.ifc --diagnostic-proxies \
  --diagnostics diagnostic.json
```

## Schema

`schema_version` is currently `1`. Additive fields may appear in a future
minor release; incompatible changes must increment this number.

| Field | Type | Meaning |
|---|---:|---|
| `schema_version` | integer | Diagnostics schema version. |
| `mode` | string | `placeholder`, `default`, or `diagnostic_proxies`. |
| `input` | object | Revit version/build/redacted path metadata and required stream presence. |
| `decoded` | object | Counts recovered by reader-side scanners before IFC emission. |
| `exported` | object | IFC model entity counts, geometry count, materials, units, storeys. |
| `skipped` | array | Suppressed items, grouped by reason and class. |
| `unsupported_features` | array | Known exporter gaps that affected this output. |
| `warnings` | array | User-facing caveats for this specific export. |
| `confidence` | object | Coarse export-readiness level and booleans for metadata/elements/geometry. |

Important nested fields:

| Field | Type | Meaning |
|---|---:|---|
| `decoded.production_walker_elements` | integer | Elements accepted by the conservative production walker path. |
| `decoded.diagnostic_proxy_candidates` | integer | Low-confidence candidates available to diagnostic export. |
| `decoded.arcwall_records` | integer | Version-gated ArcWall records exported as `IFCWALL`. |
| `exported.by_ifc_type` | object | Count of exported building elements grouped by STEP entity type. |
| `exported.building_elements_with_geometry` | integer | Exported elements with enough placement/body data for geometry. |
| `confidence.level` | string | `scaffold`, `typed_no_geometry`, `geometry`, `diagnostic_partial`, or `proxy_only`. |
| `confidence.score` | number | Heuristic 0..1 readiness score for UI sorting and dashboards. |

## Example

```json
{
  "schema_version": 1,
  "mode": "default",
  "input": {
    "revit_version": 2024,
    "build": "20230308_1635(x64)",
    "original_path": "C:\\path\\model.rvt",
    "project_name": "Sample",
    "stream_count": 24,
    "has_basic_file_info": true,
    "has_part_atom": true,
    "has_formats_latest": true,
    "has_global_latest": true
  },
  "decoded": {
    "production_walker_elements": 0,
    "diagnostic_proxy_candidates": 9,
    "arcwall_records": 0,
    "class_counts": {
      "HostObjAttr": 9
    }
  },
  "exported": {
    "total_entities": 1,
    "building_elements": 0,
    "building_elements_with_geometry": 0,
    "by_ifc_type": {},
    "classification_count": 0,
    "unit_assignment_count": 0,
    "material_count": 0,
    "storey_count": 0
  },
  "skipped": [
    {
      "reason": "low_confidence_schema_scan_candidate",
      "count": 9,
      "classes": {
        "HostObjAttr": 9
      },
      "sample_names": ["HostObjAttr-12345"]
    }
  ],
  "unsupported_features": ["real_file_element_geometry"],
  "warnings": ["No building elements were exported; output is scaffold-only."],
  "confidence": {
    "level": "scaffold",
    "score": 0.25,
    "has_project_metadata": true,
    "has_typed_elements": false,
    "has_geometry": false,
    "has_diagnostic_proxies": false,
    "warning_count": 1
  }
}
```
