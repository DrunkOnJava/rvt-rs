# RE-228 — A wall's `ExtendToStructure`

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports) and RE1 Architecture (Revit 2025, Revit's IFC2X3 export), as Measure fetches them
**Runs:** Measure [40230063078](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40230063078) and [40231468767](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40231468767) (`examples/probe_re228_wall_record_scalars.rs` with `tools/re/record_scalars_vs_ifc.py`), and [40232649595](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40232649595) (this change against `main`)
**Result:** positive, exact on every wall Revit's export writes it on.

## 1. What was missing

Revit's export writes `Pset_WallCommon.ExtendToStructure` on every wall of the BIM4LCA office, true on 465 of the 607 as built, and rvt-rs wrote none. revit-ifc's `ExtendToStructureCalculator` (CLEANROOM.md source 5) takes it from Revit's `ExporterIFCUtils.IsWallJoinedToTop`, whose public source does not show what it reads.

## 2. Where the file holds it

The probe prints every scalar of the first 400 objects of each wall's native record, and the scorer ranks each field by how well one of its values tells Revit's true from false. One object stands out: `JoinToRoofGStep`, the step that joins a wall's base or top to a roof or floor, with `m_baseOrTop` 0 (base) or 1 (top).

| walls, as built | count | Revit's `ExtendToStructure` |
|---|---:|---|
| no `JoinToRoofGStep` | 127 | false on all |
| one, at the top | 432 | true on all |
| two, base and top | 29 | true on all |
| two, both at the top | 4 | true on all |
| one, at the base | 15 | false on all |

So a wall extends to structure where one of its `JoinToRoofGStep`s has `m_baseOrTop` 1: 465 true and 142 false, Revit's on all 607. At building permit the same rule gives 472 and 117, all 589.

## 3. What rvt-rs does

`native_walls_and_top_joins` reads, in the same pass as RE-222's native walls, whether each `SWall` or `ArcWall` record has such a step. `add_extend_to_structure` writes it in the wall's `Pset_WallCommon`. A wall whose record is not read gets nothing.

## 4. Measured

`psets_vs_ifc`, this change against `main`:

| file | `ExtendToStructure` equal | different | missing |
|---|---:|---:|---:|
| office as built | 607 of 607 | 0 | 0 |
| office at permit | 589 of 589 | 0 | 0 |
| RE1 Architecture | 7 of 7 | 0 | 0 |

Core Interior's walls carry it too. Revit's export of Core Interior writes no property sets, so it has no value to compare. Einhoven and RE1 Electrical, Mechanical and Plumbing export byte for byte as before.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref feat/wall-extend-to-structure -f ref=feat/wall-extend-to-structure -f base=none \
  -f nordic=true -f probe=probe_re228_wall_record_scalars -f probe_scorer=record_scalars_vs_ifc
gh workflow run measure.yml --ref main -f ref=feat/wall-extend-to-structure -f base=main -f nordic=true
```
