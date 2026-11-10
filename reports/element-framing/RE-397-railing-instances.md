# RE-397: the instances of a railing's saved graphics that the walk skipped, and the flip control that is no surface

**Date:** 2026-11-10
**Issues:** #1635 (railings are written as record boxes, about a thousandth of Revit's volume), #1624, #1247; follows RE-224 (what keeps a saved mesh from being whole), RE-303 (a saved flip control holds no geometry, but marks railings whose mesh is not whole) and RE-318 (the branches the visibility flags hide)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024; Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them); puzzbobb's four Revit 2025 `Rooms_Stairs` models (Revit 25.0.2's IFC4 exports); RE1 Architecture (Revit 2025, IFC2X3); Autodesk's sample projects, which Measure fetches for a run and never stores
**Runs:** Measure [42874785566](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42874785566) (`probe_re397_railing_saved_graphics`), [42876011110](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42876011110) (`probe_re397_railing_graphics_tree`), [42877186261](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42877186261) (`probe_re397_railing_hidden_graphics`), [42878121436](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42878121436) (`probe_re397_railing_symbol_meshes`), [42880570119](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42880570119) (`probe_re397_railing_instances`) [42882772399](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42882772399) (`probe_re397_symbol_graphics_tree`), [42894014427](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42894014427) (`probe_re397_railing_detail_levels`), [42895612609](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42895612609) (`probe_re397_post_symbols`) and [42901720548](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42901720548) (`probe_re397_graphics_copies`), each on the office as built with the probe of this change's branch; Measure [42884022203](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42884022203) for the change, and [42889243281](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889243281) with [42889259936](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889259936) (`probe_re397_mesh_classes`, on the change and on `main`) for the classes it moves
**Status:** Positive where Revit's export checks it, and not as registered on the second project (section 5.2), which the maintainer decided on 2026-11-10 ships with the miss stated. The instances of a railing's saved graphics that the walk skipped, hidden in the normal draw and forbidden the targets 53246, are the railing's continuous rails (35 of the office's 41 railings list continuous rails, and exactly those 35 hold restricted instances); they hold the part of its body RE-303 found missing, and a flip control is no surface. Drawn, with the flip control left out of the walk, 24 railings that Revit's own exports hold come from their saved meshes (15 of the office's 41, in both of its files, 2 on each of the four Revit 2025 stair models and 1 on RE1 Architecture), all with their bounding box within 0.01 ft of Revit's. 19 of the 24 have their volume within 1% (the ten `KAI-4` and the `KAI-1` of the office, and the 8 Revit 2025 railings). Of the other 5, the two small `KAI-5` are 2% under (round sections that rvt-rs draws with fewer segments than Revit's polygons, section 5.3) and RE1's one is 1.75% over, with every vertex within 0.011 ft of Revit's surface, and the two `KAI-3` are 19.5% over, all of it in their posts: the file's saved graphics hold a round post of 0.328 ft diameter (0.1793 ft3) where Revit's export has a prism of about 0.256 by 0.204 ft (0.1125 ft3) whose corners lie on that circle, and their rails are 0.3% over (section 5.1). The row pre-registered on #1636 for the BIM4LCA housing and the timber office is not met as registered: the 43 and 56 railings of the large housing files are all drawn from their saved meshes, with every bounding box and every vertex within 0.01 ft of Revit's, but 31 and 40 of them are within 2% in volume (72% and 71%, under the registered 3 in 4), and the rest are 8.9% to 19.9% over for the reason of the `KAI-3` posts: on the 30 mm round rails the saved mesh draws the circle as a 32-gon and Revit's export as a hexagon inscribed in it (section 5.2). The four Revit 2025 stairs that aggregate their railings go from 2.6 to 3.0 times Revit's volume to Revit's. 224 more bodies on 19 of Autodesk's samples change from a record box to a saved mesh by the same walk, and no Revit export is there to score them. 26 railings of the office keep their boxes.

## 1. The question

