# RE-230 — A curtain wall type's name

**Date:** 2026-10-25
**Issues:** #35 (parameters), #322 (system-family types), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40240930743](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40240930743), [40242398097](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40242398097) and [40243655766](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40243655766) (`examples/probe_re230_curtain_wall_types.rs`), and [40245093306](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40245093306) (this change against `main`)
**Result:** positive: every curtain wall of the office is named and typed as Revit's export names and types it.

## 1. What was missing

rvt-rs named the office's 220 curtain walls `CurtainWall-<ElementId>`, where Revit's export names them `Curtain Wall:<type>:<ElementId>` with `ObjectType` `Curtain Wall:<type>`, and wrote none of their `Pset_CurtainWallCommon.Reference` and `Pset_EnvironmentalImpactIndicators.Reference`, which hold the type's name.

## 2. The type

A wall takes its type the way every system-family element does (#322, RE-44): the one type-definition record its element record's reference list names (RE-28). Every curtain wall's record names exactly one wall type definition, as every wall's does (220 of 220 and 607 of 607), and the curtain walls name five.

None of the five has the name a basic wall type gives itself in its serialised data (RE-44): the probe finds no framed string after any place their ElementIds occur, all of which are references. Their native records do have one. Each is a `NewCurtainWallType`, and its `SymbolInfo.m_name` is the type's name:

| type | `SymbolInfo.m_name` |
|---:|---|
| 451405 | US-8 |
| 451406 | VS-7 |
| 1906499 | JSV-2 |
| 2744609 | VS-8 |
| 3299256 | JSV-1 |

## 3. What rvt-rs does

`native_walls::curtain_wall_type_names` reads `SymbolInfo.m_name` of each picked type whose serialised data gave no name and whose native record is a `NewCurtainWallType`. The curtain wall then takes it as its type's name; its family is Curtain Wall, as RE-63 has it, and its type object (RE-110) is written.

## 4. Measured

`names_vs_ifc`, `psets_vs_ifc` and `type_objects_vs_ifc`, this change against `main`, on both office files:

| | before | after |
|---|---:|---:|
| curtain walls named as Revit names them | 0 of 220 | 220 of 220 |
| `ObjectType` Revit's | 0 of 220 | 220 of 220 |
| `Pset_CurtainWallCommon.Reference` equal | 0 of 220 | 220 of 220 |
| `Pset_EnvironmentalImpactIndicators.Reference` equal (all elements, as built) | 12,349 of 12,693 | 12,569 of 12,693 |
| curtain walls typed, with Revit's entity, Tag and GlobalId | 0 of 220 | 220 of 220 |

No value differs that did not before, and none is added that Revit's export does not write. Core Interior, Einhoven and the RE1 models export byte for byte as before.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f probe=probe_re230_curtain_wall_types
```
