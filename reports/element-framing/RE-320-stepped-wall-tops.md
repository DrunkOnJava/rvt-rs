# RE-320: a wall a floor covers in part is drawn stepped, with its top at the floor's underside where the floor covers it

**Date:** 2026-11-03
**Issues:** #1067 (walls drawn taller than Revit's body), #1016 (W7: walls' `Height`), W6 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-257 (a wall's `Height` is its body's span), RE-258, RE-267 (Revit's top is the floor's underside where the floor covers the wall) and RE-270
**Artefacts:** the BIM4LCA office as built (`ARK_NordicLCA_Office_Concrete_As-Built.rvt`, sha256 `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906`; Revit's export `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc`, `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382`) and at building permit (`..._BuildingPermit.rvt`, `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1`; `..._BuildingPermit_Revit.ifc`, `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d`), Revit 2024 and Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, VTT's ARCH.zip as pinned in `research/bim4lca-measure.tsv`; Core Interior, RE1 Architecture, puzzbobb's R2025 models and Autodesk's sample projects as Measure fetches them
**Runs:** Measure [41732207324](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41732207324) (main 7fd339e2, the baseline), [41736018290](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41736018290) (`probe_1067_native_dump`, both offices), [41736828459](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41736828459), [41739861635](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41739861635) and [41741031542](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741031542) (the branch against `main`, both offices; the last on the final code), [41737388173](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41737388173) (Core Interior, RE1 Architecture and the 24 R2025 models), [41739882963](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41739882963) and [41741031542](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741031542) (both offices, the quantity, property set, schema, layer and opening scorers), [41741013741](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741013741) (every model: `-f nordic=true -f r2025=true -f samples=true -f holdout=true`)
**Result:** positive, shipped. A wall that a floor, roof or footing named by its top join or a Join Geometry cut pair covers in part is drawn stepped, on 49 of the BIM4LCA office's walls as built and 46 at building permit. rvt-rs's top is then Revit's over the whole plan on 583 of 607 walls as built (544 before) and 571 of 589 at permit (530), the area it draws at a wall's full height where Revit's body is cut falls from 1,916 to 238 ft2 and from 1,888 to 38 ft2, and no wall that was equal moved; every wall's lowest and highest point, the 64 and 70 walls cut whole, the quantity, property set, schema, layer and opening scores and the exports of Core Interior, RE1 Architecture and the R2025 models are unchanged. An element that only a cut pair names now cuts a wall's top only where it reaches the top, which narrows RE-267 (three walls as built, §5). Not drawn, and measured: the walls whose layers Revit ends at different heights, a top Revit cuts at a height no join names, the gaps and ends of 78 walls as built that no floor reaches, and the 0.0243 ft2 stub of one wall (3004978) that Revit cuts and nothing separates from the pieces it keeps (§4, §7).

## 1. How Revit draws these walls

Revit's IFC4 export of the office writes each wall's Body representation in one of two forms (IfcOpenShell 0.9 reading the files):

| | as built | at permit |
|---|---:|---:|
| walls | 607 | 589 |
| `SweptSolid` | 276 | 298 |
| `Tessellation` (one `IfcPolygonalFaceSet` per layer) | 331 | 291 |

A `SweptSolid` wall is one `IfcExtrudedAreaSolid` per material layer. Two profile orientations occur: for a wall whose top is one height, the layer's cross-section (thickness by height) extruded along the wall; for a wall whose top steps, each layer's elevation outline (length by height, the steps in it, `IfcArbitraryProfileDefWithVoids` where an opening cuts it) extruded across the layer's thickness. A `Tessellation` wall is one polygonal face set per layer. Of the 62 walls as built whose full-height plan area rvt-rs drew larger than Revit's before this change, 14 are `SweptSolid` and 48 `Tessellation`, so the stepped walls have no single form to copy, and a profile drawn across the thickness cannot draw a floor edge that runs along the wall.

rvt-rs draws the same body as plan prisms in one `SweptSolid` representation (`SolidShape::Compound`): the wall's own plan from its base to the lowest underside, and over the part of the plan the elements do not cover, one prism per convex piece from the underside to the next underside, or to the wall's top. Every piece is an `IfcExtrudedAreaSolid` of an `IfcArbitraryClosedProfileDef`, the first on the wall's own profile. The wall's `extrusion` stays the full prism, which the passes that read a wall's plan and its record's box use.

## 2. The rule

RE-267 §2: Revit's top over a wall is the underside of the floor, roof or footing that the wall's top join (`JoinToRoofGStep`, `m_baseOrTop` 1) or a Join Geometry cut pair (`CutPairGStep`) names, over the part of the wall's plan that element covers, and the wall's own top elsewhere. RE-267 drew that where one element covers the whole plan. `ifc::export_content::wall_tops` now draws it where the named elements cover part of the plan, on rvt-rs's own bodies:

- The elements are the ones RE-267 reads: slabs, roofs and footings drawn as plain extrusions at one height, whose underside lies inside the wall's height (0.001 ft in). A wall that one element covers whole (the plan outside its outline, and inside its holes and openings, under 1e-4 ft2) is cut as before, byte for byte.
- Otherwise the elements that reach the plan (over 1e-4 ft2 of it, inside the outline and outside the holes and openings) are grouped by underside (within 0.001 ft). The wall must be one plain extrusion with no hole in its plan and no edited elevation profile; its plan may be any simple polygon. The plan is broken into convex pieces (itself where it is convex, else its ear-clipped triangles with every two that make a convex piece joined). Taking each outline's triangles from them in turn, lowest underside first, what is left after the elements at one underside and the lower ones is the part of the plan whose top is higher. It is kept as convex pieces, joined again where two make a convex piece, and a piece under 1e-4 ft2 is dropped.
- The body is one `SweptSolid` representation: the wall's own profile from its base to the lowest underside, and for each underside the pieces left above it, from that underside to the next one, or to the wall's top for the last. Where the lowest underside leaves nothing, the wall is cut whole, as RE-267 draws it.
- The holes and openings of an element count as uncovered where one element alone is named. More than 32 pieces, a plan that does not break into convex pieces, several elements that share a hole or opening the wall crosses, and the cases of §4 and §5 leave the wall as it was.

A stepped wall carries `BodyHeightSource` `wall_top_stepped_at_covering_floor`, `TopCoverElementId` (the lowest element) and, for several, `TopCoverElementIds`. Its `extrusion` stays the full prism, so the passes that read a wall's plan, thickness and record box (joins, openings, quantities, layers) see what they saw; `Height`, the side areas and the volumes are not written on it, as on every wall with a top join (RE-267 §7, RE-270). The plan outline of a stepped wall (`body_geometry::plan_outline`, read by the plan sheet and by the depth of a door's or window's cut) is the profile of its first item, as its extrusion's was, and not the hull of its pieces.

## 3. Measured

Measure runs of this branch against `main`, both BIM4LCA office files, with Revit's own IFC4 export as the reference. `tools/re/wall_top_cover_vs_ifc.py` was extended for this: it lines Revit's body up with rvt-rs's across each wall's thickness (the fit of Revit's frame onto rvt-rs's leaves up to 0.02 ft, which moves a floor edge that runs along a wall by more than the 0.01 ft it compares), reads the top at five rows across the wall and not on the centre line alone, and compares the area drawn at the wall's full height in each file. Before is `main` (7fd339e2 or later), after is the branch, both scored by the same scorer.

| BIM4LCA office | as built, before | as built, after | at permit, before | at permit, after |
|---|---:|---:|---:|---:|
| walls | 607 | 607 | 589 | 589 |
| walls drawn stepped | 0 | 49 | 0 | 46 |
| walls cut whole (RE-267) | 64 | 64 | 70 | 70 |
| top equal to Revit's along the centre line (RE-267's measure) | 472 | 480 | 419 | 426 |
| top equal to Revit's at every point compared across the wall | 544 | 583 | 530 | 571 |
| points compared | 203,715 | 203,715 | 193,962 | 193,962 |
| ... equal within 0.01 ft | 182,215 | 200,887 | 175,243 | 193,125 |
| ... rvt-rs higher than Revit's | 21,500 | 2,824 | 18,719 | 833 |
| ... rvt-rs lower than Revit's | 0 | 4 | 0 | 4 |
| area at the wall's full height in rvt-rs's body and not in Revit's (ft2) | 1,915.80 | 238.20 | 1,888.10 | 38.10 |
| area at the wall's full height in Revit's body and not in rvt-rs's (ft2) | 0.0002 | 3.85 | 0.0001 | 5.75 |