#1635: the 41 railings of the office are written as their record boxes, about a thousandth of Revit's volume (`box_bodies_vs_ifc`: fill 0.001), because no railing's saved mesh is whole. RE-303 found that a `GBiFlipControl` is reachable in the 14 meshes it alone keeps as boxes, that the control holds no geometry, and that leaving it out draws meshes none of which is Revit's body (12 cover about 40% of Revit's volume). It left open why a railing's mesh lacks parts of Revit's body.

## 2. What a railing's saved graphics are

A railing's channel 103 record is a `GElement` with its `GInstance`s, their `InstanceInfo`s and an empty `Geometry`. Each instance names another element, by `m_symbolId`, as its symbol, and the railing's mesh is the meshes of those symbols (`probe_re397_railing_graphics_tree`). On the `KAI` railings the visible symbol is the railing's own ElementId plus one; a second symbol has a higher id, and the one read (`2620342`) is a wrapper whose own graph is one instance of yet another symbol.

`probe_re397_railing_instances` lists every instance of the 41 railings and the volume of the symbol it names, read as an element of its own. The 240 instances are of two kinds only, all rigid (their matrices are orthonormal, so a symbol's volume is the instance's):

| kind | flags | visibility | `m_targets` | instances |
|---|---:|---:|---:|---:|
| visible, unrestricted | 557060 | 0 | 0 | 19 |
| hidden in the normal draw, restricted | 557092 | 1 | 53246 | 221 |

The restricted instances are the railing's continuous rails. 35 of the 41 railings list one in `BaseRailing.m_continuousRailIds` (each an element with an ElementId of its own), and those 35 are exactly the railings that hold restricted instances: the 6 with an empty list (`KAI-3` 2614259 and 2618205 among them) hold none. On each of the 35 a restricted instance names the listed continuous rail's id (12 railings) or the id after it (23). Revit's IFC exporter adds the geometry of a railing's sub-elements, its top rail, its hand rails and their supports, to the railing's body (section 8), which fits the continuous rails being the part of the body that the railing's own saved graphics hold only as hidden, restricted instances. Whether the listed ids are those sub-elements is read from the names and the counts, not measured, and the match is the office's, 35 of 35 on one project; it is the reason to expect the pair to hold on the 224 railings of the samples that no export scores, and no more.

The traversal skips the second kind twice: as hidden in the normal draw (visibility 1, RE-318), and, were they walked (`extract_including_hidden`), by refusing a non-zero `m_targets` (`instance target restrictions are unsupported`), so even the hidden decode of `probe_re397_railing_hidden_graphics` finds no primitive more than the default one on any railing.

## 3. Their volume against Revit's

For each railing, the sum of the volumes of the symbols its visible instances name, and of those plus its restricted instances', over the volume of its body in Revit's export:

| railings | visible instances alone | visible plus restricted |
|---|---:|---:|
| `KAI-4`, 10 railings | 0.385 to 0.437 | 1.0000 to 1.0016 |
| `KAI-5`, 2903645 and 2903804 | 0.541 | 0.980 |
| `KAI-1`, 1485880 (no restricted instance) | 1.0002 | 1.0002 |
| `KAI-3`, 2614259 and 2618205 | 1.195 and 1.198 | the same |
| `KAI-2`, 1749890 and 2902063 | 0.612 | the same |
| `KAI-5`, 2598350 | 0.170 | 0.170 |
| `KAI-6`, 3076853 | 0 | 0 |
| `KJO-1`, 22 stair railings | 0 | 0.020 to 0.029 |

The 59% of a `KAI-4` railing's body that RE-303 found missing is in the restricted instance. Where the symbols are flat-faced the sum is Revit's to 0.2%. The two `KAI-3` railings read 1.195 and 1.198, and tessellation is not the reason: shell by shell, the 12 posts of 2618205 are 0.1793 ft3 each in rvt-rs's mesh against 0.1125 in Revit's (the 39 of 2614259 likewise) and the rest of the railing is 2.7553 ft3 against 2.7462 (+0.33%), so the posts are 0.802 ft3 of the railing's 0.810 ft3 excess (section 5.1). The rest are not an instance question: `KAI-2`, `KAI-5` and `KAI-6` lose faces to the parametric reader (`shared curved edge ...`, 105 diagnostics on `KAI-6`), and the 22 `KJO-1` railings hold only restricted instances whose symbols are mostly nested detail-family filters and refused faces.

