# RE-329 — What keeps a saved mesh from decoding whole, face by face; a plane face split into regions is drawn whole

**Date:** 2026-11-04
**Issues:** #1144, #1247; follows RE-224, RE-302, RE-304, RE-305, RE-311
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, CC BY-SA 4.0, `ARCH.zip` sha256 `025e33fb0c0ce3b7a869f4177a0862918cc987408f4cd34e58a0eabd8d9917c4`, 1,579,459,850 bytes), Core Interior (Revit 2024, MIT, `2024_Core_Interior.rvt` sha256 `c805df44…`), RE1 Architecture, Electrical, Mechanical and Plumbing (Revit 2025, MIT) and puzzbobb's 24 Revit 2025 models (Apache-2.0, `R2025_Models.zip` sha256 `8673d803fb0b9b6aa20d08448eeed16eb34222566bfaf7a2dc761c064dc30d6a`, 25,563,111 bytes), each with Revit's own IFC export, as Measure fetches them
**Runs:** Measure [41786625424](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41786625424) (the census, `probe_re329_saved_graphics_census` at a9bc2575), [41787545512](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41787545512) (RE-305's reader on today's `main`, 8a24df6e, scored), [41788644583](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41788644583) (the 30 fittings face by face, 60720f27), [41789549785](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41789549785) (what a face with regions holds, d6601e5e) and [41790592582](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41790592582) (the reader of this change, fe9be560, scored against `main`); then, for the whole Measure, [41791824472](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41791824472) (9c16f226 against `main`, unfiltered, with the R2025 models, every year of Autodesk's samples and the held-out models), [41792431082](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41792431082) and [41793017480](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41793017480) (exports only, the same set, at 1c6f05e7 and 63a2caa2: the stricter tiling check, then the pure core), [41793911979](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41793911979) (the held-out models' mesh, box and saved-mesh counts, 63a2caa2, with the scrub of #1340) and [41793980360](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41793980360) (the census on Autodesk's 2024 samples at a9bc2575)
**Result:** two parts. (1) A census, face by face, of what keeps a saved mesh from decoding whole on the four corpora: the largest cause that stands alone is one kind of face on 30 office fittings, and what it leaves out of the agreed bar is 72 of Revit's 1,083 vertices, all nearest surfaces of revolution: 26 in RE-302's junction pattern and 46 at 0.0011 to 0.0023 ft on three other faces, whose cause is not shown (negative; the reader stays off `main`). (2) Positive: the census found, as the only cause that keeps a door or window of these corpora as a box, a plane face split into regions; drawn whole, it frees 3 windows in each of six Revit 2025 models, 18 of 18 exact against Revit's export, and, as a first measurement on the held-out models, 88 windows in eight of them, 88 of 88 exact (section 3).

## 1. The census

`probe_re329_saved_graphics_census` takes the elements `saved_meshes::attach` would replace (an exported building element whose body is its record box, not profile-resolved, whose Tag no other entity carries), decides as `attach` does which keep their box (a diagnostic left that is not a nested detail filter or a view component, or no surface at all), and for those meshes every face of every selected `Geometry`, not only the first that fails: the graphics walk stops a `Geometry` at its first failing face (`graphics_impl`), so `GraphicsMeshes::diagnostics` names one failure per carrier and cannot tell a sole blocker from one that co-occurs. Each failing face is read from the graph for its surface class, its profile curve classes and its trim (one loop that runs round its UV box, one that does not, several loops), independent of the order in which the mesher fails. An element's causes are the union over its faces: `surface:<class>`, `profile:<class>`, `nonrect:<class>`, `multiloop:<class>`, `err:<class>:<message>` for the mesher's message when none of those explains it, and `traversal:<message>` and `carrier:<message>` for the graphics walk and for carriers that are not a `Geometry`. It prints counts only, and opens the file with `--large-model`'s limits (`streams_not_read` was 0 on every file).

**Candidates and outcomes** (Measure 41786625424):

| file | candidates | door or window drawn | other drawn | kept as box (partial / no surface) | no saved graphics |
|---|---:|---:|---:|---:|---:|
| office as built | 11,246 | 538 | 10,540 | 124 / 22 | 22 |
| office at permit | 11,363 | 534 | 10,659 | 124 / 23 | 23 |
| Core Interior | 138 | 138 | 0 | 0 | 0 |
| RE1 Architecture | 58 | 5 | 48 | 4 / 0 | 1 |
| RE1 Electrical | 49 | 0 | 47 | 2 / 0 | 0 |
| RE1 Plumbing | 61 | 0 | 60 | 0 | 1 |
| RE1 Mechanical | 68 | 0 | 68 | 0 | 0 |
| 24 Revit 2025 models | 95 | 6 | 6 | 8 / 57 | 0 |

On every file but the Revit 2025 models, no door or window is blocked. In the six Revit 2025 models with rooms (12 to 17) three windows of each are, which makes 18 door or window bodies kept as a box (partial; they are not in the "kept as box" column above, which counts the other classes), and the cause is the same on all 18 (section 3).

**The office as built, by what blocks an element** (the permit file has the same sets, plus one `IfcSpace`; 146 of 11,246 blocked: 30 `IfcFurniture`, 19 `IfcRailing`, 97 `IfcSanitaryTerminal`):

| elements | causes (all of them are needed) | classes and earlier reports |
|---:|---|---|
| 43 | `GEllipse` profile, ruled surface with an arc profile, `SurfRev` with a trim that is not its UV box | the `WC-1` toilets, RE-302 |
| 30 | `multiloop:CylSurf`, nothing else | `Stay_Wall_Mounted_:VAR-1`, RE-305; section 2 |
| 19 | `GHermiteSpline` profile and `HermiteSurf` | sanitary terminals; 512 `HermiteSurf` faces in all, 268 with a trim that is not its UV box |
| 19 | `traversal: unsupported reachable graphics class GBiFlipControl` (15 alone, 4 with plane edge failures or a `HermiteSurf`) | railings, RE-303 |
| 10 | `traversal: scaled instance mode is not qualified` | sanitary terminals |
| 15 | several of: `multiloop:CylSurf`, `nonrect:SurfRev`, `HermiteSurf`, a ruled surface whose profile pointer does not resolve (`missing surface pointer target`), disconnected trims | sanitary terminals |
| 8 | `GEllipse`, `GHermiteSpline` and `GNurbSpline` profiles, `HermiteSurf`, `multiloop:RuledSurf`, `nonrect:RuledSurf`, `nonrect:SurfRev` | sanitary terminals |
| 2 | `carrier: unsupported faceted topology` and a plane edge that is not isoparametric | sanitary terminals |

The 512 `HermiteSurf` faces are the one surface class no reader handles (`unsupported surface HermiteSurf`); no element of the office is blocked by it alone. Failing faces by class (office as built, the faces of the blocked elements): 512 `HermiteSurf`, 507 `RuledSurf`, 477 `SurfRev`, 150 `CylSurf` (105 with several loops, 45 `disconnected UV boundary`) and 1,505 `Plane`; against 4,114 `CylSurf`, 1,209 `SurfRev` and 9,247 `Plane` faces that decode in the same elements. A plane face often fails with its curved neighbour's message, because its edge is refined on the neighbour's grid (`refined_plane_edge`): the plane lines in `census face_fail` are the neighbours' causes, not planes the mesher cannot read.

RE1: Architecture's 4 are the shower head (`nonrect:SurfRev` alone), two furniture elements (a ruled surface with an arc profile, `nonrect:SurfRev` and disconnected trims) and a railing (`GBiFlipControl`); Electrical's 2 are RE-311's panels (`GHermiteSpline`, `GNurbSpline`, `nonrect`, `multiloop:RuledSurf`). Revit 2025 models (the 24): 18 windows (section 3), 54 proxies that `traversal: instance target restrictions are unsupported` (9 in each of the six files with rooms), 10 railing or stair parts with `GBiFlipControl` (2 with a `HermiteSurf`) and one stair flight with a `GCylindricalHelix` profile.

## 2. The largest cause that stands alone: cylinder faces with several loops

30 office fittings (section 1) are blocked by `multiloop:CylSurf` alone, which RE-305 read: a loop inside another is a hole, each outer ring with the rings directly inside it earcut and its triangles cut into strips across U. RE-305 measured them 1.109 to 1.120 of Revit's volume and read the excess as surface rvt-rs draws and Revit does not, cause open.

**Re-measure on today's `main`** (Measure 41787545512, RE-305's commit cherry-picked at 8a24df6e, `--saved-meshes`, `mesh_bodies_vs_ifc` with the volume taken about each body's own centre since RE-311): the office as built scores 1,650 → 1,680 saved-mesh bodies, the 30 new bodies being all the fittings, and the 30 are 0 on every column of the scorer (exact, within 0.01 ft, volume within 1%, "ref on ours"). The volume ratio is 1.1102 to 1.1105 (the 15 listed), Revit's vertices lie 0.004725 to 0.005098 ft from the mesh and the mesh's up to 0.01179 ft from Revit's. So RE-305's figures were not the origin artefact of RE-311; the shells are the same as there (the second 0.0239 against Revit's 0.0185 cubic ft).