- No wall that was equal before differs now, on the centre line or across the plan. The centre line gains 8 walls as built (1486440, 1630966, 1656493, 1693193, 1892033, 1892039, 3039425, 3123954) and 7 at permit; the plan gains 39 and 41, at least 31 and 34 of which were already equal on the centre line (a floor covering a strip beside it, which the centre-line measure cannot see).
- The area rvt-rs draws lower than Revit's (3.85 and 5.75 ft2) is a misfit along faces, not a piece drawn too low: 12 walls as built and 13 at permit, most of them 60 to 220 ft long, differ by 0.06 to 0.52 ft2 each, nearly all of them in both directions at once (a floor edge 0.002 ft off along the wall).
- The walls cut whole are unchanged (64 and 70; every wall that is not drawn stepped has the same meshed volume as on `main`), and every wall's lowest and highest point (`wall_heights_vs_ifc`) and the GLB wall bodies (`wall_bodies_vs_ifc`) are byte-identical to `main`'s. Core Interior, RE1 Architecture and the 24 R2025 models export byte for byte as before (Measure 41737388173).
- The meshed volume of a stepped wall moves toward Revit's on 47 of the 49 walls as built (the sum of the differences falls from 5,435 to 2,607 ft3) and on 39 of the 46 at permit (6,384 to 3,853 ft3). The walls it moves away from are the finish layers Revit ends lower than the core (1348323 and its neighbours, §7).
- Every model (Measure [41741013741](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741013741), `-f nordic=true -f r2025=true -f samples=true -f holdout=true`, 111 models, the branch at `b23adf4f` against `main` at `4ad28023`): the export hashes of 94 models are identical, among them Core Interior, RE1 Architecture, Einhoven, the 24 R2025 models and 17 of the 18 held-out models. 17 differ: the two offices, 14 of Autodesk's sample projects (swept solids +0 to +7 on 13 of them and +94 on Snowdon Towers' architectural sample; none has a Revit export in Measure's reach, so they are counted: Snowdon Towers' architectural sample has a Revit IFC4 export, but it is local only and not measured here) and the held-out AA-SingleDwellingHouse-RVT-CIADD, whose 137 count lines are unchanged; that its hash moved for stepped walls is inferred, since no count line sees swept items. The never-wrong evidence is one building in two saves. The export diagnostics are byte-identical on all 93 models that have them. Of the scorers, only `schema.txt` differs (the swept-solid count, on 15 models), and `wall_ends_list` and `space_overlaps_vs_ifc` list the same rows in another order (but one sign, §7). `qtos_vs_ifc`, `psets_vs_ifc`, `layer_sets_vs_ifc`, `wall_layers_vs_ifc`, `glb_material_colours_vs_ifc`, `wall_bodies_vs_ifc`, `wall_heights_vs_ifc` and the opening scorers are byte-identical on every model with a Revit export, the offices included.
- The 49 and 46 walls are drawn from 2 to 10 prisms as built and 2 to 21 at permit (a layered wall's plan is not convex, so its pieces are many). Revit draws 14 and 13 of them as `SweptSolid` and 35 and 33 as `Tessellation`. Four walls in each file stand under floors at two undersides (1348536, 1348537, 1348538 and 1488456 as built; 1347198, 1348536, 1348537 and 1348538 at permit) and carry `TopCoverElementIds`.

A stepped wall as the IFC writes it (wall 1341937 as built, 0.435 m thick, whose floor covers 0.15 m of the thickness; metres): one prism of the wall's own profile to 3.45 m and, over the other 0.285 m, one piece standing on it for 0.45 m,

```
#10233=IFCEXTRUDEDAREASOLID(#10232,#21,#19,3.450000);
#10247=IFCAXIS2PLACEMENT3D(#10244,#10245,#10246);   origin (0,0,3.45)
#10248=IFCEXTRUDEDAREASOLID(#10237,#10247,#19,0.450000);
#10249=IFCSHAPEREPRESENTATION(#22,'Body','SweptSolid',(#10233,#10248));
```

## 4. Slivers, small pieces and the stub

The pieces of a wall's plan outside the floor are drawn at the wall's full height, so each piece Revit cuts anyway is a defect. The scorer prints, for each piece of rvt-rs's top, how much of it lies at the wall's full height in Revit's body too (`piece` lines). As built:

| wall | piece | least width (ft) | area (ft2) | at full height in Revit's body | inside another wall |
|---|---|---:|---:|---:|---:|
| 1457415 | strip along a face, 19.7 ft | 0.0321 | 0.631 | 99.6% | none |
| 1892033 | end past the floor's edge | 0.0321 | 0.0210 | 100% | none |
| 3123954 | end past the floor's edge | 0.0328 | 0.0127 | 81% | none |
| 1488456 | sliver along a face (Revit's export has the same piece) | 0.0008 | 0.0090 | 60% | none |
| 1348323 | appendage of a layered plan | 0.0984 | 0.0048 | 100% | none |
| 1348323 | appendage of a layered plan | 0.1160 | 0.0135 | 100% | none |
| 3039425 | appendage of a layered plan | 0.1804 | 0.0163 | 100% | none |
| 1630966 | part of the plan outside the floor | 0.2133 | 4.31 | 100% | all |
| 3004978 | stub at the wall's end | 0.0853 | 0.0243 | 0% | all |

At permit, 1429758's piece (0.0696 ft wide, 0.0048 ft2) is kept too. Revit keeps the three 0.032 to 0.033 ft slivers RE-267 §5 listed (1457415's strip, 1892033's end and 3123954's start), so they are drawn. The one piece it cuts is the stub of 3004978, 0.085 ft long past the floor's edge and wholly inside the plan of the wall it abuts (1630903, which shares the storey). Nothing separates it from the pieces Revit keeps:

- Width does not: kept pieces measure 0.0328, 0.0696, 0.0984 ft and wider, the cut one 0.0853.
- Area does not: kept pieces measure 0.0048 to 0.021 ft2 and the cut one 0.0243.
- Lying inside another wall does not: 1630966's piece (4.3 ft2) and 1493966's (213 ft2) lie inside other walls and are kept. In the plans the pass reads, the small pieces (0.0005 to 0.03 ft2) of 2984235, 2984610, 2992096 and 1478932 lie inside other walls too, and Revit keeps them: a first draft that left every wall with a small piece inside another wall as it was took those four walls off Revit's tops (the full-height area rvt-rs draws where Revit cuts rose from 236.6 to 381.8 ft2 as built, 38.1 to 163.0 at permit).
- What separates it is that it is all of the wall that lies outside the floor: 0.0243 ft2 of 1.616, every piece of it inside another wall. No other wall in either file has so little outside its floor and all of it inside neighbours (1892033, 3123954 and 1488456 have as little, none of it inside a neighbour). The wall is left as it was, with its full height, listed here, and the rule rests on this one wall: `wall_tops::embedded`.

## 5. An element that only a cut pair names

RE-267 reads the elements a wall's top join (`JoinToRoofGStep`, `m_baseOrTop` 1) and its `CutPairGStep`s name, and treats them alike. The first stepped draft cut three walls wrongly. 3484876, 3490093 and 3490551 (as built) name their slab only by a cut pair (`probe_1067_native_dump`: `CutPairGStep` and `CleanPairGStep`, no `JoinToRoofGStep`), and that slab's top (182.087 ft) lies 0.984 ft below the walls' top (183.071 ft): the walls pass through it, and Revit leaves their tops whole. Cutting them at the slab's underside put rvt-rs 19.5 ft2 lower than Revit's body (11.4, 7.3 and 0.8 ft2; the plan measure's `lower` was 198 points) and made 3484876, which was equal, differ.