## 4. The flip control

`probe_re397_symbol_graphics_tree` prints the whole graph of the visible symbol of a `KAI-4`, the `KAI-1`, the `KAI-3` and the small `KAI-5` railing. Each holds exactly one `GBiFlipControl`, a leaf under the root `GElement`: category 193, command 38798, an origin, a base vector and a `m_length` of 16.0, and no pointer to anything. It is the grip drawn in a view to flip the railing, as RE-303 read from its fields; it is no surface, and the railing's mesh without it is the railing's body, which was not the case when RE-303 left it out because the restricted instances were still missing.

## 5. The change

`native_graphics_traversal` takes an instance flagged hidden in the normal draw and forbidden the targets 53246 as drawn (`drawn_hidden_instance`), and walks a `GBiFlipControl` as a branch with no surface, as it does a curve or a point. Every other hidden instance stays skipped and every other target mask stays refused.

Measure 42884022203, unfiltered (`nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`), head `c12f408c` against `main` at `2713ebcb`, 118 models on each side. 26 exports move, with no census regression and no body or placement regression, and 263 bodies go from a record box to a saved mesh: 15 on each of the office's two files, 2 on each of the four Revit 2025 stair models, 1 on RE1 Architecture and 224 on 19 of Autodesk's samples (101 on Snowdon Towers Architectural 2024). The same-project gates pass on the head (15 samples held to their earlier saves).

| file | record boxes | railings drawn from a mesh | against Revit's export |
|---|---|---:|---|
| office as built and at permit | 183 to 168 and 184 to 169 | 15 of 41 each | bounding box within 0.01 ft on 15 of 15; volume within 1% on 11 (the ten `KAI-4` and the `KAI-1`; median 1.0009); every vertex of each within 0.01 ft of the other's surface on 11 (eight `KAI-4`, the two small `KAI-5` and the `KAI-1`: two `KAI-4` have a vertex of rvt-rs's 0.0154 ft from Revit's surface at a volume of 1.0010), both bars on 9; the two `KAI-3` read 1.195 and 1.198 and the two small `KAI-5` 0.980 (their vertices within 0.0008 ft; what the 2% is is not known) |
| Revit 2025 `Rooms_Stairs` 13 to 16 | 12 to 10 each | 2 of 2 each | volume ratio 1.0000, Revit's vertices within 3e-6 ft of the surface, 8 of 8 |
| RE1 Architecture | 4 to 3 | 1 of 1 | bounding box within 0.01 ft; volume 1.0175, vertices 0.011 ft |

### 5.1 The posts of the KAI-3 railings

Both `KAI-3` railings name one symbol for their posts, 2620217 (12 instances in the symbol of 2618205, 39 in that of 2614259), and rvt-rs draws each as the symbol's one solid: two cylindrical faces of radius 0.16404 ft and two planar caps, 2.1325 ft high, with a tessellated volume of 0.1793 ft3 (`probe_re397_post_symbols`; the symbol also holds 24 lines under 32 filters, 30 of them on the view direction, which are no surface). Revit's own export draws each post as a closed prism of 48 or 49 vertices and 92 or 94 triangles, 2.133 ft high, with extents 0.263 by 0.291 ft and a volume of 0.1125 ft3 on the 12 posts of 2618205 and 0.1126 on the 39 of 2614259: a rectangle of about 0.256 by 0.204 ft whose four corners lie at the radius of rvt-rs's cylinder, 0.164 ft, so the cylinder is the circle through the corners of the prism. The rails agree (2.7553 ft3 against 2.7462 on 2618205, +0.33%), and the 12 and the 39 posts are as many as Revit's.

What was looked at and does not explain it:

