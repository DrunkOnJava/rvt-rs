# RE-351: the 16 `rac_basic` walls its saves draw to different heights

**Date:** 2026-11-05
**Issues:** #1471 (16 walls of Autodesk's `rac_basic` differ between its 2025 and 2026 saves by up to 3.19 m: reader defect or project edit); follows RE-267 (a wall's top cut at the underside of the floor that covers it), RE-346 (the same-project census that found them) and RE-211 (a roof by extrusion)
**Probe:** `probes/examples/probe_re351_wall_top_covers.rs`
**Artefacts:** Autodesk's own `rac_basic` sample project, pinned in `research/autodesk-sample-projects.tsv`, fetched by `tools/fetch-autodesk-samples.sh` and never stored: every release 2015 to 2027 (2025 `41f1893a…d4293`, 2026 `bc547fe9…d99dc`).
**Runs:** Measure [42106436497](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106436497) (head b2b5716e, `main` plus the probe; `samples=true models=- scorers=none base=none`): the probe on all 13 saves, sections 1 to 3 below. Measure [42063046730](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42063046730) (head 3149973f, #1454's first head): its kept 2025 and 2026 IFC files, section 1's first reading, which `main`'s probe run then repeated (same tops on all 16).
**Status:** a reader defect, not an edit of the project. The 16 walls' stored data (record box, base and top Levels, joins, the element their top join names) is the same on all 13 saves; what differs is a roof, 243274, that the 2025 reader draws as a solid and the 2024, 2026 and 2027 readers draw as its record box, and the wall-top pass takes a record box for a roof with a level underside. **The change of section 4 is not yet measured on its own head**, and it is not Revit's body either (section 4): it is a smaller error than the cut it removes, not a right answer. This branch is stacked on #1454 (merged in at f1460b32), whose census pins these 16 walls; the pins come out when #1454 is on `main`.

## 0. The question

16 walls of `rac_basic` are drawn from the same source on its 2025 and 2026 saves with different extents, up to 3.19 m, every one smaller on 2026, six sharing one 3.1865 m offset (#1471, found by RE-346's census on #1454): 428588, 428745, 430064, 506386, 234869, 429964, 430318, 430361, 430412, 430859, 497540, 506797, 768442, 937935, 938974, 939084. Is that the reader or the project? It is already so on `main`.

## 1. The difference is in the walls' height, and only there

Written plan, base and record-box height are equal on the saves that draw these walls (run 42106436497, the probe's `wall` lines, compared to 1e-9 ft: the placement and the extrusion without its height of each of the 16, 2024, 2026 and 2027 against 2025: 16 of 16 equal on each). What differs is the top. On the 2025 save each wall is drawn up to its record box. On 2024, 2026 and 2027 it is cut at 5.7265 m, with `BodyHeightSource` (`wall_top_cut_at_covering_floor`, and `wall_top_stepped_at_covering_floor` on 428588 and, on 2026 and 2027, 428745, which keep the part of their plan the cover does not reach) and `TopCoverElementId` 243274.

| wall | 2025 source | base | 2025 top (the box's) | cut top, 2026 and 2027 | lower by |
|---|---|---:|---:|---:|---:|
| 428588 | join_trimmed | 2.700 | 6.0000 | 5.7265, and 6.0000 beyond the roof's plan | 0.2735 |
| 428745 | join_trimmed | 2.200 | 6.0000 | 5.7265, and 6.0000 beyond the roof's plan (2026, 2027; uncut on 2024) | 0.2735 |
| 430064 | join_trimmed | 3.000 | 7.4728 | 5.7265 | 1.7463 |
| 506386 | join_trimmed | 3.000 | 8.1103 | 5.7265 | 2.3838 |
| 234869 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 429964 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 430318 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 430361 | layer_joins | 3.000 | 7.6447 | 5.7265 | 1.9181 |
| 430412 | layer_joins | 3.000 | 7.9525 | 5.7265 | 2.2260 |
| 430859 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 497540 | layer_joins | 3.000 | 8.8449 | 5.7265 | 3.1184 |
| 506797 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 768442 | layer_joins | 3.000 | 7.9525 | 5.7265 | 2.2260 |
| 937935 | layer_joins | 3.000 | 7.5292 | 5.7265 | 1.8027 |
| 938974 | layer_joins | 3.000 | 8.9131 | 5.7265 | 3.1865 |
| 939084 | layer_joins | 3.000 | 8.2088 | 5.7265 | 2.4823 |

(Metres, from `location_feet` and `height_feet` of the probe's `wall` lines; `join_trimmed` is `partition_element_record_join_trimmed` and `layer_joins` `partition_wall_centreline_layer_joins`, 4 and 12 walls.) The six walls that are 3.1865 m lower are the six whose box top is 8.9131 m: the box's top is the wall's highest point, and 5.7265 m is the cover's underside. The first reading of this table, from the IFC files of run 42063046730 (#1454's code), has the same tops.

The 2025, 2026 and 2027 saves hold 47 walls each, and the 31 others are written alike in every field the probe compares (the placement, the extrusion, the solid shape; 2025 against 2026: 16 differ, all among the 16; 2024 against 2026: 1 differs, 428745). The two walls that differ between 2026 and 2027 (198694 and 849032) differ by 4e-15 ft in a height: the last bit of a stored box.

## 2. The cover is a roof the saves draw differently

On every one of the 13 saves the record of each wall lists 243274 among its references (the type, the two Levels and the walls and elements it is joined to), and from 2024 the native `SWall` record of each names it in its top join or cut pair alike, **the 2025 save's included**: the partition reader's `m_wall_top_cutters` is `[243274]` for all 16 walls on 2024, 2025, 2026 and 2027 (section 3). The 2025 save does not cut because the roof there is a solid, not because its walls do not name it. The roof is `Basic Roof: SG Metal Panels roof` (`IfcRoof`); its native record has a `RoofExtrusionHelper`, an `ExtrusionGStep` and a `SlabShapeEditGStep` on every save, so it is a roof by extrusion everywhere. Its record box is 19.5 m by 6.36 m and 3.472 m deep, from 5.7265 m, on every save (`BoundingBoxHeight` 3.47216). It is a gable: the second read of #1476, from the 2025 save's solid, puts the ridge's underside 3.1865 m above the roof's base, 8.9131 m absolute (not reproduced in this report; it is the number the six tallest walls' box tops share in section 1).

The 2025 save draws it as a solid (`BodySource` `partition_roof_extrusion`, RE-211: its section read from its sketch lines); every other save draws it as its record box (`partition_element_record_bbox`, an `IfcExtrudedAreaSolid` over the box). `attach_extrusion_roofs` runs only on the releases in `SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS`, which is 2025 alone. The wall-top pass (`ifc::export_content::wall_tops`, RE-267) cuts a wall's top at the underside of the slab, roof or footing its top join names where that element is drawn as a plain extrusion, whose underside it takes to be level across the wall: a roof drawn from its slopes or its extrusion is a solid and is left out. A roof that is its record box is a plain extrusion, so on 2024, 2026 and 2027 it is a cutter, and its underside is the bottom of its box, 3.47 m below the box's top.

That is a reader defect. The cut is a statement that Revit's wall stops at a level; for a roof whose shape is not read, nothing says the box's underside is one. The roof has no material layer set in the 2025 and 2026 exports either (`attach_slab_layers` gives a roof its type's layers only where they add up to its record box's height, so here they do not add up to 3.472 m or are not read).

## 3. Every save of `rac_basic` (run 42106436497)

**Stored data, all 13 saves, 2015 to 2027.**

- **Record frames.** Each of the 16 walls and the roof has one frame on every save. The box, the category, the container word, the placement kind and the design option (none) are equal on all 13 saves to 1e-9 ft, for all 17 elements. Nothing about the box moved between saves, so the project's walls and roof were not edited.
- **Reference lists.** Each wall names 243274 on all 13 saves, and its list is equal on 2016 to 2027; 428745's lists 846939 from 2016 on and not on 2015, and the roof's lists 960489 on 2015 alone (neither is a wall or a cover of these walls).
- **Levels.** Level 2 (the base, 245423) is 9.84252 ft, Roof Line (the top, 196629) 19.685039 ft, Ceiling (694, the top of 234869) 8.858268 ft, on all 13 saves.
- **Native `SWall` records, 2024 to 2027.** Every one of the 16 has the same base Level 245423, the same top Level (196629; 694 for 234869), the same phase, `m_locLineOffset` and `m_keyRefOffset`, the same `CleanPairGStep` and `CutPairGStep` elements and a `JoinToRoofGStep` with `m_baseOrTop` 1, on all four saves. The partition reader's `m_wall_top_cutters` is `[243274]` on all four for each wall, and so is its centreline and every `m_wall_*` field (2025 against 2024 and 2026: 16 of 16 equal; against 2027: 15 of 16, 234869 below). On 2015 to 2022 this native reader is not measured and its probe refuses; on 2015 to 2023 the Level ids of a native record read as none, so only the frames and the partition reader's fields speak there.
- **2015 to 2023.** No wall is cut: the top-cut pass reads its elements from the native records of 2024 and later. All 16 walls are drawn to their record boxes there, with heights and bases equal to 2025's (16 of 16 on each of the 9 saves), the four `join_trimmed` walls with the 2025 save's placement and extrusion exactly, and the other twelve as `join_trimmed` boxes (those releases read no centreline), so their plan is not the 2025 save's layered profile. The roof is its record box on all nine.

**Written tops** (the probe's `wall` and `roof` lines):

| save | walls with `BodyHeightSource` of the 16 | roof 243274 | walls' tops |
|---|---:|---|---|
| 2015 to 2023 | 0 | record box | record box |
| 2024 | 15 (428588 stepped, 14 cut; 428745 uncut) | record box | 5.7265 m on the 15; the box on 428745 |
| 2025 | 0 | `partition_roof_extrusion` | record box |
| 2026 | 16 (428588 and 428745 stepped, 14 cut) | record box | 5.7265 m |
| 2027 | 16 (the same) | record box | 5.7265 m |

**Where a save disagrees.**

- **2024 is not 2026 on one wall.** My expectation that the 2024 save cuts as the 2026 one does holds for 15 walls (2024 against 2026: written geometry equal on 15 of the 16, and on all 31 other walls). **428745 is not cut on 2024** and is on 2026 and 2027, with the same `m_wall_top_cutters` `[243274]` and `m_wall_top_pair_only` `[243274]` on all four saves, so the walls' data does not say why. Not measured: the pass skips a wall that has an elevation-profile opening (`profiled`), and those openings are read on 2024 alone (`WALL_PROFILE_OPENING_REVIT_VERSIONS`); the probe prints `profile_openings` for the next run to say whether 428745 has one. The 2024 wall equals the 2025 save's in every field compared.
- **234869 on 2027 has no confirmed type or curve length.** Its decoded fields on 2027 lack `m_wall_type_confirmed` and `m_wall_curve_length` that 2024 to 2026 carry, and the probe's own native read of the walls does not list it on 2027 (14 of 16 listed). Why, from the `classes` histograms of the probe's `native_record` lines: the native wall reader leaves out a wall whose record does not hold exactly one level curve, and 234869's record holds 2 `GLine` on 2027 against 1 on 2024 to 2026. **428745 is listed on no save** because its record holds 7 `GLine` on 2024 to 2026 (8 on 2027) and a `WallEmbedWallGStep` (it embeds a curtain wall), against 1 `GLine` for a listed wall such as 430064. The probe printed only the walls the reader lists, which hid both; its `native_wall` line is now printed for all 16 with `read` false where the reader leaves one out. 234869's written geometry, source and cut are the 2026 save's, and the gap touches its Length and not its height.
- **No other save disagrees.** The saves of 2024 to 2027 hold 47 walls each (2015 to 2023 hold 56), and the 31 walls other than the 16 are written alike on 2024, 2025, 2026 and 2027.

## 4. The change

A roof counts as a cutter of a wall's top only where it is one of the roofs whose record box is exactly its type's layers (`m_slab_layers`, the field `attach_slab_layers` gives a floor, roof or ceiling whose layers add up to its record box's height, RE-57) **and** whose plan outline is read (`ProfileResolved`). The layers are the proof that the underside is level, which a body-source test (the roof is not a record box) would not be: it is why the rule keeps the layer test. The outline is what the roof covers: a roof whose outline is not read is drawn over its box's rectangle, so an L-shaped plate roof would cover a wall it does not reach. The second read finds every roof of the 2026 and 2027 saves unresolved, so none is a cutter there. A roof with a deeper box, or whose layers or outline are not read, is left out and the wall keeps its record box. Slabs and footings are as before. The base pass (#1384) is as before: it takes a roof's top as a base level on the same grounds, which has the same flaw for a wall standing on a roof drawn as its box; none of the ten exports of run 42063046730 has a wall cut at its base, and no model has been checked for one (the second read files it, with the gate for it in #1470's `decide_base`).

**What it does, on this one project.** Section 3 says the 15 cut walls of 2024 and the 16 of 2026 and 2027 lose their cut and are written as the 2025 save writes them, since every other input of their bodies is equal to the 2025 save's; 428745 on 2024 is unchanged. The roof stays its record box on 2024, 2026 and 2027 (RE-341 keeps roofs boxes, and the contributor of #1454 found the spline list wider changes no byte of a 2026 export). Whether the roof by extrusion can be read on those saves is a separate question, and not asked here.

**Five values vanish on a wall like 506386.** Of the 16, only 506386 carries `Qto_WallBaseQuantities` beyond `Length` and `Width` on the 2026 save (the exports of run 42063046730, #1454's first head): `Height` 2.726517, `NetVolume` and `GrossVolume` 0.392618, `NetSideArea` and `GrossSideArea` 3.271821, which `wall_areas` writes for a wall its joins name floors and roofs for once the roof clears the cut top. Its 2025 save writes `Length` and `Width` alone. With the change the roof no longer clears the wall, so the five go and the two stay, as on 2025; the other 15 carry one `Qto_WallBaseQuantities` value on both saves, equal, and are unchanged. Measured on this head in run 42554270370 (below): the 2026 save's walls write no value the 2025 save does not.

**Smaller error, not the right body.** The second read of #1476 compares the 2025 save's roof solid with the walls' stored box tops: for 13 of the 16 walls the box top equals the roof underside's highest point over the wall's plan to 0.0001 m; 937935 is 0.0096 m under it; 428588 and 428745 sit 0.085 m above it, and they are the two that step. So Revit's walls attach to a sloped underside, and the prism this change restores is not Revit's body either; the same read's rough estimate is about +5.1 m3 over Revit's across the 16 with the change, against about -13.5 m3 under with the flat cut. That is a smaller error and not a right answer: a wall cut along the read roof's sloped underside is the follow-up the second read files, and so is reading roofs by extrusion on 2026 and 2027. Not reproduced in this report.

**What exercises the rule.** Only roof 243274 exercises the roof-cover path anywhere, and no wall is cut by a plate roof, so the retained `plates.contains` branch is unmeasured. (The second read's count over the models Measure scores; the final run below counts it again on Snowdon.)

**Measured** (Measure run 42554270370, head 584c8a89 against `main` at fc29acbf, unfiltered: the reference models, the BIM4LCA office as built and at permit, the R2025 models, Autodesk's samples of every year and the held-out models, 118 exports). Four exports move: `rac_basic`'s 2024, 2026 and 2027 saves and the held-out AA house (Revit 2026). The other 114 are byte for byte `main`'s, the office's among them, so no wall of the office stops at a roof it no longer reaches (606 of its 607 walls as built have Revit's base and top on both sides). On the 2026 and 2027 saves all 16 walls are now the 2025 save's bodies. The same-project census pairs 441 bodies on each, 424 the same and 1 differing: roof 243274, which stays the record box `ROOFS_LEFT_AS_BOXES` expects. On `main` the 16 walls differed as pinned. The 11 walls that host cut openings equal the 2025 save's with their openings (5 of 11 on `main`). The 16 no longer carry the `BodyHeightSource` and `TopCoverElementId` the 2025 save does not write, nor 506386 its five quantities. The run's same-project check fails only because the 16 pins healed: each reads "is now the 2025 save's: remove its pin". This head removes them, and the check then expects `pinned walls as pinned 0 of 0`. The AA house's export moves, but nothing in that run's scrubbed counts scores a wall's height. So the scrub now keeps `wall heights:` count lines (tools/ci/scrub-holdout-results.sh), and Measure run 42556745265 (head 8ddb0ad2 against `main` at 9c66e9bd, Autodesk's samples of 2024 to 2027 and the held-out models) counts them. Of the house's 59 walls, those with Revit's base and top go from 43 to 56; those with Revit's base and not its top go from 16 to 3, all of them on Revit's storey. Nothing else in its counts moves. The house is Revit 26.4's IFC4X3 export, held out and counts only, and the only Revit export on which this rule changes a wall. In that run the same-project check passes on that head, with `pinned walls as pinned 0 of 0` and 440 of 441 bodies the 2025 save's on each of `rac_basic`'s 2026 and 2027 saves (the 441st is roof 243274). One more export moves there than in run 42554270370, the held-out Gdynia 2021, because the branch's base was older than `main`. Gdynia's export on this head equals its export in run 42554270370.

## 5. Not measured

- A Revit export of a `rac_basic` save: none exists to compare with. Autodesk's samples have no export from Revit, and three.js's IFC2X3 export of `rac_advanced` is counts only. The walls under roof 243274 therefore have no oracle of their own. Roof 243274 is a gable, whose sloped underside Revit's walls attach to (section 4): the 2025 save's drawing of them, a wall to its record box, is the earlier save's and is a smaller error than the flat cut at the roof's lowest level (its base, 5.7265 m), and is not Revit's body.
- Snowdon Towers (local only, which Measure does not fetch; its architectural file has 20 `IfcRoof` and 1,078 walls): not run. The BIM4LCA office's exports are byte for byte `main`'s (section 4). Nothing but roof 243274 is known to exercise the roof-cover path, and the `plates.contains` branch is unmeasured (section 4).
- Why 3 of the held-out AA house's 59 walls still miss Revit's top after the change (section 4): its counts give no wall, only the totals.
- Why 428745 is uncut on 2024: the probe prints `profile_openings` and the full native wall lines, and run 42554270370 did not run it.
- The walls' quantities across saves, except 506386's five values on the 2025 and 2026 exports of run 42063046730 (section 4).

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2027
cargo run --profile ci -p rvt-probes --example probe_re351_wall_top_covers -- samples/2026-racbasicsampleproject.rvt > 2026.jsonl
```

The probe's run is `gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f models=- -f scorers=none -f probe=probe_re351_wall_top_covers -f probe_scorer=-`. The run that scores the change, once #1454 is on `main` and its pins are empty, is unfiltered: `-f base=main -f samples=true -f nordic=true -f r2025=true -f holdout=true -f keep_ifc=true -f probe=probe_re351_wall_top_covers`, every year. Its expected movers are `rac_basic` 2024, 2026 and 2027 alone; any other export that moves stops the work.
