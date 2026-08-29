# IFC comparison tooling (`rvt-ifc-compare`)

M5-05 / #41.

Compare an `rvt-rs` IFC4 STEP export against a reference IFC (typically a
Revit IFC export of the same model). The tool is a lightweight STEP
summarizer — not a full EXPRESS validator and not a geometry mesh diff.

## Usage

```bash
# Human summary on stdout
./target/release/rvt-ifc-compare rvt-export.ifc revit-reference.ifc

# JSON report for CI / notebooks
./target/release/rvt-ifc-compare rvt-export.ifc revit-reference.ifc \
  --json /tmp/ifc-compare.json

# Fail with exit code 2 when structural deltas exist
./target/release/rvt-ifc-compare a.ifc b.ifc --fail-on-diff
```

Dimensions compared:

| Dimension | Source |
|---|---|
| Entity-type counts | DATA-section `TYPE(...)` histogram |
| Storeys | `IfcBuildingStorey` Name + elevation |
| Bounding box | Axis-aligned extents of `IfcCartesianPoint` |
| Objects | Product entities (`IfcWall`, `IfcDoor`, …) Name |
| Materials | Distinct `IfcMaterial` names |
| Property keys | Distinct `IfcPropertySingleValue` Name values |

## Known divergences

When a delta touches a type that is still below the RE-15 / CLASS recall
targets, the JSON/human report attaches a note linking the open issue:

| IFC type | Tracking |
|---|---|
| `IFCWALL` | #81 RE-15-01 |
| `IFCDOOR` | #82 RE-15-02 |
| `IFCSLAB` | #83 RE-15-03, #87 RE-15-07 |
| `IFCSPACE` | #84 RE-15-04, #90 RE-15-10 |
| `IFCWINDOW` | #91 CLASS-11 |
| `IFCMATERIALLAYERSETUSAGE` | #88 RE-15-08 |
| `IFCOPENINGELEMENT` | #89 RE-15-09 |

Scaffold-only `rvt-ifc` exports (no typed products) will report large
object/entity gaps versus real Revit IFCs — that is expected until the
linked recall work lands, not a bug in the compare tool.

## Library API

`rvt::ifc::compare::{summarize_ifc_step, compare_summaries, format_human_report}`.
