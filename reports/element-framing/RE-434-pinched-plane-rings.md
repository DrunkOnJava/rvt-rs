# RE-434: a plane face whose outline is one ring that visits a vertex twice is split there and meshed ring by ring, and 21 sanitary terminals of Snowdon Towers Architectural are drawn from their saved meshes

**Date:** 2026-11-11
**Issues:** #1732 (found by RE-430's run, which let these 21 elements reach this face); follows RE-430 (#1715, the trimmed faces) and RE-426 (#1709)
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural (Revit 2024, as Measure fetches the sample) and Revit's IFC4 export of it (local only: ratios, distances and triangle counts only here, no volumes); rvt-rs's exports by `main` and by the change, which Measure kept (`keep_ifc`)
**Probe and scorer:** a diagnostic build of the planar path (a scratch branch, not for `main`, kept at `refs/archive/2026-11-11/scratch/1732-area-detail`); `tools/re/ifc_body_diff.py`, `tools/re/cut_ifc.py` and `tools/re/mesh_bodies_vs_ifc.py`; the pure helper `split_pinched_rings` is tested in `src/research/native_saved_mesh.rs` (five failure-first tests: a ring that visits no vertex twice, two lobes, a ring that starts at the shared vertex, a spike, three lobes at one vertex)
**Runs:** the diagnostics, Measure [43054812102](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43054812102) and [43055872726](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43055872726) (the 2024 samples, head only); the change, [43058503567](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43058503567) (unfiltered, `keep_ifc`, head `4f47b837` against `main` `aa5dc4c6`) and its `timing` [43058508332](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43058508332)
**Status:** Positive. The area check failed on one face of one symbol, whose second loop is two lobes that touch at a point; split there, the face meshes, and all 21 elements that hold it are drawn from their saved meshes. 125 of 126 exports are byte for byte `main`'s, the 126th moves by exactly those 21 elements, and no element drawn at `main` changes. The cost is triangles: 34,260 per element, +9.9% on the model's IFC.

## 1. The question

After RE-430 meshed the revolved and ruled faces of Snowdon Towers Architectural's sanitary terminals, 21 of them were refused for `triangulated area mismatch` on a plane face, where they had been refused for `multiple parametric trim loops unsupported` before: the plane face had been behind the first refusal. The refusal text keeps no digit, so which face it was, and why the area did not add up, were not in the diagnostics.

## 2. What the diagnostic found

A scratch build printed, for each plane face that failed the check, its loops (points, signed area, nesting depth), the holes assigned to the outer ring, the earcut triangles, the triangulated area against the expected one, and how many hole vertices touch the outer ring or another hole or lie outside it (runs 43054812102 and 43055872726). All 22 lines (21 elements and a duplicate) are one face, face 171 of the toilet's symbol, a plane with three loops:

- loops 0 and 2: four points each, two curved pieces of equal area in the bottom corners of a plate 0.250 ft wide and 0.240 ft tall, each about 0.025 ft across;
- loop 1: 22 points, `holes=0`, `touch_outer=0`, `outside_outer=0`, `touch_holes=0`, all three loops at nesting depth 0, so they are three disjoint outer rings, as the face is.

Loop 1 is two thin spandrels under a flat arch: it runs down the left edge (x = 0, from y = 0.240 ft to 0.215 ft), along the arch up to the crown at (0.125, 0.240) and down to the right edge, up that edge, and back along the top line through the crown to the start. Its points 10 and 21 are the same point, `(0.124999590, 0.239948770)`, the crown, where the two lobes touch. Both lobes turn counter-clockwise, so the ring's area is the two lobes' sum. Earcut, given the ring whole, filters the repeated point, covers a sliver under the crown as well, and returns 19 triangles whose areas add up to 0.65% more than the ring's: the check (1e-7 ft² for a face under 1 ft²) refuses the face, and every element that holds it.

## 3. The rule

When the area check fails, the rings are split at every vertex a ring visits twice (`split_pinched_rings`): the ring `r[i..j]` between two visits of one point is a ring, and the rest is another; a ring of two points has no area and is dropped; each ring starts at a vertex that is not the shared one, so that the nesting test (`inside(r[0], other)`) does not read the point where two rings touch. The face is triangulated again from the split rings; if the check then passes, the face is drawn from them, else it is refused as before, with the original text. A face that passes the check takes the path it always took, so no element that is drawn can change: the change only turns refusals into meshes. The triangulation itself moved into `plane_mesh` unchanged.

## 4. Against Revit's export

The 21 elements are all one type, `Toilet-Domestic-3D`; Revit's export holds all of them (as `IfcFlowTerminal`; rvt-rs writes them as `IfcSanitaryTerminal`, as `main` does for the boxes they replace), scored by `tools/re/mesh_bodies_vs_ifc.py` on a cut of the 21 (`tools/re/cut_ifc.py` on both exports), volume rvt-rs's over Revit's:

