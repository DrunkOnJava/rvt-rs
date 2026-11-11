# RE-422: four of Snowdon Towers Architectural's six block roofs are drawn from their saved meshes, and a twisted face between two straight profiles needs a quarter of the cells the code asked for

**Date:** 2026-11-11
**Issues:** #1683 (the six decoders; this report covers the four ruled-surface roofs), #1706 (the two roofs that stay blocks); follows RE-420 (why the roofs stay blocks), RE-389 (#1612) and RE-407 (multi-loop cylinder faces)
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural (Revit 2024, as Measure fetches the sample) and Revit's IFC4 export of it (local only: percentages, distances and triangle counts only here, no Revit-side volumes); Autodesk's Snowdon Towers Sample Structural (no Revit IFC4 export exists; the VIM export of the model, `vimaec/vim-hackathon`, the 2027 edition of the sample, local only, scores its footings); rvt-rs's exports of both by `main` and by each step of the fix, which Measure kept (`keep_ifc`)
**Probe and scorer:** `probes/examples/probe_re422_ruled_boundary.rs`, `probes/examples/probe_1683_ruled_grids.rs`, `tools/re/roof_bodies_vs_ifc.py`, `tools/re/footing_meshes_vs_vim.py`
**Runs:** Measure [42950491724](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42950491724) (the refusal names the fault), [42954101363](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42954101363) (the boundary probe), [42955514580](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42955514580) (the guard narrowed), [42966023343](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42966023343) (a grid for each face), [42968296323](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42968296323) (`u` and `v` apart), [42974359390](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42974359390) (the grid probe), [42977161897](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42977161897) (the quarter bound), [42980660819](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980660819) (unfiltered, the quarter bound merged with `main`), [42984591545](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42984591545), [42984594727](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42984594727) and [42984598295](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42984598295) (the held-out house at three steps), [42984875710](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42984875710) (unfiltered, the rule as it lands) and [42988906515](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42988906515) (`keep_ifc` of the rule as it lands); the baselines are [42961835081](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42961835081) (`keep_ifc` of `main`, the branch's parent, for the steps before the merge) and [42993080184](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42993080184) (`keep_ifc` of `main` 16411282, the rule as it lands)
**Status:** Positive for four roofs: each is drawn from its saved mesh, within 0.11% of Revit's volume, with every vertex of Revit's body within 0.000358 ft of rvt-rs's surface, in 198,762 triangles where the first version needed 1,510,643. Negative for two: roofs 1916596 and 964559 stay blocks, each with its one decoder diagnostic (section 7, #1706). Of the 39 rows registered on #1683 for the steps, 21 held, 4 held in part, 10 missed and 4 could not be read as written (section 9).

## 1. The question

RE-420 found that six of Snowdon Towers Architectural's roofs stay the level block of their sketch outline, 33% to 133% above Revit's volume, because each has one decoder diagnostic and its saved mesh holds no primitive. Four of them had `malformed ruled-surface edge is connected to face #`: `ruled_surface_component` refused a whole component of ruled faces when one edge of it did not resolve to two `Face` objects through `m_pFace`, though the edge was on the roof's boundary, a token-only reference, or a pointer into another symbol. What does the saved graph hold at that edge, and can the roofs be drawn from it?

## 2. The guard, narrowed

`probe_re422_ruled_boundary` (Measure 42954101363) prints the shapes of every `Edge`'s `m_pFace`. Each of the four roofs holds ruled faces (23, 3, 7 and 2) in a component with plane faces, and one cylinder on two of them. The edges whose `m_pFace` is a `Face` and nothing else number 59, 9, 19 and 8; in the 12, 9, 12 and 8 each list prints, every one borders a plane or a cylinder and none a ruled face. The ruled faces themselves are closed.

The component exists to size one grid for its ruled faces, so that the straight generator edges they share meet. A ruled face's neighbours are the faces its own edges name, and an open edge of a plane or a cylinder cannot join two ruled faces. So the component is refused now only where a ruled face of it borders such an edge (`a97c4ae67a9c`), and the four roofs are drawn from their saved meshes.

Against Revit's body their volumes read -0.0015%, +0.0032%, -0.0004% and -0.098% (1926040, 1975810, 1985275, 2112673; the blocks were 33% to 133% over). The cost was size: the four roofs came to 1,510,643 triangles where Revit's own bodies are 1,259, 138, 240 and 192, Architectural's IFC grew from 189.1 to 245.7 MB, Snowdon Structural's from 13.5 to 23.6 MB (its 43 footings, the only other elements to take a mesh, gained 312,788 triangles), and IfcOpenShell needed over 11 GB to mesh the roofs.

## 3. What set the cells

The first rule gave a component one `n` by `n` grid, the largest over its faces of the twist's cells and one step a foot along a straight profile. Giving each face its own grid (`b4e2df588b44`) and then `u` and `v` apart (`bb4e21dc56a8`) took the four roofs to 776,757 and 761,635 triangles, 100,000 or fewer asked, and the footings' 312,788 added triangles to none. The reading that followed, that the trim's points become grid lines across a face, was wrong.

`probe_1683_ruled_grids` prints, for every ruled face a saved graph holds, the formula's counts, the final knots, the distinct `u` and `v` the trim adds, the twist and whether the face is flat. On this model it finds 400 ruled faces holding 505,762 cells (483,441 by the formula alone); the faces whose final cells are 4 times the formula's or more hold 1.3% of them; the faces that are not flat hold 98.3%; the largest 5 faces hold 22.2%, the largest 25 hold 70.3%. The fifteen largest faces are all between two straight profiles, with 11.6 to 24.0 ft of twist, and a trim that is their own four corners (record 1926040 face 447: knots 155 by 156 against the formula's 155 by 155).

## 4. The rule

A surface between two straight lines is a bilinear patch whose second difference of corners is the twist `W`. Cut into `n_u` by `n_v` cells of two triangles, the patch leaves them by `|W| / (4 n_u n_v)` at most, at the middle of a cell. The rule asked `|W| / 0.001 ft` cells (`main`'s the same: `n = sqrt(mixed / CHORD)` for the component), four times what holds the 0.001 ft bar. `ecd6d328e3fd` asks `|W| / (4 * 0.001 ft)` for a face of two straight profiles; a face with an arc or an ellipse keeps `|W| / 0.001 ft`, its own bow (up to the bar) adding to the twist's. Along the profiles (`u`) a face takes the larger of its arcs' steps and the square root of its cells, and between them (`v`) the rest, at least one; a face whose four corners lie in one plane is flat and asks none.

The rule applies only to a component `main`'s guard would have refused, which are the four roofs and the 43 footings (`1cd344d58c79`). A component `main` drew keeps its shared grid and the refusal text is `main`'s. The unfiltered run of the quarter bound alone found why that is needed: the held-out house's 15 windows, drawn from saved meshes on both sides, fell from 9 to 2 on the count whose two bodies lie within 0.001 ft of each other in both directions (every vertex of Revit's body stayed within 0.001 ft of rvt-rs's surface, 15 of 15). Three single-model runs put the fall at the `u`/`v` split of `bb4e21dc56a8`, not at the guard or the per-face grid, because a face of arcs between straight generators takes its arcs' steps and one cell across where `main`'s grid gave it as many across as along. The scorer's count is tessellation-sensitive and the class has an exact count, so the grid of a face `main` already draws stays `main`'s.

## 5. Against Revit

The roof scorer of `main` 16411282 on a cut of the four roofs from rvt-rs's export and from Revit's IFC4, the IfcSite placement undone, on the export of the quarter bound (the rule as it lands draws these four roofs with the same grids, section 6):

| roof | triangles, rule of section 2 / `u` and `v` apart / quarter bound | volume over Revit's | farthest vertex of Revit's body from rvt-rs's surface |
|---|---|---:|---:|
| 1926040 | 1,139,441 / 584,311 / 152,060 | -0.0019% | 0.000222 ft |
| 1975810 | 91,566 / 50,286 / 13,238 | +0.0029% | 0.0000267 ft |
| 1985275 | 271,678 / 126,368 / 32,794 | -0.0007% | 0.00000843 ft |
| 2112673 | 8,958 / 670 / 670 | -0.103% | 0.000358 ft |
| all four | 1,510,643 / 761,635 / 198,762 | | 4 of 4 within 0.001 ft |

Revit's own bodies of the four are 1,259, 138, 240 and 192 triangles. The scorer peaked at 9.1 GB under a 10 GB limit on this cut.

## 6. What else moves

The unfiltered Measure of the rule as it lands ([42984875710](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42984875710): `1cd344d58c79` merged with `main` 16411282, every flag and scorer, 126 models each side): 124 of 126 exports are byte-identical to `main`'s, among them the held-out house, RE1 Architecture, `rac_basic` 2023 to 2027 and Snowdon Electrical, HVAC and Plumbing, which the quarter bound alone moved. The two that move are Snowdon Towers Architectural and Structural. The scorer diff holds one `placement composition` line on each (2,409 to 2,405 swept solids on Architectural, 1,214 to 1,171 on Structural: the four roofs and the 43 footings have a mesh in place of a box), and `same-project gates` pass. The held-out house's `IfcWindow` count on the 0.001 ft column reads 9 of 15 again.

The kept exports, read element by element (Tag, class, `BodySource`, `SavedMeshRefusal` and a hash of each triangulated face set's points and indices; Measure 42988906515 against 42993080184, `main` 16411282):

- **Architectural**: 6,049 products on both sides; exactly 4 change, the four `IfcRoof`, record box to saved mesh; the 3,837 elements drawn from a saved mesh on both sides have identical meshes; no other element's geometry or refusal differs. The file goes from 189.6 MB and 4,218,653 triangles to 196.6 MB and 4,417,415: the 198,762 added are the four roofs' (the blocks they replace are extrusions).
- **Structural**: 1,289 products on both sides; exactly 43 change, all `IfcFooting`, record box to saved mesh, each from `partial: malformed ruled-surface edge is connected to face ##`; the 83 elements drawn from a saved mesh on both sides are identical; nothing else differs. The file goes from 13.5 MB and 230,626 triangles to 14.2 MB and 248,294. (The quarter bound alone left it at 58,286 triangles, because it also re-meshed those 83 bodies coarser; the rule as it lands leaves them as `main` draws them.)

**The 43 footings, scored.** Structural has no Revit IFC4 export, but the VIM export of the model (`vimaec/vim-hackathon`, the 2027 edition of the sample, local only) holds Revit's own meshes for the same ElementIds, and `tools/re/footing_meshes_vs_vim.py` reads them. All 43 are `SFD_Round` shafts (14 of 24 by 20 ft, 28 of 24 by 25 ft, 1 of 30 by 25 ft). The height of each equals the VIM's within 0.00001 m (43 of 43), as does its lowest point (43 of 43), and its volume reads at most 0.55% over the VIM's: the VIM draws each shaft as a 32-sided prism (128 triangles), which holds 0.64% less than its circle, against rvt-rs's finer 396 to 460. The 47 footings `main` already drew from a mesh read the same way: height and lowest point equal on 47 of 47, volume within 0.44%. The record box these 43 had is 9, 18 or 23 inches taller than the shaft Revit draws (the 21 `SFD_Round` shafts `main` already draws from a mesh equal their box), so the mesh is the closer body, and the box a known overstatement.

The done-when of #1683 asked for a rule that moves no other element's export. It holds for 124 of 126 exports; in the two that move, every element is unchanged except the four roofs (the issue's subject) and the 43 footings, each scored against Revit's own geometry, the roofs against its IFC4 export and the footings against the VIM.

## 7. The two roofs that stay blocks

- **1916596** (74% over Revit's volume): `shared curved edge changes its fixed parameter`, raised by `refined_plane_edge` in `src/research/native_saved_mesh.rs`. A plane face that shares an edge with a curved face (a cylinder, a surface of revolution, a ruled or a cone surface) refines that edge in the curved face's parameter domain, and requires the edge to be an isoparametric interval there: one of `u` and `v` constant along it. The edge of this roof is not, as the edge of a slanted plane across a cylinder is not. The same diagnostic is on 25 elements of this model (the railings' KAI-5 and KAI-6 among them) and on `rac_basic` and the `rme` samples.
- **964559** (42% over): `multiple cylinder trim loops unsupported`, RE-407's negative: a cylinder face with more than one trim loop.

Neither is touched here. #1706 holds them, and the done-when of #1683 is met by naming them.

## 8. Reproduce

The measurement runs are the ones in the header; each names the code it measured. The commits are kept under `refs/archive/2026-11-11/cited/` (`c0e125691fe2`, `a97c4ae67a9c`, `b4e2df588b44`, `bb4e21dc56a8`, `954eb50229e8`, `ecd6d328e3fd`, `1cd344d58c79`, `7e8053238ced`); the merged rule is the squash commit that carries this report.

```bash
# the roofs' diagnostics and the open edges (probes read the 2024 samples)
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2024 2024" -f scorers=none \
  -f probe="probe_re389_roof_blocks probe_re422_ruled_boundary probe_1683_ruled_grids" -f probe_scorer="- - -"
# the exports the element read compares (kept)
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2024 2024" -f scorers=none -f keep_ifc=true
# the roof scorer, on a cut of the four roofs from rvt-rs's export and from Revit's local IFC4
python3 tools/re/roof_bodies_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
```

The unfiltered check before merging is `-f base=<main sha> -f nordic=true -f r2025=true -f samples=true -f holdout=true -f bimcollab=true -f racadv=true`, every scorer: 124 of 126 exports byte-identical to `main`'s, the two that move named in section 6.

## 9. Rows registered on #1683

Thirty-nine rows, each registered in the comment before its run; the comment that reads it follows.

| step | rows | held | in part | missed | not read as written |
|---|---:|---:|---:|---:|---:|
| the refusal names the fault ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6785765992), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6786057128)) | 4 | 4 | | | |
| what the graph holds at the open edge ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6786297933), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6786488160)): the open edges number 59, 9, 19 and 8, not 8 or fewer on each | 4 | 1 | | 1 | 2 (one empty as written, one not measured) |
| the guard narrowed ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6786488160), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6787364744)): the volumes held, the vertex test could not run, Structural's 43 footings moved | 4 | 2 | 1 | 1 | |
| a grid for each face ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6787364744), [row 5 added](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6787918966), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6788226869)): 776,757 triangles, not 100,000 | 5 | 1 | 1 | 2 | 1 |
| `u` and `v` apart ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6788226869), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6789040200)): 761,635 triangles; ten exports move, not the models V1 moved | 5 | 2 | | 2 | 1 (no unfiltered run yet) |
| the grid probe ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6789040200), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6789438295)): the trim lattice is 1.3% of the cells; the twist sets 98.3% | 4 | 1 | | 3 | |
| the quarter bound ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6789438295), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6790065111), [unfiltered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6790618165)) | 6 | 4 | 1 | 1 (the held-out house's windows lose an exact count) | |
| the diagnosis, three single-model runs ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6790618165), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6790707721)) | 3 | 3 | | | |
| the rule as it lands ([registered](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6790662744), [read](https://github.com/DrunkOnJava/rvt-rs/issues/1683#issuecomment-6791253349), element read in section 6): the roofs read 198,762 triangles; the registered 58,286 for Structural's file was misspecified (it was V3 re-meshing 83 bodies), and the footings' meshes equal V3's | 4 | 3 | 1 | | |

What the misses taught, in order: the guard was right for the faces it meant and wrong about the planes beside them (the narrowing); one grid for a component is as fine as its finest face (V1); the formula's counts, not the trim, set the cells (V2 and the probe); the formula's bound was four times the bilinear patch's (V3); and any rule on a face `main` already draws moves an exact-count class, so the rule is scoped to the components the narrowed guard admits (V4).
