# RE-318: A door's or window's width is the extent of the box edges its symbol's graphics hold

**Date:** 2026-11-03
**Issues:** #810 (the two doors left 0.1137 ft wider than Revit's `OverallWidth`); follows RE-215 (cut loops), RE-218 (sizes from the record box), RE-225 (cut solids) and RE-229 (stored lengths)
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |

Measure fetches all four by byte range and stores none (`research/bim4lca-measure.tsv`). The Measure runs below also read the reference models, the R2025 models and, for the held-out check, the held-out models, as the workflow fetches them.
**Probes:** `probes/examples/probe_re318_door_width_sources.rs`, `probe_re318_family_document.rs`, `probe_re318_hidden_geometry.rs`, `probe_re318_filtered_geometry.rs`, `probe_re318_symbol_graphics_tree.rs`, `probe_re318_symbol_curve_extents.rs`. **Scorers:** `tools/re/door_width_sources_vs_ifc.py`, `hidden_geometry_vs_ifc.py`, `symbol_curve_extents_vs_ifc.py`, `door_window_sizes_vs_ifc.py`.
**Status:** positive on the BIM4LCA office, one building's families: the extent along X of the lines of no category in the saved graphics of the symbol that draws a door or window is Revit's `OverallWidth` on every one of the office's doors and windows Revit sizes. Outside the office the rule fires on one door, RE1 Architecture's curtain-wall door 445975, and leaves its width as it was (§6). What the boxes are, and why their extent is Revit's width, is inferred from Revit's widths, not documented.

## 1. The two doors are wall hatches, not doors with a sidelight

Doors 1756089 and 1765267 are `IfcDoor`s with `PredefinedType` `TRAPDOOR` in Revit's export: family `LU-3.VSS`, types `LU-3.VSS.700x1200_90°` (symbol 2058852, type 1106810) and `LU-3.VSS.700x1200_180°` (symbol 1766487, type 1106812), set in walls 1347201 and 1347200. Revit writes both 1328 mm high and 1787.00001 mm (5.8629 ft) wide. Its body for the 180° type is a 950 mm frame ring and an 882 by 50 mm leaf standing open beside it, 1787 mm across; its body for the 90° type is the same frame and a leaf across the wall, within the frame's 950 mm. So the 90° hatch's `OverallWidth` is not its own body's width. The record box is 5.9766 ft on both (RE-218).

revit-ifc sizes a door or window from the symbol that draws it: `FamilyInstanceExporter.GetDoorWindowDimensionFromSymbol` takes the width from `ExporterIFCUtils.GetMinSymbolWidth`, which is closed (CLEANROOM.md source 5). What it measures is therefore inferred from Revit's widths below, not documented.

## 2. Candidates 1 and 2

Measure [41718364745](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41718364745), `probe_re318_door_width_sources` on both office files. **The cut solid's unbounded plane (negative).** On both symbols the one face of the cut solid that does not mesh is a `Plane` at `y` 0.7792 ft whose normal (x × y of its axes) is along Y: it adds nothing along X, and the bounded faces span 3.1168 ft. `m_closurePlanes`, `m_cutDirs` and `m_cutIndices` are empty on both, and `m_cutPlaneHeights` is [8.2021, 7.5459].

Measure [41718666650](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41718666650), `probe_re318_family_document` (the new `native_family_forms::family_records`). **The loaded family's own document explains the 90° hatch.** Both types are of family 1107790, whose document holds the yes/no parameters `Door_90_degrees` and `Door_180_degrees` and two leaf extrusions (1109146, 1109153), each with its `Visible` bound to one of them: each type shows one leaf and hides the other. The document's forms depend on the type's parameter values, so it is not a reader of a symbol's width by itself.

## 3. The symbol's saved graphics

The extent along X, in the symbol's own frame, of the surfaces of the symbol's channel 103 record (same run, and Measure [41720185112](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41720185112), `probe_re318_hidden_geometry`; [41721522407](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41721522407), `probe_re318_filtered_geometry`), against Revit's `OverallWidth` on the 268 doors Revit sizes and the 269 windows (as built; the permit file gives the same except 265 windows):

| surfaces | doors equal | windows equal | misses |
|---|---:|---:|---|
| visible ones | 266 | 269 | 1756089 (3.1168 ft), 1722215 |
| and the branches the visibility flags hide | 267 | 269 | 1722215 |
| and the branches a filter rejects | 267 | 269 | 1722215 |

The hidden branches are the 180° leaf in the 90° symbol: with them its extent is 5.8629 ft, Revit's. The 180° hatch's visible surfaces already give 5.8629 ft (frame -1.5584 to 1.5584 ft, leaf 1.4108 to 4.3045 ft). Filtered branches change no extent anywhere. Door 1722215 (family O-16, type `O-16.1200x2000_180°`, symbol 1727541) stays 9.0224 ft against Revit's 9.0387: 5 mm short, whether the surfaces are the symbol's or the instance's. Its record box, 9.0387 ft, and Revit's width agree; its cut solid (4.8228 ft) and Revit's opening (1470 by 300 mm) are not it.

