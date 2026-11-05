# RE-351: the 16 `rac_basic` walls its saves draw to different heights

**Date:** 2026-11-05
**Issues:** #1471 (16 walls of Autodesk's `rac_basic` differ between its 2025 and 2026 saves by up to 3.19 m: reader defect or project edit); follows RE-267 (a wall's top cut at the underside of the floor that covers it), RE-346 (the same-project census that found them) and RE-211 (a roof by extrusion)
**Probe:** `probes/examples/probe_re351_wall_top_covers.rs`
**Artefacts:** Autodesk's own `rac_basic` sample project, pinned in `research/autodesk-sample-projects.tsv`, fetched by `tools/fetch-autodesk-samples.sh` and never stored: every release 2015 to 2027 (2025 `41f1893a…d4293`, 2026 `bc547fe9…d99dc`).
**Runs:** Measure [42106436497](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42106436497) (head b2b5716e, `main` plus the probe; `samples=true models=- scorers=none base=none`): the probe on all 13 saves, sections 1 to 3 below. Measure [42063046730](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42063046730) (head 3149973f, #1454's first head): its kept 2025 and 2026 IFC files, section 1's first reading, which `main`'s probe run then repeated (same tops on all 16).
**Status:** a reader defect, not an edit of the project. The 16 walls' stored data (record box, base and top Levels, joins, the element their top join names) is the same on all 13 saves; what differs is a roof, 243274, that the 2025 reader draws as a solid and the 2024, 2026 and 2027 readers draw as its record box, and the wall-top pass takes a record box for a roof with a level underside. **The change of section 4 is not yet measured on its own head.**

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

On every one of the 13 saves the record of each wall lists 243274 among its references (the type, the two Levels and the walls and elements it is joined to), and from 2024 the native `SWall` record of each names it in its top join or cut pair alike (section 3). The roof is `Basic Roof: SG Metal Panels roof` (`IfcRoof`); its native record has a `RoofExtrusionHelper`, an `ExtrusionGStep` and a `SlabShapeEditGStep` on every save, so it is a roof by extrusion everywhere. Its record box is 19.5 m by 6.36 m and 3.472 m deep, from 5.7265 m, on every save (`BoundingBoxHeight` 3.47216).

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
- **234869 on 2027 has no confirmed type or curve length.** Its decoded fields on 2027 lack `m_wall_type_confirmed` and `m_wall_curve_length` that 2024 to 2026 carry, and the probe's own native read of the walls does not list it on 2027 (14 of 16 listed; 428745 is listed on no save, the reader leaving out a wall whose record does not hold exactly one level curve). Its written geometry, source and cut are the 2026 save's. This touches its Length and not its height, and is left as it is.
- **No other save disagrees.** The saves of 2024 to 2027 hold 47 walls each (2015 to 2023 hold 56), and the 31 walls other than the 16 are written alike on 2024, 2025, 2026 and 2027.

## 4. The change

A roof counts as a cutter of a wall's top only where it is one of the roofs whose record box is exactly its type's layers (`m_slab_layers`, the field `attach_slab_layers` gives a floor, roof or ceiling whose layers add up to its record box's height, RE-57). A plate of that kind is level by construction; a roof with a deeper box, or whose layers are not read, is left out and the wall keeps its record box. Slabs and footings are as before. The base pass (#1384) is as before: it takes a roof's top as a base level on the same grounds, which has the same flaw for a wall standing on a roof drawn as its box; none of the ten exports of run 42063046730 has a wall cut at its base, and no model has been checked for one.

What section 3 says it does, on this one project: the 15 cut walls of 2024 and the 16 of 2026 and 2027 lose their cut and are written as the 2025 save writes them, since every other input of their bodies is equal to the 2025 save's; 428745 on 2024 is unchanged. The roof stays its record box on 2024, 2026 and 2027 (RE-341 keeps roofs boxes, and the contributor of #1454 found the spline list wider changes no byte of a 2026 export). Whether the roof by extrusion can be read on those saves is a separate question, and not asked here.

**Not yet measured on this head:** the walls of the 2024, 2026 and 2027 saves after the change, the walls of every other model (the BIM4LCA office and the R2025 models in particular, where a flat roof may cut a wall), and the walls' quantities (`sets`, which run 42106436497's probe did not print).

## 5. Not measured

- A Revit export of a `rac_basic` save: none exists to compare with. Autodesk's samples have no export from Revit, and three.js's IFC2X3 export of `rac_advanced` is counts only. The walls under roof 243274 therefore have no oracle of their own; the 2025 save's drawing of them, a wall to its record box, is the earlier save's, and is not Revit's cut by the roof.
- Whether a flat roof that cuts a wall on the BIM4LCA office as built or at permit is still cut: not yet measured on this head.
- Why 428745 is uncut on 2024 (section 3).
- The walls' quantities across saves.

## 6. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2027
cargo run --profile ci -p rvt-probes --example probe_re351_wall_top_covers -- samples/2026-racbasicsampleproject.rvt > 2026.jsonl
```

Measure's run is `gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f models=- -f scorers=none -f probe=probe_re351_wall_top_covers -f probe_scorer=-`.