**Face by face** (Measure 41788644583, `probe_re329_wall_mounted_faces` at 60720f27 and `toilet_faces_vs_ifc.py`, four fittings, ids 1832498, 2794370, 3073846 and 3121894): each is 86 faces and 8,434 triangles (8,444 for the one rotated) and 4.0527 sq ft against Revit's 2,162 triangles and 3.9609 sq ft (+2.3%); the extents are the same to 0.001 ft (0.338 by 2.713 by 0.984).

- **Revit's vertices on our surface.** Of Revit's 1,083 vertices, 797 are within 0.0001 ft, 214 within 0.001, 32 within 0.002, 39 within 0.005 and one within 0.01. On the cylinders and planes none is more than 0.001 ft off. The 72 that are, 0.0011 to 0.0051 ft, are all nearest a surface of revolution (faces 46: 22, 47: 12, 49: 12, 34: 10, 32: 8, 35: 5, 33: 3). On faces 34 and 35, and 32 and 33, which meet in the fitting's mirror plane (x = 275.193 on the first fitting), they sit about as far from each face as from its boundary (0.0036 to 0.0051 ft), the junction pattern of RE-302 section 6. 26 of the 72 meet that test (within 0.0009 ft of the same distance from the next face, and the nearest edge within 0.0006 ft of the face's own): all are on faces 32 to 35, 0.0019 to 0.0051 ft off. The other 46, on faces 46 (22), 47 (12) and 49 (12), are 0.0011 to 0.0023 ft off and do not meet it; this report shows no cause for them.
- **Surface drawn where Revit has none.** None. 77 of our 8,434 triangles have their centre more than 0.01 ft from Revit's surface (largest 0.0115 ft), on faces 6 (4), 14 to 17 (33) and 32 to 35 (40). Faces 14 to 17 are four half-cylinders of radius 0.036 ft (a circle fitted to our own vertices, residual under 1e-5 ft; lengths 2.33 and 1.86 ft); a distance of 0.0105 to 0.0109 ft from a surface of that radius is the sag of a chord of Revit's of about 90° (`R(1 − cos(θ/2))`), and all 75 of Revit's vertices nearest those four faces lie on them within 0.001 ft. That fits Revit's inscribed polyhedron of a thin rod, as RE-311's elbows are, and it is inferred from distances, not from Revit's triangles.
- **So RE-305's reading was not right.** The excess volume is not extra surface: the area is 2.3% more than Revit's, spread over curved faces with no face away from Revit's surface, and the shell that is 29% over is where the thin rods are. Whether it is all Revit's faceting is not shown.

