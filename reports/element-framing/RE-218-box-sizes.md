# RE-218: A door's or window's overall size from its record box, where its symbol cuts no loop

**Date:** 2026-10-25
**Issues:** #810 (the BIM4LCA office's 269 windows and 82 doors whose symbols hold no cut loop get no `OverallWidth`/`OverallHeight`). Follows RE-215 (a door's or window's size from the outline its symbol cuts), RE-87 (a family instance's transform), RE-93, RE-94 and RE-187 (sizes from a type's lengths).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |
| `RE1-Architecture.rvt` (Revit 2025) and its Revit IFC | see `research/witness-registry.json` | MIT, Drshelden/IFC-ECS |

Measure fetches all of them and stores none.
**Probes:** `examples/probe_re187_door_window_sizes.rs`, `examples/probe_810_odd_doors.rs`. **Scorers:** `tools/re/window_sizes_list.py`, `tools/re/window_openings_vs_box.py`, `tools/re/door_window_sizes_vs_ifc.py` (first run on the research branch `re/810-window-sizes`).
**Runs:**
- [40160224764](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40160224764): the RE-187 probe on the office: no type length equals Revit's size on any of its 269 windows;
- [40161031071](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40161031071): each window type's Revit size against its type's lengths and its record box;
- [40161793641](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40161793641): the box rule on every door and window without a cut loop, against `main` (13c14e12);
- [40166926324](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40166926324): `examples/probe_810_odd_doors.rs`: the references, transforms and native members of the doors the box rule does not size as Revit;
- [40167653898](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40167653898): this change against `main` (eb369d9f).

**Status:** positive on every window of the office, on 71 more of its doors, and on RE1 Architecture's curtain-wall door. Negative on 10 office doors, whose size is not yet read.

## 1. What was missing

RE-215 sizes a door or window from the outline its symbol cuts in its host. On the BIM4LCA office 187 of the 269 doors cut one; the 269 windows and the other 82 doors are drawn by symbols whose `m_pCutLoops` is empty. rvt-rs wrote `$` for all 269 windows and 71 of those doors, and on 11 doors fell back to the type's Width and Height (RE-187), which on these Finnish families is 10 mm short of Revit's.

## 2. The record box

The RE-187 probe prints each window's type Width and Height (-1001301, -1001300) and its element record's box. On all 269 windows Revit's `OverallWidth` is the box's extent along the window's own X axis, and its `OverallHeight` the box's height; neither type length is:

| windows | type | Revit's OverallWidth × OverallHeight, ft | type Width × Height, ft | record box, ft |
|---:|---|---|---|---|
| 132 | 1948380 | 5.2822 × 8.5140 | 5.2493 × 8.2021 | 5.2822 × 6.3402 × 8.5140 |
| 66 | 3479421 | 5.2822 × 7.8579 | 5.2493 × 7.5459 | 5.2822 × 6.3402 × 7.8579 |
| 36 | 1948380 | 5.2822 × 8.5140 | 5.2493 × 8.2021 | 6.3402 × 5.2822 × 8.5140 |
| 16 | 3479421 | 5.2822 × 7.8579 | 5.2493 × 7.5459 | 6.3402 × 5.2822 × 7.8579 |
| 10 | 2875546 | 3.9698 × 8.6781 | 3.9370 × 8.3661 | 3.9698 × 5.0279 × 8.6781 |
| 4 | 2873853 | 5.2822 × 4.2489 | 5.2493 × 3.9370 | 5.2822 × 6.3402 × 4.2489 |
| 4 | 3445267 | 3.9698 × 7.6893 | 3.9370 × 7.5459 | 3.9698 × 5.1214 × 7.6893 |
| 1 | 3479646 | 3.3136 × 7.8579 | 3.2808 × 7.5459 | 4.3717 × 3.3136 × 7.8579 |

Which horizontal extent counts is the instance's own: the 36 and 16 windows of the third and fourth rows stand in walls along the model's Y axis, where the box's X and Y extents trade places. Among the doors, the extent along X is sometimes the larger of the two (door 1446815: box 4.0354 by 5.9383 ft, Revit's width 5.9383 ft), so it is the instance's axis, not the smaller extent, that decides.

