# RE-494: A cylinder face bounded by two trim loops on part of a turn

**Date:** 2026-11-20
**Issues:** #1191 (W6), #2102 (this change).
**Artefacts:** the BIM4LCA office (as built, at permit, and its timber version) and housing files (VTT, CC BY-SA 4.0; Revit 2024), each with Revit's own IFC4 export; the held-out house; Autodesk's sample projects of Revit 2023 to 2027 and Snowdon Towers' 2024 models, which have no Revit export in Measure. Outside Measure, kept local: Revit's IFC4 export of Snowdon Towers Architectural (bldrs-ai/test-models, no licence) and the VIM export of Snowdon Towers' 2027 edition (vimaec/vim-hackathon).
**Probes:** `probe_2102_cylinder_loops` (`6327fb09`) and `probe_2098_face_cracks` widened to two-loop cylinder faces (`06e22382`), naming each body's two-loop graphs (`1ceb4191`) and marking two-loop faces in its face classes (`87b23899`). None is landed; they are kept as `refs/archive/2026-11-19/measure-2102-cylinder-loops`, `refs/archive/2026-11-19/measure-2102-cracks`, `refs/archive/2026-11-19/measure-2102-reach` and `refs/archive/2026-11-20/measure-2102-tee-faces`. The reads of exports below (the bar's sections and shells, the placement of Revit's vertices, the two-way distances, the record-box check, the VIM shells and extents) are scratch tools kept as `refs/archive/2026-11-20/measure-2102-reads` (`e42a7df5`, `research-tools/2102/`).
**Status:** positive, with one family Revit's export draws as flats on a round. 1,289 bodies that were record boxes (1,256 element-record boxes and 33 turned family-instance boxes) are drawn from their saved meshes on the exports kept, and 10 on the held-out house. Of those Revit's exports score, the 16 sauna doors (`O-5:O-5.9x21sauna`, 4 on each housing file) and the held-out house's 10 doors are within 0.01 ft both ways and within 1 per cent of Revit's volume. The 30 grab bars on each office file (`Stay_Wall_Mounted_:VAR-1`) are not: Revit's export draws their round bar as a square along its legs. They ship drawn round, as the file saves them, with that miss on record (Griffin, 2026-11-20); landing with that row failed is outside the outcomes registered in the pull request, by that decision. Against Revit's export of Snowdon Towers Architectural, outside Measure, its column and five trash chutes have every Revit vertex within 0.001 ft of rvt-rs's surface, while rvt-rs's stand up to 0.0129 ft off Revit's, past the 0.01 ft bar. The rows were registered in the pull request before each run was read.

## 1. The rule

A cylinder face whose trim is two loops on part of a turn is meshed strip by strip across U (`band_mesh`). The strips' edges fall on every point of both rings, so each ring edge crosses a strip whole or not at all, and each strip is covered between its crossings taken in pairs by height. A hole inside the other loop and two separate regions are covered alike. The face's points are both rings', so a plane beside it rebuilds the same steps (`refined_plane_edge`). Two loops that make a full turn, a tube with no seam edge, are refused as `two cylinder trim loops on a full turn unsupported`: no face of the 27 files the probe read has one (Run C), and refusing them changes no export and no `diagnostics.json` on any model (Run B). Three or more loops are refused as before.

The mesher pairs crossings by height and does not read the rings' winding. Read with the probe on the office's and the housing's two-loop faces, a ring whose signed area in the surface's parameters has the sign of the face's orientation (the surface's `m_orientFlag`, flipped by bit 2 of the face's `m_faceFlags_v9`) is an outer boundary and one of the other sign a hole, and the two Stay symbols, which hold the same faces with the orientation the other way round, flip every ring's sign with it. One face is an exception: graph 1059725 face 13 has an inner ring wound like its outer. No body this change draws holds it (§2, Runs D and E).

## 2. Measured

**Run A** ([44649733126](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44649733126), `1ab9c54e` against `4e05cfb0`; nordic, r2025, samples, holdout, every scorer, exports kept).

- 27 of 120 exports move. On 22, record boxes become saved meshes; on the five `rac_basic` saves only refusal texts move.
- No count of `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc`, `roof_bodies_vs_ifc` or `stair_bodies_vs_ifc` falls, and no `psets_vs_ifc` or `qtos_vs_ifc` line moves. `door_bodies_vs_ifc` moves 4 housing doors from "mirror rule wrong" to "rvt-rs's box is Revit's".
- No body drawn from a saved mesh on both sides changes its mesh, on all 94 export pairs read (`ifc_body_diff.py`). The held-out house's export is not kept, so it was not read.
- `multiple cylinder trim loops unsupported` leaves 1,289 bodies that named it alone, now drawn, and 97 that named it with other reasons, which keep only those (60, 21, 10 and 6 by reason). No body names a two-loop reason.
- The same-project gates pass: 15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save.

**Run B** ([44660808578](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44660808578), `d46bfeff`, which removes the full-turn reader, against `1ab9c54e`; nordic, r2025, samples, holdout): 120 of 120 exports and 102 of 102 `diagnostics.json` identical.

**Run C** ([44659164312](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44659164312), `probe_2102_cylinder_loops` on every 2024 to 2027 sample, and the office and housing runs before it): every two-loop cylinder face covers part of a turn, 1 to 51 faces a file.

