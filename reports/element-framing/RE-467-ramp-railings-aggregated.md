# RE-467: a railing that names a ramp is aggregated under it when it has no level of its own

**Date:** 2026-11-14
**Issues:** #1988; follows RE-390 (a stair's railings are aggregated under it) and RE-402 section 6.1 (the BIM4LCA housing, where two railings per file are not)
**Probes:** `probes/examples/probe_re390_railing_stairs.rs` (every railing's `BaseRailing` fields and the stair or ramp it names), `probes/examples/probe_1988_ramp_records.rs` (every `Ramp` and `Level` record's fields)
**Artefacts:** the BIM4LCA housing (the six files; Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and office (concrete as built; timber at permit); Autodesk's Snowdon Towers Architectural 2024 sample and Revit's IFC4 export of it (local only)
**Runs:** probes [43579937639](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43579937639) (housing as built), [43580675541](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43580675541) (office), [43581112070](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43581112070) and [43581948235](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43581948235) (ramp and level records); Measure [43582910387](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43582910387) (the rule alone), [43585759293](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43585759293) (the rule with the saved-mesh repair of section 6); Snowdon [43585762750](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43585762750)
**Status:** Positive, narrow. Revit's export aggregates a ramp's railings only when it exports the ramp as a container, which the file does not say; the railings that name a ramp and carry no level of their own are Revit's on every project measured, and the ones that carry a level are not.

## 1. What Revit's export does

For a railing hosted on a stair or a ramp, `revit-ifc` (CLEANROOM.md source 5; read, not copied) decides in `RailingExporter` (`GetStairOrRampHostId`): the railing joins its host's aggregate only when the host is already in the stair-and-ramp container cache (`StairRampContainerInfoCache`). A ramp is entered in that cache by `RampExporter.Export` when `StairsExporter.ExportLegacyStairOrRampAsContainer` answers for it, with an `IfcRampFlight` per run; otherwise `RampExporter` writes a plain `IfcRamp` with its own body and the railings stay elements of their storey. Which ramps the legacy answer covers is decided inside Revit.

## 2. What the files show

| Project | Ramp | Railings that name it | Revit's export |
|---|---|---|---|
| BIM4LCA housing, all six files | 1347699 | 1347781, 1663091 | an `IfcRamp` whose parts are an `IfcRampFlight` (Tag 1347699) and both railings |
| BIM4LCA office | 1439414, 1440087, 1465824, 1465840 | 2 each, 8 in all | an `IfcRamp` with no part; the 8 railings are plain elements of their storey |
| BIM4LCA office | 1440596 | none | an `IfcRamp` with one `IfcRampFlight` |
| Snowdon Towers Architectural | 832422, 1501264 | 2 each, 4 in all | an `IfcRamp` with an `IfcRampFlight` and both railings |

rvt-rs wrote every one of those `IfcRamp`s with no part, so the 2 railings per housing file (6 files) and the 4 of Snowdon Towers were plain where Revit aggregates them.

## 3. What tells them apart

The records of the ramps hold the same classes of object (`Ramp`, `RampBaseGStep`, `RampRunLengthGStep`, `GeomStepList` and three parameter sets), so a ramp's record does not say which it is. Its railings differ:

| `BaseRailing` field | the housing's 2 (aggregated) | the office's 8 (plain) |
|---|---|---|
| `m_stairsId` | the ramp | the ramp |
| `m_stairsComponentId` | none | none |
| `m_assocLevelId` | none | Level_01 |
| `m_placementOffset` | 0.0 | -0.0509 ft |
| own IFC parameters | none | `HANDRAIL`, `IfcRailing` |

The office's 8 are railings drawn on a level (Level_01) beside ramps of the Basement; the housing's 2 are the railings the ramp generated. `m_assocLevelId` alone is the field that fits every ramp railing measured, and it is not a rule for stairs: 3 stair railings of the housing and office that carry a level of their own are aggregated by Revit (1749890 and 2902063 on their stair's top level, 2222756 on a level that is neither its stair's base nor its top).

## 4. A rule that fails

"A railing is plain unless its level is its host's base or top level" fits 74 of the 75 railings that name a stair or a ramp on the housing and the office as built. The miss is the timber housing's railing 2222756, on a stair from level 694 to level 417999, whose own level 341662 Revit aggregates anyway. Level 341662 is a placeholder at z = 0 ft (plane origin -46.23, -42.72, 0) that the office's four plain ramps are also based on, with a base offset of 182.087 ft; the housing's ramp and the office's 1440596, which are aggregated, are based on real levels with offset 0. That separates the 6 ramps whose records were read (2 aggregated, 4 plain), and it is not used: it names a level, not a mechanism.

## 5. What rvt-rs writes

A railing whose record names a `Ramp` (`m_stairsId`), names no component of it and has no level of its own (`m_assocLevelId`) is aggregated under that ramp on Revit 2024, the release of the BIM4LCA files and of the Snowdon sample. The stair rule of RE-390 is unchanged. A ramp keeps its own body and its saved mesh: it is not a whole the saved-mesh pass skips, as a stair or a curtain wall drawn by its parts is. A railing that carries a level stays plain, as the office's 8 do, and so does one that names a ramp on another release, which none measured.

## 6. Measured

Measure [43585759293](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43585759293) (`90817e75`, the rule with the saved-mesh repair, against `main` at `e8f2eeff`, unfiltered, 126 models): 119 exports are identical to `main`'s. The 7 that differ are the six BIM4LCA housing files and the Snowdon Towers Architectural sample, and on the models with a Revit export only two scorers move, both on the six housing files. The office files, whose 8 railings carry a level, are identical, as are the R2025 models and every other sample.

| Housing file | `IfcRailing` only Revit aggregates | same whole | entities |
|---|---|---|---|
| concrete, as built | 2 to 0 | 14 to 16 | 10,734 to 10,735 |
| concrete, permit | 2 to 0 | 14 to 16 | 10,396 to 10,397 |
| terrain, concrete | 2 to 0 | 0 to 2 | 3,477 to 3,478 |
| terrain, timber | 2 to 0 | 0 to 2 | 3,483 to 3,484 |
| timber, as built | 2 to 0 | 21 to 23 | 12,246 to 12,247 |
| timber, permit | 2 to 0 | 21 to 23 | 11,777 to 11,778 |

- **Aggregates.** Every railing Revit aggregates under the ramp is under it, and no file lists a railing that only rvt-rs aggregates. The one added entity of each file is the `IfcRelAggregates` of the ramp and its two railings; the 2 railings leave their storey's contents (the elements reaching their storey fall from 1,124 to 1,122 on the concrete file as built).
- **Bodies.** `mesh_bodies_vs_ifc` reads a whole through its parts. Revit's ramp is an `IfcRampFlight` and both railings (27 shells on the concrete file as built), and rvt-rs's was the ramp alone (1 shell): volume ratio 0.4907 and 3.248 ft to the farthest vertex. With the railings its parts, rvt-rs's ramp reads 27 shells too: ratio 0.5109 (Revit's export holds the flight's faces twice, which doubles its signed volume) and 0.0066 ft. The terrain timber file reads 0.4983 and 3.248 ft, then 0.5020 and 0.0065 ft. The ramp's own body is not changed: box bodies, saved meshes and every other scorer line are identical to `main`'s. The list of the 15 worst vertex readings loses the ramp's 3.248 ft and takes a downpipe (`IfcCovering` 1766041, ratio 1.0054, 0.0073 ft) at its end.
- **First run.** Measure [43582910387](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43582910387) (the rule alone) showed what the repair is for: the ramp became a whole drawn by its parts, the saved-mesh pass skipped it, and a box body took the place of its saved mesh (box bodies 58 to 59, saved meshes 853 to 852; the semantic check failed on `IfcRamp`).
- **Snowdon Towers** (no Revit export on Measure; Measure [43585762750](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43585762750), `keep_ifc`, read against the local IFC4 export). Each export holds 142 `IfcRailing` entities, 131 distinct ElementIds in both. 80 are aggregated in both (76 under stairs and 4 under the sample's two ramps), the 69 distinct ElementIds of them each under the same whole as in Revit's export, and the other 62 are plain in both (11 ElementIds appear twice, the multistory stair copies of #1977). The file gains 2 entities, one aggregate for each ramp.

## 7. Not shown

- **The container criterion.** Why Revit makes the housing's ramp, the office's 1440596 and Snowdon's two a container and the office's other four a plain `IfcRamp` is not in the file: the ramp records hold the same classes, and the field that differs between them is the level they are based on (a real level against the z = 0 placeholder), which is a guess from the 6 ramps of two projects whose records were read (Snowdon Towers' two are local only and were not). A ramp with railings and no level of their own that Revit leaves plain would be aggregated here and not there; none of the 8 ramps measured is one.
- **The `IfcRampFlight`.** Revit writes one under each container ramp (the housing's 1, the office's 1, Snowdon's 2); rvt-rs writes none, for any ramp. The ramp keeps its own body.
- **Snowdon Towers on Measure.** Its Revit export is local only, so no scorer compares it: the railings of section 6 were matched by ElementId and by the whole each `IfcRelAggregates` names, with a text-level reader over the sample's kept export (Measure 43585762750, `keep_ifc`) and the local IFC4 export. The two ramps of the sample are the only ones that run measured outside the BIM4LCA files, and their records were not read (section 3 reads the BIM4LCA records only).

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<head> -f base=none -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-office-as-built" -f scorers=aggregates_vs_ifc \
  -f keep_ifc=true -f probe=probe_re390_railing_stairs -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=<head> -f base=none -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-office-as-built nordic-housing-timber-as-built" \
  -f scorers=none -f probe=probe_1988_ramp_records -f probe_scorer=-
```

`<head>` is the merge commit of the pull request that carries this report.
