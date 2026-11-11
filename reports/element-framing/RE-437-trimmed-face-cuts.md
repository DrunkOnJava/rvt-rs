# RE-437: a face cut by its trim is cut across at the chord's v steps and at the v of the trim's own steps and cusps, where every trim point's v was a cut: 25 of 126 exports move, and their triangles from saved graphics fall by 0.5% to 10.6%, with no crack added

**Date:** 2026-11-11
**Issues:** #1737 (the triangle density of the faces RE-430 and RE-426 draw: this is its second lever); follows RE-430 (#1715), RE-435 (#1744, the first lever) and RE-426 (#1709); the crack criterion is rvtDevelop's, from the read of PR #1769
**Artefacts:** the BIM4LCA office, housing and terrain models, Autodesk's samples of Revit 2023 to 2027 and the reference models, and Revit's IFC4 exports of those that have one (RE1, the office, the housing: scored by Measure itself; Snowdon Towers Architectural's is local only: ratios, distances and triangle counts only here); rvt-rs's exports by `main` (with #1744) and by the change, which Measure kept (`keep_ifc`)
**Probe and scorer:** the diagnostic build of RE-435 (`refs/archive/2026-11-11/scratch/1737-density`), whose DENSITYFACE lines an offline replay of the strip rule reads; `tools/re/mesh_bodies_vs_ifc.py` (Measure's own results), `tools/re/ifc_body_diff.py` and `tools/re/mesh_tjunctions.py` (new: the cracks, section 2); the pure helpers `trim_steps` and `trimmed_cells` are tested in `src/research/native_parametric_mesh.rs` (four failure-first tests beside the existing ones: a trim point between the `v` knots is a corner and not a cut, a point on a vertical side between them is covered all the same, the vertices a strip beside passes by are the steps and the cusps, and a step of the trim is a vertex of the strips on both sides of it when its `v` is a knot; the refusal of a point off the `u` knots stays)
**Runs:** the change as measured, Measure [43091480051](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43091480051) (unfiltered, `keep_ifc`, code `70bef87a`, which is `ee2d9dfe` merged over `main` `a0c1c61a`, against `main` `a0c1c61a`; GitHub's artifact outage of 2026-11-11 failed the uploads of 8 shards, which `gh run rerun --failed` re-ran as attempt 2, so every other result is attempt 1's) and its `timing` [43091485302](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43091485302); the first design, Measure [43078686248](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43078686248) (code `aafa5b1a`, against `main` `5e3ac5b0`) and its `timing` [43082087751](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43082087751), kept as a negative result (section 2)
**Status:** Positive, with one measured negative. The six rows registered on #1737 before the run hold on the figures they name. No element drawn at `main` stops being drawn (BodySource changes 0) and 25 of 126 exports move, the same 25 as the first design, by the same objects on every pair whose IFC was kept. The triangles from saved graphics fall by 0.5% to 10.6% (the offices by 7.5% to 7.7%, a toilet by 2,509, Snowdon Towers Architectural by 6.8%, `rac_advanced` by 10.4% to 10.6%) and the IFCs by 0.4% to 6.6%. The cracks (a vertex of a face's cells inside a boundary edge of the same face's cells) are identical per class to `main`'s on the office as built, the timber office, the housing and RE1, and fewer on `rac_advanced`; none is in a toilet. No class's median volume against Revit's export moves and no count of bodies within 1% of its volume, or with every vertex of Revit's within 0.001 ft, falls. The negative: one body of RE1 Architecture, a shower head, has its farthest vertex from Revit's surface move from 0.009123 to 0.01007 ft, over the 0.01 ft column. Export time reads 0.97 to 1.02 and peak memory 0.98 to 1.05 of the base's.

## 1. The question

After RE-435 a toilet of the BIM4LCA office is 14,025 triangles where Revit's export has 1,668, and two of its faces are cut by their trim: 3,310 and 837 triangles on the grid RE-435 found, 4,147 together, a quarter of the toilet. Which of their cells does the chord the grid keeps not need?

## 2. Where a trimmed face's cells come from

`trimmed_cells` cuts a face into strips between the knots of `u`, so that each trim segment that meets a strip crosses it whole and the strip's region is a set of trapezoids between segments, and cuts each trapezoid across at every knot of `v` that lies inside it. `face_grid` made the knots of both axes the chord's steps and every trim point's coordinate. A trim point's `u` has to be a knot: it is what makes every segment cross its strips whole. A trim point's `v` as a cut ran across every trapezoid that held it, however far the trim point lay from the cell: a trim of T points left every trapezoid T cuts more, so the cells grew with the product of the strips and the points.

An offline replay of the rule on the DENSITYFACE lines of the office (the four distinct trimmed faces, one ruled and three revolved, whose 9,308 triangles the replay reproduces within 2) counts what the strips give with the cuts at the chord's own steps alone: 4,904 (-47%). A toilet's two trimmed faces go from 837 to 88 and from 3,310 to 912 triangles, the ruled face from 4,678 to 3,718.

### The first design left cracks, and what its rows had not looked for

That design (the cuts at the chord's steps alone; Measure 43078686248, code `aafa5b1a`, archived as `refs/archive/2026-11-11/cited/aafa5b1a2f15`) held the five rows registered for it: the office's triangles fell by 8.1% (a toilet from 14,025 to 11,314) and no class's median volume against Revit's export moved. rvtDevelop's read of #1769 counted what those rows did not: the vertices of a face's cells that lie strictly inside a boundary edge of the same face's cells, at 0.02 to 0.98 of the edge's length and within 0.003 ft of it (`tools/re/mesh_tjunctions.py`), where the edge of one cell runs straight past a vertex of the next and the two leave a crack as wide as the surface bows over the edge. On the office as built `main` has 160 such vertices in 57 face sets (doors 66, railings 32, light fixtures 62) and the first design 504 in 100, the 344 more all in the 43 toilets (8 each, 0.0008 to 0.0018 ft from the edge they sit in).

The cause is a vertex of the trim on a strip boundary that the trim does not run through: the end of a step of the trim along the boundary, or a cusp such as the tip of a hole. It is a vertex of the cells of one strip only, and the edge of the next strip's cells runs straight past it. `main` had none of them because every trim point's `v` was a cut: the cut across the whole face made it a vertex of both strips' cells. The first design dropped all of those cuts, and these with them.

## 3. The rule

`face_grid` adds the trim points' `u` to the knots of `u` and, for a face cut by its trim, only the `v` of the trim's steps and cusps (`trim_steps`) to those of `v`; a face meshed on its grid keeps every trim point on both axes, as before. A vertex of a trim ring is a step or a cusp where its two neighbours are not on opposite sides of its `u` (a neighbour on the strip line, within the tolerance `trimmed_cells` uses for a segment along a strip's edge, is on neither side): the trim runs through every other vertex from one strip to the next, so both strips have it for a corner and it needs no cut. `trimmed_cells` no longer asks a trim point's `v` to be a knot (a trim point off the `u` knots is refused as before). What is lost against `main` is a trim point strictly inside a run along one `u` that is not a step: it is a vertex only where a chord step cuts at its `v`, and no edge of its two strips' cells runs past it. A plane face that shares an edge with a curved face takes its vertices along that edge from the curved face's grid (`refined_plane_edge`, from `face_grid`'s `v`), so it follows the change without one.

## 4. Against Revit's export

Measure scores the changed bodies against Revit's export itself for the office, the housing and the reference models (`mesh_bodies_vs_ifc.py`). Over the 24 results that hold a table, one class row differs between the two builds, `IfcFlowTerminal` of RE1 Architecture; the largest change of any class's median volume ratio is 0.0000, and no class's count of bodies with the volume within 1% of Revit's, or with every vertex of Revit's within 0.001 ft of rvt-rs's surface, falls:

| what | `main` | now |
|---|---|---|
| office, `IfcSanitaryTerminal` (the toilets; 90 of 90 within 1%) | median volume 1.0018 | 1.0018 |
| housing toilet `WC-istuin:WC-1` (Tag 1714069) | volume 1.0060, Revit's vertices 0.005007 ft from the mesh, rvt-rs's own 0.02144 ft from Revit's surface | 1.0060, 0.005007 ft, 0.02142 ft |
| RE1 Architecture `IfcFlowTerminal` (7 bodies) | volume within 1%: 5; every vertex of rvt-rs's within 0.01 ft of Revit's surface: 5, within 0.001 ft: 2; every vertex of Revit's within 0.001 ft of rvt-rs's: 3 | 5; 4, 2; 3 |

**A measured negative, not a pass.** The shower head of RE1 Architecture ("Shower Head, Hand Held Sprayer", Tag 369643) has its farthest vertex of rvt-rs's from Revit's surface move from 0.009123 to 0.01007 ft, over the 0.01 ft column of its class (5 to 4); its volume ratio 1.1047 to 1.1046 and Revit's vertices to rvt-rs's surface 0.00103 ft both times. The figure is of rvt-rs's vertices against Revit's own tessellation, which lies up to 0.005 ft from the true surface, and the registered columns (the median volume ratio, the volume within 1%, every vertex of Revit's within 0.001 ft of rvt-rs's) do not move; it is reported because it is a line the first design crossed too, and a vertex the steps design cut out of this one body is still one that was nearer Revit's.

## 5. The whole run, and what the change costs

Measure 43091480051, unfiltered with `keep_ifc` and every model set, `70bef87a` against `main` `a0c1c61a`: 126 exports, **101 identical and 25 moved** (24 with a kept result, one held-out one with counts only), the same 25 as the first design. `tools/re/ifc_body_diff.py` on the seven pairs whose IFCs I kept: BodySource changes 0 on each and no object on one side only, and the meshes that differ are 43 of the 91 `IfcSanitaryTerminal` (the 43 toilets) of the office as built and of the timber office as built, 15 of the housing concrete as built (9 `IfcCovering`, 6 `IfcSanitaryTerminal`), the 9 `IfcFurniture` of the housing terrain timber model, 1 `IfcSanitaryTerminal` of RE1 Architecture, 71 of `rac_advanced` 2023 (70 `IfcFurniture`, 1 `IfcRailing`) and 140 of the 4,303 objects Snowdon Towers Architectural draws from a saved mesh (102 `IfcFurniture`, 37 `IfcSanitaryTerminal`, 1 `IfcSpace`). The diagnostics of all 25 are the same on both sides: `body_sources` and `saved_mesh_refusals` do not move.

| exports | triangles from saved graphics, `main` to the change | IFC |
|---|---|---|
| BIM4LCA office (4) | 1,432,889 to 1,325,002 (-7.53%) as built, -107,887 on each of the four (timber -7.65%); a toilet 14,025 to 11,516 | 117.1 to 113.6 MB (-3.0%); timber 107.2 to 103.6 MB (-3.3%) |
| BIM4LCA housing, the four large | 372,251 to 355,251 (-4.57%) | 23.4 to 22.8 MB (-2.4%) |
| BIM4LCA housing, the two terrain | 291,458 to 281,888 (-3.28%) | 13.9 to 13.6 MB (-2.4%) |
| Snowdon Towers Architectural | 6,252,307 to 5,826,472 (-6.81%) | 272.1 to 257.6 MB (-5.3%) |
| Snowdon Towers Plumbing, Structural | 2,470,237 to 2,436,229 (-1.38%) and 257,598 to 254,182 (-1.33%) | 118.5 to 117.3 MB (-1.0%) and 14.6 to 14.5 MB (-0.8%) |
| `rac_advanced` 2023 to 2027 | 1,457,506 to 1,305,375 (-10.44%) on the 2023 save, -152,131 on each | 81.1 to 76.0 MB (-6.4%) on the 2023 save, -5.2 MB on each |
| `rac_basic` 2023 to 2027 | 713,280 to 709,642 (-0.51%) on the 2023 save, -3,638 on each | 29.4 to 29.2 MB (-0.4%) |
| RE1 Architecture | 23,466 to 23,082 (-1.64%) | 1.4 MB (-0.9%) |

The steps design keeps 93% of the first design's saving on the office (107,887 of 116,573 triangles), 83% on the large housing files (17,000 of 20,451), 69% on `rac_advanced` (152,131 of 219,521), 79% on Snowdon Towers Architectural (425,835 of 541,807) and 36% on the terrain models (9,570 of 26,796); what it gives back is the cuts that stop its cracks.

**Cracks** (`tools/re/mesh_tjunctions.py`, vertices of a face's cells inside a boundary edge of the same face's cells, `main` to the change): the office as built 160 in 57 face sets to 160 in 57, class by class the same (`IfcDoor` 66, `IfcRailing` 32, `IfcLightFixture` 62), with none in an `IfcSanitaryTerminal` (the first design: 504 in 100); the timber office as built 168 in 57 to 168 in 57 (`IfcDoor` 74, `IfcRailing` 34, `IfcLightFixture` 60); the housing concrete as built 56 in 7 to 56 in 7 (all `IfcDoor`, the six changed toilets hold none); the housing terrain timber model 81 in 11 to 81 in 11; RE1 Architecture 2 to 2; `rac_advanced` 2023 394 in 19 to 374 in 19 (`IfcFurniture` 346 to 326); Snowdon Towers Architectural 2,272 in 105 face sets to 1,957 in 105 (`IfcSanitaryTerminal` 1,225 to 910, the other classes the same: `IfcElectricAppliance` 594, `IfcBuildingElementProxy` 420, `IfcRailing` 28, `IfcRoof` 3, `IfcSpace` 2).

Export time and peak memory (`timing` 43091485302, three rounds of both builds on one runner, 74 files) read 0.97 to 1.02 and 0.98 to 1.05 of the base's; the office as built reads 1.00 and 1.00 (34.1 s and 1,221 MiB), Snowdon Architectural 1.00 in time (44,612 to 44,635 ms) and 0.98 in peak memory (1,430 to 1,407 MiB). The one file above 1.03 is `racbasicsampleproject` 2023 (peak 347 to 363 MiB, 1.046), inside the 4% to 5% that RE-435 measured as the noise of peak memory on a file whose export it moves by 0.5%.

## 6. The registered rows

Registered on #1737 before the run ([comment 6805810741](https://github.com/DrunkOnJava/rvt-rs/issues/1737#issuecomment-6805810741)):

1. **Held.** No export grows: of the 24 moved exports with a kept result none has more triangles than `main`'s, and the one held-out export that moves holds counts only.
2. **Held.** Every element drawn at `main` is still drawn from its saved mesh (BodySource changes 0; the diagnostics of the 25 are identical, so no refusal is new); the meshes that change are the same objects as the first design's, an element with a face cut by its trim or a plane face that shares an edge with one.
3. **Held on the registered columns, with the shower head as a measured negative.** No class's median volume ratio moves by more than 0.002 (the largest change is 0.0000) and the counts within 1% of Revit's volume and with every vertex of Revit's within 0.001 ft fall on no class; the 0.01 ft column of rvt-rs's own vertices, reported and not held, falls from 5 to 4 on RE1's `IfcFlowTerminal` (section 4).
4. **Held.** The office's triangles from saved graphics fall by 7.53% (5% to 8% registered; about 2,100 fewer a toilet predicted and 2,509 measured, all 107,887 of them in the 43 toilets) and Snowdon Towers Architectural's by 6.81%; the sizes are in section 5.
5. **Held.** No class gains a crack against `main` on the office as built, the timber office and the housing (identical per class), RE1 or `rac_advanced`, and no toilet holds one; Snowdon Towers Architectural, which the row did not name, loses 315 (its sanitary terminals).
6. **Held.** Export time and peak memory read no more than 1.05 of the base's on any file (1.046 at most, `racbasicsampleproject` 2023) and the office reads 1.00 in both.

## 7. What the change leaves

The chord stays 0.001 ft, and a toilet is still 11,516 triangles where Revit's has 1,668. What is left is the chord itself: Revit's own export holds its vertices up to 0.005 ft from the surface where the grid holds 0.001 ft. A coarser chord is a decision about the accuracy bar the scorers hold meshes to (every vertex of Revit's within 0.001 ft of the mesh) and not a defect to fix; #1737 holds its measurement (the third lever, scratch branches `research/1737-chord-2x`, `3x` and `5x`, never for `main`) and the maintainer's owner decides it. The faces cut by their trim keep every trim point's `u` as a strip boundary and the `v` of their steps and cusps as a cut, so a face of a long trim is still a few hundred triangles; a sparser design (chord-sized cells whose boundary cells only are cut by the trim) comes to 2,784 on the office's four trimmed faces against the 4,904 of the first design, but every whole cell it leaves has a neighbour that holds the trim's vertices on their shared edge, and closing those T-junctions is a cost this does not pay.

## 8. Reproduce

```bash
# the change's run (section 5): the scorers come from main, the code from the archived head
gh workflow run measure.yml --ref main -f ref=70bef87acfbec0808e14f07f3556b319a1d11bc0 -f base=a0c1c61aa6677b06e14fd461466cfb95508ab32d \
  -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
gh workflow run measure.yml --ref main -f ref=70bef87acfbec0808e14f07f3556b319a1d11bc0 -f base=a0c1c61aa6677b06e14fd461466cfb95508ab32d \
  -f samples=true -f nordic=true -f timing=true -f scorers=none
python3 tools/re/ifc_body_diff.py <main model.ifc> <change model.ifc>
python3 tools/re/mesh_tjunctions.py <model.ifc>   # needs ifcopenshell and numpy; prints the cracks per class
```

`70bef87a` is archived as `refs/archive/2026-11-11/cited/70bef87acfbe` (the branch's head when it was measured: `ee2d9dfe`, the steps rule, merged over `main` `a0c1c61a`; the commits after it change the report, the changelog, the reconnaissance addendum and the pull request, and no source).