A Join Geometry cut takes the volume the two elements share. It is the wall's top only where the element reaches it. The rule now: an element that only a cut pair names cuts a wall's top only where its top lies at or above the wall's top (0.001 ft). A top join cuts wherever the element is: 1 wall as built (1486440, 0.089 ft) and 4 at permit (1486440, 3039425 and 3074316 at 0.098 ft, 1630966 at 0.919 ft) name a floor by a top join whose top lies below the wall's, and Revit cuts them all.

Evidence: of the 23 office walls as built that a floor named only by a cut pair cut in the first draft, 20 have that floor's top at or above the wall's (0.000 to 1.476 ft) and Revit cuts them (the 2 RE-267 cut whole and 18 stepped; all within the 0.5 ft2 of the misfit above but 1347198 and 1347199, whose insulation layer Revit ends lower, §7); 3 have it 0.984 ft below, and Revit leaves them. At permit all 10 reach the wall's top. The rule narrows RE-267's: it applies to the whole cut as well, and changes none of its 64 and 70 walls, whose elements all reach the wall's top.

## 6. Quantities, property sets, schema, layers, openings and other outputs

Measure [41741031542](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41741031542) (both offices, every scorer below, `main` against the branch; the same scorers on the head before the glTF change are in 41739882963 and agree where they overlap).

