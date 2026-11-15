# RE-364: Revit 2015 to 2023 walls along their lines, joined by their join lists

**Date:** 2026-11-06
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-49 and RE-53 (a wall's line and layers), RE-70 (join lists), RE-114 and RE-120 (2023 walls keep their box, cut at their joins), RE-180 and RE-181 (no cross-section word before 2021), RE-280 (native records on 2015 to 2022).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored.
**Scorer:** `tools/re/save_pair_bodies.py`, with its new `--no-openings`; `probes/examples/probe_re364_wall_join_lists_32.rs`.
**Status:** positive against the same samples saved by a later release. **No wall of a 2015 to 2023 file is scored here against a Revit export**, and a later save cannot catch an error it shares. RE-120's measurement, every wall of two 2023 projects as Revit's, was of the cut-back box, which a wall along a model axis keeps on the narrowed head described next. The `racadv-*` pairs and BIMcollab's Revit 2015 example have such exports; they are for Measure, which reports counts only, and none was opened for this work.


**Narrowed in review, 2026-11-08 (the maintainer).** Measure run 42516794247 on the first head scored this rule against Revit's own exports and found two problems the later saves share. On Warmond (Revit 2021, held out, counts only), the wall quantities it added were 91 equal against 45 different for `Height`, 0 against 45 for `NetSideArea`, 22 against 23 for `NetVolume` and 26 against 11 for `GrossFootprintArea`. On Vitoria `ALD26 Before` (Revit 2018), axis-parallel walls with both ends within 0.001 ft of Revit's fell from 160 to 125 of 199, while angled walls' faces rose from 1 to 33 of 48. So on Revit 2015 to 2023: a wall along a model axis keeps its record box cut back at its joins, an angled wall is drawn along its line, a wall's `Height`, areas and volumes are not written (the record is read without its control joins, so what reads as uncut is not known to be), and on the releases with no cross-section word an angled body must be the rectangle its box closes on. The figures in the sections below were measured before this narrowing, on the contributor's first head.

**What those two findings can and cannot show.** Warmond's export went through another tool, and Vitoria's is IFC2X3, so under the standing rule both are counted apart and neither shows a value wrong. The one untouched IFC4 export that scored the first head's wall quantities, `modelo_bim`, had 1 of 4 equal and 0 different. So the quantities are left out because a record on these releases is read without its control joins, not because a Revit IFC4 export disagrees. Leaving them out keeps `main`'s output, and #1535 looks at them again with the angled walls.

**What the narrowed head does, measured on `430a2606`** (Measure run 42522812941, main's side `ab5f8e1c`, unfiltered with samples, held-out models, `racadv` and BIMcollab; the code is that of the landing head, which adds only main's commits and text). 100 of 118 exports are byte for byte main's and 18 move, and `same-project gates` passes.

- Warmond's wall quantities are main's: equal 272, different 5, missing 3,135, values Revit does not write 5.
- On Vitoria `ALD26 Before`, axis-parallel walls return to 160 of 199 with both ends within 0.001 ft. **Its angled walls stay as on main**, faces 1 of 48 and ends 4 of 48, and none takes a line body. The first head's 33 of 48 is not kept: 2018 has no cross-section word, and the check that a body must be the rectangle its box closes on refuses the joint-to-joint fallback those walls took. Between the two heads it is the only change that touches an angled wall's body. No value read on 2015 to 2020 marks a slanted wall, so the check stays.
- A line body reaches 24 walls in 12 sample exports (`rac_basic` of 2015 to 2023, `rst_basic` of 2021 to 2023), and none on Vitoria `ALD26 Before`, `modelo_bim`, `Exemplo_data` or the `racadv` pairs. Attributed afterwards by Measure run 42532072920 (`main` at `1e2d3de4`, held-out counts): a line body reaches four of the five held-out exports that moved, all with a Revit export: bim-whale's Revit 2021 project 259 walls (135 `partition_wall_centreline`, 124 with layer joins), Warmond 6, Gdynia 1, and Frigento (Revit 2020, where the box must close) 2. Vitoria's project takes none. On bim-whale 259 of 390 angled walls have both faces within 0.001 ft of Revit's export, as many as take a line body, though the counts do not pair them wall by wall; that export is IFC2X3, and Warmond's went through another tool, so both are counted apart.
- The `racadv` pairs of 2015 to 2020 are byte for byte main's. So are RE-120's two projects, in Measure run 42525247402 (head `9bef6e08`, main's side `60cb1a81`, `wall_bodies_vs_ifc`, counts only): `Exemplo_data` 17 of 17 and `modelo_bim` 4 of 4 on faces and on ends within 0.001 ft on both sides, so RE-120's scores stand.
- Layer sets gain on three held-out models (counts only): Frigento 10 to 12, all 12 whole; Gdynia 108 to 109, 107 of 108 whole; bim-whale 43 to 302, 300 of 300 whole.

## 1. Before

A wall on 2015 to 2023 kept its record box, cut back at its joins (RE-120), because `attach_wall_layers` turned centreline bodies off for every 32-bit release. RE-114's reason was that "2023's join lists are not read, and a line without them would lose the corners the box already has". On `main`, `rac_advanced`'s walls of `exported.body_sources`:

| | 2023 | 2024 |
|---|---:|---:|
| `partition_wall_centreline_layer_joins` | 0 | 118 |
| `partition_wall_centreline` | 0 | 1 |
| `partition_element_record_join_trimmed` | 176 | 58 |

## 2. The join list's 32-bit layout

`scan_wall_join_partners` failed closed on these releases. The list is there. Wall 148769 of `rac_advanced`, whose partner is `0x0235c5`:

```
2024  07000000 7d9f0b00 02000000 | count 01000000 | ac000000 c5350200 00000000 05000000
2023  07000000 7d9f0b00 02000000 07000000 + 13 zero bytes | count 01000000 | ac000000 c5350200 05000000
```

The count sits 29 bytes past the opening word, and an entry is `u32 · u32 ElementId · u32`, 12 bytes. The tag is the same on 2015 to 2024. What the `u32` at +12 holds is not decoded; the reader requires the 13 bytes after it to be zero.

Every wall's partners against its 2024 save, by `probe_re364_wall_join_lists_32`: `rac_advanced` 146 of 146 on each of its nine saves of 2015 to 2023, 57 of them naming a partner, and `rst_basic` 9 of 9 on 2016 to 2023. `rac_basic` is 46 of 47 on 2016 to 2023, where wall 493879 names no partner and its 2024 save names 627729, and 45 of 47 on 2015, where wall 849032 also names 977133, which its 2024 save does not. `rst_basic` 2015 is another model (RE-181).

## 3. The change

- `partition_compound_structure`: `wall_join_lists_32`, `WALL_JOIN_32BIT_REVIT_VERSIONS` (2015 to 2023). `wall_join_count_offset` and the 64-bit reader are unchanged.
- `attach_wall_layers`: `WALL_CENTRELINE_REVIT_VERSIONS` (2015 to 2027) replaces the 32-bit test. 2008 to 2014 stay as they were.
- 2015 to 2020 have no cross-section word (RE-180, RE-181), so nothing marks a slanted wall there and each is drawn vertical. That is not proved separately: the walls these samples hold are the ones scored below.
- `native_walls_and_top_joins_into` names `NATIVE_RELEASES`, as RE-280's readers do, so 2015 to 2022 take the facts 2023 takes.
- The export's warning says so on 2015 to 2023.

## 4. Bodies against a later save

Each wall the change draws along its line, paired by `Tag`, to 3 mm on the world box and 0.1% on the volume. "No openings" meshes the wall without its voids; door and window cuts are read on 2024 and not on these releases, so "with cuts" also counts that gap, which this change does not touch.

| save | reference | no openings | with cuts |
|---|---|---:|---:|
| `rac_advanced` 2015 to 2020 (each) | 2024 | 107 of 118 | 73 of 118 |
| `rac_advanced` 2021 | 2024 | 99 of 103 | 70 of 103 |
| `rac_advanced` 2022, 2023 (each) | 2024 | 118 of 118 | 80 of 118 |
| `rst_basic` 2016 to 2023 (each) | 2024 | 2 of 2 | 2 of 2 |
| `rac_basic` 2015 to 2022 (each) | 2023 | 17 of 22 | 17 of 22 |
| `rac_basic` 2023 | 2024 | 5 of 18 | 5 of 18 |
| `rme_basic`, `rme_advanced` 2015, 2019 (each) | 2024 | 1 of 1 | 1 of 1 |

`rst_advanced` 2015 and 2019, and `rst_basic` 2015, have no wall drawn along its line by this change.

On `main` the same walls of `rac_advanced` 2023 match on 1 of 118 with no openings, and those of `rst_basic` and `rac_basic` on none.

**The 11 on `rac_advanced` 2015 to 2020** are within 8 mm on the box and 0.21% on the volume. The saves' raw boxes differ from 2024's on those walls too; why is not decoded.

**The 4 on `rac_advanced` 2021** are 15 to 31 mm off, and their raw boxes differ as well. That save also draws 103 walls along their lines, not 118; the other 15 keep their cut-back box, and why is not decoded.

**`rac_basic` is scored against its 2023 save.** 13 of its walls move by up to 3.19 m in the 2024 save, in the raw boxes (`main`'s export) as much as here. Of the 5 that differ from the 2023 save, up to 0.18 m on the box and 2.64% on the volume, 4 are ones that save leaves as cut-back boxes; the fifth, 906885, is drawn along its line on both and is 13 mm off, which is not decoded.

**Two walls end farther from the later save's box than on `main`**: 160150 and 160284 of `rac_advanced` 2015 to 2020, 8 mm where `main` has 6 mm. No wall that matched on `main` stops matching.

## 5. What else changes

Across the 31 sample saves of 2015 to 2023 that have a reference:

- **Nothing outside walls.** No other element changes a property, a storey or a GlobalId, and no element appears or goes.
- **Thickness and materials.** On `rac_advanced` 2023, `Thickness` goes from 84 to 93 of the 93 changed walls that equal 2024's, and the 8 parapets take the layer set usage 2024 writes. `main` gives `rac_basic`'s wall 493790 a thickness of 9.75 where 2024 gives 0.28.
- **Quantities gained.** `Height` on 784 walls, 781 equal to 2024's and 3 not in it. `GrossSideArea` and `NetSideArea` on 481, 462 equal and 19 where 2024 writes none. `GrossFootprintArea`, `GrossVolume` and `NetVolume` the same, except on wall 418079 of `rac_basic` 2015, where `GrossVolume` is 2.42 and 2024's 1.75; that save's own box differs, and why is not decoded. No quantity or property is lost or changes value.
- **One storey.** `rst_basic`'s wall 627729, 2016 to 2023, moves from a storey made from its base elevation to `Default Building`, as the 2024 save has it. No Revit export says which is right.
- **`Length` is left out.** A 2015 to 2023 record is read without its control joins (`joins`, `side_joins`, `exposed`, also `shells` and `type_id`, all empty on wall 143858 of 2023, all present on 2024). `attach_wall_lengths` takes an end with none as joined to nothing, and wrote a `Length` on 44 walls of `rac_advanced` 2023 where 2024 writes none, so it does not run on these releases.
- **Byte identity.** Of 206 local project files of 2008 to 2027, 170 export IFC and diagnostics byte for byte as on `main` (`SOURCE_DATE_EPOCH=1700000000`). The other 36 are files of 2017 to 2023: 32 change their IFC, and 4 only the warning in their diagnostics. No file of 2024 or later, or before 2015, changes.
- **Cost.** No difference above the noise of single runs on this machine, where one export varied between 3 and 6 s.

## 6. Not measured, not read

- Against a Revit export: see the status above.
- RE-127's joined-wall lists: two walls of `rac_advanced` hold the pattern, with count 0 on 2024, so there is nothing to measure the layout against and it is not read. The L joints those lists decide take what the join lists give.
- A record's control joins on 2015 to 2023, and with them `Length`: why the graph holds none is not decoded (#1158).
- A wall's profile openings and end steps (each its own list, 2024), its type's face angles (2024, 2025).
- Door and window cuts.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
S=partition_wall_centreline,partition_wall_centreline_layer_joins
python3 tools/re/save_pair_bodies.py samples/2023-racadvancedsampleproject.ifc samples/2024-racadvancedsampleproject.ifc --sources $S --no-openings
# Section 2: the wall ids are the `Tag`s of the 2024 export's IfcWalls, one file for both saves.
cargo run --profile ci -p rvt-probes --example probe_re364_wall_join_lists_32 -- samples/2023-racadvancedsampleproject.rvt 2023 walls.txt > 2023.txt
cargo run --profile ci -p rvt-probes --example probe_re364_wall_join_lists_32 -- samples/2024-racadvancedsampleproject.rvt 2024 walls.txt > 2024.txt
diff 2023.txt 2024.txt
```

## 8. Addendum, 2026-11-08: angled walls on Revit 2015 to 2020 keep their box (#1535)

On 2015 to 2020 an angled wall keeps its record box unless the box closes on its line's rectangle (section 3), because nothing read on those releases marks a slanted wall. #1535 asked whether the line bodies of the first head (`0aad0b0c`, archived at `refs/archive/2026-11-08/pr1517-first-head`) could come back wall by wall. It was answered on the spent Vitoria `ALD26 Before` (Revit 2018, IFC2X3), the one model whose walls can be listed, against the code before the change.

**Faces gain and ends do not.** A trial that takes an angled wall's faces from its line and its ends from its box (`7180ba95`, Measure run 42532798082) takes `ALD26 Before`'s angled walls with both faces within 0.001 ft of Revit's from 1 to 33 of 48. Its two held-out projects show the same gain: Frigento (Revit 2020, IFC4) from 2 to 69 of 83, and Vitoria's `ALD26 Project` from 1 to 33 of 74, both counts only, and both already scored for this work. Taking each wall's larger error, faces or ends, against its box, 25 walls are better, 15 the same and 8 worse. Those 8 are worse at their ends alone, and their faces are exact.

**What the 8 walls' ends are** (Measure run 42558474888, with a temporary scorer that signs each end's difference: positive where rvt-rs's body runs past Revit's end):

| wall | box: start, end (ft) | line: start, end (ft) |
|---|---|---|
| 705842 | +0.164, +0.164 | +0.278, +0.277 |
| 706031 | +0.163, +0.165 | +0.279, +0.276 |
| 706347 | +0.174, +0.014 | +0.279, -0.002 |
| 706416 | +0.174, +0.009 | +0.277, -0.004 |
| 706468 | +0.169, +0.171 | +0.275, +0.278 |
| 706632 | +0.186, +0.186 | +0.279, +0.279 |
| 706787 | +0.184, +0.184 | +0.278, +0.279 |
| 706109 | +0.000, -0.000 | -0.003, -0.001 |

On 7 of them Revit's wall stops 0.275 to 0.279 ft (about 85 mm) short of the line's end, at one or both ends, and short of the box's end as well. That fits a trim at a join, and it fits a rule for the one wall type as well. All 8 are `LKS_T9 10cm`, and the 58 ends of that type's 29 angled walls under the line body fall on a few values: 12 about 0.28 ft past Revit's, 12 about 0.17 ft past, 4 about 0.39 ft short, 4 0.037 ft short and 26 within 0.02 ft. No step compared Revit's end with a neighbouring wall's face, so neither reading is tested. None of the 48 angled walls has a join reach on these releases. Four other explanations were tried and are ruled out: the wall type's thickness, the wall lying within 5° of a model axis, the run estimate (taking it from the dominant axis left the same 8 worse by the same amounts, run 42541485876) and how the box's run estimates differ. Of the 25 walls the trial improves, only 5 have both ends within 0.01 ft of Revit's. The other 20 have 28 ends beyond 0.01 ft: 16 of them are about 0.17 or 0.39 ft, and the rest are between 0.02 and 0.28 ft.

**Result.** By the second case of #1535's done-when, these walls set the limit. An angled wall of Revit 2015 to 2020 whose box does not close on its line's rectangle stays its box, as on `main`. Which rule gives its ends is not known. If it is a join trim, it needs the control joins these releases' records do not yet give (section 6, #1158). The decision holds either way. The wall quantities the first head wrote on 2015 to 2023 remain separate work under #1535.

## 9. Addendum, 2026-11-09: a wall's `Height` on Revit 2015 to 2023, and why its areas stay out (#1535)

The narrowing above left out a wall's `Height`, areas and volumes on 2015 to 2023 because a record there is read without its control joins, "so what reads as uncut is not known to be". The flags these quantities need do not come from the control joins. `Height` needs the record's `uncut` flag, and the side areas, footprint and volumes need `whole` as well and a body that is the record box cut back at its joins (RE-266, RE-270, RE-292). Both flags come from the `*Step` classes of the wall's native record, which `native_walls_and_top_joins_into` reads on these releases (section 3).

**The steps read as on the later save.** `probe_1535_wall_steps` lists each wall record's step classes and both flags (Measure run 42562621272, probe at `29a382b9`, on Autodesk's samples of every year). Compared wall by wall with the same project's 2024 save (`rac_basic` 2015 to 2022 with its 2023 save): 2,894 of 2,896 walls hold the same step classes, and all 2,896 the same `uncut` and `whole`. The two that differ are `rac_basic` 2015's, with one `JoinEndGStep` where 2023 has two and a `WallEmbedWallGStep`, and three where 2023 has two. Neither changes a flag. `rst_basic` 2015's four walls are not in its 2024 save, and `rst_advanced` has no later save.

**A trial head writes both** (`748eff38`, archived at `refs/archive/2026-11-09/1535-areas-trial-head`; Measure run 42564158277 with `keep_ifc`). Each 2015 to 2023 save against the same project's 2024 save with `tools/re/save_pair_wall_quantities.py`, summed over the 31 saves that have one:

| quantity | equal | different | only the old save, wall in 2024 | only the old save, wall not in 2024 | only 2024 |
|---|---:|---:|---:|---:|---:|
| `Height` | 781 | 0 | 0 | 7 | 10 |
| `GrossSideArea`, `NetSideArea` (each) | 462 | 0 | 203 | 2 | 5 |
| `GrossVolume`, `NetVolume` (each) | 461 | 1 | 203 | 2 | 5 |
| `GrossFootprintArea` | 461 | 1 | 203 | 2 | 1 |

`main` writes none of these on 2015 to 2023. All 31 of those saves' exports move, and none of the 25 sample exports of 2024 to 2027.

**`Height` holds.** No value differs. The 7 on walls the 2024 save lacks are `rac_basic` 2023's 704275, 708088 and 709245 and `rst_basic` 2015's four (another model, RE-181), checked against nothing. Of the 10 only the 2024 save writes, 8 are walls 573660 and 573664 of `rme_basic` and `rme_advanced` 2015 and 2019, which hold curtain grid steps and read `uncut` on both saves, and for which the old save writes no quantity at all; why is not decoded. The other 2 are `rac_basic` 2023's (below). Each is a value left out, not a wrong one.

**The areas and volumes do not.** On 25 walls, 203 wall saves in all, the old save writes all five and the 2024 save, which holds the wall, none. There the 2024 save draws the wall from its centreline, with its layers' joins on 170 of the 203 (`partition_wall_centreline_layer_joins`, RE-71) and without on 33 (`partition_wall_centreline`, RE-54), bodies the area rule does not take, while the old save, which reads neither, draws and measures the record box cut back at its joins. The old save's `GrossVolume` differs from the volume of the 2024 save's body, meshed without its openings, on all 203, by 0.003 to 3.55 m3. The one value that differs where both write it is wall 418079 of `rac_basic` 2015, whose box is 0.28 m thick where 2024's is 0.202 m, so its volumes and footprint differ and its side areas do not. Nothing a 2015 to 2023 record gives tells these walls apart: their step classes fall on 3 signatures, and each is also held by walls whose areas equal the 2024 save's (158 walls). So `whole` is still not set on these releases, and the areas and volumes stay out, as on `main`. These figures supersede section 5's, which were of the contributor's first head.

**`rac_basic`'s 2023 save writes no `Qto_WallBaseQuantities` at all on walls 418079 and 845266**, which its 2022 and 2024 saves write, `Width` included; why is not decoded. It is why the counts above hold each save to the 2024 save, not to 2023 as section 4 does.

**The landing head** (`8817a762`) sets `uncut` and not `whole` on 2015 to 2023. In Measure run 42567326561 (against `main` at `83417fde`, unfiltered, with the samples of every year, the BIM4LCA office, the Revit 2025 models, the held-out models, `racadv` and BIMcollab), 45 of 118 exports move, all of Revit 2015 to 2023: the 31 sample saves of those years that hold such a wall, the six `racadv` pairs, BIMcollab's 2015 model and seven held-out models of 2018 to 2023. The other 73, every 2024 to 2027 export among them, are byte for byte `main`'s, and `same-project gates` passes. The scorers change only where a `Height` is added:

- `modelo_bim` (Revit 2023, IFC4, held out, counts only): 1 equal, none different, 3 missing.
- BIMcollab's Revit 2015 model (IFC2X3, through Solibri) and bim-whale's Revit 2021 project (IFC2X3, held out): 88 and 151 that Revit's export does not write. Vitoria `ALD26 Before`'s export has base quantities off, and its 97 are counted apart.
- Warmond (Revit 2021, held out, counts only; its export went through another tool): 91 equal and 45 different, as on the contributor's first head, and 40 of the 45 that the scrubbed output lists are exactly twice Revit's. Revit's export gives 84 walls' Tags to more than one object, which reads as walls split at their storeys (an export option) and is not tested further, since a held-out file is not opened. The quantity scorer keyed each such Tag to one of its pieces, so `tools/re/qtos_vs_ifc.py` now also counts the values on such Tags (Measure run 42569616611, head `a8d3fdd5`): on them `Height` is 0 equal, 45 different and 39 missing, and `Width` 84 equal. So all 45 differences sit on walls Revit's export writes as several pieces, and on the walls it writes once, rvt-rs's `Height` is 91 equal and none different. Counting such values apart in W7, rather than as different, is a change to the scorer of its own (#1576).

**Reproduce:**

```bash
gh workflow run measure.yml --ref main -f ref=748eff38ecdeb0fc22e15259826453eea48570df -f base=main -f samples=true -f keep_ifc=true
# then, for each sample save of 2015 to 2023 in the run's measure-head-samples-2008-2023 artifact:
python3 tools/re/save_pair_wall_quantities.py sample-2023-racadvancedsampleproject/model.ifc \
  ../measure-head-samples-2024-2099/sample-2024-racadvancedsampleproject/model.ifc
```

## 10. Addendum, 2026-11-14: the control joins are in the graph on 2015 to 2023 (#1584)

Section 5 says a 2015 to 2023 record is "read without its control joins" and section 6 that "why the graph holds none is not decoded". **The graph holds them.** `probe_1584_wall_control_joins` (Measure run 43666839412, probe at `35e4130e`, `-f samples=true`, all 59 sample models of 2015 to 2027) prints for each wall whether its record has a `VWallDriver`, whether that holds `m_controlJoinsSet`, the entries whose ElementId decodes, and its `CDJoinInfo` and `JoinEndGStep` objects. Counts and ElementIds only.

- **Every wall record has the driver and the field.** In all 59 saves the wall records equal the walls with a `VWallDriver`, and none lacks `m_controlJoinsSet`.
- **The sets are as full on 2015 to 2023 as on 2024 to 2027.** `rac_advanced` holds 179 walls with joins and 1 with an empty set on every save of 2015 to 2027; `rac_basic` 59 and 1; `rme_basic` and `rme_advanced` 151 and 15; `rst_basic` 9 and 0 on 2016 to 2027 (its 2015 save is another model with 4 walls and empty sets, RE-181); `rst_advanced` 6 and 0. A wall with a non-empty set always has an entry whose ElementId decodes, as `{"m_id":{"m_id":n}}` before 2024 and `{"m_id":{"m_id64":n}}` after (RE-280).
- **Held to the same project's later save by ElementId**, with `tools/re/wall_control_joins_across_saves.py`: `rac_advanced` 2021, 2022 and 2023 against 2024, 180 of 180 walls hold the same sets, the same decoded entries, the same `CDJoinInfo` entries and the same `JoinEndGStep` and exposed-face entries; so do `rac_basic` 2019, 2020 and 2022 against 2023 (60 of 60), `rst_basic` 2016, 2022 and 2023 against 2024 (9 of 9), `rme_basic` 2019 against 2023 and `rme_advanced` 2019 against 2024 (166 of 166), and `rac_advanced` 2024 against 2027 (180 of 180).
- **Where the sets differ they are the project's earlier state, as far as these files can show.** `rac_advanced` 2015 to 2020 are alike (180 of 180 between 2015, 2019 and 2020) and differ from 2021 on 33 walls; `rac_basic` 2015 differs from 2019 on 8. No set of the earlier save names a wall the later save does not: the later save holds more partners on all 33 and on 6 of the 8, and on the other 2 the same walls at other ends. That is what a project edited between saves looks like, and also what an earlier read missing entries would look like: the files cannot tell the two apart, so those saves are held to the earliest save that equals the later release's (`rac_advanced` 2021, `rac_basic` 2019), not to the later release's.

**Why production reads them empty.** The native wall reader takes every ElementId through `native_json::element_id`, which reads `/m_id/m_id64` only. On 2015 to 2023 the field is `/m_id/m_id`, so `joins`, `side_joins` and `exposed` come back empty there, as do `shells` and `type_id` (section 5); `identifier_i64`, which RE-280 added, reads both. That is a reading of the source, not yet measured: the pull request that changes it measures it.

**Not measured:** `Length`, the side areas, the footprint and the volumes written from these joins; the angled walls of 2015 to 2020 with their ends trimmed at the joins. Each is its own pull request held to Measure, as #1584 says.

**Reproduce** (the probe's commit is archived under `refs/archive/2026-11-15/cited/35e4130ec3d5` because the pull request is squashed):

```bash
gh workflow run measure.yml --ref main -f ref=35e4130ec3d5787c1f1dbd48af5b588d3e51aa11 -f base=none -f samples=true \
  -f probe=probe_1584_wall_control_joins
# then, in the run's downloaded measure-head directory (a sample-<year>-<project>/probe.txt each):
python3 tools/re/wall_control_joins_across_saves.py measure-head
```

## 11. Addendum, 2026-11-15: the wall reader reads either id shape, and a Revit 2023 curtain wall gets its `Length` (#1584)

Section 10's cause is measured. The native wall reader took every ElementId through `native_json::element_id`, which reads `/m_id/m_id64`; a 2015 to 2023 save holds `/m_id/m_id`. The control joins, side joins, exposed faces, step partners and the wall's type now go through `identifier_u64`, which reads either. The `Length` rule (`attach_wall_lengths`), which section 5 kept off these releases, runs on them.

**A step's ids stay in the 2024 shape, on purpose.** The first head read every id in either shape (Measure run 43668542421, against `main` at `f961f2aa`) and moved 7 exports, all of Revit 2015 to 2023. The kept exports of `rme_basic` 2023, held to the same project's 2024 save, showed one of the moves wrong: reading the ids a `JoinToRoofGStep` and a `CutPairGStep` name starts the wall top cover (RE-267) there, and 5 of the 6 walls it reached took `wall_top_cut_at_covering_floor`, a flat top at the covering floor, where the 2024 save has `wall_top_stepped_at_covering_floor` for the same floor, while wall 573321 lost the `Height` the 2024 save writes. Those rules, and the areas of walls joined to floors (RE-270), are measured on 2024 and later (RE-353), so the final head reads a step's top-join, cut-pair and base-join ids as before (`named_by_steps`, `base_joined_by_steps` and the bounded ids), and reading them on 2015 to 2023 is a change of its own, with this to hold it to. The one step id that is read in either shape is the partner of a clean, cut or embed step (`NativeWall::partners`): it only decides whether a cut-back length can be trusted, and the length is refused where a partner is not read.

**What the final head moves**, in two Measure runs on the same inputs (unfiltered, with samples, the BIM4LCA office and housing, the Revit 2025 models, the held-out models, `racadv` and BIMcollab):

- **The reads alone** (run 43671572302, `e4aa0767`, against `main` at `f961f2aa`): 126 of 127 exports are byte for byte `main`'s, every Revit 2024 to 2027 export, the `racadv` pairs of 2015 to 2020, BIMcollab's model and all the sample saves among them. The one that moves is a held-out Revit 2021 project, whose Revit export is IFC2X3 and so is counted apart: its angled walls with both ends along Revit's within 0.001 ft go from 124 to 167 of 390 (within 0.1 ft 126 to 173, within 1 ft 360 to 363, the largest miss 5.783 ft before and after). 36 of 102 `diagnostics.json` gain `decoded.walls_type_confirmed`, the walls whose type the native record confirms. `same-project gates` pass.
- **`Length` on top** (run 43671576196, `22e4013e`, against the reads): 5 of 127 exports move, four of Revit 2023 and the held-out project of Revit 2021, and only by `Length`. `Qto_CurtainWallQuantities.Length` is written on 34 curtain walls of `rac_advanced` 2023, 9 of `rac_basic` 2023 and 33 of `rme_basic` 2023, and each project's 2023 save then agrees with its 2024 save on all of them (`save_pair_props.py`: the same value on every wall, none different, none only one of the two writes); Einhoven (Revit 2023) gains one such value; the held-out Revit 2021 project 8, which its IFC2X3 export does not write and which are counted apart. `rme_basic` 2023 is otherwise as on `main`: its wall tops and `Height` are `main`'s (Measure run 43674102994). `same-project gates` pass.

**Not written.** An ordinary wall's `Length`: the 2024 save writes one on 1 of `rac_advanced`'s 180 walls, 2 of `rst_basic`'s 9 and 10 of `rme_advanced`'s 166, where the wall's cut-back curve is as long as its curve, and the 2015 to 2023 saves write none of them. `Length` of a curtain wall on the sample saves of 2015 to 2022: none gains one, though the held-out Revit 2021 project's 8 do; why the samples' do not is not decoded. The side areas, footprint and volumes (section 9) and the angled walls of 2015 to 2020 with their ends trimmed at the joins are not touched: no sample save of 2015 to 2020 moves, so the reads change no wall there.

**Against Revit's export.** No untouched Revit IFC4 export of a 2015 to 2023 file holds a curtain wall's `Length` to compare: the 2023 values are held to the 2024 save's, which is measured against Revit's on the reference models, and what that cannot catch is an error the two saves share. The held-out project's improvement is on an IFC2X3 export, counted apart; it does not make the rule right.

**Reproduce:**

```bash
gh workflow run measure.yml --ref main -f ref=e4aa07677f1d86888add3a9b25db29b29ccf1da8 -f base=f961f2aa7f69e75d59576f8827a27b4af320b5fe \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true -f racadv=true -f bimcollab=true
gh workflow run measure.yml --ref main -f ref=22e4013eb8844ec133ecd5d27f26df6d643dae74 -f base=e4aa07677f1d86888add3a9b25db29b29ccf1da8 \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true -f racadv=true -f bimcollab=true
# the kept exports of the 2023 and 2024 saves, held to each other:
gh workflow run measure.yml --ref main -f ref=22e4013eb8844ec133ecd5d27f26df6d643dae74 -f base=none -f samples=true -f keep_ifc=true \
  -f scorers=none -f models="sample-2023-rmebasicsampleproject sample-2024-rmebasicsampleproject"
python3 tools/re/save_pair_props.py sample-2023-rmebasicsampleproject/model.ifc sample-2024-rmebasicsampleproject/model.ifc
```

The measured commits `e4aa07677f1d`, `22e4013eb884` and the first head `27f9090cbaa9` are archived under `refs/archive/2026-11-15/cited/` because the pull request is squashed.
