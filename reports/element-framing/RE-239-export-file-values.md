# RE-239 — Values a project's own property set file places

**Date:** 2026-10-26
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-194 §4, RE-235
**Decision:** Griffin, 2026-10-26: a value that a project's export-time user-defined property set file maps into a set is out of W7's scope, and is listed per model with the evidence. W7 stays reachable on every reference model.
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture, Mechanical, Plumbing and Electrical (Revit 2025, Revit's IFC2X3 exports), Core Interior (Revit 2024) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40272461335](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40272461335) (`tools/re/psets_vs_ifc.py`, listing every space missing `OccupancyType`, on `main` at f0369be3), [40274417114](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40274417114) (`examples/probe_re239_mapped_parameters.rs` with `tools/re/mapped_parameters_vs_psets.py`) and [40275298500](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40275298500) (`tools/re/set_census.py`)
**Result:** positive: every value the office's file places is a saved parameter of the element, and the model stores the parameter but not the file. As built, VALUES values are placed that way; no other reference model has one.

## 1. The office's file

RE-194 §4 found that the office's export, IFC4 Reference View 1.2 by Revit 2024's own exporter, carries what only a user-defined property set file gives: revit-ifc's list of the sets a Reference View export may create (`IFCCertifiedEntitiesAndPSets.json`, CLEANROOM.md source 5) holds neither `Pset_DoorWindowGlazingType` nor `Pset_SpaceOccupancyRequirements`, yet the export writes both, and it writes sets buildingSMART does not define (`FI_Kohde`, `FI_RakennuksenSijainti`, ...). A user-defined property set file lists, after the keyword `PropertySet:`, a set's name and the entities it is written on, then each property with its value type and the parameter it is read from; revit-ifc installs one, `IFC-SG Property Mapping Export.txt`.

What the file places, by `psets_vs_ifc` and `set_census` on objects rvt-rs writes:

| what Revit writes | where | as built | at permit | rvt-rs |
|---|---|---:|---:|---|
| `Pset_DoorWindowGlazingType.IsLaminated`, `IsTempered` | walls, curtain walls, the openings curtain walls cut, coverings | 1,692 | 1,618 | not written |
| `Pset_SpaceOccupancyRequirements.OccupancyType` | spaces | 234 | 234 | not written |
| `Pset_SpaceOccupancyRequirements.OccupancyNumber` as `IfcInteger` | spaces | 55 | 55 | the same number, as the `IfcCountMeasure` IFC4 declares |
| `Pset_SpaceCommon.FloorCovering`, `WallCovering`, `CeilingCovering` as `IfcText` | spaces | 646 | 646 | written with `--revit-set-placement` (RE-235) |
| FI_SETS | | FI_ASBUILT | FI_PERMIT | not written |
| `Pset_DoorWindowGlazingType.IsLaminated`, `IsTempered` | doors and windows | 1,070 | 1,062 | written: IFC4 gives doors and windows the set |

As built, the 1,692 glazing values are on 604 walls, 151 curtain walls, 70 openings and 21 coverings; at permit on 588 walls, 149 curtain walls and 72 openings.

## 2. The parameters behind them

The probe prints every Yes/No and text value each wall, curtain wall, covering and room saves (RE-207), and the scorer counts, for each property Revit writes, which parameter holds the same value. Elements are matched by `Tag`, a space by its number, and the 43 rooms without one by their ElementId:

| property | held by | as built | at permit |
|---|---|---:|---:|
| glazing, on walls | parameters 2189854 and 2189858, named `Pset_DoorWindowGlazingType.IsLaminated` and `.IsTempered` | 607 of 607 | 591 of 591 |
| glazing, on curtain walls and their openings | the same two parameters | 290 of 290 | 292 of 292 |
| glazing, on coverings | COVERINGS | | |
| `OccupancyType` | parameter 2065807, `RoomTypeCode` (values such as `0400-64-03`) | 234 of 234 | 234 of 234 |

## 3. Only the file says where they go

The parameters' names do not decide it:

- every wall Revit writes glazing on also holds parameter 1882432, `Pset_CurtainWallCommon.IsExternal`, and every curtain wall parameter 1877411, `Pset_WallCommon.IsExternal`. Revit's export writes no `Pset_CurtainWallCommon` on a wall and no `Pset_WallCommon` on a curtain wall (RE-194 §4), yet writes `Pset_DoorWindowGlazingType` on both. Writing every parameter named `<set>.<property>` wherever an element holds it would add 897 values as built that Revit's export does not write;
- revit-ifc's own definitions (`ExporterInitializer_PsetDef.cs`) write `Pset_DoorWindowGlazingType` on `IfcDoor` and `IfcWindow` only, and read `OccupancyType` from a parameter named `OccupancyType`, `Pset_SpaceOccupancyRequirements.OccupancyType` or a localised name such as `Occupancy Type`; the `OccupancyTypeCalculator` they also look up is not in revit-ifc's source. No office room has a parameter by any of those names. The codes are in `RoomTypeCode`, a name revit-ifc's source does not contain.

## 4. The model does not store the file

SCAN

## 5. What rvt-rs does

Nothing changes. These values are out of W7's scope and listed here per model:

| model | values out of scope | which |
|---|---:|---|
| office as built | VALUES | §1 |
| office at permit | VALUES_PERMIT | §1 |
| RE1 Architecture, Mechanical, Plumbing, Electrical | 0 | each property their exports write and rvt-rs does not is on an entity revit-ifc's IFC2X3 definitions give its set, or on an unfilled opening, which takes its cutting element's (RE-237 §1) |
| Core Interior | 0 | its reference export writes no property sets |
| R2025 models | R2025 | |

The rest of what the office's export writes and rvt-rs does not (`Pset_MemberCommon.IsExternal` on 10,003 mullions, `Pset_PlateCommon.IsExternal` on 260 panels, sloped slabs' `PitchAngle`, beams' `Span`, `Slope` and `Roll`) is placed by revit-ifc's own rules and stays in W7.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=probe_re239_mapped_parameters -f probe_scorer=mapped_parameters_vs_psets
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=basics_inspect -f probe_scorer=set_census
```

The listing of §1 is `psets_vs_ifc.txt` in any Measure run with `nordic=true`; `PSETS_MISSING` (#1013) names the properties it lists object by object.