## 4. The box edges

Measure [41721882540](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41721882540), `probe_re318_symbol_graphics_tree` (symbols 1727541, 2058852, 1766487 and 2393482). Besides its surfaces, a symbol's graphics hold curves, among them `GLine` nodes of category -1 (no category) with `m_GInfo.m_flags` 0x1088004 (visibility bits 0x1e0 clear), in sets of 12 edges of a box, each line with a `GConditionDir` (`m_comp` 10) on its own direction, a view-direction test. Their extent along X:

| symbol | what | lines' extent along X | Revit's width |
|---|---|---:|---:|
| 1727541 | O-16 door 1722215 | -6.6109 to 2.4278 ft, 9.0387 ft | 9.0387 |
| 2058852 | 90° hatch 1756089 | -1.5584 to 4.3045 ft, 5.8629 ft | 5.8629 |
| 1766487 | 180° hatch 1765267 | -1.5584 to 4.3045 ft, 5.8629 ft | 5.8629 |

The O-16 leaf's lines reach 5 mm beyond its faces, and the 90° hatch's boxes include the hidden leaf. Measure [41722740913](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41722740913), `probe_re318_symbol_curve_extents`, groups the curves of the symbol behind every door and window by class, category and flags. Their extent along X, from the `GLine`s of category -1 whose visibility bits are clear ("wire"), against Revit's `OverallWidth` within 0.001 ft:

| file | class | Revit gives a width | wire equal | Revit gives none |
|---|---|---:|---:|---|
| as built | IfcDoor | 268 | **268** | 1746275 (0 by 0) |
| as built | IfcWindow | 269 | **269** | |
| building permit | IfcDoor | 268 | **268** | 1746275 (0 by 0) |
| building permit | IfcWindow | 265 | **265** | |

Of the as-built office's 105 door and window symbols, 104 hold such lines (8 to 316 of them); the one that holds none, 3195622, is door 1746275's, which Revit writes 0 by 0. The surfaces and the record box each miss some of the rest (§3, RE-218); every curve of the symbol, hidden or not, gives Revit's width on all of them too. What the boxes are is inferred: their extent is Revit's width on every door and window the office has, and the view-direction filters on them are not read.

## 5. What rvt-rs does

`native_symbol_cuts::symbol_wire_widths` reads, in one native pass over channel 103, the extent along X of those lines for each symbol it is given. `attach_cut_loop_sizes` gives it to a door or window whose symbol cuts no loop and which RE-218 already sizes from its record box (`SymbolWireWidth`, `SYMBOL_WIRE_WIDTH_FIELD`), and the STEP writer takes `OverallWidth` from it ahead of the record box's width. The height, the cut loop (RE-215), the opening (RE-93, RE-94) and the type's size (RE-187) are as before.

**Fail closed.** A symbol with no such line, or whose extent is not finite and positive, or whose graphics hold a `GInstance` or `GComponentRef` (lines that would sit in another frame), or that is read twice with different extents, gets no `SymbolWireWidth`, and its door or window keeps the width RE-218 gave it (the record box's, else the type's); today's code is unchanged there. The reader walks the graph's object list once, with no recursion; the native record path bounds the number of objects (`Options::max_graph_objects`) and values, and no value from the file is used unchecked or unwrapped.

## 6. Measured

Measure [41725344128](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41725344128) (this change at `ae2078d3` against `main` at `53e06c02`, default models plus `-f nordic=true -f r2025=true`, every scorer; an earlier run of the same change at `04917ca0`, [41723977857](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41723977857), gave the same scorer diff). `door_window_sizes_vs_ifc`, the one scorer whose output moves:

| model | class | equal before / after | differ before / after | `$` before / after |
|---|---|---:|---:|---:|
| office as built | IfcDoor (269) | 266 / **268** | 2 / **0** | 1 / 1 |
| office as built | IfcWindow (269) | 269 / 269 | 0 / 0 | 0 / 0 |
| office at building permit | IfcDoor (269) | 266 / **268** | 2 / **0** | 1 / 1 |
| office at building permit | IfcWindow (265) | 265 / 265 | 0 / 0 | 0 / 0 |

The door still `$` is 1746275, which Revit writes 0 by 0 and whose symbol has no wire to measure.

**What moves.** Comparing the `OverallHeight` and `OverallWidth` of every `IfcDoor` and `IfcWindow` in the two office exports before and after (Measure [41728328224](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41728328224), `-f keep_ifc=true`): as built, three change, and at building permit two. Doors 1756089 and 1765267 go from 1.821669 m (5.9766 ft) to 1.787000 m, Revit's 1787 mm, on both files; as built, door 1722215 goes from 2.755007 m to 2.755000 m, Revit's 2755 mm, seven micrometres. No other height or width of the office's 538 and 534 doors and windows changes.

