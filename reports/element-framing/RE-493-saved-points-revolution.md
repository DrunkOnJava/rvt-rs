# RE-493: A surface of revolution or a cone meets a plane along the edge's saved points

**Date:** 2026-11-19
**Issues:** #2098 (the cracks), #2099 (this change); follows RE-492 §2, which gave the rule to ruled faces with a spline profile (#2094).
**Artefacts:** the BIM4LCA office (as built, at permit, and its timber version) and housing files (VTT, CC BY-SA 4.0; Revit 2024), each with Revit's own IFC4 export; Autodesk's sample projects of Revit 2023 to 2027 and Snowdon Towers' 2024 models, counted only, since they have no Revit export.
**Probes:** `probe_2098_face_cracks` (`ee09811f`, the change with the probe, and `6b6df1ad`, `main` at `69ac3a26` with the probe, which differ in `src/` by the rule alone), the scorer `tools/re/far_vertices_vs_ifc.py` and the tool `tools/re/body_displacement.py` (`ad80037d` and `dab3b0f7`). None is landed; the three commits are kept as `refs/archive/2026-11-19/measure-2098-cand-probe`, `refs/archive/2026-11-19/measure-2098-main-base` and `refs/archive/2026-11-19/measure-2094-far-vertices-dab3b0f7`, the last holding `ad80037d`.
**Status:** positive on the crack it targets, and neutral against Revit. Every gap more than 0.001 ft between a plane and a surface of revolution closes; the 2,163 boundary vertices left over 0.001 ft from every other face lie between two curved faces. No cone gap was over 0.001 ft on either side: what the rule does on cones is Plumbing's vertices over 0.0001 ft going from 130 to 0, on a sample with no Revit export. No count scored against a Revit export moves, and every body that moves and has a Revit export is one toilet family, `WC-istuin:WC-1`, on the BIM4LCA office and housing: the moved tags of a housing file carry that name, and each office file's 43 moved are its 43 of that family. Their point lists differ between instances, and the largest move is the same on every file. The rows were registered in the pull request before each run was read.

## 1. The rule

Where a plane shares an edge with a face whose samples cross both of the face's parameters, the plane had been drawn through the curve it cuts from the surface (`refined_plane_edge`, #1706), and the curved face straight in its own parameters between the edge's saved points; the two parted by the edge's bow. RE-492 §2 put both faces on the saved points' chords for ruled faces with a spline profile. `keeps_saved_chords` now also takes surfaces of revolution and cones (`403070ad`). A ruled face between plain curves stays as it was: the one such body measured (2181762 on Snowdon Towers Architectural, 22 vertices over 0.0001 ft and none over 0.001 ft) meets no better for the rule.

## 2. Measured

**Run H** ([44579061107](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44579061107), `probe_2098_face_cracks` on both sides; nordic, r2025, samples). The base side repeats the earlier search-mesher reading: 1,094 bodies, 662 meshed on both sides, 3,962 boundary vertices over 0.001 ft from every other face, the largest 0.006017 ft, 5,894,559 triangles.

| | base | this change |
|---|---:|---:|
| boundary vertices over 0.001 ft (meshed on both) | 3,962 | 2,163 |
| the largest | 0.006017 ft | 0.002354 ft |
| triangles | 5,894,559 | 5,857,140 |
| bodies that gain a vertex over 0.001 ft | | 0 |

- By file: 987 left on Snowdon Towers Architectural, 258 on each BIM4LCA office file, 36 on each housing file.
- In the probe's tally, summed over the nine files with tally lines, the 2,163 left are 1,260 between two surfaces of revolution, 588 between two cylinders and 315 between a cylinder and a surface of revolution; none lies at a plane. The 1,799 of the base side's 3,962 that lay at a plane are the ones that close.
- No body's gaps over 0.001 ft all close (0 of the 217 that had any): what is left of each is between two curved faces.
- Plumbing's vertices over 0.0001 ft go to 0 (cones), Structural's stay 124.
- Two bodies are newly meshed, Snowdon Towers' 2110101 and 2110417 (`IfcFurniture`, surfaces of revolution only), which the base refused for `surface coordinates outside envelope`.

**Run I** ([44588392591](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44588392591), `ac956181` against `69ac3a26`; nordic, r2025, samples, holdout, exports kept).

- 104 of 120 exports are byte-identical. The 16 that move are those Run H lists a body on: the eight BIM4LCA files, `rac_advanced` 2023 to 2027, and Snowdon Towers Architectural, Plumbing and Structural. No held-out export moves.
- No count of `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc`, `roof_bodies_vs_ifc` or `stair_bodies_vs_ifc` moves, and no `psets_vs_ifc` or `qtos_vs_ifc` line; only `mesh_bodies_vs_ifc`'s list of its farthest bodies changes.
- The only refusal count that moves is Snowdon Towers Architectural's `surface coordinates outside envelope`, 2 to 0.
- The same-project gates pass, with 15 samples held to their earlier saves.