Applied to every door and window with no cut loop, the rule gives Revit's size to 71 more of the office's doors and to RE1's curtain-wall door, and a wrong one to 8 doors (Measure run 40161793641): four doors of type 1107673, 2.2966 ft square in Revit's export with a box 1.6562 ft high, and four of type 3267242, 5.2165 by 5.2665 ft against a box 3.3531 ft high. Revit's height for those is the box's extent along the instance's Y axis, a horizontal one. Their element records name no wall and no curtain wall, where every door the rule sizes as Revit names one (`examples/probe_810_odd_doors.rs`, Measure run 40166926324). The host a door's record names is the signal, not the host rvt-rs binds: a door set in a curtain wall keeps no bound host (#439), so the bound host leaves out 66 of the doors the rule is right on. Floors, roofs and ceilings among the references, and a cut loop flat in the symbol's XY plane, were each tried and change nothing.

## 3. What rvt-rs does

For a door or window whose symbol cuts no loop and whose element record names a wall or a curtain wall, `attach_cut_loop_sizes` reads the instance's transform (RE-87). Where the instance is upright and its plan X axis lies along a model axis to 1e-6, it records the record box's extent along that axis and the box's height (`INSTANCE_BOX_SIZE_FIELDS`, properties `BoxWidth` and `BoxHeight`). The STEP writer takes a door's or window's `OverallWidth` and `OverallHeight` from, in order: the cut loop (RE-215), the opening its type gives (RE-93, RE-94), the record box, and the type's Width and Height (RE-187). An instance turned off the model's axes, whose box is not along its own, gets no box size.

The type's opening (RE-93, RE-94) stays ahead of the box: it is what Snowdon Towers' windows are measured on (56 of 68 sized as Revit), and Snowdon is local only, so it is not re-measured.

## 4. Measured

`door_window_sizes_vs_ifc`, Measure run [40167653898](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40167653898), this change against `main`:

| model | class | equal before | equal after | differ before / after | `$` before / after |
|---|---|---:|---:|---:|---:|
| BIM4LCA office as built | IfcWindow (269) | 0 | 269 | 0 / 0 | 269 / 0 |
| BIM4LCA office as built | IfcDoor (269) | 187 | 258 | 11 / 6 | 71 / 5 |
| BIM4LCA office at building permit | IfcWindow (265) | 0 | 265 | 0 / 0 | 265 / 0 |
| BIM4LCA office at building permit | IfcDoor (269) | 187 | 258 | 9 / 6 | 73 / 5 |
| RE1 Architecture | IfcDoor (5) | 4 | 5 | 0 / 0 | 1 / 0 |

RE1 Architecture's fifth door is its curtain-wall door 445975, whose type cuts no loop (RE-215, #844). Every door and window that was equal stays equal, and none goes from `$` to a wrong size. No other scorer's output changes. Core Interior, Einhoven, the other three RE1 models and the 23 R2025 models are byte-identical: every door and window of theirs cuts a loop but RE1's 445975, and Einhoven (Revit 2023) is outside RE-215's releases.

The 6 office doors that still differ were wrong on `main` too:

- 1756089 and 1765267 (types 1106810, 1106812): 5.9766 by 4.3570 ft against Revit's 5.8629 by 4.3570. Their records name a wall, so the box sizes them; its height is Revit's, and its width 0.1137 ft over Revit's (on `main` both were the type's 3.1168 by 4.2717).
- 3269270, 3270587, 3270669 and 3270703 (type 3267242): their records name no wall, so they keep the type's 5.2165 by 4.4948 ft, against Revit's 5.2165 by 5.2665. §6 sizes them.

The 5 with `$` are the four doors of type 1107673 and one more, which had `$` on `main` too. §6 sizes the four.

## 5. What it does not cover

- A door or window turned off the model's axes, whose record box is not along its own axes.
- A window's opening. Where its symbol cuts no loop, its opening stays what it was. Against Revit's on the office (`tools/re/window_openings_vs_box.py`, Measure run [40163933374](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40163933374)), it is exact along the wall and at the top on all 269. Revit's runs 0.059 ft (18 mm) lower at the bottom, and 0.328 ft (100 mm) past the host wall's band on each side, 2.0833 ft through a 1.4272 ft wall. Those are the families' own void geometry, which is not decoded.

## 6. Addendum (2026-10-25): a door no wall hosts takes its height along its own Y axis

revit-ifc states the rule §2 measured for the eight doors whose records name no wall. `FamilyInstanceExporter.GetDoorWindowDimensionFromSymbol` (CLEANROOM.md source 5) takes a door's or window's width from `ExporterIFCUtils.GetMinSymbolWidth` and, where its host is a `Wall` or a `FaceWall`, its height from `GetMinSymbolHeight`; for any other host, "the height of the non-wall hosted families is measured in Y direction", as the extent along Y of the bounding box of the symbol's geometry. A curtain wall is a `Wall` there, so the doors set in one keep their height through the box.

`attach_cut_loop_sizes` now sizes every door and window whose symbol cuts no loop, upright and along a model axis, by its record box: its width along its own X axis, as before, and its height through the box where its record names a wall or a curtain wall, else along its own Y axis. The office's four roof hatches (type 1107673) stand upright like every other door; their height is a plan extent because their Y axis is horizontal, not because they lie down. A first attempt that took every such door's height along its own Z axis gave the hatches their box's 1.6562 ft and type 3267242 its 3.3531 ft, both wrong (Measure run [40176316442](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40176316442)).

`door_window_sizes_vs_ifc`, Measure run [40177398023](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40177398023), this change against `main` (9afafa7a), default models plus `-f nordic=true -f r2025=true`:

| model | IfcDoor equal before / after | differ before / after | `$` before / after |
|---|---:|---:|---:|
| BIM4LCA office as built | 258 / 266 | 6 / 2 | 5 / 1 |
| BIM4LCA office at building permit | 258 / 266 | 6 / 2 | 5 / 1 |

The four hatches go from `$` to Revit's 2.2966 ft square, and the four doors of type 3267242 from the type's 5.2165 by 4.4948 ft to Revit's 5.2165 by 5.2665. The two that still differ are 1756089 and 1765267, 0.1137 ft wider than Revit's (§4). Every window stays equal, only the two office files' IFC changes, and no other scorer's output does.
