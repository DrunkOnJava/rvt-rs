# RE-340: which walls that embed a curtain wall keep their end stubs is in their saved mesh, not in their cut partners

**Date:** 2026-11-04
**Issues:** #1385 (a byte signal for which hosts keep their end stubs); follows RE-335 (the six hosts), RE-258 (the base), RE-267 and RE-320 (the top)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), as Measure fetches them
**Probes, tools:** `probes/examples/probe_1385_cut_partners.rs` (the cut partners' boxes and native steps), `probes/examples/probe_1385_wall_saved_mesh.rs` (the saved graphics of the hosts), `tools/re/wall_stub_bodies_vs_ifc.py` (the horizontal faces of a body, in rvt-rs's export and Revit's)
**Runs:** Measure [41906631624](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41906631624) (RE-335's, `main` at 5026d863), [41940093060](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41940093060) (the frames of the walls and columns), [41940892298](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41940892298) (the hosts' bodies in both exports), [41941196428](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41941196428) (the partners' native steps, full-precision boxes), [41941553641](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41941553641) (the partner columns' bodies in both exports), [41942743319](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41942743319) (the hosts' saved graphics, `probe_1385_wall_saved_mesh` at f8e19519), [41945943795](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41945943795) (the rule, both offices, Core Interior and RE1 Architecture, against `main`; section 5), [41947083013](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41947083013) (the held-out models; section 5) and [41948046351](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41948046351) (Einhoven, puzzbobb's Revit 2025 models and Autodesk's samples, exports against `main`; section 5)
**Status:** positive. The lowest vertex of a host's saved mesh is Revit's lowest point on 113 of 113 host states, and the cut partners' records are not (section 2). Walls that embed a curtain wall and whose mesh starts at least 0.5 ft above their base are drawn from that mesh.

## 1. The question

RE-335 found that Revit's body for a wall that embeds a curtain wall is the wall less the curtain wall's prism and less the volumes of the elements that cut its end stubs, and that the six hosts Revit raises (8.2021 ft as built) and the hosts that keep their stubs (2103576 as built, 2177158) have the same embed step. The issue asks for a byte-level reading that tells them apart.

Ground truth, for every wall of the office that names a `WallEmbedWallGStep` (57 as built, 56 at permit): Revit's lowest point against the wall's base, from the IFC exports. 14 host states are raised: 2098938, 2105250, 2177212, 2293116, 2294996 and 3450274 as built, by 8.2021 ft, and 1378125, 1416796 and 2294996 by 9.514 ft and 2098938, 2103576, 2105250, 2177212 and 2495664 by 8.2021 ft at permit. The other 99 start at their base.

## 2. The cut partners' records do not separate them

Each raised host has `CutPairGStep`s and `CleanPairGStep`s with columns (1927726 and 1927727 for 2098938), and the end stubs, the stretches of the wall the curtain wall does not span, lie inside those columns. Taking the element-record frame box of each cut and clean partner (newest frame of each id), and a host's end stubs as its body range along its line outside the curtain wall's:

| | raised in Revit | not raised |
|---|---:|---:|
| every stub inside the union of its partners' boxes, or no stub | 14 | 3 |
| a stub outside | 0 | 24 |

So the condition is necessary on all 14, and it is not sufficient: the three are 2177158 (as built and at permit) and 2041479 (at permit). 2177158 and 2177212 are one wall mirrored (11.874 ft long, 0.387 ft thick, the same curtain wall, a 0.1936 ft stub at the same end, a column of 1.2467 ft square whose box covers the stub with a margin of 0 to 3e-6 ft on both); Revit's body keeps all of 2177158's stub (a downward face of 0.0749 ft² at its base, 0.1936 x 0.3872) and none of 2177212's. Revit's bodies of the columns 1927726 and 1927727 are whole prisms (17.848 ft³ = 1.5543 ft² x 11.483 ft), so the columns are not the cut ones. 2041479 at permit keeps 0.0645 ft² of a 0.381 ft² stub that its column's box covers.

Read for those pairs and found not to separate them: the cut and clean steps' `m_userSet`, `m_hasCoincFaces`, history table lengths, affected-edge tags and step ids and order, and the join tweak step's edge tags; the wall's `m_isFlipped` (the twins differ, other raised hosts do not); the wall's join sets (the twins differ only in the ElementIds of the walls joined at the other end); the side alignments (2177158's second is -0.0295 ft and 2177212's is 0, which would tell that pair apart, but 2041479 at permit has none, so it does not tell the three apart); the curtain wall's offsets; and the signed margins of the partners' boxes over the stubs (all within 3e-6 ft). Beyond those the twins' records differ only in what a mirror changes: ids, positions and the flip.

## 3. The wall's saved graphics do

A Revit file keeps the graphics it last drew for each element (`saved_meshes`, RE-224). `probe_1385_wall_saved_mesh` decodes those of every wall that names a `WallEmbedWallGStep` (88 as built, 97 at permit) and prints their horizontal faces as `wall_stub_bodies_vs_ifc.py` prints a body. The 57 and 56 of them that Revit's export also has as walls with the same Tag are the host states of section 1.

- **The lowest vertex of the saved mesh is Revit's lowest point on 57 of 57 host states as built and 56 of 56 at permit**, to within 0.001 ft: the 14 raised stand 8.2021 or 9.514 ft above their base (the mesh's low z minus the wall's base), the other 99 on it. No record of the cut partners, nor any other read, separates them as this does.
- **12 of the 14 raised hosts' meshes are Revit's body**: the same volume to 0.02 ft³ and the same downward faces (level and area to 0.002 ft²) in the two exports. The other two are 2177212, as built (14.5901 ft³ against Revit's 14.7352) and at permit (14.5475 against 14.6925): the mesh is a box of 6 faces whose plan is 0.0442 ft² (0.114 ft of the wall's length) short of Revit's. 2177158's mesh keeps its stub (0.0749 ft² at the base, level and area as Revit's).
- The meshes of the 14 are the strips over the curtain walls: 11 of them a box of 6 faces and 12 triangles (the stubs gone to their top, as RE-335's 2098938) and 3 (2293116 as built, 2294996 as built and at permit) the strip with the cuts of the columns and joins, 12 to 18 faces.
- None of the 185 meshes has a diagnostic. 184 report `unbounded` faces (1 to 16 as built, 1 to 11 at permit), the 12 equal meshes among them, so the count is not a hole in the body; `saved_meshes::attach` has never held it against a mesh.

Revit saves the body it drew, stubs cut or kept; that is the signal, and no field of the records that make it is read.

## 4. The rule

`saved_meshes::attach_scoped_with_control` (every scope that reads saved graphics, on Revit 2024 to 2027: `wall_cut_steps` reads the embed step from the walls' native records, which name the ElementId in another shape in Revit 2023 and earlier and so give no steps there) now also takes the walls that embed a curtain wall: a wall exported as a plain extrusion or with RE-320's stepped top (not one whose base #1384 cut at a floor), alone in its Tag (no other entity of any kind has it, decided before a piece is dropped for what it hosts or how its base was cut), with nothing hosted in it but the opening its own curtain wall cuts (a door, a window, a floor void and an opening for any other element keep the wall as it was; Revit's export holds the curtain wall's void and the raised strip both), whose native record names a `WallEmbedWallGStep` whose `m_instId` is an `IfcCurtainWall` of the model (the 72 embedded walls of the office are all curtain walls). It reads their saved graphics in the pass that already reads doors' and windows', and when the mesh decodes whole (no diagnostic, RE-224) and its lowest vertex is at least 0.5 ft above the wall's base and at least 0.5 ft below its top, the wall's body is the mesh (`BodySource` `saved_graphics_mesh`, in the wall's frame as every saved mesh is). Any other wall keeps the body it had, and a file with no curtain wall reads no wall record for this. The walls' steps are read once for a file's handle (`wall_cut_steps` keeps them on it), so the pass that writes the walls' openings and this one do not read the same records twice. Where the records cannot be read the export says why (`ExportNote::EmbedHostBodiesSkipped`) and keeps the bodies.

The wall's `extrusion` stays the whole prism, which the passes that read a wall's plan use, as for a stepped top (RE-320); the writer and the glTF draw the mesh. The walls carry no base quantity of their own to correct: a wall with an embed step is one whose record holds a cutting step, and gets none (`add_wall_areas_and_volumes`).

## 5. Results

Measure [41945943795](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41945943795): this branch at 847c3735 against `main` at 77d71cbb (which holds #1384's footings), both offices, Core Interior and RE1 Architecture, with the seven wall and mesh scorers.

| | as built | at permit |
|---|---:|---:|
| walls that embed a curtain wall and that the rule considers (plain or RE-320 stepped, alone in their Tag, nothing hosted in them, not on a base cut at a floor) | 44 | 43 |
| of those, drawn from their saved mesh | 6 | 8 |
| kept their body for a mesh that did not decode whole | 0 | 0 |
| the 14 hosts Revit raises: base now Revit's (`wall_heights_vs_ifc`) | 6 of 6 | 8 of 8 |
| walls whose base and top are Revit's | 598 to 604 of 607 | 577 to 585 of 589 |
| top Revit's, base not | 8 to 2 | 12 to 4 |
| `wall_quantities_vs_ifc`: walls whose `Height` rvt-rs's body span makes Revit's | 572 to 578 of 579 | 557 to 565 of 565 |
| the same for `NetVolume` (rvt-rs's body volume) | 297 to 303 of 607 | 275 to 283 of 589 |
| `mesh_bodies_vs_ifc`, `IfcWall` bodies: volume within 0.01%, every vertex of rvt-rs's on Revit's surface and of Revit's on rvt-rs's | 5 of 6 | 7 of 8 |

- Only the 14 hosts' rows moved. Every other row of the seven scorers in the two office files is the same, but for the scorers' own fit: the plan turn they estimate from the walls' middles moves by 5e-5 degrees (40.332546 to 40.332493), which shifts the 4th decimal of the Revit frame's coordinates in rows that did not change. 2103576 as built and 2177158 are unchanged, hosts that keep their stubs in Revit.
- The 2 and 4 walls still off are the Basement walls 2057668 and 2057670 (and 2058499 and 2058501 at permit), 0.123 and 0.044 ft under Revit's base on their footings, which are not embed hosts and are not touched.
- The one host whose mesh is not Revit's body is 2177212 (both dates): volume ratio 0.9902 and 0.9901, a vertex of Revit's 0.3018 ft from rvt-rs's surface, and none of rvt-rs's off Revit's by more than 1.9e-6 ft. Its base is Revit's. The mesh is Revit's own saved graphics and Revit's IFC export draws the strip 0.114 ft longer (section 3). It agrees with Revit's own quantities of the wall, which the same export writes: `NetVolume` 14.590085 ft³ against the mesh's 14.590083 and `GrossSideArea` 37.686932 ft² against 37.686926 as built, and 14.547453 and 37.576811 against 14.547453 and 37.576811 at permit (`wall_quantities_vs_ifc`, run 41945943795). So the 1.0% is Revit's IFC tessellation of the wall against Revit's own quantities of it, not the mesh being short, and the wall stays in the rule.
- **Core Interior and RE1 Architecture: `ifc.sha256` identical to `main`'s**, and neither file has a wall that embeds a curtain wall (no note). The offices' exports differ in the 14 hosts' bodies only: a note line per office, `diagnostics.json` and the hash.
- The cost is one extra read of the plain and stepped walls' native records in a file that has an `IfcCurtainWall` (none in a file that has none: Core Interior and RE1 took 4.41 and 1.17 s against 4.39 and 1.12 s), and the hosts' saved graphics in the pass that already reads doors' and windows'. The offices' export took 20.1 and 17.1 s against 18.3 and 15.5 s on the same CPU model, while the scorers this change does not touch took 5% to 17% longer on the same runner (`wall_bodies_vs_ifc` 126 s against 110, `saved_meshes_vs_ifc` 209 against 179): no cost beyond the pair's runner noise is shown, and none is claimed.

Einhoven, puzzbobb's Revit 2025 models and Autodesk's samples (Measure [41948046351](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41948046351): 3be52203 against `main` at 77d71cbb, `-f samples=true -f r2025=true -f models="einhoven r2025-*" -f scorers=none`, so exported and hashed, not scored): 83 models (Einhoven, 24 Revit 2025 models, 58 samples of Revit 2015 to 2027 including the five Snowdon Towers files), 601 files on each side, all identical but the export time and, in the `rvt-ifc` logs of 16 samples of 2024 to 2027, the new note, which says that 0 of 1 to 15 walls that embed a curtain wall (Snowdon Towers architectural: 0 of 15) were drawn from their mesh. `ifc.sha256` and `diagnostics.json` are the same on all 83: the rule draws nothing on any of them. Einhoven has no wall that embeds a curtain wall (no note), so the viewer's saved-mesh specs, which pin its doors' and windows' bodies, are unaffected.

**The layers, quantities and properties** (Measure [41952317218](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41952317218): 4750acc3 against `main` at 3861cc34, both offices, `qtos_vs_ifc`, `psets_vs_ifc`, `wall_layers_vs_ifc`): the quantity and property census files are identical to `main`'s. The glTF no longer splits the hosts into layers: `partition_wall_centreline_layer_joins` bodies fall from 272 to 266 as built, `wall_layers_vs_ifc` counts 588 walls drawn in layers in the GLB as built before and 582 after (the six hosts were all walls of two or more layers: 295 to 289; one layer 289 to 289), and 503 and 501 at permit (two hosts of one layer: 272 to 270). The comparisons that remain are the ones that were: the layer centres within 0.001 ft 569 of 584 before and 563 of 578 after (the six that left were within it; the 15 that were not still are, the worst 0.4921 ft on 1494012), the colours 696 and 690 the same as Revit's. What the multi-layer hosts lose is their layer split in the glTF, which draws each as one solid of the category's colour: `layered_stepped_meshes` splits only a compound of stepped extrusions and has nothing for a mesh. The IFC keeps their material layer set and usage (the pass changes the body and its source only), over a body that is Revit's own saved mesh, which Revit's export writes as one solid too. #1405's `one_layer_solid_meshes` will cover the hosts of one layer once both are in; a host of several layers needs the mesh split by its layers, which is not done.

**Snowdon Towers (architectural)** has no raised host in Revit's own export. Revit's IFC4 export of it (the bldrs-ai copy, local; IfcOpenShell 0.9.0, 1,078 walls meshed in world coordinates) has one wall that starts 0.5 ft or more above its placement, 1996153, a parapet cap raised 1.9167 ft, and none that starts below it. 17 walls of the file name an embed step (`probe_1385_wall_saved_mesh` on the 2024 samples, run 41952518382 at 4750acc3), and 1996153 is not one of them; so none of the 17 is raised in Revit's export, and the rule drawing none of the 15 it considers is right.

Held-out models (`-f holdout=true`, Measure [41947083013](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41947083013): f016fc32 against `main` at 77d71cbb, `mesh_bodies_vs_ifc` and `saved_meshes_vs_ifc`): 19 models, 18 held out and the spent Vitoria 2018 file, 46 files on each side, all identical but the export time in one log. `ifc.sha256` is the same on all 19, so no held-out export changes, and the one log kept (the spent file's) has no walls-that-embed line, so no host was drawn from a mesh there. Whether any of the 18 holds a wall that embeds a curtain wall is not known from what is kept (their logs are not), and the rule reads Revit 2023 to 2027 records only.

## 6. What the 0.5 ft rule stands on

The threshold was fitted and checked on the same two files: the BIM4LCA office as built and at building permit, 113 host states, of which 14 are raised and make 10 distinct walls (2098938, 2105250, 2177212, 2293116, 2294996, 3450274, 1378125, 1416796, 2103576 and 2495664). Every raise there is 8.2021 or 9.514 ft and every other wall is within 0.001 ft of its base, so any threshold between 0.001 and 8.2 ft draws the same walls; 0.5 ft is a round figure in that gap, not a measured edge. The held-out models and Autodesk's samples gave no host to score (their exports are `main`'s byte for byte, or the rule drew nothing on them), so nothing outside these two files has checked it.

## 7. Not measured, and not drawn

- A wall that embeds a curtain wall and has an opening hosted in it, or an embedded wall that is not a curtain wall (none of the office's 72), keeps its body.
- Walls Revit raises for another reason (the 14 Basement walls on sloped footings, RE-335 section 1) are not touched; the saved mesh may well draw them, but the rule's scope is the embed step.
- 2041479 at permit keeps 17% of a stub in Revit; its mesh keeps 0.0203 ft² of it, so it is not raised, and is drawn as it was.
- The saved mesh is not always Revit's IFC body in the stubs' areas: 2103576 as built keeps 0.6656 ft² of stubs in the mesh against Revit's 0.754. That is the existing body's, left as it is.
- Revit 2023 and earlier: the walls' native records give no embed step, so no wall is drawn from its mesh. The rule applies on Revit 2024 to 2027, and a raised host has been seen on the two Revit 2024 office files alone.
- The pull request's Measure runs are filtered (`-f scorers=` the seven wall and mesh scorers on the offices, Core Interior and RE1 Architecture; the held-out models with the two mesh scorers; exports only for the rest): no unfiltered Measure with every scorer on every model has been run on this change. The rule fires on any Revit 2023 to 2027 file that has an `IfcCurtainWall` and a wall that embeds one, so a model outside the ones above (Revit 2025's puzzbobb models are in the last run, but only hashed) is measured against Revit's export only if a scorer was run on it.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref <this branch> -f ref=f8e195196b4aa231c7c3c749a51087f9eeb9ddde -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
  -f probe=probe_1385_wall_saved_mesh -f probe_scorer=-
gh workflow run measure.yml --ref <this branch> -f ref=eeaa44f35d7dd7dd8957685c58df7172e11410c4 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers="wall_stub_bodies_vs_ifc" \
  -f probe=probe_1385_cut_partners -f probe_scorer=-
```

The first prints, for each wall that embeds a curtain wall, `saved tag:<id> status ...`, its volume and box (z low and high, x, y), and a `face up|down z <level> area <ft²>` line per level of horizontal faces; the second, the partners' frames, steps and (through the scorer) bodies in rvt-rs's export and Revit's.