**Outside the office.** The IFC of 31 of the 34 measured models is byte for byte what `main` writes (sha256): Core Interior, Einhoven, the other three RE1 models, the 24 R2025 models and the two terrain office files have no door or window whose symbol cuts no loop. Three differ: the two office files and RE1 Architecture. On RE1 Architecture the only door or window the rule reaches is the curtain-wall door 445975 (RE-215, #844): it gains `SymbolWireWidth` of 1.831871 m, equal to its `BoxWidth`, and its `OverallWidth` stays 1.831871 m, Revit's (Measure [41728305941](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41728305941), `-f keep_ifc=true`; its five doors are equal to Revit's before and after). No width outside the office moves, and none moves to a value Revit does not write.

**Held-out models.** Measure [41725349619](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41725349619) (`-f base=main -f holdout=true -f models="holdout-*"`, this change at `ae2078d3`): the counts of all 18 held-out models, and the hash of each IFC, are identical to `main`'s, and the scorer diff is empty. Counts only are quoted here.

**W7.** `psets_vs_ifc` lists property values, and `OverallWidth` is an `IfcDoor` attribute, so it has no line for it: its output on the as-built office is identical before and after (Revit values on objects rvt-rs writes 45525; equal 44163; different 1; missing 1361), as is `qtos_vs_ifc`'s (39388; equal 29448; different 1; missing 9939). The attribute is scored by `door_window_sizes_vs_ifc` above (differ 2 to 0 on each office file). The new `SymbolWireWidth` is in rvt-rs's own `RvtElementRecordGeometry` set, which both scorers leave out.

## 7. What it does not cover

- One building's families. The wire is Revit's width on all 537 doors and windows Revit sizes in the as-built office and all 533 at building permit, and RE-215's loops, RE-218's boxes and this agree where they overlap. Other models' families are as measured in §6; Snowdon Towers is local only and not measured.
- Height. `OverallHeight` is as RE-218 left it; the boxes' Z extent against Revit's height was not tried.
- Releases other than 2024 and 2025 (`CUT_LOOP_REVIT_VERSIONS`).
- The meaning of the boxes and of their view-direction filters, which is inferred from Revit's widths and not documented.
- Exposure outside the office. The 31 measured models that write the same IFC, and the 18 held-out models, have no door or window the rule reaches but RE1's 445975, so they show it changes nothing there, not that it is right there. The `GInstance` and `GComponentRef` exclusion never fires on the office, so it is untested.
- Edges wider than the record box. No measured symbol's edges reach past its instance's record box along the width; where they would by more than 0.001 ft, the record box's width is kept (`SYMBOL_WIRE_BOX_SLACK_FEET`, added after review, 2026-11-03), so a stray set of lines of no category cannot widen a door past its box.

## 8. Reproduce

Each probe and its scorer, at the commit it was run on (`--ref` names the branch that carries the scorers; the commit is the code and probe measured):

```bash
# 1. §2 candidate 1, the cut solid's planes (74d20f3f)
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=74d20f3f32fe4c8771f05c4e2db720b072ff65ff -f base=none \
  -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re318_door_width_sources -f probe_scorer=door_width_sources_vs_ifc
# 2. §2 the family document (c551ef27)
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=c551ef274c819db1f8903a7b0fcff37b64ac2c0c -f base=none \
  -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re318_family_document -f probe_scorer=-
# 3. §3 hidden branches (44c2b3cb) and filtered branches (a92e13e7)
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=44c2b3cb81a3f606fd66721555040b07ee13d5a1 -f base=none \
  -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re318_hidden_geometry -f probe_scorer=hidden_geometry_vs_ifc
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=a92e13e7f4d192391845cfc7d1c266b24b2546d8 -f base=none \
  -f nordic=true -f models="nordic-office-*" -f scorers=none -f probe=probe_re318_filtered_geometry -f probe_scorer=hidden_geometry_vs_ifc
# 4. §4 the tree dump (1007807c) and the wire against Revit's widths (ae2078d3)
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=1007807c379e37eb171936807d1151789b27eb68 -f base=none \
  -f nordic=true -f models="nordic-office-as-built" -f scorers=none -f probe=probe_re318_symbol_graphics_tree -f probe_scorer=-
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=ae2078d3895d6b858eaf6332d78292569f1c0c51 -f base=none \
  -f nordic=true -f models="nordic-office-as-built nordic-office-permit" -f scorers=none -f probe=probe_re318_symbol_curve_extents -f probe_scorer=symbol_curve_extents_vs_ifc
# 5. §6, this change against main, and the held-out models
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=ae2078d3895d6b858eaf6332d78292569f1c0c51 -f base=main -f nordic=true -f r2025=true
gh workflow run measure.yml --ref re/810-sidelight-width -f ref=ae2078d3895d6b858eaf6332d78292569f1c0c51 -f base=main -f holdout=true -f models="holdout-*"
```

The Revit export's door `1756089` is also readable without a scorer: its `IfcDoor` line carries `OverallHeight` 1328. and `OverallWidth` 1787.00001 (millimetres) in `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc`.