**The bar.** Griffin chose "ref on ours", Revit's vertices within 0.001 ft of the mesh, for opt-in `--saved-meshes` bodies on 2026-11-03 (#1280): Revit's volume and a 0.01 ft test both ways are not the bar, since Revit's export is an inscribed polyhedron of the true surface (RE-311). Under it the 30 fittings fail on 72 of 1,083 vertices, all nearest surfaces of revolution: 26 at junctions between two of them, the pattern of RE-302's open item (whether our boundary between two curved faces, the straight UV line between Revit's saved samples, which strays up to 0.01 ft from the true edge, or Revit's triangulation is off; only edges beside a plane are traced), and 46 on three other faces at 0.0011 to 0.0023 ft, whose cause this report does not show. Tracing curved-to-curved edges could close the 26; whether it closes the 46, and so lets these fittings meet the existing rule, is not known. This change does not do it, and does not land RE-305's reader: it stays on `research/1247-cylinder-holes` at 0f377661 (cherry-picked onto `main` as 8a24df6e on the scratch branch for the re-measure).

## 3. Face regions

The census finds one cause of a default-scope body kept as a box: `face regions need subdivision`, raised for a face whose `m_faceRegions` is not empty. It is the only cause for 3 `IfcWindow` bodies (`Window-Fixed:24" x 36"`) in each of r2025-12 to r2025-17 (`census face_fail Plane 1loop-rect face regions need subdivision 24` per file, counted once for each failing face of each `Geometry` a window draws: 3 windows by 8 faces; the other door or window of r2025-12 is drawn).

