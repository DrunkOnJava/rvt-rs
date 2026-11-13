# RE-460: a HermiteSurf is a grid of nodes whose bicubic patches meet Revit's edge points to 0.001 ft, and to the last digit on three samples in five

**Date:** 2026-11-13
**Status:** positive on the layout (rows 1 and 2c); negative on exactness (rows 2a, 2b, 2e, 2g, 2l, 2n, 2o, 2p and 2q fail as registered); positive on the bound that decides the tessellator (every sample within 0.001 ft); the cause of the remainder is not explained.
**Probe:** `probes/examples/probe_re460_hermite_surf.rs`
**Follows:** RE-329 (512 `unsupported surface HermiteSurf` faces on the BIM4LCA office as built), RE-434 (19 refused elements on Snowdon Towers Architectural), #1191 (W6), and mahach666's layout in Discussion #1241 (RvtIO, MIT).
**Files:** the ten models of Measure whose saved graphics hold a `HermiteSurf` (BIM4LCA housing, concrete and timber, and office, as built; the r2025 `17_Rooms_Stairs_Spiral`; Autodesk's `rac_basic` 2023 to 2027 and Snowdon Towers Architectural 2024), with their sha256 in `research/bim4lca-measure.tsv`, `research/r2025-models.tsv` and `research/autodesk-sample-projects.tsv`; the 33 models of Revit 2023 to 2027 that the runs read. The reference is the file itself: each sample of an edge holds its (u, v) on both faces it borders, and a `Plane` is exact.

## 1. The question

No reader of ours decoded `HermiteSurf`, so every element with such a face in its saved graphics kept its record box. mahach666 gave its layout in Discussion #1241 and reports edge ends meeting their neighbours at a median 1.8e-14 ft. The rows registered on #1191 asked whether the layout holds on our files (row 1), whether the surface at an edge's stored (u, v) meets the neighbouring face within 1e-6 ft on 99 of every 100 ends (row 2), and whether a tessellator for it draws the elements it blocked to Revit's own bar (row 3).

## 2. What the records hold (row 1: holds)

479 objects in 39 records on 10 models of Revit 2023 to 2027 (run 43491955005, `46a8cfd9`), every one of the one shape: `m_Envelope` (`m_corners`, two corners of two f64), `m_NodeArray` (per node `m_iPoint` of 3 f64, `m_iTangent` of two 3-f64 vectors and `m_iMixedDer` of 3 f64), `m_constructedOK`, `m_orientFlag`, `m_periodic` (two flags) and the parameter arrays `m_uParams` and `m_vParams`. In every object the node count is the product of the two parameter counts, both arrays increase and neither direction is periodic. The grids are 2 to 141 by 2 to 107 parameters (6 to 705 nodes). The stated envelope against the grid's first and last parameters: equal on 193 objects, inside it on 100, outside it on 135 and crossing it on 51. No file before Revit 2023 reaches the native reader here, so none is measured.

## 3. The patch

A cell of the grid is the bicubic Hermite patch of its four nodes: the nodes' points, their tangents scaled by the cell's width in `u` or `v`, and their mixed derivatives scaled by its area, each with the cubic Hermite basis, the node of `(iu, iv)` at `iv * nu + iu`, `m_iTangent[0]` along `u` and `[1]` along `v`. A (u, v) past the grid takes the nearest cell's patch continued. Of the four ways to read the layout, the discussion's is the one that fits the data: on the 1,660 ends of edges between a `HermiteSurf` and a `Plane`, 80.6% are within 1e-6 ft of the plane's point at its stored (u, v), against 47.7% with the tangents the other way round, 23.6% with the nodes the other way round and 22.3% with both (run 43495347372, `d5cdde48`; row 2c).

## 4. How close the patch is, and what it is not (rows 2a to 2q)

Over the 4,737 samples of the Hermite-plane edges that the grid holds (ends and interior), the patch's distance from the plane's point at the stored (u, v) is at most 1e-12 ft on 2,840 (60.0%), 1e-9 on 3,117 (65.8%), 1e-6 on 3,697 (78.0%), 1e-5 on 4,335 (91.5%), 1e-4 on 4,631 (97.8%) and 1e-3 on all 4,737 (runs 43506677030, `f7ea5e19`, and 43509898172, `104995b9`). 202 further samples lie past the grid, inside the envelope: with the nearest cell's patch continued all 202 are within 1e-9 and 200 within 1e-12 (row 2i holds). On edges between two `HermiteSurf`s, 76.0% of 866 ends are within 1e-6 and none is over 1e-3.

Each way I registered to explain the remainder failed, and each failure is a result:

| Row | Question | Result |
|---|---|---|
| 2a, 2b | at least 99 in 100 ends within 1e-6 ft (plane, Hermite) | 80.6% and 76.0%: fail |
| 2e | planes meet less exactly than 99 in 100 (the tail is the file's tolerance) | planes meet to 1e-6 on 99.994% of 1,452,130 ends and to 1e-12 on 1,451,754: fail, so it is not tolerance |
| 2g, 2k | a scale of the tangents or twist removes the miss | fitted scales 1.0001, 0.99999 and 0.985; the root-mean-square left is 3.12e-5 ft, 3.125e-5 with every scale 1: fail |
| 2j | the misses lie in cells with twist | every cell holds a twist: not readable |
| 2l | at least 90 in 100 misses are mostly off the neighbouring plane | 34.7% are, 63.0% are mostly in the plane: fail |
| 2n | the stored (u, v) are `f32` values | 6.6% of the hits and 2.6% of the misses: fail |
| 2o | each miss is within the `f32` rounding distance of its parameters | none of 280: the smallest ratio is 4, the median about 3,500: fail |
| 2p | the patch's closest point to the plane's point is within 1e-9 ft for 99 in 100 misses | 88 of 1,040 (8.5%), 321 within 1e-6, 916 within 1e-5, 996 within 1e-4, all within 1e-3: fail |
| 2q | the parameters move to that point by at most 1e-3 of the cell | 86.2% in `u` and 91.4% in `v`; the largest shifts are 0.056 and 0.044: fail |

So the patch is Revit's surface to the last digit on three samples in five and on every sample past the grid, and is within 1e-6 to 1e-3 ft of it on the rest (63.0% of those misses lie mostly along the neighbour's plane, 34.7% mostly off it). RvtIO's residual of 5e-6 to 1e-5 ft in the interior of edges is that band. **Why is not explained.** What is excluded: the file's tolerance, a mis-scaled tangent or twist, the neighbouring plane's evaluation, `f32` rounding of the parameters and the stored parameters. What is not tested: a patch of another form than the tensor product of two cubics, and an approximation of an underlying surface that Revit's own edges follow.

## 5. What changes

`ParametricSurface::HermiteSurf` reads, evaluates (point and normal, the normal from the analytic derivatives of the basis) and refuses a periodic surface or a node count that is not the grid's. Its mesh grid is cut at every node parameter and at the chord of the patch's curvature, 0.0005 ft, half the 0.001 ft the other surfaces keep, because the patch's own distance from Revit's points is within 0.001 ft on every sample measured and the mesh's chord is added to it. Row 3 (the elements blocked by this class alone are drawn from their saved meshes, each to Revit's own 0.001 ft bar) is decided by the Measure of the pull request, below.

