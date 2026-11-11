# RE-437: a face cut by its trim is cut across at the chord's v steps alone, where every trim point's v was a cut as well: 25 of 126 exports move, and their triangles from saved graphics fall by 0.6% to 15%

**Date:** 2026-11-11
**Issues:** #1737 (the triangle density of the faces RE-430 and RE-426 draw: this is its second lever); follows RE-430 (#1715), RE-435 (#1744, the first lever) and RE-426 (#1709)
**Artefacts:** the BIM4LCA office, housing and terrain models, Autodesk's samples of Revit 2023 to 2027 and the reference models, and Revit's IFC4 exports of those that have one (RE1, the office, the housing: scored by Measure itself; Snowdon Towers Architectural's is local only: ratios, distances and triangle counts only here); rvt-rs's exports by `main` (with #1744) and by the change, which Measure kept (`keep_ifc`)
**Probe and scorer:** the diagnostic build of RE-435 (`refs/archive/2026-11-11/scratch/1737-density`), whose DENSITYFACE lines an offline replay of the strip rule reads; `tools/re/mesh_bodies_vs_ifc.py` (Measure's own results) and `tools/re/ifc_body_diff.py`; the pure helper `trimmed_cells` is tested in `src/research/native_parametric_mesh.rs` (two new failure-first tests: a trim point between the `v` knots is a corner and not a cut, and a point on a vertical side between them is covered all the same; the refusal of a point off the `u` knots stays)
**Runs:** the change, Measure [43078686248](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43078686248) (unfiltered, `keep_ifc`, head `aafa5b1a` against `main` `5e3ac5b0`) and its `timing` [43082087751](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43082087751)
**Status:** Positive: the rows registered on #1737 hold on the figures they name, and one neighbouring figure and one letter of the timing row are reported below. No element drawn at `main` stops being drawn (BodySource changes 0) and 25 of 126 exports move, by the meshes of 766 of the 82,295 objects drawn from a saved mesh on both sides of the 24 whose IFC Measure kept (0.9%: furniture, sanitary terminals, coverings, railings, beams and a space). The triangles from saved graphics fall by 0.6% to 15% (the offices by 8.1% to 8.3%, a toilet from 14,025 to 11,314, Snowdon Towers Architectural by 8.7%, `rac_advanced` by 15%) and the IFCs by 0.5% to 9.4%. No class's median volume against Revit's export moves and no count of bodies within 1% of its volume, or with every vertex of Revit's within 0.001 ft, falls; one figure next to them crosses its line (a shower head of RE1 Architecture: its farthest vertex from Revit's surface 0.0091 to 0.0100 ft). Export time reads 0.95 to 1.03 and peak memory 0.96 to 1.04 of the base's.

## 1. The question

After RE-435 a toilet of the BIM4LCA office is 14,025 triangles where Revit's export has 1,668, and two of its faces are cut by their trim: 3,310 and 837 triangles on the grid RE-435 found, 4,147 together, a quarter of the toilet. Which of their cells does the chord the grid keeps not need?

## 2. Where a trimmed face's cells come from

`trimmed_cells` cuts a face into strips between the knots of `u`, so that each trim segment that meets a strip crosses it whole and the strip's region is a set of trapezoids between segments, and cuts each trapezoid across at every knot of `v` that lies inside it. `face_grid` made the knots of both axes the chord's steps and every trim point's coordinate. A trim point's `u` has to be a knot: it is what makes every segment cross its strips whole. A trim point's `v` need not be one, and as a cut it ran across every trapezoid that held it, however far the trim point lay from the cell: a trim of T points left every trapezoid T cuts more, so the cells grew with the product of the strips and the points.

An offline replay of the rule on the DENSITYFACE lines of the office (the four distinct trimmed faces, one ruled and three revolved, whose 9,308 triangles the replay reproduces within 2) counts what the strips give with the cuts at the chord's own steps alone: 4,904 (-47%). A toilet's two trimmed faces go from 837 to 88 and from 3,310 to 912 triangles, the ruled face from 4,678 to 3,718. A sparser design (chord-sized cells whose boundary cells only are cut by the trim) comes to 2,784 on the same faces, but every whole cell it leaves has a neighbour that holds the trim's vertices on their shared edge, and closing those T-junctions is a cost this does not pay: the strips stay and the cuts across them are the chord's.

## 3. The rule

`face_grid` adds the trim points' `u` to the knots of `u` and, for a face cut by its trim, nothing to the knots of `v`; a face meshed on its grid keeps every trim point on both axes, as before. `trimmed_cells` no longer asks a trim point's `v` to be a knot (a trim point off the `u` knots is refused as before). Every point where the trim changes direction is a corner of the cells, as before, since its `u` is a knot. What is lost is a trim point strictly inside a run along one `u` (a vertical side with samples between its corners): it is a vertex only where a chord step cuts at its `v`. The replay finds 4 such points among the 62 of the biggest face and 0 to 2 on the others. A plane face that shares an edge with a curved face takes its vertices along that edge from the curved face's grid (`refined_plane_edge`, from `face_grid`'s `v`), so it follows the change without one.

## 4. Against Revit's export

Measure scores the changed bodies against Revit's export itself for the office, the housing and the reference models (`mesh_bodies_vs_ifc.py`). Over the 24 results that hold a table, one class row differs between the two builds, `IfcFlowTerminal` of RE1 Architecture; the largest change of any class's median volume ratio is 0.0000, and no class's count of bodies with the volume within 1% of Revit's, or with every vertex of Revit's within 0.001 ft of rvt-rs's surface, falls:

| what | before | now |
|---|---|---|
| office, `IfcSanitaryTerminal` (the toilets; 90 of 90 within 1%) | median volume 1.0018 | 1.0018 |
| housing toilets `WC-istuin:WC-1` | volume 1.0060, Revit's vertices 0.0050 ft from the mesh, rvt-rs's own up to 0.0214 ft from Revit's surface | 1.0060, 0.0050 ft, up to 0.0214 ft |
| RE1 Architecture `IfcFlowTerminal` (7 bodies) | volume within 1%: 5; every vertex of rvt-rs's within 0.01 ft of Revit's surface: 5, within 0.001 ft: 2; every vertex of Revit's within 0.001 ft of rvt-rs's: 3 | 5; 4, 2; 3 |

The one figure that crosses a line is the 0.01 ft column of that class: the shower head of RE1 Architecture, whose farthest vertex of rvt-rs's from Revit's surface is 0.0091 ft before and 0.0100 ft now (0.01004), with its volume ratio 1.1047 to 1.1040 and Revit's vertices 0.00103 ft from the mesh both times. It is the figure of rvt-rs's vertices from Revit's surface, which is the chord's own scale (Revit's surface is itself a tessellation up to 0.005 ft from the true one), and the registered columns (the median volume ratio, the volume within 1%, every vertex of Revit's within 0.001 ft of rvt-rs's) are not moved by it.

## 5. The whole run, and what the change costs

Measure 43078686248, unfiltered with `keep_ifc` and every model set, `aafa5b1a` against `main` `5e3ac5b0`: 126 exports, **101 identical and 25 moved** (24 with a kept IFC, one held-out one with counts only). The same-project gates pass. `tools/re/ifc_body_diff.py` on the 24: no element drawn from a saved mesh stops being so (BodySource changes: 0), no object is only on one side, and the meshes of 766 of the 82,295 objects drawn from a saved mesh on both sides (0.9%, an element of each save counted each time) differ: 470 `IfcFurniture`, 252 `IfcSanitaryTerminal`, 36 `IfcCovering`, 5 `IfcRailing`, 2 `IfcBeam` and 1 `IfcSpace`. The diagnostics of all 25 are the same on both sides: `body_sources` and `saved_mesh_refusals` do not move.

| exports | triangles from saved graphics, `main` to the change | IFC |
|---|---|---|
| BIM4LCA office (4) | 1,432,889 to 1,316,316 (-8.1%) as built, -116,573 on each of the four (timber -8.3%); 43 toilets of 2,711 fewer: a toilet 14,025 to 11,314 | 117.1 to 113.3 MB (-3.3%); timber 107.2 to 103.4 MB (-3.6%) |
| BIM4LCA housing, the four large | 372,251 to 351,800 (-5.5%) | 23.4 to 22.7 MB (-2.9%) |
| BIM4LCA housing, the two terrain | 291,458 to 264,662 (-9.2%) | 13.9 to 13.0 MB (-6.6%) |
| Snowdon Towers Architectural | 6,252,307 to 5,710,500 (-8.7%) | 272.1 to 253.8 MB (-6.7%) |
| Snowdon Towers Plumbing, Structural | 2,470,237 to 2,434,019 (-1.5%) and 257,598 to 250,982 (-2.6%) | 118.5 to 117.2 MB (-1.1%) and 14.6 to 14.4 MB (-1.5%) |
| `rac_advanced` 2023 to 2027 | 1,457,506 to 1,237,985 (-15.1%) on the 2023 save, -219,521 on each | 81.1 to 73.8 MB (-9.1%) on the 2023 save, -7.4 MB on each |
| `rac_basic` 2023 to 2027 | 713,280 to 709,098 (-0.6%) on the 2023 save, -4,182 on each | 29.4 to 29.2 MB (-0.5%) |
| RE1 Architecture | 23,466 to 22,510 (-4.1%) | 1.4 MB (-2.2%) |

Export time and peak memory (`timing` 43082087751, three rounds of both builds on one runner, 74 files) read 0.95 to 1.03 and 0.96 to 1.04 of the base's; Snowdon Architectural reads 1.00 in time (31,250 to 31,321 ms) and 0.98 in peak memory (1,427 to 1,402 MiB). The one file above 1.03 is `racbasicsampleproject` 2027 (peak 1.04), inside the 4% that RE-435 measured as the noise of peak memory on a file whose export it moves by 0.6%.

## 6. The registered rows

Registered on #1737 before the run ([comment 6803888670](https://github.com/DrunkOnJava/rvt-rs/issues/1737#issuecomment-6803888670)):

1. **Held, on the results that can show it.** No export grows: of the 24 moved exports with a kept result none has more triangles than the base's, and the one held-out export that moves holds counts only.
2. **Held.** Every element drawn at the base is still drawn from its saved mesh (BodySource changes: 0 of 24 exports) and no refusal is new (the diagnostics of the 25 are identical); the meshes that change are 766 objects of six classes, an element with a face cut by its trim or a plane face that shares an edge with one (the classes of section 5).
3. **Held on the registered columns.** No class's median volume ratio moves by more than 0.002 (the largest change is 0.0000) and the counts within 1% of Revit's volume and with every vertex of Revit's within 0.001 ft fall on no class; the one figure that crosses a line is the shower head's 0.01 ft column (section 4).
4. **Held.** The office's triangles from saved graphics fall by 8.1% (7% to 11% predicted; 43 toilets of 2,711 fewer) and Snowdon Towers Architectural's by 8.7%; the sizes are in section 5.
5. **Held on the bound, missed on one letter.** Export time and peak memory read no more than 1.04 of the base's on any file (1.05 asked) and Snowdon Architectural's peak memory reads 0.98; its export time reads 1.00, not below it: meshing is a small part of that file's 31 seconds.

## 7. What the change leaves

The chord stays 0.001 ft, and a toilet is still 11,314 triangles where Revit's has 1,668. What is left is the chord itself: Revit's own export holds its vertices up to 0.005 ft from the surface where the grid holds 0.001 ft, and a chord of 0.005 ft would take a toilet to about 2,300 triangles (the grid's triangles scale with the chord's inverse), at the price of the vertex agreement the 0.001 ft was chosen for (#1737, the third lever, a decision for the maintainer's owner and not made here). The faces cut by their trim keep every trim point's `u` as a strip boundary, so a face of a long trim is still a few hundred triangles; a sparser design is in section 2.

## 8. Reproduce

```bash
# the change's run (section 5): the scorers come from main, the code from the archived head
gh workflow run measure.yml --ref main -f ref=aafa5b1a2f159bccc4f9556153711dd1a930c8b0 -f base=5e3ac5b0fefb359c265c80b35029f0a3668884a2 \
  -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
gh workflow run measure.yml --ref main -f ref=aafa5b1a2f159bccc4f9556153711dd1a930c8b0 -f base=5e3ac5b0fefb359c265c80b35029f0a3668884a2 \
  -f samples=true -f nordic=true -f timing=true -f scorers=none
python3 tools/re/ifc_body_diff.py <main model.ifc> <change model.ifc>
```

`aafa5b1a2f15` is archived as `refs/archive/2026-11-11/cited/aafa5b1a2f15` (the branch's head when it was measured; the commits after it change the report, the changelog and the reconnaissance addendum, and no source).
