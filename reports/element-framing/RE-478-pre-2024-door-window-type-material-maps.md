# RE-478: the geometry-material map to read for a Revit 2015 to 2023 door or window is the drawn symbol's, which is Revit's material set on 558 of 992 before three rules are applied; the master symbol's is never right where it is not

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-477 (the doors and windows the list join leaves untyped are typed from their native record), RE-473 (the type material map is on every 2015 to 2023 save, in the older id shape), RE-203 (`FamilySymbol.m_geomTag2MaterialId`), RE-204 to RE-206 (when Revit writes `<Unnamed>`, and the materials only hidden forms hold).
**Artefacts:** the spent Revit 2018 project and the 18 held-out projects of Measure (counts only; one of them is the held-out save of the spent project, read as the spent project is and counted once), with Revit's own export of each; BIM Whale's `AdvancedProject` is among them.
**Probe and scorer:** `probe_1603_window_door_type_maps_pre2024` and `tools/re/window_door_type_maps_vs_ifc.py`, at `8b5fd1dde0c4` (Measure 43700046741), archived under `refs/archive/2026-11-15/cited/`. Nothing in the export changes.
**Status:** positive for which symbol, negative for the map alone. The symbol an instance's record draws (`InstanceInfo.m_symbolId`, Revit's type) holds the map that names Revit's materials, and its master symbol's map is never equal where the drawn one is not. Of the 992 doors and windows of the spent project and 17 held-out projects that Revit's export gives materials, the drawn map is Revit's set as written on 558 (603 once Revit's spelling of the unset material is dropped), and on every element of 13 of the 18 projects. The other 389 differ in three ways that need rules the 2015 to 2023 readers do not have: 299 only in whether `<Unnamed>` is written, 81 by names Revit's set does not hold, and 9 by one material Revit names that none of the four records name. A reader that wrote the map as it stands would be wrong on those 389; the next reader is the rules, not the map.

## 1. The question

RE-473 found the type material map on every Revit 2015 to 2023 save, in a shape the reader does not take, and found that the 70 windows of #1603 named no type to read it from. RE-477 gave them the type from their native record. A door or window's record names two symbols: the one it draws (`m_symbolId`, which is Revit's Tag of the type) and its master (`m_masterSymbolId`, which holds the type's name). Which of the two holds the map that equals Revit's material set for the element, and where it does not, why?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints the map of the drawn symbol and of the master symbol (each in either id shape, geometry mapped to no material (-1) as one more entry), and the materials of the file that the drawn symbol's record, the master's, the family's and the instance's name anywhere but in the map. The scorer compares the drawn map with Revit's material set for the same Tag (the occurrence's, or its type's, through IfcOpenShell) and puts each element in the first of these that holds:

- **equal:** the same names, with `<Unnamed>` for geometry mapped to no material;
- **equal with the unset material spelled apart:** the same once the white space around each of Revit's names is dropped (Revit 2018's IFC2X3 export names it " <Unnamed>");
- **unset material only:** the same but for whether `<Unnamed>` is in the set;
- **Revit more:** Revit's set holds names the map does not, found in the drawn symbol's record, the master's, the family's or the instance's, or in none of them;
- **drawn more:** the map holds names Revit's set does not;
- **apart.**

