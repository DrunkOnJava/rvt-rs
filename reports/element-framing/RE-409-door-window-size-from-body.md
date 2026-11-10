# RE-409: a door's or window's `OverallWidth` and `OverallHeight` where no cut loop is read

**Date:** 2026-11-10
**Issues:** #1636 (the second-project measurement), #1016 (quantities), #1193 (property values without a rule); follows RE-93, RE-94, RE-187, RE-215 and RE-373 (the sizes of a door or window) and RE-402 (the first reading of those rules on the BIM4LCA housing and timber office)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), and rvt-rs's export of it by `main` at `237d6ef6`, which Measure [42906770416](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906770416) kept (`keep_ifc`); the other BIM4LCA files, the Revit 2025 models and the samples through Measure's scorer
**Scorer:** `tools/re/door_window_mesh_extents_vs_ifc.py`
**Status:** In progress: the one pair that was kept is read; the scorer's run over the reference models (Measure 42913823748) has not read, and no rule is written yet.

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

## 3. Not yet

No rule is written. The reading over the other models decides whether the body's extents replace the size where no cut loop is read, for windows only or for doors too.