**Runs D and E** ([44664659724](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44664659724) on the housing and [44666836079](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44666836079) on the office, `probe_2098_face_cracks` at `1ceb4191`, which names each body's two-loop graphs): the bodies that reach graph 1059725 are showers, 22 on the housing (`Suihku 1:SU-1`, `Suihku 2:SU-2`) and 10 on the office (`Shower_columns_15486:SU-1`), and each still stops at an earlier refusal. The housing's sauna doors' two-loop faces are all in graph 1902301.

**The grab bars** (`Stay_Wall_Mounted_:VAR-1`, 30 on each office file, two of Revit's shapes, 15 each, read alike on one of each).

| | rvt-rs | Revit's export |
|---|---|---|
| the body's volume | 1.11 of Revit's | |
| the bar shell's volume | 1.29 of Revit's | |
| a leg's section, at 0.3 to 0.6 of the shell's length on both legs and 0.7 on one | a round, radius 0.0110 m, 0.000377 m² | a square, corners 0.0110 m from the centre, 0.000241 to 0.000247 m² |
| the U-bend's outer edge, about the centre of the half round the legs make (0.2059 ft out, 0.1337 ft in) | | five of its nine vertices off the round lie at the middles of pieces of 25.7° whose ends would lie on that round, 0.0052 ft in; the one end read is on it |
| the U-bend's inner edge | | six vertices at the middles of pieces of 30° whose ends would lie on the inner round, 0.0046 ft out |

- Revit writes the body as explicit facets (`IfcPolygonalFaceSet`), so the flats are its own drawing.
- In one frame the bars' boxes agree within 0.0001 ft on five sides; on the sixth rvt-rs's reaches the U-bend's tip, 0.0052 ft past Revit's flat.
- Of Revit's 1,083 vertices, 76 are beyond 0.001 ft of rvt-rs's surface: 15 at the U-bend, 11 at a turn at the other end, and 50 on a second shell, whose rvt-rs shell has edges only one triangle holds at a 1e-6 m weld, so their side is not read. rvt-rs's are up to 0.0116 ft off Revit's, against 0.0106 ft a square in the round predicts.
- The faces meet within 0.000878 ft (`probe_2098_face_cracks`, [44646090830](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44646090830)).

**The bodies no Revit export in Measure scores** (1,153 on the samples).

- Against the record box each replaced (Revit's own box for the element): none reaches past it but the 108 Snowdon Plumbing sanitary terminals, by up to 1.45 mm. The rme pipe fittings fill theirs within 0.1 mm on all nine rme saves. Snowdon HVAC's tees and grilles stop up to 42 and 21 mm short of their boxes; on two of each read, the VIM's mesh has rvt-rs's height and its extents within 9 mm.
- Against Revit's IFC4 export of Snowdon Towers Architectural (its 7: a column, five trash chutes and a beverage dispenser, all explicit facets in Revit's export): Revit's vertices are within 0.001 ft of rvt-rs's surface on all 7, and the volume within 0.8 per cent; rvt-rs's stand up to 0.0129 ft off Revit's on six.
- Against the VIM (`tools/re/footing_meshes_vs_vim.py --document HVAC|Plumbing`): height and lowest point within 0.001 m on all 36 duct fittings and 108 sanitary terminals and 391 of 404 air terminals (the rest by 0.00107 m); the air terminals' volume 1.1 per cent over the VIM's, the duct fittings' within 8.7 per cent. The sanitary terminals' volume figure is not a volume: on the three read, the trap body is an open shell in both exports.
- Whether their faces meet (`probe_2098_face_cracks`, [44663576285](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44663576285)): Snowdon Plumbing's 121 bodies within 0.00093 ft; the rme fittings within 0.00092 ft on the 2024 to 2027 saves; Snowdon HVAC's 36 newly drawn `Round Tee:Standard` each with 32 boundary vertices beyond 0.001 ft, up to 0.00158 ft, where one cylinder meets another. With two-loop faces marked (`87b23899`, [44679890567](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44679890567)), each tee's 32 split evenly: 16 on its two-loop face stand off the one-loop cylinder beside it, and 16 on that cylinder stand off the two-loop face, both up to 0.00158 ft. So each face puts points on their shared edge that the other does not hold, the kind RE-493 left between two cylinders (588 of its 2,163).

**The export's size.** Snowdon Plumbing's export grows from 112 MiB to 130 MiB, past the 120 MiB Measure validates (#2103). Read locally with IfcOpenShell 0.9.0.post1 as Measure reads it: 0 validation and 0 EXPRESS-rule findings.

## 3. Not claimed

- **The grab bars against Revit.** They are drawn as the file saves them, round; Revit's export squares them, and the miss stays on record. Of rvt-rs's standoff, 0.0001 to 0.0005 ft is more than the square's flats explain, and the pieces of the turn at the bar's other end are not read.
- **Snowdon Towers Architectural's column and five trash chutes against Revit.** Revit's vertices are on rvt-rs's surface, but rvt-rs's stand up to 0.0129 ft off Revit's flats; which of their parts are round is not read.
- **Why the sanitary terminals reach past their boxes.** On the two read, the vertices past the box are on a toilet connection's 0.19 m shell (up to 1.45 mm) and a sink trap's body (0.99 mm); whether Revit's box is the box of a coarser drawing there is not read. All 108 hold the VIM's height and lowest point within 0.57 mm.
- **The 0.001 ft and "ref on ours" bars on the sauna doors.** Of one door's 1,752 Revit vertices, 366 are beyond 0.001 ft of rvt-rs's surface, all within 0.0043 ft, mostly inside it; whether those are flats of the same kind is not read.
- **Snowdon Towers' 556 bodies.** The model has one save, so no gate holds them; what is read of them is above.
- **The tees' gaps.** Where a two-loop face meets another cylinder, each samples their shared edge at its own steps; putting both on the edge's saved points, as RE-492 and RE-493 do at a plane, is not done here.
- **A full turn.** No face of any file read is a tube between two circles, so that pair of loops stays refused.
- **The winding of one face** (graph 1059725 face 13), read by the even-odd pairing as a hole; no body this change draws holds it.
