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
`strict`. These modes also shape *content*:

| Mode | Emission |
|---|---|
| `scaffold` | Framework + production elements; geometry when recovered. No HostObjAttr proxies. |
| `typed-no-geometry` | Mapped/typed elements only; geometry fields stripped. |
| `geometry` / `strict` | Mapped/typed elements; Lane Six curves/loops/hosts/elevations when present. |

`strict` fails before writing IFC when required model data is missing;
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
| `source_coverage` | object (optional) | A10 measured coverage from export/decode counts. `status` is `measured` when at least one fraction has a trustworthy denominator; otherwise `unset` with null fractions. Never invents ratios. |
| `formats_latest_integrity` | object (optional) | `Formats/Latest` page-boundary integrity under the Finding 1 narrow gate (strip stays **disabled**). Multipage streams report `integrity_status: uncertain` and `diagnostic_code: RVT_FORMATS_MULTIPAGE_UNVERIFIED` without claiming completeness. `schema_scan_truncated` / `schema_scanned_bytes` report whether schema parsing stopped at the 65536-byte scan limit — a "we stopped looking" signal, not a claim about the unscanned tail. |
`skipped.reason` is stable enough for automation. Geometry-related reasons use
the `unsupported_geometry_*` prefix and currently include
`unsupported_geometry_curve`, `unsupported_geometry_profile`,
`unsupported_geometry_unresolved_host`, `unsupported_geometry_missing_level`,
and `unsupported_geometry_missing_dimensions`. A single element can appear in
more than one geometry bucket because these are condition counts, not a
deduplicated element total.

`element_record_without_element_id` counts the element records of every
recovered category whose frame is in place (the `BuiltInCategory` at `+0x12`
and the bounding-box marker at `+0x50`) but whose `+0x00` holds no ElementId,
keyed by class in `classes`. The recovered categories are walls, doors,
windows, columns, floors, building pads and rooms, plus the RE-33 product
categories: furniture, casework, plumbing fixtures, specialty equipment,
ceilings, curtain-wall mullions and panels, railings, wall sweeps, ducts,
duct fittings, pipes and pipe fittings. Only placed instances count (no
container reference, placement kind `0xffffef7f`), so container members and
type symbols do not inflate it. The fail-closed decode cannot attribute these
records, so none is exported, and a matching warning says the model is
incomplete. It is absent on `2024_Core_Interior.rvt` and on every
project-count fixture. On Autodesk's Snowdon Towers 2024 architectural sample
it is 4,886 against 43 decodable instances, and there it matches Revit's own
export exactly for mullions, columns, railings, ceilings and furniture (see
`reports/element-framing/RE-30-snowdon-generalisation.md` and
`reports/element-framing/RE-33-product-categories.md`).

`unsupported_features` carries exactly one geometry-coverage code:
`real_file_element_geometry` when **no** exported building element has a
recovered body, and `partial_element_geometry` when some do and some do not
(Core Interior after #204: 256 `IFCCOLUMN` with bodies, 82 slabs/spaces
without). Neither appears once every exported element carries geometry.

Important nested fields:

| Field | Type | Meaning |
|---|---:|---|
| `decoded.production_walker_elements` | integer | Elements accepted by the conservative production walker path. |
| `decoded.diagnostic_proxy_candidates` | integer | Low-confidence candidates available to diagnostic export. |
| `decoded.arcwall_records` | integer | Version-gated ArcWall records exported as `IFCWALL`. |
| `decoded.recovered_unit_identifiers` | array | Revit `autodesk.unit.*` identifiers selected for IFC unit assignment. |
| `decoded.unknown_unit_identifiers` | array | Revit unit identifiers observed but not mapped to IFC units. |
| `exported.by_ifc_type` | object | Count of exported building elements grouped by STEP entity type. |
| `exported.building_elements_with_geometry` | integer | Exported elements with enough placement/body data for geometry. |
| `exported.storey_names` | array | Recovered building-storey display names, in emission order. |
| `exported.storey_elevations_feet` | array | Recovered storey elevations in feet, aligned with `storey_names`. An all-zero list means only Level *name* strings were recovered — no elevation evidence was found. |
| `exported.storey_bound_elements` | integer | Building elements contained in a specific storey. The rest are contained in the `IfcBuilding`, never in a named storey. |
| `confidence.level` | string | `scaffold`, `typed_no_geometry`, `geometry`, `diagnostic_partial`, or `proxy_only`. |
| `confidence.score` | number | Heuristic 0..1 readiness score for UI sorting and dashboards. Its element terms (elements, typed elements, geometry) are scaled by the exported share when `unexported_element_records` is non-zero. |
| `confidence.unexported_element_records` | integer | Element records of a recovered category (walls, doors, windows, columns, floors, rooms and the RE-33 product categories) the partition scan found but could not export because no ElementId is attributable (RE-30). Non-zero means the model is incomplete. Zero is not a completeness claim. |