- **The detail level.** The walk reads the same 160 primitives and the same volume (4.906547 ft3 on 2618205) at detail levels 1, 2 and 3 (`probe_re397_railing_detail_levels`), and the post's symbol holds no filter on the detail level.
- **A second copy.** Every channel 103 record of the two railings, their symbols and the post symbol exists once in the file, in partition 425, the one its stored revision routes to; of the file's 72,799 channel 103 records, 5 are in two partitions and identical (`probe_re397_graphics_copies`, RE-145, RE-216).
- **The railing's continuous rails.** The two railings list none, and hold no restricted instance (section 2), so no sub-element's geometry is added to their body in Revit's export.
- **Tessellation of the rails.** The rails agree to 0.3%, and the cylinder's volume is the analytic one to 0.5%.

These did not explain it. Section 5.2 does: the same difference, measured shell by shell on the housing, where Revit's export draws a round section as an inscribed polygon with few corners. For these posts the cap of Revit's prism has 24 vertices, four of them at 0.164 ft from its centre, the radius of the saved cylinder, and the others on the chords between them.

### 5.2 The second project

A row pre-registered on #1636 ([comment](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6779306666), 2026-11-10 21:08 UTC, before any run of this rule on these models) held the rule to the BIM4LCA housing and the timber office: every railing the change draws from its saved mesh has its bounding box within 0.01 ft of Revit's; its volume is within 2% of Revit's on at least 3 of every 4 of them and within 25% on every one; no stair's volume moves away from Revit's. Measure run [42902914843](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42902914843) (head `0e64354c` against `main` at `0bb8067f`, the eight models, every default scorer):

| file | record boxes, before to after | railings drawn from a mesh | bounding box within 0.01 ft | volume within 2% | worst volume |
|---|---|---:|---:|---|---:|
| housing, concrete, as built and at permit | 119 to 76 | 43 | 43 | 31 (72%) | 1.1989 |
| housing, timber, as built and at permit | 132 to 76 | 56 | 56 | 40 (71%) | 1.1989 |
| timber office, as built and at permit | 182 to 167 and 191 to 176 | 15 | 15 | 13 (87%): 11 within 1% and the two small `KAI-5` at 0.980 | 1.198 |
| the two housing terrain files | 11 and 11 | none | | | |

On every housing railing the vertices of each export are within 0.01 ft of the other's surface (at most 0.0079 ft from Revit's to rvt-rs's, 0.0083 from rvt-rs's to Revit's), and every stair of the housing moves toward Revit's volume (`stair_bodies_vs_ifc`: the aggregate that carried the railing boxes, 1,083.5 ft3 on stair 1497456, is 73.35, Revit's 73.31).

**As registered, the row does not hold on the four large housing files**: its volume clause reads 31 of 43 and 40 of 56, under 3 in 4, while the bounding-box, 25% and stair clauses hold; it holds on the timber office. The 12 housing railings outside the bar on the concrete files and the 16 on the timber files, of the types `KJO-1`, `KJO-2`, `KAI-6`, `KAI-7` and `KAI-8`, read 1.089 to 1.199.

**What the outliers are (post hoc, 2026-11-10, after the reading).** Of the outliers that `mesh_bodies_vs_ifc` lists shell by shell, every shell that differs reads 1.198 to 1.200 of Revit's, and every other shell of the railing is Revit's to 4 digits: `KAI-7` 1347781 has two shells of 0.2083 ft3 against 0.1737 and 11 that are equal. The shells that differ are the railings' 30 mm round rails (their extent across is 0.0984 ft). The planar end faces of a rail in rvt-rs's export (kept by run 42906770416) and in Revit's own (`ARK_NordicLCA_Housing_Concrete_As-Built_Revit.ifc`, fetched by `tools/ci/fetch-bim4lca.py`), for the railings 1668202 and 1660002:

| | rvt-rs | Revit |
|---|---|---|
| end face | 32 vertices, 30 triangles | 24 vertices, 22 triangles |
| area | 0.00754 ft2 | 0.00629 ft2 |
| distance of its vertices from the face's centre | 0.04921 ft, on all 32 | 0.0423 to 0.0497 ft |

The area of Revit's face is that of a regular hexagon inscribed in the circle of rvt-rs's face, 2.598 x 0.04921 squared = 0.00629 ft2: 6 corners lie at 0.0495 to 0.0497 ft from the centre, 0.0003 to 0.0005 ft beyond the circle, and the other 18 vertices lie on the hexagon's edges (the nearest, 0.0423 to 0.0427 ft, is its apothem, 0.866 x 0.04921 = 0.0426). The ratio of the two areas, 1.1987, is the ratio of the shells' volumes (1.1988 to 1.1998), and the shells' height differs by 0.0132 ft, which is 2R - sqrt(3)R for R = 0.04921: a hexagon turned the way the tube leans. So the volume the scorer compares is the area of two polygons inscribed in one circle, one with 32 corners and one with 6: Revit's export draws the 30 mm rail with 6 segments around it, and rvt-rs's mesh of the faces Revit saved with 32. The `KAI-3` posts are the same difference at a larger radius (section 5.1: a quadrilateral inscribed in the saved cylinder's circle). The registered volume clause is blind to it, and the geometry bar of the scorers, each export's vertices within 0.01 ft of the other's surface, holds on all 43 and 56.

