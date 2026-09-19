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

When a delta touches a type whose recovery still has a known gap, the
JSON/human report attaches a note naming what is recovered and the open
issue that tracks the remainder:

| IFC type | Recovered | Remaining gap tracked in |
|---|---|---|
| `IFCWALL` | Revit 2024 set exact (RE-21), joins cut back (RE-26) | #238 (31 over-trimmed ends), #23 (2024 ArcWall records) |
| `IFCDOOR` | Revit 2024 set and host binding exact (RE-21, RE-23) | #227 (opening cut from the wall curve) |
| `IFCSLAB` | Revit 2024 set with sketch-line profiles (RE-22, RE-25) | #219 (plate storey containment) |
| `IFCSPACE` | Revit 2024 room set, names, numbers and storeys exact; record-box body (RE-29) | #90 (boundary polygon) |
| `IFCWINDOW` | Revit 2024 set and host binding exact (RE-21, RE-23) | #219 (storey containment) |
| `IFCMATERIALLAYERSETUSAGE` | nominal type thickness only | #88 RE-15-08 |
| `IFCOPENINGELEMENT` | opening, void and fill chain per host (RE-23) | #227 (true opening cut) |

Scaffold-only `rvt-ifc` exports (no typed products) will report large
object/entity gaps versus real Revit IFCs — that is expected until the
linked recall work lands, not a bug in the compare tool.

## Library API

`rvt::ifc::compare::{summarize_ifc_step, compare_summaries, format_human_report}`.