## Storey provenance

`storey_count` / `storey_names` / `storey_elevations_feet` describe one of two
different recoveries, and the elevations are how a reader tells them apart.

- **Name-only.** Partition `Level`-like strings were recovered but nothing in
  the file gave them an elevation, so every entry in
  `storey_elevations_feet` is `0.0`. The storeys are real names in an
  arbitrary order; they are not positions, and no element is contained in
  one.
- **Measured elevations.** Base elevations were recovered — from 2023 ArcWall
  trailers, or on Revit 2024 from the partition element-record bounding boxes
  (#213) — so `storey_elevations_feet` carries distinct values and
  `storey_bound_elements` counts the elements that matched one. Storeys whose
  elevation was measured but whose name was not are labelled
  `Elevation N ft`; a Level name is applied only when there is exactly one
  recovered name per measured elevation, because any other pairing would be a
  guess about which Level sits where. The unpaired names remain visible in
  `decoded.production_class_counts.Level`, and a warning states how many were
  left unplaced.

A third recovery sits in front of both and is the only one the file *states*
rather than infers. On Revit 2024 a partition element record's counted
reference list names the `Level` that hosts the element, and when it names
exactly one recovered Level the element is contained in that Level's storey
(#219, RE-27). Such an element carries `LevelBindResolved = true`,
`LevelElementId`, and `LevelBindSource =
partition_element_record_reference_list`. A record that names no Level, or two
— a column and a wall each carry a base *and* a top constraint, with nothing
in the bytes saying which slot is which — resolves to nothing and keeps the
elevation join.

Every element that reached a storey says how, in `StoreyBindSource`:

| value | join |
|---|---|
| `record_level_reference` | the Level ElementId the record names (#219) |
| `record_base_elevation` | the record's base face equals a storey elevation (#213) |
| `record_top_elevation` | the record's top face equals one — plates hang below their level (#212) |

The property's *absence* is the honest unbound state. An unbound element is
contained in the `IfcBuilding`, which says "in this building, storey unknown";
it is never written into the first storey, because that would state a
containment nothing measured.

`LevelBindResolved` still stays `false` for every element whose storey came
from an elevation match, and for the typed `Floor` / `Room` `m_level_id` join,
which remains unrecovered (#86 / RE-20).

## Geometry placement

`exported.building_elements_with_geometry` counts elements that carry a body.
Where that body sits is fixed by one rule: **the element translation is carried
exactly once**, by the element's own `IfcLocalPlacement`. The swept solid's
`Position` is the project-level identity placement, and the profile is authored
in the element-local frame.

That matters to anyone comparing a diagnostics count against what a viewer
draws. Until #232 the writer put the element's own `IfcAxis2Placement3D` in
both slots, so a consumer that composes `ObjectPlacement × Position` — which
IFC4 requires — placed every element at twice its translation, while viewers
that ignore `Position` drew it correctly. Two tools could therefore disagree
about the same file with neither of them wrong about what it read. A geometry
count that looks right is not evidence that the geometry is where it belongs;
compose the two and check, as `tools/ci/ifc_schema_arity.py` now does on every
export.

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
    },
    "recovered_unit_identifiers": [
      "autodesk.unit.unit:meters-1.0.0"
    ],
    "unknown_unit_identifiers": []
  },
  "exported": {
    "total_entities": 1,
    "building_elements": 0,
    "building_elements_with_geometry": 0,
    "by_ifc_type": {},
    "classification_count": 0,
    "unit_assignment_count": 1,
    "material_count": 0,
    "storey_count": 0,
    "storey_names": [],
    "storey_elevations_feet": [],
    "storey_bound_elements": 0
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
    "warning_count": 1,
    "unexported_element_records": 0
  },
  "source_coverage": {
    "status": "unset",
    "notes": "Export source-coverage fractions are unset: no trustworthy denominator was available; fields stay null."
  }
}
```

Important `source_coverage` fields:

| Field | Meaning when measured |
|---|---|
| `decoded_element_fraction` | `production_walker_elements /` the distinct ElementIds `Global/ElemTable` declares, when walker ≤ that count. (Before 2026-09-22 the denominator was the header's `element_count`, which is a per-release constant, not a count.) |
| `exported_element_fraction` | `building_elements / production_walker_elements` |
| `geometry_element_fraction` | `building_elements_with_geometry / building_elements` |

These are inspection ratios from counts already in the sidecar — not a claim that the file was fully converted.