**What a face with regions holds** (Measure 41789549785, `probe_re329_face_regions` at d6601e5e, r2025-12 and r2025-17). Of 751 faces in the graphs of r2025-12 (913 in r2025-17), 16 hold regions, each exactly 2 (`census regions 2 faces 16`). These are unique faces, 8 in each of two owner graphs; the census's 24 per file counts a face once for each window that draws it, which fits two of the three windows drawing one owner's geometry (the 16 and the 24 were not matched window by window). A region is itself a `Face`: a `Plane` with the parent's origin, `m_xVec` and `m_yVec`, one loop, no regions of its own, its own render style and a `GFilling`. The two regions of a parent rectangle (UV envelope 1.9167 by 0.1667 ft) are the strips 1.9167 by 0.14583 and 1.9167 by 0.02083, which tile it exactly (0.27951 + 0.03993 = 0.31944 sq ft). The probe prints four such parents: two are split at v = 0.14583 ft and two at v = 0.10417 ft, each into the two strips either side of the line (the reader's own tiling check passed on all 24 faces the census counts, and the scores in the next table are what they draw).

**The reader.** A plane face with regions is drawn whole, as it was with none, where the parent has one loop and every region is a plane face on the same plane (origin and vectors within 1e-9), listed once, with one loop and no regions of its own, and the regions tile the parent. Tiling is checked on areas, in UV units squared, each within 1e-7 of the parent's area: every region vertex lies inside the parent's outline or within 1e-7 UV units of it; the area a region shares with the parent equals its own (which also refuses an edge that crosses a notch of a non-convex parent, and a region lying in one); no two regions share area; and the regions' areas add up to the parent's. A shared area is measured on triangulations, each outline earcut and each pair of triangles clipped (Sutherland–Hodgman), not by a centroid test, since two non-convex tiles can overlap with neither centroid inside the other, and not by bounding boxes, which say nothing of a notch; the work is bounded (256 regions, 4 million triangle pairs). Any other face with regions keeps its message and the element its box: faces on a cylinder or a parametric surface refuse regions as before, as does a parent with holes, and a region that does not tile. The whole face takes the parent's render style; the regions' styles are not used. Nothing else changes in the reader. The tiling geometry is `regions_tile`, apart from the graph reading in `plane_regions_tile`, so that it can be tested on rings (`cargo test --lib`, which this change adds to CI's Linux test job). A first version (fe9be560, the one measured below) checked only bounding boxes and the area sum, which a region listed twice with another missing would satisfy; the stricter check above is the one in this change.

**Measured** (Measure 41790592582, fe9be560 against `main`, default exports, `mesh_bodies_vs_ifc` and `schema`, r2025-12 to r2025-17):

| file | saved-mesh bodies scored, before → after | `IfcWindow` exact (0.001 ft) | within 0.01 ft | volume within 1% | "ref on ours" | Revit's vertices to the mesh and back |
|---|---|---:|---:|---:|---:|---|
| each of the six | 1 → 4 | 3 of 3 | 3 of 3 | 3 of 3 | 3 of 3 | 1.6e-6 to 1.8e-6 ft both ways, volume ratio 1.0000 |

The 18 windows are exact against Revit's export, not only within the bar. Each file's swept solids with a Position of their own fall by 3 (r2025-12: 23 → 20), the windows' record boxes; no other line of any scorer's output in the six files moves, and the six files' IFC hashes all change (r2025-12: `22d84019…` → `69923ed9…`). The census at the reader's commit lists `face regions need subdivision` in no file, and r2025-12 and r2025-17 have all 4 door or window bodies drawn (1 of 4 before). Windows are drawn from saved meshes by default (Revit 2023 to 2027), so this moves the default export of these files: 3 windows each from their record box to their mesh.

**The whole Measure** (run 41791824472, 9c16f226 against `main`, `-f nordic=true -f r2025=true -f samples=true -f holdout=true`, every scorer, 111 models): `ifc.sha256` moves on 15. The office as built and at permit and its terrain variants, Core Interior, Einhoven, RE1, the other 18 Revit 2025 models, the other held-out models and every other Autodesk sample of every release are byte for byte as they were, and the scorer diff holds nothing but the files below.

- **Revit 2025 models 12 to 17** (6 exports): the 18 windows above. Besides `mesh_bodies_vs_ifc`, `box_bodies_vs_ifc` drops the 3 window boxes from each file (record-box bodies 12 → 9; their fill, Revit's body volume over the box, was 0.170) and `saved_meshes_vs_ifc` scores 3 more `IfcWindow` within 0.01 ft of Revit's world box (1 → 4 of 4); the schema step's swept solids with a Position of their own fall by 3 in each.
- **Snowdon Towers architectural** (Autodesk's 2024 sample, no Revit export, counted, not scored): `rvt-ifc` draws 172 of 172 door and window bodies from their saved graphics where it drew 130 of 172 (217,380 → 225,940 triangles; `saved_graphics_mesh` bodies 130 → 172, record-box bodies 3,479 → 3,437), so 42 bodies go from box to mesh. The census at a9bc2575 (run 41793980360) puts all 42 in `IfcWindow` (`census class_blocked IFCWINDOW 42`), with `face regions need subdivision` as the only cause of each (`census cause_set door_window err:Plane:face regions need subdivision 42`). Some plane faces with regions in that file have a trim that is not their UV box (36 of the 465 failing ones, counted once per `Geometry` that draws them); the census does not say whether they belong to the 42 windows, and nothing scores the 42. Another 21 bodies of that file that are not doors or windows have regions and `multiloop:CylSurf` and keep their box (opt-in only). Autodesk's other 2024 samples hold no door or window blocked by regions (census, run 41793980360): the 17 doors of rme basic and of rme advanced that keep their box are held by `multiloop:CylSurf` alone, and the rest are drawn.
- **Held-out models** (eight of eighteen, counts only, run 41793911979): the held-out summary kept no count of saved-mesh bodies, so every held-out `counts.txt` was identical before and after in 41791824472; #1340 makes it keep the totals and class rows of the three scorers (numbers beside an IFC class name), and the run applies it to both sides. Newly drawn, all `IfcWindow`:

| held-out models | newly drawn windows each | record-box bodies | saved-mesh bodies scored | `IfcWindow`: n, volume within 1%, within 0.01%, within 0.01 ft, within 0.001 ft, "ref on ours" | median volume ratio |
|---|---:|---|---|---|---|
| `rvt-holdout` 2024, 2025, 2026, 2027 | 12 | 72 → 60 (71 → 59 for 2025) | 3 → 15 | 12, 12, 12, 12, 12, 12 | 1.0000 |
| `rvt-holdout-office` 2024, 2025, 2026, 2027 | 10 | 59 → 49 | 4 → 14 | 10, 10, 10, 10, 10, 10 | 1.0000 |

88 windows, 88 of 88 right under "ref on ours", and exact (within 0.001 ft, volume within 0.01%); `saved_meshes_vs_ifc` has all 88 within 0.01 ft of Revit's world box. Their `IfcDoor` rows and every other class's are as they were (the three doors of each `rvt-holdout` model are drawn before and after, and not equal to Revit's within 0.001 ft; they are not new). The other ten held-out models, and their counts, are unchanged. This was the rule's first measurement on these pairs: it passed, so they stay held out, and nothing was tuned on what they hold (the rule was written from the public Revit 2025 models). The score cache did not see these outputs: all 54 steps (18 models by 3 scorers) logged `not cached ...: held out now or once, only counts of it leave the runner`, with 0 hits, 0 misses and none stored.

**The stricter check changes no export.** The first version of the reader (fe9be560) was measured above. Exports-only runs of the review's stricter tiling check (1c6f05e7, run 41792431082) and of the pure core factored out with its tests (63a2caa2, run 41793017480) give an `ifc.sha256` equal to run 41791824472's on all 111 models, on the head side and on the base side.

## 4. Not claimed

- That regions explain #1247's 142 boxed doors and windows: the reporter's census (the same probe, counts only) will say which kinds block them.
- That a face with regions is always two strips: 16 faces of one window family were read. The reader's condition holds for any number of tiling plane regions, but no other case is measured.
- That the 30 fittings' excess volume is wholly Revit's faceting, or that any of the 72 off vertices is Revit's error and not the mesh's; or a cause for the 46 on faces 46, 47 and 49.
- That the reader's refusals fire on a real file: none of the 24 faces fails its check, and no model here holds a region listed twice, one that overlaps another, or one in a notch. The geometry that decides them, `regions_tile`, is a pure function of 2D rings with nine fast unit tests (the pure-helper exception, Griffin, 2026-11-04): six refusals (a region listed twice with the other half missing, regions that overlap and leave a gap of the same area, a gap, a region in an L's notch, a region past the boundary by more than the tolerance, a self-crossing ring) and three passes (the window's two strips, an L's two arms, a region 5e-8 over the boundary). The graph reading around it is proved only by the 24 real faces.
- Anything about `HermiteSurf`, the graphics walk's causes (`GBiFlipControl`, `scaled instance mode`, `instance target restrictions`), ruled surfaces whose profile pointer does not resolve (15 office elements, always with other causes) or `GCylindricalHelix`: counted here, not read.
- That the census is exhaustive: an element's causes are those the mesher or the probe's graph reading names; a face that fails for a reason behind the first one it names is counted for the first (a `GNurbSpline` profile hides the trim that fails after it, RE-311).
- The reader on any file but the six Revit 2025 models.

## 5. Reproduce

```bash
# the census (a9bc2575 is the probe as the reporter of #1247 was given it; ffa6af2d is the same file)
gh workflow run measure.yml --ref main -f ref=a9bc2575 -f base=none -f nordic=true -f r2025=true \
  -f models="re1-* nordic-office-* r2025-* core-interior" -f scorers=none \
  -f probe=probe_re329_saved_graphics_census
# RE-305's reader on today's main, scored (8a24df6e is on the scratch branch re/329-saved-graphics-census)
gh workflow run measure.yml --ref main -f ref=8a24df6e -f base=main -f nordic=true \
  -f ifc_flags=--saved-meshes -f models="nordic-office-as-built re1-*" -f scorers="mesh_bodies_vs_ifc" \
  -f probe=probe_re329_saved_graphics_census
# the fittings face by face (60720f27, the same branch)
gh workflow run measure.yml --ref main -f ref=60720f27 -f base=none -f nordic=true \
  -f models="nordic-office-as-built" -f scorers=none \
  -f probe=probe_re329_wall_mounted_faces -f probe_scorer=toilet_faces_vs_ifc
# what a face with regions holds
gh workflow run measure.yml --ref main -f ref=d6601e5e -f base=none -f r2025=true \
  -f models="r2025-12_Rooms_Door_Windows r2025-17_Rooms_Stairs_Spiral" -f scorers=none \
  -f probe=probe_re329_face_regions
# the reader of this change, scored against main
gh workflow run measure.yml --ref main -f ref=fe9be560 -f base=main -f r2025=true \
  -f models="r2025-12* r2025-13* r2025-14* r2025-15* r2025-16* r2025-17*" \
  -f scorers="mesh_bodies_vs_ifc schema" -f probe=probe_re329_saved_graphics_census
# on a file of your own, counts only (opens it with --large-model's limits)
cargo run --profile ci -p rvt-probes --example probe_re329_saved_graphics_census -- model.rvt
```

(`a9bc2575`, `8a24df6e`, `60720f27` and `d6601e5e` are commits of the scratch branch `re/329-saved-graphics-census`, archived at `refs/archive/2026-11-04/re329-evidence`; `fe9be560` and the probes are on this change's branch.)