## 6. Reproduce

```text
python3 -I .claude/scripts/measure-dispatch.py --ref 104995b9f7c0f112a18fe48e2eeda7b3eab66d89 --base none -f nordic=true -f r2025=true \
  -f samples=true -f sample_years='2023 2027' -f probe=probe_re460_hermite_surf -f scorers=none \
  -f models='nordic-housing-concrete-as-built nordic-housing-timber-as-built nordic-office-as-built r2025-17_Rooms_Stairs_Spiral sample-2023-racbasicsampleproject sample-2024-racbasicsampleproject sample-2024-snowdon_towers_sample_architectural sample-2025-racbasicsampleproject sample-2026-racbasicsampleproject sample-2027-racbasicsampleproject'
```

The probe's stages and the commit each ran at: layout (`46a8cfd9`, run 43491955005), edge ends in four readings (`d5cdde48`, 43495347372), the plane control and the causes (`2189ae7b`, 43497671776), the scale fit (`ac226f37`, 43500051444), the continued cell and the classes (`4a2547a5`, 43502940864), the split off and in the plane (`8132042f`, 43504673637), the `f32` test (`f7ea5e19`, 43506677030) and the closest point (`104995b9`, 43509898172).

## 7. Row 3

To be filled from the unfiltered Measure of the pull request (rows 3a and 3b on #1191).
