# RE-215: A door's or window's overall size is the outline its symbol cuts

**Date:** 2026-10-24
**Issues:** #844 (Core Interior's six windows and one RE1 door write no `OverallWidth` / `OverallHeight`), #810 (door widths on the BIM4LCA office), #227 (openings cut from the host). Follows RE-93 (a window's opening from its type), RE-94 (a door's rough opening) and RE-187 (a door's type size).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `2024_Core_Interior.rvt` (Revit 2024) | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` | MIT, magnetar-io/revit-test-datasets |
| `2024_Core_Interior_slim.ifc` (Revit's IFC4 export) | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` | MIT |
| `RE1-Architecture.rvt` (Revit 2025) | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` | MIT, Drshelden/IFC-ECS |
| `RE1-Architecture.ifc` (Revit's IFC2X3 export) | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` | MIT |
| R2025 models 12 to 17 and Revit 25.0.2's IFC4 exports of them | pinned in `research/r2025-models.tsv` | Apache-2.0, puzzbobb (#356) |

Measure fetches all of them and stores none.
**Probe:** `examples/probe_re215_symbol_cuts.rs`. **Scorer:** `tools/re/symbol_cuts_vs_ifc.py`.
**Runs:**
- research runs [40012344435](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40012344435) and [40014065012](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40014065012), on branch `re/810-symbol-cuts`, whose commits call this RE-213 (the number went to #935 first);
- the end-to-end run [40114863653](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40114863653), this branch against main.

**Status:** positive on every door and window measured with a cut loop.

## 1. What Revit's exporter takes the size from

revit-ifc (CLEANROOM.md source 5) writes a door's or window's `OverallWidth` and `OverallHeight` from `GetDoorWindowDimensionFromSymbol`. That function is given `ExporterIFCUtils.GetOriginalSymbol(familyInstance)`, which the exporter's comment calls the symbol that "has the right geometry", not the instance's type. It measures the symbol's width along X and, in a wall, its height along Z. #844's run 39187209511 had already found Revit's sizes equal to the extents of the opening each element fills. So the size is the symbol's opening cut.

## 2. The cut is in the symbol's own record

A `FamilySymbol` record names `m_pCutLoops`, `m_cutDirs`, `m_cutIndices`, `m_cutPlaneHeights` and `m_closurePlanes`. On every symbol with a cut in the three models, `m_pCutLoops` points at one `CurveLoop` (`m_open` false). Its `m_curves` are four `GLine`s (`m_origin`, `m_dirVec`, `m_endParams`), joined end to end into a rectangle in the symbol's XZ plane. The rectangle lies at `y` equal to plus or minus half the host wall's thickness: 0.25 ft on a 0.5 ft wall, 0.3333 on a 0.6667 ft wall. For door type 17331 (`I_Single-Flush:3 x 8 2 Hour`), the loop runs from x −1.5 to 1.5 and from z 0 to 8: 3 by 8 ft, Revit's `OverallWidth` and `OverallHeight` for its 50 doors.

## 3. The symbol that draws an instance

Core Interior's windows break the rule if the type is taken. Their type, 17335 (`Single Window:Standard`), cuts 3.3136 ft square (1010 mm), its stored Width and Height (RE-187). Revit writes 4.9213 by 8.8583 ft (1500 by 2700 mm), and so the six windows had `$`.

A family instance names its type in `m_masterSymbolId`. A second object of its graph, `InstanceInfo`, names another symbol in `m_symbolId`. For all six windows (and a seventh instance of the type, which Revit's export leaves out), that is 19275, a symbol of the same family (17270) that is no element's type in Revit's export. Its loop runs from x −2.4606 to 2.4606 and z 3.2808 to 12.1391: 4.9213 by 8.8583 ft. The windows' own `FamilyParams` hold Width and Height (−1001301, −1001300), so the instances size the family themselves, and their symbol carries the geometry that gives. Every door names a symbol of its own too: 19269 to 19274, 132750 and 132875 for types 17331 and 17333, and 24300, 24301, 132824 and 132825 for type 23756. Each has its type's loop extents and differs only in the plane `y` the host wall gives it.

## 4. Measured

Per element, the X and Z extents of the loop of the symbol `InstanceInfo.m_symbolId` names, against Revit's `OverallWidth` and `OverallHeight`, within 0.001 ft (run 40014065012):

| model | class | elements | equal | differ | no loop |
|---|---|---:|---:|---:|---:|
| Core Interior | IfcDoor | 132 | 132 | 0 | 0 |
| Core Interior | IfcWindow | 6 | 6 | 0 | 0 |
| RE1 Architecture | IfcDoor | 5 | 4 | 0 | 1 |
| R2025 12 to 17, each | IfcDoor | 1 | 1 | 0 | 0 |
| R2025 12 to 17, each | IfcWindow | 3 | 3 | 0 | 0 |

The element with no loop is RE1's curtain-wall door 445975 (type 480990, `Door-Curtain-Wall-Double-Glas:69" x 72"`): its symbol's record holds no cut loop. The type's loop is equal too on every door and on the R2025 windows; only Core Interior's windows need the instance's symbol.

## 5. What rvt-rs does

- `native_symbol_cuts::instance_cut_loops` reads, in one native pass, the given family instances and every `FamilySymbol`. For each instance it takes the symbol `InstanceInfo.m_symbolId` names. A symbol counts only if `m_pCutLoops` resolves to one closed `CurveLoop` of `GLine`s, each line's start within 1e-6 ft of the previous line's end, flat in `y` and with some width and height. Anything else is left out.
- `partition_schema_mvp::attach_cut_loop_sizes` gives each door and window on Revit 2024 and 2025 the loop's X and Z extents. A door that is an opening alone (RE-84) is skipped, since it is written as its opening.
- The IFC export reports them as `CutWidth` and `CutHeight` in the element's rvt-rs set, and writes them as `OverallWidth` and `OverallHeight`. They come before RE-93's opening and RE-187's type size, which apply where no loop is read.

End to end against main (run 40114863653), the export changes on Core Interior, RE1 Architecture and R2025 models 12 to 17 only:

- `door_window_sizes_vs_ifc` on Core Interior's windows goes from `equal 0, $ 6` to `equal 6, $ 0`;
- every other scorer output is identical on every model, Einhoven (2023) and the RE1 MEP models included;
- each door and window with a loop gains the two properties, 276 property values on Core Interior, so its two witness observations are regenerated;
- `tests/door_overall_sizes.rs` now asserts the six windows' 2700 by 1500 mm.

## 6. What it does not cover

- **Releases.** Read on 2024 and 2025, the releases measured. Einhoven (2023) has five symbols with loops, but no Revit export of it is in reach, so 2023 is left out, as are 2026 and 2027.
- **Arcs.** A loop with an arc, such as an arched window's, is not read. None of the measured loops has one, and how Revit's `GetMinSymbolHeight` measures one is not known.
- **Snowdon Towers.** Snowdon is local only and was not re-measured. RE-93 sizes 56 of its windows and RE-94 57 of its doors from their type, both equal to Revit's. Where a loop is read it now comes first, and whether it equals those type sizes on Snowdon is **NOT MEASURED**.
- **The BIM4LCA office (#810).** Its doors' `OverallWidth` is 10 mm over their type's Width. The loop may account for that, but the office's host times out, so it is **NOT MEASURED**.
- **The opening's body (#227).** The opening is still cut from the type's sizes or the element's box. Revit places a door's or window's IFC frame at its opening's corner: in that frame the opening runs 0 to `OverallWidth`, 0 to the wall's thickness and 0 to `OverallHeight`. Comparing it with the loop's position needs the instance transform (RE-87) and is the next step.
- **RE1's curtain-wall door** keeps `$` (#844's remaining case).
