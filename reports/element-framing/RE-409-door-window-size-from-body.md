# RE-409: a door's saved mesh gives the height Revit writes on 65 doors of the timber office at permit and takes it from one door of RE1 Architecture, so nothing ships

**Date:** 2026-11-10
**Issues:** #1636 (the second-project measurement), #1016 (quantities), #1193 (property values without a rule); follows RE-93, RE-94, RE-187, RE-215 and RE-373 (the sizes of a door or window) and RE-402 (the first reading of those rules on the BIM4LCA housing and timber office)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), and rvt-rs's export of it by `main` at `237d6ef6`, which Measure [42906770416](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906770416) kept (`keep_ifc`); the other BIM4LCA files, the Revit 2025 models, Core Interior, RE1 (Revit's IFC2X3 export of its architectural model) and the BIMcollab and rac_advanced pairs through Measure's scorer
**Scorer:** `tools/re/door_window_mesh_extents_vs_ifc.py`
**Runs:** Measure 42906770416 for section 2, [42913823748](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42913823748) (the scorer over every reference model, `main` at `5dd1277d`) for section 3, [42915659430](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42915659430) (the first form of the change) for section 4, [42925615659](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42925615659) (the scorer with each door's host class) for section 5, and [42928930010](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42928930010) (the narrowed rule, head `3e4914821e056ccfb4cece5718f9162a150c559d` against `main` at `6e58c27cf4ad9a821fe71378a38e9a65adc94f9b`, kept under `refs/archive/2026-11-10/cited/3e4914821e05`) for section 7
**Status:** Negative for shipping, positive as a measurement. The height of a door's saved mesh is Revit's `OverallHeight` on the 65 doors of the timber office at permit whose written height is 15 or 45 mm over it, and a rule that takes it moves those 65 to equal. The same rule moves the one door of RE1 Architecture's export it reaches from equal to 1.9 mm short, and nothing in the data separates the two cases but a threshold fitted to them. No rule ships; the scorer and the measurement stay.

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

The 27 windows are the housing's (13, 7 and 7 on the concrete as built, the concrete at permit and the timber at permit; the timber as built has none, 11 of its windows differ in both). The 45 doors are the timber office's at permit (the 65 that differ, of which 20 differ in both). The 10 windows and 277 doors whose written size is Revit's and whose body differs are Core Interior's 6 windows and 132 doors, RE1 Architecture's 5 doors, the office as built's 4 windows, the office's doors (30 on each of three states and 10 on the fourth) and 10 doors of each housing file: sizes RE-215 reads from the outline the symbol cuts, which the body is not.

## 4. The first form, and why it is not the change

The first form of the change took the mesh's extents along X and Z as a door's or window's `OverallWidth` and `OverallHeight` wherever the size did not come from a cut loop. Its unfiltered Measure (42915659430, `main` at `5dd1277d`) moved 13 of the 33 door and window census lines, `equal / different / `$`` of each, base to head:

| file | doors | windows |
|---|---|---|
| housing, concrete as built | 65 / 0 / 9 to 59 / 6 / 9 | 47 / 17 / 0 to 60 / 4 / 0 |
| housing, concrete at permit | 65 / 0 / 9 to 59 / 6 / 9 | 54 / 11 / 0 to 61 / 4 / 0 |
| housing, timber as built | 67 / 0 / 9 to 61 / 6 / 9 | |
| housing, timber at permit | 67 / 0 / 9 to 61 / 6 / 9 | 54 / 11 / 0 to 61 / 4 / 0 |
| office as built | 268 / 0 / 1 to 238 / 30 / 1 | 269 / 0 / 0 to 265 / 4 / 0 |
| office at permit | 268 / 0 / 1 to 238 / 30 / 1 | |
| timber office as built | 265 / 0 / 1 to 235 / 30 / 1 | |
| timber office at permit | 200 / 65 / 1 to 235 / 30 / 1 | |
| RE1 Architecture | 5 / 0 / 0 to 4 / 1 / 0 | |

It mended 45 doors of the timber office at permit and 27 housing windows (13, 7 and 7), and broke 30 doors on each of the other three office files, 10 on the timber office at permit, 6 on each housing file and 1 of RE1's, and 4 windows of the office as built. The doors it broke are those whose written size is Revit's, which the opening, the record box or the type gave, and whose mesh differs from it: in width (22 of the 30 on the office as built have Revit's height and a wider or narrower body), or in height (the 8 that are 1.64 or 4.438 ft short). A mesh's X is not always a door's width, so no width and no window is taken from a mesh in the rule below.

## 5. What separates the doors that need it

The 65 doors of the timber office at permit whose written size differs from Revit's (section 3) are hosted in no wall in either export: `door_window_mesh_extents_vs_ifc` counts each by the class of the element it fills an opening in, which is `-` in rvt-rs's export and in Revit's for all 65, and for 8 of the 10 doors there whose written size is right and whose body differs (the other 2 are in walls). So no host separates them. The height of their mesh does. The scorer lists up to 40 doors of each cell:

| doors, timber office at permit | listed | written height less Revit's | mesh height less Revit's |
|---|---:|---|---|
| size written differs, body equal (45) | 40 | +0.049 ft on 38, +0.148 ft on 2 | 0 on all |
| size written differs, body differs (20) | 20 | +0.148 ft on 13, +0.049 ft on 7 | 0 on all |
| size written equal, body differs (10) | 10 | 0 | -1.64 ft on 4, -4.438 ft on 4, 0 on 2 |

The mesh height is Revit's `OverallHeight` on every door listed (60 of the 65), and the width of the same doors is Revit's already for the 45 whose body is equal. Of the 30 doors whose written size is Revit's and whose body differs on the office as built, 22 have the mesh height equal to it as well (their width differs), and 4 and 4 are the 1.64 and 4.438 ft short ones. The doors that a height rule would move wrongly are those whose written height is Revit's, that have no cut loop and whose mesh height is near but not equal to it; `door_window_mesh_extents_vs_ifc` counts them (`size written equal, mesh height within 0.2 ft of Revit's`). On the housing as built there is none (59 equal, 6 more than 0.2 ft off).

## 6. The rule that was tried

`saved_meshes::add_door_heights`, in the pass that draws a body from the mesh Revit saved (commit `3e491482`): a door drawn from a whole mesh whose size was not read from the outline its symbol cuts (`FillerSize::cut_loop`, RE-215) takes the height of the mesh (its extent along Z) as its `OverallHeight` where that is within 0.2 ft of the height it has. Its width, a window's size and a door with no size are as before. The 0.2 ft is the gap in one file's data (section 5): the 65 doors are 0.148 ft or less off and the 8 whose height is right are 1.64 ft or more off. It is not a reach that means anything about a door.

## 7. Measured

Unfiltered Measure 42928930010, every flag (`samples`, `nordic`, `r2025`, `holdout`, `bimcollab`, `racadv`), 126 models on each side: 116 exports are byte-identical and 10 move.

| model | doors `OverallWidth` and `OverallHeight` equal / different / `$`, before to after | windows |
|---|---|---|
| timber office at permit | 200 / 65 / 1 to 265 / 0 / 1 | 265 / 0 / 0, unchanged |
| RE1 Architecture | 5 / 0 / 0 to 4 / 1 / 0 | none |
| 8 Autodesk samples (`rac_basic` and `rac_advanced`, 2024 to 2027) | no Revit export scores them | |

- **The timber office at permit** is the one file the rule was written for: 65 doors become equal. Every other scorer output of the file is unchanged, the `Qto_DoorBaseQuantities` and `Qto_WindowBaseQuantities` census lines included, which do not use `OverallHeight`.
- **RE1 Architecture** has one door the rule reaches: 445975, Revit 6.0101 x 7.6767 ft. rvt-rs wrote 7.6767 ft, Revit's, and writes 7.6706 ft after, the height of the mesh, 0.0061 ft (1.9 mm) lower. The mesh is not Revit's height there, and nothing in the file says it is not.
- **No housing file moves.** The 59 doors of the concrete as built whose mesh height equals the written one are unchanged and the 6 whose mesh is more than 0.2 ft off are outside the reach, so the housing, the second design, does not exercise the rule at all. Core Interior, the held-out models, BIMcollab's `bimcollab-str-2015` and the six `racadv` models of Revit's IFC2X3 exports, and the 24 Revit 2025 models are byte-identical.

## 8. Why it does not ship

- Of the 126 models, two with a Revit export exercise the rule, one of each outcome: 65 doors mended on the timber office at permit, and 1 broken on RE1 Architecture, Revit's export of a Revit 2025 model. No other model with a Revit export reaches it.
- What separates them is the gap between the written height and the mesh's: 0.049 and 0.148 ft on the 65 doors, 0.0061 ft on RE1's. A threshold anywhere between 0.0061 and 0.049 ft would keep RE1's door and take the 65, and it would be fitted to these two cases: the 15 and 45 mm are one file's, and nothing in the bytes says why Revit's value there is the mesh's and on RE1's is not.
- RE1's export is IFC2X3, which the standing rule counts apart: an IFC2X3 export never removes a value. Here it removes nothing: it is the only reading of a rule that rests on one file's 65 doors in a second project, and it reads against it.

A rule that is right on most elements and wrong on some is narrowed until it is never wrong (the 2026-10-31 standing rule), and this one cannot be, so it is not written. The 65 doors stay as RE-402 left them, 15 or 45 mm over Revit's.

## 9. Not shown

- **A cause for the 15 and 45 mm.** The 65 doors are hosted in no wall in either export, their mesh height is Revit's and their written height is the record box's or the type's, 15 or 45 mm higher; why that happens at building permit and not as built is not read. A cause would give a rule with a reason where this one has a threshold.
- **Windows.** The mesh gives 13 of the housing's windows their size and breaks 4 of the office's; nothing in the bytes tells them apart, so no window is changed.
- **Doors with no size, and widths.** The 28 doors whose body is Revit's and the 12 whose body is not stay `$`, and a door's width stays what it was.
- **A mesh with parts.** A door whose saved mesh holds a threshold, a swing arc or hardware higher or lower than its leaf would read taller than its opening; RE1's 1.9 mm is the one such reading in the models measured.

## 10. Reproduce

The run of section 7:

```bash
gh workflow run measure.yml --ref research/door-window-mesh-extents -f ref=3e4914821e056ccfb4cece5718f9162a150c559d -f base=6e58c27cf4ad9a821fe71378a38e9a65adc94f9b \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
```

The scorer, over the housing's concrete as built and the timber office at permit, once it is on `main` (the code is the one of section 2, `main` at `237d6ef6`):

```bash
gh workflow run measure.yml --ref main -f ref=237d6ef6b5f7eac67c4c08e70533417284d4b427 -f base=none -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-office-timber-permit" -f scorers=door_window_mesh_extents_vs_ifc
```