Whether a rule that missed its registered bar ships, and whether the bar is restated, is the maintainers' call: the maintainer decided on 2026-11-10 that #1652 ships with the miss stated, the bar not restated, and the volume scorer's blindness to a round section is repaired in its own pull request, which pre-registers its own row before the next measurement.

The `IfcStair` volume of the four Revit 2025 stair models, which aggregates the railings, goes from 25.92, 23.76, 30.93 and 24.99 ft3 (Revit's 8.78, 8.71, 9.30 and 15.29) to 8.782099, 8.707642, 9.297138 and 15.289523: the stair volume negative RE-390 disclosed, the railing boxes the stair carries, is closed on the 2025 stairs (all four are within 1e-6 of Revit's volume and every vertex of Revit's stair body lies within 3e-6 ft of rvt-rs's surface; one matches by the scorer's bar of the nearest vertex and the volume together, and the other three keep a vertex of their flights that is 0.033, 0.033 and 0.579 ft from rvt-rs's nearest, as the flight rows show unchanged, the second model's stair from 0.922 ft). The office's stairs are as they were: their railings are the 22 `KJO-1`, which keep their boxes.

**Which classes move.** `probe_re397_mesh_classes` counts the bodies of the default export by entity, on `main` and on the change (Measure [42889259936](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889259936) and [42889243281](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889243281): the office as built and the 2024 samples). The only entity whose saved-mesh bodies change is `IfcRailing`: 15 on the office, 8 on rac_advanced, 7 on rac_basic, 6 on each rme sample and 101 on Snowdon Towers Architectural, where `main` drew none from a mesh; no other class gains or loses one on those files, so leaving the flip control out of the walk changes no door, window, fixture or furniture body there. The 224 bodies on the 19 samples are railings, the 2023 and 2025 to 2027 saves of the same projects giving the same counts.

### 5.3 The same railings read shell by shell (#1658)

#1658 gives `mesh_bodies_vs_ifc.py` a reading of a volume ratio that is not 1 (`volume_readings.py`): the body is split into shells, each pair of shells is compared, and a shell is `explained` only where Revit's end faces are polygons inscribed in the circle of rvt-rs's end faces (corners within 0.001 ft of it, every other vertex on an edge) and its volume ratio is the ratio of the end faces' areas. Measure [42914771157](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42914771157) (the scorers of `6d498b8b`, the code of `main` at `5dd1277d`, the 12 BIM4LCA models) reads the railings of this section so:

| Railings | Volume ratio (shells that differ) | End-face area ratio | Reading |
|---|---:|---:|---|
| Housing, concrete as built: 12 of 43 (`KJO-1` x4, `KJO-2` x4, `KAI-7` x2, `KAI-8` x2) | 1.1985 to 1.1989 | 1.1989 | all `explained` |
| Housing, timber as built: 16 of 56 | 1.1985 to 1.1989 | 1.1989 | all `explained` |
| Office, `KAI-3` x2 (51 posts) | 1.592 to 1.593 | 1.602 | `explained` (0.7% apart) |
| Office, small `KAI-5` x2 (14 or 15 shells) | 0.9643 | 0.9643 | `explained` |

So the housing's 43 and 56 railings are within 2% in volume or explained, beside the 31 and 40 of the row as registered, which stays as registered; and the office's 15 railings drawn from saved meshes are within 1% in volume or explained. The two small `KAI-5` (0.980, "not located" before) are the other way round from the rest: rvt-rs draws their circles with fewer segments than Revit's polygons have, and the volume ratio is the end faces' area ratio to four digits. Nothing is `explained` that is not such a polygon: the housing's ramp flight (0.4907, a vertex 3.2 ft from Revit's surface and 26 more shells in Revit's), the doors with a shell Revit's export lacks and the walls stay `other`. The same reading finds that Revit's export writes the roofs and ramp flights that read exactly 0.5000 with every face twice (the housing's `VK-2` is 24 triangles of which 12 are distinct, and its distinct volume is rvt-rs's), which is not a railing's matter and is in #1658.

## 6. Not shown

- **Other projects.** The pair of visibility 1 and `m_targets` 53246 is the only restricted kind the office holds, and it holds it in railings only; the Revit 2025 stair models and RE1 Architecture check it on 11 more railings, and Autodesk's samples move 224 bodies that no Revit export scores, so what the pair means in a project whose railings Revit's export does not hold is the same walk, not a measurement. Every other hidden kind and every other mask stays refused.
- **What the mask means.** Which bits of `m_targets` name which draw target is not decoded; the rule is the measured pair, not a reading of the bits.
- **The railings that stay boxes.** `KAI-2` (2), `KAI-5` (1), `KAI-6` (1) and the 22 `KJO-1` keep their boxes for the reasons of section 3. Each of them has the diagnostics of its symbols, and #1645 puts the reason on the element.
- **The volume of a round section.** Revit's export draws a round section as an inscribed polygon with few corners (a hexagon on the housing's 30 mm rails, a quadrilateral on the `KAI-3` posts), so a railing with such a section reads over Revit's volume by the polygons' area ratio; the vertices bar is the one that holds on them (section 5.2).
- **The 1.75%.** RE1's one railing reads 1.0175 of Revit's volume with every vertex within 0.011 ft of Revit's surface; the part that differs is not located (the two small `KAI-5` that read 0.980 are located in section 5.3, and RE1 was not in that run), and a quantity is not written for a railing, so none carries a wrong one.

## 7. Reproduce

The probes come from `-f ref`, the commit of this change; the scorers from `--ref main`.

```bash
gh workflow run measure.yml --ref main -f ref=157971cc3d0185aa655278108e82d13cf6a65ec8 -f base=none \
    -f nordic=true -f models=nordic-office-as-built -f scorers=none \
    -f probe=probe_re397_railing_instances -f probe_scorer=-
```

The other eight probes name the same run with their own `-f probe`.

## 8. Sources

Autodesk's open-source exporter `revit-ifc` was read for what Revit's export writes of a railing, as `CLEANROOM.md` allows (the file and class are cited, nothing is copied or translated): `Source/Revit.IFC.Export/Exporter/RailingExporter.cs` (`ExportRailing` takes the railing's own solids and meshes and adds those of its sub-elements) and `Source/Revit.IFC.Export/Utility/GeometryUtil.cs` (`CollectSubElements` lists a railing's top rail, its hand rails and their supports, `GetIFCExportGeometryOptions` asks for the Fine detail level unless the export is restricted to a view, and `GetOneLevelGeometryElement` takes the symbol geometry of the instance at the base level). No Autodesk SDK source, disassembly or documentation under NDA was used.