Counts only for the held-out projects (`tools/ci/scrub-holdout-results.sh` keeps the scorer's count lines); the spent project is listed per element with `--list`.

## 3. Measured

**Reach (row 1).** Every door and window Revit gives materials has both symbols' maps read: on the spent project 70 of 70 windows and 4 of 4 doors (2 of them members of a model group); on every held-out project the read count equals the element count. The scorer skips an element Revit gives no material: the spent project's other 64 doors have a map of unset values only (the probe's lines) and no material in Revit's export.

**The drawn symbol against Revit's set (rows 2 and 3).** Doors and windows Revit gives materials, per kind (all 992: 630 windows and 362 doors; equal as written 558, spelled apart 45, unset material only 299, drawn more 81, Revit more not found 9, apart 0):

| Project | Elements | Equal | Spelled apart | Unset material only | Revit more, not found | Drawn more |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Spent Revit 2018, windows | 70 | 2 | 45 | 14 | 9 | 0 |
| Spent Revit 2018, doors (2 of the 4 grouped) | 4 | 4 | 0 | 0 | 0 | 0 |
| Held-out Revit 2020, windows | 29 | 29 | 0 | 0 | 0 | 0 |
| Held-out Revit 2020, doors | 27 | 4 | 0 | 22 | 0 | 1 |
| BIM Whale Revit 2021, windows | 263 | 0 | 0 | 263 | 0 | 0 |
| BIM Whale Revit 2021, doors | 126 | 116 | 0 | 0 | 0 | 10 |
| Held-out A, windows | 110 | 56 | 0 | 0 | 0 | 54 |
| Held-out A, doors | 140 | 126 | 0 | 0 | 0 | 14 |
| Held-out B, windows | 16 | 16 | 0 | 0 | 0 | 0 |
| Held-out B, doors | 13 | 11 | 0 | 0 | 0 | 2 |
| Held-out C, windows | 21 | 21 | 0 | 0 | 0 | 0 |
| Held-out C, doors | 16 | 16 | 0 | 0 | 0 | 0 |
| The other 12 held-out projects (ten saves of two projects, and two others), windows and doors | 1 to 12 and 1 to 4 each | all | 0 | 0 | 0 | 0 |

No element is apart. "Revit more" found in the drawn symbol's record, the master's, the family's or the instance's: 0 in every project, so the 9 windows' extra material is named in none of the four records.

**The master symbol (row 2).** On every project the number of elements whose master map equals Revit's set is the number whose drawn map does, and the number where both are equal is the same: the master is never right where the drawn symbol is wrong. Where the drawn symbol adds a nested component's materials (RE-203: the spent project's windows add `LKS_Ceramico_Fachada` and a shutter box to the glass and frame), the master symbol's map is the frame and glass alone, and Revit's set holds the nested materials.

**Group members (row 4).** The spent project's 2 grouped doors: the drawn symbol is a copy of the master (RE-477), their maps are equal to each other and to Revit's, 2 of 2.

**Elements with no map (row 5).** None: every counted door and window has a map read on both symbols.

## 4. The registered rows

Registered on #1603 before the first run (2026-11-15), read from Measure 43700046741.

1. **Reach.** Reported above: 70 of 70 windows and 4 of 4 doors on the spent project, all on the held-out projects.
2. **Which symbol.** Not met as registered (one symbol equal everywhere it is read, none different). The drawn symbol is the symbol, and the master's is never better. As written, the drawn map equals Revit's set on 2 of the spent project's 70 windows; 47 once Revit's leading-space spelling is dropped; 61 once whether `<Unnamed>` is in the set is not counted; the last 9 have a material Revit names that none of the four records name.
3. **Held-out.** Reported in the table. The drawn map is exactly Revit's on every window and door of 13 of the 18 projects (the spent project and 17 held-out ones; the spent project is not among the 13); on the other five it is the "unset material only" and "drawn more" cases below.
4. **Group members.** Met: 2 of 2.
5. **No map.** Met: 0.

## 5. What this decides and does not

- **Decided.** The map to read is the one on the symbol the instance's record draws (`m_symbolId`, the type RE-477 writes), not on its master. For a model-group member the two are the same material set (2 of 2).
- **Not decided: when `<Unnamed>` is written.** 14 of the spent project's windows, all 263 of BIM Whale's windows and 22 of the held-out Revit 2020 project's 27 doors are "unset material only": the map holds geometry mapped to no material and Revit's set has no `<Unnamed>`. On Revit 2024 and later this is RE-204 and RE-206 (a family whose unset geometry is hidden or void has none), read from `native_family_forms`, which takes the 2024 shape of an ElementId only. That the same rule decides these 299 elements is not measured here.
- **Not decided: the drawn symbol's extra materials.** 54 of one held-out project's 110 windows, and 14, 10, 2 and 1 doors of four others have names in the map that Revit's set does not. On Revit 2024 and later, a material only hidden forms hold is not exported (RE-205), also read from the family forms. Not measured here.
- **Not decided: the 9 windows with one more material.** Six types of the spent project, 9 windows (every window of those types), have `LKS_VEN_MADERA` in Revit's set, in none of the drawn symbol's, master's, family's or instance's records outside the map. On Revit 2024 and later the instances of a type with unset geometry take their material parameters (`attach_partly_unset_materials`, columns and products only). Whether these windows' frames take it from a parameter is not measured.
- **Not decided: the unset material's spelling.** Revit 2018's IFC2X3 export of the spent project names it " <Unnamed>" on 52 of the 70 windows. `<Unnamed>` is the spelling RE-149 measured on Revit 2024. The other projects' counts show no such name; that other Revit 2015 to 2018 exports spell it so rests on this one project.
- **What follows.** A reader that wrote the drawn map as it stands would write a set different from Revit's on 389 of the 992 elements, and `<Unnamed>` on the spent project's 64 doors that Revit gives no material. The next reader is therefore the family forms in the older id shape (when `<Unnamed>` is written, which materials only hidden forms hold) and the partly unset types' material parameters, each measured against these counts, with the type map in the older id shape (RE-473) last. Whether the family documents hold their forms in the same members before Revit 2024 is not measured.

## 6. Reproduce

```bash
# the probe at 8b5fd1dde0c4, the spent project and every held-out project (the scorer's counts for held-out projects need the scrub of this change; main holds it once it lands)
gh workflow run measure.yml --ref main -f ref=8b5fd1dde0c437e82f577a8991e911b931547cb1 -f base=none -f holdout=true \
  -f models='spent-* holdout-*' -f scorers=none -f probe=probe_1603_window_door_type_maps_pre2024 -f probe_scorer=window_door_type_maps_vs_ifc
# each model's probe.txt (the spent project's probe_score.txt lists the elements that differ) holds the `wmap` lines; counts.txt of a held-out model holds the `window door type maps:` lines
```

Four runs preceded it and none of their counts is used. 43696501001 scored nothing, since Measure passes every probe scorer a `--list` flag the scorer refused; 43697495164 and 43698470056 compared names with every space written as an underscore, which cannot be told from one in a name like `LKS_ALU`, and the first of them lost the held-out counts to the scrub of `main`, which does not keep these lines; 43699387932 named the kinds above but not where Revit's extra materials are, which 43700046741 does.