| class in Revit's export | type | n | volume within 1% | median volume | every vertex of Revit's within 0.001 ft | farthest vertex of Revit's |
|---|---|---:|---:|---:|---:|---:|
| `IfcFlowTerminal` | `Toilet-Domestic-3D` | 21 | 21 | 1.0063 | 0 | 0.0047 ft |

The scorer measures at most 400 vertices of a body, so the farthest figure is a sample's (RE-430 section 4); it is the same 0.0047 ft on all 21, as the toilets of the BIM4LCA office (0.0048 to 0.0050 ft) and the chairs of RE-430. rvt-rs's vertices lie up to 0.0126 ft from Revit's surface, as a mesh of flat triangles lies off the curved surface it is inscribed in.

## 5. The whole run, and what the change costs

Measure 43058503567, unfiltered with `keep_ifc` and every model set, `4f47b837` against `main` `aa5dc4c6`: 126 exports, **125 identical and 1 moved**, Snowdon Towers Architectural; the same-project gates and the semantic check pass. `tools/re/ifc_body_diff.py` on the moved export: 6,350 objects in both files, **21 change BodySource, all `IfcSanitaryTerminal` from `partition_element_record_bbox` to `saved_graphics_mesh`**; 4,282 objects are drawn from a saved mesh in both and 0 of their meshes differ; no object is drawn at `main` and not now; and no other object differs in geometry or in the text of its refusal. In the diagnostics, `body_sources.partition_element_record_bbox` goes from 492 to 471 and `saved_graphics_mesh` from 4,282 to 4,303, and `saved_mesh_refusals` from 269 to 248 entries: `triangulated area mismatch` 21 to 0 and nothing else.

The cost is in triangles, as RE-430 section 8 found for revolved faces: each of the 21 is drawn with 34,260 triangles (Revit's toilet of the BIM4LCA office has 1,668), 719,460 in all, so the model's triangles from saved graphics go from 6,228,638 to 6,948,098 (+11.6%) and its IFC from 269.5 to 296.3 MB (+9.9%). `timing` (three rounds of both builds on one runner, 74 files): Snowdon Architectural reads 1.02 of `main`'s export time and 1.03 of its peak memory, the other files 0.98 to 1.02 and 0.96 to 1.04, the noise of runs on files the change cannot move. The density is #1737.

## 6. The registered rows

Registered on #1732 before the run ([comment 6801073430](https://github.com/DrunkOnJava/rvt-rs/issues/1732#issuecomment-6801073430)):

1. **Held.** The 21 sanitary terminals of Snowdon Towers Architectural are drawn from their saved meshes; none reads another reason.
2. **Held.** No element drawn at `main` changes: of the 4,282 drawn on both sides, 0 meshes differ, and every BodySource that changes is a box becoming a saved mesh.
3. **Held.** Of the new elements that Revit's export holds, 21 of 21 (100%, 80% asked) have the volume within 1% of Revit's, median 1.0063; the class is listed in section 4.
4. **Held.** The other 125 exports are byte for byte `main`'s; Snowdon Architectural's IFC grows 9.9%, its export time reads 1.02 and its peak memory 1.03 of `main`'s.

## 7. What is still refused

The refusals of Snowdon Towers Architectural are otherwise as RE-430 left them (248 entries, the larger ones: 127 `no_saved_mesh` and 26 `whole_drawn_by_its_parts` are not candidates, 22 `face regions need subdivision` and 21 more with `multiple cylinder trim loops unsupported`, 19 `unsupported surface HermiteSurf`, 14 `GHermiteSpline`, 12 `multiple cylinder trim loops unsupported`, 2 `surface coordinates outside envelope`). A ring that visits a vertex twice is split once and exactly; a plane face whose outline touches itself without repeating a vertex, or whose rings cross, still fails the area check and is refused, as before.

## 8. Reproduce

```bash
# the change's run (section 5): the scorers come from main, the code from the archived head
gh workflow run measure.yml --ref main -f ref=4f47b837bb2b7744f2d2416482829eaa686c841d -f base=aa5dc4c6d93e861015c10e0a0d8f9f16c8b57e71 \
  -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
python3 tools/re/ifc_body_diff.py <main model.ifc> <change model.ifc> newly-drawn.txt     # the Snowdon exports, about ten minutes
python3 tools/re/cut_ifc.py <rvt-rs model.ifc> ours.ifc $(cut -f2 newly-drawn.txt)       # and the same on Revit's export
python3 tools/re/mesh_bodies_vs_ifc.py ours.ifc revit.ifc
```

`4f47b837bb2b` is archived as `refs/archive/2026-11-11/cited/4f47b837bb2b` (the branch's head when it was measured; the commits after it change the report, the changelog and the reconnaissance addendum, and no source). The diagnostic build is `refs/archive/2026-11-11/scratch/1732-area-detail`.
