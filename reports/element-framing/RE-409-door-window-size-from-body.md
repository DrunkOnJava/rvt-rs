# RE-409: a door's or window's `OverallWidth` and `OverallHeight` from the extents of its saved mesh where no cut loop is read

**Date:** 2026-11-10
**Issues:** #1636 (the second-project measurement), #1016 (quantities), #1193 (property values without a rule); follows RE-93, RE-94, RE-187, RE-215 and RE-373 (the sizes of a door or window) and RE-402 (the first reading of those rules on the BIM4LCA housing and timber office)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), and rvt-rs's export of it by `main` at `237d6ef6`, which Measure [42906770416](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906770416) kept (`keep_ifc`); the other BIM4LCA files, the Revit 2025 models, Core Interior, RE1 and the BIMcollab and rac_advanced pairs through Measure's scorer
**Scorer:** `tools/re/door_window_mesh_extents_vs_ifc.py`
**Runs:** Measure 42906770416 for section 2, [42913823748](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42913823748) (the scorer over every reference model, `main` at `5dd1277d`, scorer commit `4c70a701`) for section 3, and the unfiltered run of the rule for section 5
**Status:** In progress: sections 2 and 3 are measured; the rule is written and its unfiltered Measure is running (section 5).

## 1. What RE-402 left open

`door_window_sizes_vs_ifc` reads 17, 11, 11 and 11 windows of the four large housing files whose `OverallWidth` or `OverallHeight` differs from Revit's, 9 doors of the concrete as built that rvt-rs writes none for, and, on the timber office at permit, 65 doors whose height is 15 or 45 mm over Revit's (RE-402 section 3).

## 2. What the concrete as built shows

Revit's export holds 74 doors and 64 windows that rvt-rs's export holds under the same `Tag` and gives a size. `door_window_mesh_extents_vs_ifc` reads, for each, the size rvt-rs writes against Revit's and the extent of the body rvt-rs draws it with, along its own X (the width) and Z (the height), against Revit's:

| class | size written | body extents | elements |
|---|---|---|---:|
| window | equal | equal | 47 |
| window | differs | equal | 13 |
| window | differs | differs | 4 |
| door | equal | equal | 55 |
| door | equal | differs | 10 |
| door | `$` | equal | 7 |
| door | `$` | differs | 2 |

The body Revit draws a window with has the extents of Revit's own `OverallWidth` and `OverallHeight` on 60 of its 64 windows and on 62 of its 74 doors. revit-ifc sizes a door or window from `GetDoorWindowDimensionFromSymbol` in `FamilyInstanceExporter.cs`, which asks `ExporterIFCUtils.GetMinSymbolWidth` and `GetMinSymbolHeight`, a call into Revit; RE-215 found the outline the symbol cuts to be what it measures where one is read.

The 13 windows whose written size differs and whose body is Revit's are 13 instances of five types: rvt-rs wrote the record box's height (6 windows of `IKK-4.6x6`, 0.7857 m for Revit's 0.6857 m) or the type's `Width` and `Height` (`IKK-10.14x18`, `IKK-7.10x10` and others), which Revit's export writes in `Qto_WindowBaseQuantities` and not in `OverallWidth` and `OverallHeight`. The 4 whose body differs too are 30 mm wider and 15 mm lower in their body than in Revit's size (tags 1553512, 1670074, 1670341 and 1670408). Of the doors, the 10 whose written size is Revit's and whose body differs are written from an outline that is not the body; the 2 that stay `$` with a body that differs are turned in plan (Revit's 1.2728 m is the diagonal of their 0.9 m).

## 3. Every reference model

The scorer over the 16 reference models whose Revit export holds a door or a window (run 42913823748):

| class | size written equal, body equal | written equal, body differs | written differs, body equal | written differs, body differs | `$`, body equal | `$`, body differs |
|---|---:|---:|---:|---:|---:|---:|
| windows (1,346) | 1,286 | 10 | 27 | 23 | | |
| doors (1,513) | 1,131 | 277 | 45 | 20 | 28 | 12 |

The 27 windows are the housing's (13, 7 and 7 on the concrete as built, the concrete at permit and the timber at permit; the timber as built has none, 11 of its windows differ in both). The 45 doors are the timber office's at permit (the 65 that differ, of which 20 differ in both). The 10 windows and 277 doors whose written size is Revit's and whose body differs are Core Interior's 6 windows and 132 doors, RE1 Architecture's 5 doors, the office as built's 4 windows, the office's doors (30 on each of three states and 10 on the fourth) and 10 doors of each housing file: sizes RE-215 reads from the outline the symbol cuts, which the body is not. The 12 doors with no size written and a body that differs are the housing's turned doors (2 on each file) and 1 door on each office state, one of which (1746275, at the timber office at permit) Revit sizes 0.0 by 0.0 ft.

## 4. The change

`saved_meshes::add_filler_sizes`, in the pass that draws a body from the mesh Revit saved: each door and window drawn from a whole mesh whose `OverallWidth` and `OverallHeight` were not read from the outline its symbol cuts (`FillerSize::cut_loop`, RE-215) takes the mesh's extents along its own X and Z. A size that came from a cut loop stays; an element with no size is given none (the 12 doors above would read wrong).

## 5. Measured

The unfiltered run of this change against `main` (every Measure model) is dispatched from `main`; its readings go here when it finishes.

## 6. Not shown

- **Windows 30 mm and 15 mm short in the body.** The 23 windows whose body differs from Revit's size too (4 on each of three housing files and 11 on the timber as built) are written wrong before and after.
- **Doors with no size.** The 28 whose body is Revit's and the 12 whose body is not stay `$`; nothing reads which a body gives.
