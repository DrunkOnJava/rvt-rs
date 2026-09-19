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
| `IFCWALL` | Revit 2024 set exact (RE-21); run cut back by the joins the record names (RE-26, RE-29), 351 of 360 world-exact | #238 (9 over-trimmed ends at two true L corners), #23 (2024 ArcWall records) |
| `IFCDOOR` | Revit 2024 set and host binding exact (RE-21, RE-23) | #227 (opening cut from the wall curve) |
| `IFCSLAB` | Revit 2024 set with sketch-line profiles, 80 of 80 (RE-22, RE-25); all 80 storey-bound (RE-27), as are the 20 IFCSHADINGDEVICE plates | nothing slab-specific; #219's plate containment is closed |
| `IFCSPACE` | Revit 2024 room set 116 of 116 with real names, numbers and storeys; body is the record box, equal to the reference envelope on all 116 (RE-29) | #90 (boundary polygon: the record carries no sketch, measured negative) |
| `IFCWINDOW` | Revit 2024 set and host binding exact (RE-21, RE-23); all 6 storey-bound (RE-27) | nothing window-specific; #219's window containment is closed |
| `IFCMATERIALLAYERSETUSAGE` | nothing emitted — layer thicknesses are not near the wall-type record and the reference export is a `ReferenceView_V1.2` file with zero `IfcMaterialLayerSet`, so there is no oracle on this corpus (RE-28) | #88 RE-15-08 |
| `IFCOPENINGELEMENT` | opening, void and fill chain per host (RE-23) | #227 (true opening cut) |

The table lists exactly the types `catalogued_divergence_notes` in
`src/ifc/compare.rs` carries a note for; `IFCCOLUMN` has no note because
it has no remaining gap — since RE-29 its body is the record prism minus
the walls it names, exact on 256 of 256 against Revit's own export.

**The shipped note strings lag this table.** Those constants still quote
the pre-#274 figures — "31 over-trimmed wall ends" for `IFCWALL`, where
RE-29 leaves 9 — and still name #219 as the open tracker for `IFCSLAB`
and `IFCWINDOW` storey containment, which #267 / RE-27 closed for both.
The table above is the measured state; refreshing the constants is a code
change, not a docs one.

Scaffold-only `rvt-ifc` exports (no typed products) will report large
object/entity gaps versus real Revit IFCs — that is expected until the
linked recall work lands, not a bug in the compare tool.

## Library API

`rvt::ifc::compare::{summarize_ifc_step, compare_summaries, format_human_report}`.