**Run I2** ([44588403341](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44588403341), `far_vertices_vs_ifc` on both probe branches, the eight BIM4LCA files). The sanitary terminals whose body changes are 6 on each housing file and 43 on each office file, 196 in all. On every one, the count of Revit's vertices beyond 0.001 ft of rvt-rs's surface, the counts of rvt-rs's beyond 0.01 and 0.001 ft of Revit's, and the farthest distance both ways are unchanged. Each body has 96 fewer vertices, and within the breakdown of its vertices beyond 0.001 ft some move between `crease` and `smooth` under one facing of Revit's; each body's counts beyond each bar are unchanged, in total and by facing.

`mesh_bodies_vs_ifc` disagrees on the housing toilets: it gives their "rvt-rs's to Revit's" a rise of up to 0.00095 ft, and on the base side alone gives instances of one toilet 0.02049 and 0.02109 ft where `far_vertices_vs_ifc` gives all of them 0.021585 ft. The change is judged on `far_vertices_vs_ifc`, and the disagreement between the two scorers is not resolved here.

**How far the bodies move** (`ifc_body_diff.py` on Run I's kept exports, and `body_displacement.py`, both ways):

| file | bodies that move | the farthest a vertex moves |
|---|---|---|
| BIM4LCA housing, all four files | 6 each, `IfcSanitaryTerminal` | 0.00112 ft |
| BIM4LCA office, all four files | 43 each, `IfcSanitaryTerminal` | 0.00112 ft |
| `rac_advanced` 2024 | 60, `IfcFurniture`, that move alike | 0.0000774 ft |
| Snowdon Towers Architectural | 138 in 26 shapes, and the 2 newly drawn | 0.00152 ft (six toilet shapes, 21 bodies); 0.00026 ft for the rest |

Every moved body is measured on three BIM4LCA files (the concrete housing as built and at permit, the office as built, at `ad80037d`), each distinct shape on four (the timber housing at permit, the office at permit, the timber office as built and at permit, at `dab3b0f7`), and 2 of the 6 on the timber housing as built, before that run was stopped. None changes `BodySource` but the two newly drawn. The toilets move by more than the scorers' 0.001 ft vertex bar. By `far_vertices_vs_ifc`, their counts beyond 0.001 and 0.01 ft of Revit's surface are unchanged, in total and by facing, and so are both farthest distances. Their largest gap at a plane on the base side was 0.001145 ft on the two files read for it (the concrete housing and the office, as built), so the 0.00112 ft move is that gap closing. On Snowdon Towers Architectural, the six toilet shapes that move 0.00152 ft are the bodies with the run's widest cracks (0.006017 ft before, 0.002354 ft after; boundary vertices over 0.001 ft from 113 to 115 a body to 47), so the move is the crack closing, within its width. The lever moved 139 bodies there; this change moves 138, which is the count that says 2181762 does not move.

## 3. Not claimed

- **Any body the samples hold.** No Revit export of them exists.
- **More than one family against Revit.** Every body that moves and has a Revit export is one toilet family, `WC-istuin:WC-1`; a second family with such an edge, with Revit's export beside it, would test the rule further.
- **Cones against Revit.** The one cone figure is Plumbing's, a sample with no Revit export.
- **The gaps between two curved faces.** 2,163 boundary vertices stay over 0.001 ft from every other face, up to 0.002354 ft; this change touches only an edge a curved face shares with a plane.
- **Which of the two scorers is right about the housing toilets' farthest vertex** (above).
- **A trace where the step falls back.** As in RE-492 §4, where putting a face's vertices on the chords would leave a triangle with no area the face keeps its points, and no diagnostic says so.

## 4. Reproduce

The third command needs the scorer branch `measure/2094-far-vertices`, kept as `refs/archive/2026-11-19/measure-2094-far-vertices-dab3b0f7`; the probes' commits are kept as `refs/archive/2026-11-19/measure-2098-cand-probe` and `refs/archive/2026-11-19/measure-2098-main-base`.

```bash
gh workflow run measure.yml --ref main -f ref=ee09811f1feaa1db555728af1fdee6351d8affec \
  -f base=6b6df1adaa610d34b70904930e334321bd5bb0f9 -f nordic=true -f r2025=true -f samples=true \
  -f probe=probe_2098_face_cracks -f scorers=none
gh workflow run measure.yml --ref main -f ref=ac9561811375d370e3bc5213b3d469bceac44af4 \
  -f base=69ac3a26c1aab540dc9c457d19063d80caecfab4 -f nordic=true -f r2025=true -f samples=true \
  -f holdout=true -f keep_ifc=true
gh workflow run measure.yml --ref measure/2094-far-vertices -f ref=ee09811f1feaa1db555728af1fdee6351d8affec \
  -f base=6b6df1adaa610d34b70904930e334321bd5bb0f9 -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit nordic-housing-concrete-as-built nordic-housing-concrete-permit nordic-housing-timber-as-built nordic-housing-timber-permit" \
  -f scorers=far_vertices_vs_ifc
```
