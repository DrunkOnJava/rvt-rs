# RE-239 — Values a project's own property set file places

**Date:** 2026-10-26
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Decision:** Griffin, 2026-10-26: a value that a project's export-time user-defined property set file maps into a set is out of W7's scope, and is listed per model with the evidence. W7 stays reachable on every reference model.
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), RE1 Architecture, Mechanical, Plumbing and Electrical (Revit 2025, Revit's IFC2X3 exports), Core Interior (Revit 2024) and puzzbobb's R2025 models (Revit 2025, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40272461335](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40272461335) (`tools/re/psets_vs_ifc.py` listing every space missing `OccupancyType`, on `main` at f0369be3) and RUN (`examples/probe_re239_mapped_parameters.rs` with `tools/re/mapped_parameters_vs_psets.py`)
**Result:** positive: on the office, 1,926 values as built and 1,852 at permit are a saved parameter of the element placed by the export's own property set file. The model stores the parameter, not the file. No other reference model has such a value.

## 1. Values outside revit-ifc's own placements

`psets_vs_ifc` names the entities each missing value is on. On the office, two groups are placed by neither buildingSMART's IFC4 definitions nor revit-ifc's own rules (CLEANROOM.md source 5; the name-prefix match of RE-235 §1):

| what Revit writes | where | as built | at permit |
|---|---|---:|---:|
| `Pset_DoorWindowGlazingType.IsLaminated`, `IsTempered` | walls, curtain walls, the openings curtain walls cut, coverings | 846 each | 809 each |
| `Pset_SpaceOccupancyRequirements.OccupancyType` | spaces | 234 | 234 |

As built, the 846 are on 604 walls, 151 curtain walls, 70 openings and 21 coverings; at permit on 588 walls, 149 curtain walls and 72 openings. `Pset_DoorWindowGlazingType` applies to doors and windows, and no name among them begins with `IfcWall`, `IfcCurtainWall`, `IfcOpeningElement` or `IfcCovering`. `Pset_SpaceOccupancyRequirements` applies to spaces, but no parameter of the office's rooms is named `OccupancyType` or `Pset_SpaceOccupancyRequirements.OccupancyType`.

## 2. The parameters behind them

The probe prints every Yes/No and text value each wall, curtain wall, covering and room saves (RE-207), and the scorer counts, for each property Revit writes, which parameter holds the same value. Elements are matched by `Tag`, a space by its number:

| property | held by | as built | at permit |
|---|---|---:|---:|
| glazing, on walls | parameters 2189854 and 2189858, named `Pset_DoorWindowGlazingType.IsLaminated` and `.IsTempered` | 607 of 607 | 591 of 591 |
| glazing, on curtain walls (and their openings) | the same two parameters | 290 of 290 | 292 of 292 |
| glazing, on coverings | COVERINGS | | |
| `OccupancyType` | parameter 2065807, `RoomTypeCode` (values such as `0400-64-03`) | 234 of 234 | 234 of 234 |

The 43 rooms without a number are matched by their ElementId, which is their `Tag`.

## 3. The choice is the file's

The same naming gives different results on the same elements:

- every wall the export writes also holds parameter 1882432, `Pset_CurtainWallCommon.IsExternal`, and every curtain wall parameter 1877411, `Pset_WallCommon.IsExternal`. Revit's export writes no `Pset_CurtainWallCommon` on a wall and no `Pset_WallCommon` on a curtain wall, yet writes `Pset_DoorWindowGlazingType` on both;
- revit-ifc's own definitions (`ExporterInitializer_PsetDef.cs`) write `Pset_DoorWindowGlazingType` on `IfcDoor` and `IfcWindow` only, and read `OccupancyType` from a parameter named `OccupancyType`, `Pset_SpaceOccupancyRequirements.OccupancyType` or a localised name such as `Occupancy Type`. The `OccupancyTypeCalculator` they also look up is not in revit-ifc's source. No office room has a parameter by any of those names: the codes are in `RoomTypeCode`, a name revit-ifc's source does not contain.

A user-defined property set file lists, after the keyword `PropertySet:`, a set's name and the entities it is written on, then each property with the parameter it is read from; revit-ifc installs one, `IFC-SG Property Mapping Export.txt`. The office's export was made with such a file, one that puts `Pset_DoorWindowGlazingType` on walls, curtain walls and coverings and reads `OccupancyType` from `RoomTypeCode`. Writing every parameter named `<set>.<property>` wherever an element holds it would instead write the two `IsExternal` sets above: 897 values as built that Revit's export does not write.

## 4. The model does not store the file

SCAN

## 5. What rvt-rs does

Nothing changes. These values are out of W7's scope and listed here per model:

| model | values | which |
|---|---:|---|
| office as built | 1,926 | §1 |
| office at permit | 1,852 | §1 |
| RE1 Architecture, Mechanical, Plumbing, Electrical | 0 | |
| Core Interior | 0 | its reference export writes no property sets |
| R2025 models | 0 | |

The rest of what the office's export writes and rvt-rs does not (`Pset_MemberCommon.IsExternal` on 10,003 mullions, `Pset_PlateCommon.IsExternal` on 260 panels, sloped slabs' `PitchAngle`, beams' `Span`, `Slope` and `Roll`) is placed by revit-ifc's own rules and stays in W7.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f probe=probe_re239_mapped_parameters -f probe_scorer=mapped_parameters_vs_psets
```

The listing of §1 is `psets_vs_ifc.txt` in any Measure run with `nordic=true`; `PSETS_MISSING` (#1013) names the properties it lists object by object.