- **`qtos_vs_ifc`**, byte-identical to `main`'s. As built: Revit's quantities on objects rvt-rs writes 39,388, equal 29,448, different 1, missing 9,939, rvt-rs quantities Revit does not write 0. At permit: 39,679, 29,762, 1, 9,916, 0.
- **`psets_vs_ifc`**, byte-identical. As built: Revit's values on objects rvt-rs writes 45,525, equal 44,163, different 1, missing 1,361, rvt-rs values Revit does not write 10. At permit: 45,704, 44,339, 1, 1,364, 14. A stepped wall gains `BodyHeightSource`, `TopCoverElementId` and, on 4 walls in each file, `TopCoverElementIds` in rvt-rs's own `RvtElementRecordGeometry` set, which the census leaves out (as it left out the same two on RE-267's cut walls).
- **`Qto_WallBaseQuantities` on the stepped walls**: moved on 0 of the 49 (as built) and 0 of the 46 (at permit). rvt-rs writes `Width` on each and nothing else; Revit writes `Length` and `NetVolume` on every one, `Height`, `GrossVolume`, `GrossSideArea` and `NetSideArea` on 43 of 49 and 43 of 46, and `GrossFootprintArea` on 6 and 4. No quantity is computed from the stepped body: the wall's `extrusion` is the full prism, and `Height` (the body's span, RE-257), the side areas and the volumes are written only on walls with no top join (RE-266, RE-270), so none is written on a stepped wall. Writing `Height` on them would be Revit's on 43 and omitted by Revit on 6 (3 at permit), and nothing read tells those apart (RE-267 §7).
- **`wall_quantities_vs_ifc`**, the candidates a rule could write, which read rvt-rs's body and write nothing: `NetVolume = rvt-rs's volume` equals Revit's on 258 of 607 walls as built (243 before) and 251 of 589 at permit (241); the gross side area and gross volume candidates equal Revit's on 1 to 7 fewer walls (for example `GrossSideArea`'s A gross, 341 to 334 of 579 as built and 320 to 319 of 565 at permit), and `GrossFootprintArea` on 2 and 3 fewer. Since nothing is written from them, `different` and `values Revit does not write` do not move.
- **Schema step** (`schema.txt`): `validate` findings 0 and `rules` findings 4 on both offices and both sides, the same four (`IfcAirTerminalType`, `IfcElectricApplianceType` and `IfcSpaceType` `CorrectPredefinedType`, and `IfcRelAssociatesMaterial.AllowedElements`), none on an `IfcWall`, an `IfcExtrudedAreaSolid` or an `IfcRelVoidsElement`. The only line that differs is the count of swept solids with a Position distinct from their product placement, 12,747 to 12,921 as built and 12,790 to 13,013 at permit: the standing pieces. A stepped wall is written as `IfcWall`, never `IfcWallStandardCase` (rvt-rs writes none), with its `IfcMaterialLayerSetUsage` as before, and the `SweptSolid` items are extrusions of closed polylines, which no rule restricts to one.
- **Layers** (`wall_layers_vs_ifc`, `layer_sets_vs_ifc`, `glb_material_colours_vs_ifc`): byte-identical to `main`'s on both offices. As built, 602 walls are drawn in layers in the GLB (303 of one layer), 598 compared, layer centres within 0.001 ft on 583 of 598. The first stepped draft dropped 44 stepped walls from that count (602 to 558) because the glTF cut only a wall's extrusion into layers; `body_geometry::layered_stepped_meshes` cuts a stepped body into the same layers (the first item's plan sets the bands and every piece is clipped to them), and the count is back at 602.
- **Openings.** `opening_boxes_vs_ifc`, `openings_vs_ifc` and `opening_hosts_vs_ifc` are byte-identical to `main`'s. 14 of the 49 stepped walls as built hold 269 openings and 16 of the 46 at permit hold 271; every stepped wall has the same `IfcRelVoidsElement` openings as on `main`. IfcOpenShell voids each item of a multi-item body (Revit's own layered walls are 3 to 8 items with openings), and the meshed bodies agree: on the 13 and 15 stepped walls with horizontal faces below the top in the upper half of the body (opening heads), the same faces with the same areas remain after the change.
- **Other outputs.** `rvt-gltf` exits 0 on both offices; `wall_bodies_vs_ifc` (the GLB's wall faces and ends against Revit's) and `glb_material_colours_vs_ifc` are byte-identical to `main`'s. The plan sheet and the depth of a door's or window's cut read `body_geometry::plan_outline`, which now returns a stepped wall's first-item profile as the wall's extrusion's was, so a layered wall's stepped plan is not filled in by a hull. `SolidShape::Compound` is already used for stairs, and the viewer reads it through the same GLB.

## 7. What is not drawn

- The 78 walls as built (127 at permit) whose top Revit's body steps or breaks and that no floor reaches are out of scope, since this change draws floors' undersides. They are not floor tops: on 63 (99) Revit's body has no point at the first or last station of the centre line, a wall end shorter than rvt-rs's (69 such ends as built, 0.23 to 1.5 ft, 29 of them one 0.25 ft station); on 13 (24) there is a gap inside the wall, where an opening reaches its top or a hosted instance cuts it; on 2 (4) a step (2300434 and 2602205 as built).
- A wall whose layers Revit ends at different heights: the finish layers of the KS walls 1347198, 1347199, 1347200 and 1347201 end 0.5 to 1.5 ft below their core's top (Revit's tessellated layers measure 9.678 and 10.171 ft, or 10.991 and 12.467 ft, from the same base), so rvt-rs's one-height body is taller than Revit's by 102.9, 14.3, 35.7 and 11.7 ft2 as built. 2602205 (as built and at permit) and 2601954 (at permit) are cut by Revit at 190.289 ft, a height no floor, roof or footing they name gives (their floor's underside is at 194.390 ft); 1494012, 3490093 (which names three `WallCutoutGStep`s) and 1621552 differ for reasons this change does not read.
- 3004978's stub (§4) and the 3 walls of §5 keep their full height; the first is wrong by 1.6 ft2, the others are Revit's. RE-375 reads Revit's own bodies over 225 pieces of five buildings for what separates the stub from the pieces Revit keeps: nothing that rests on more than one wall.
- Several elements that share a hole or opening the wall crosses, more than 32 pieces, and a plan that does not break into convex pieces leave a wall as it was.
- `Height` and the side areas and volumes of a stepped wall are not written (§6).
- A wall that one element covers whole and another covers in part at a lower underside takes the lowest underside over its whole plan, as on `main`: it is taller than Revit's over the part the lower element does not cover.
- The room schedule's Width, Depth and Height columns are empty for a stepped wall, whose body is no longer one extrusion; and `plan_outline`'s new branch for several items reads any compound body's first item (stairs included), with no measured change.
- The GLB's stepped tops are not scored: no scorer reads a GLB wall's top, so they rest on the code (`layered_stepped_meshes`) and the layer counts above.
- `wall_ends_list` is the same rows in another order but for wall 1344124, whose end moves from `+0.0000` to `-0.0000` (below 5e-5 ft).

## 8. Reproduce

The scorer `tools/re/wall_top_cover_vs_ifc.py` is on `main` once this lands, so `--ref main` carries it (the runs above were dispatched with `--ref re/1067-stepped-wall-tops` before it landed).

```bash
gh workflow run measure.yml --ref main -f ref=b23adf4fbc99d56f45a46f1772139e5fb0c7235f -f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true
gh workflow run measure.yml --ref main -f ref=b23adf4fbc99d56f45a46f1772139e5fb0c7235f -f base=main -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" \
  -f scorers="wall_top_cover_vs_ifc wall_heights_vs_ifc wall_bodies_vs_ifc wall_layers_vs_ifc layer_sets_vs_ifc glb_material_colours_vs_ifc schema qtos_vs_ifc psets_vs_ifc opening_boxes_vs_ifc openings_vs_ifc opening_hosts_vs_ifc wall_quantities_vs_ifc"
gh workflow run measure.yml --ref main -f ref=b23adf4fbc99d56f45a46f1772139e5fb0c7235f -f base=none -f nordic=true -f models="nordic-office-*" \
  -f scorers=none -f probe=probe_1067_native_dump -f probe_scorer=-
```

The scorer is this branch's (it gains the plan-wide comparison, the `faces` and `piece` lines and the Revit body forms in this change), so `--ref` is the branch until it lands. `b23adf4fbc99d56f45a46f1772139e5fb0c7235f` is the code measured; the commits after it are documents and the merge of `main`.

