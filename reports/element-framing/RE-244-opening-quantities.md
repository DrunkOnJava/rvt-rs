# RE-244 — The base quantities of a door's or window's opening

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-215 (the openings' boxes), RE-235 §5 (Revit values left out)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports) and puzzbobb's R2025 `12_Rooms_Door_Windows` (Revit 2025, Revit's IFC4 export), as Measure fetches them
**Runs:** Measure [40312396849](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40312396849) (`tools/re/opening_qtos_list.py` on `main`), [40313328149](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40313328149) and [40314642039](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40314642039) (`tools/re/opening_axes_list.py`), [40316860978](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40316860978) and [40318608193](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40318608193) (`tools/re/opening_depth_list.py`), and [40322965161](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40322965161) (this change with RE-245 against `main` at 8bc87a5b, unfiltered)
**Result:** positive for `Depth`, written from the host wall's `Width`; `Width` and `Height` are a Revit defect, left out.

## 1. Revit's opening quantities

Revit's export gives each opening a door or window cuts `Qto_OpeningElementBaseQuantities` with `Width`, `Height` and `Depth` (the office as built: 187 door openings in walls; at permit 187; R2025 one door and three windows). On every one they are its own body's extents: `Width` the extent of its extruded profile along the profile's X axis, `Height` along its Y axis, `Depth` the extrusion's depth, and `Depth` is the `Qto_WallBaseQuantities.Width` of the wall the opening voids (187 of 187 as built and at permit, 4 of 4 on R2025).

## 2. Width and Height point the wrong way

IFC4 defines a wall opening's `Width` as its horizontal dimension and its `Height` as its vertical one (buildingSMART's `Qto_OpeningElementBaseQuantities`). `opening_axes_list.py` finds the world direction of each profile axis: on all 187 office door openings, and the R2025 door's, Revit's `Width` is the vertical extent (2.1 m on a door 1.21 m wide) and its `Height` the horizontal one; of the three R2025 window openings two are written so and one the other way. revit-ifc reads them from its profile's axes, which do not follow the opening's orientation. By the maintainer's rule (2026-10-25) a value Revit's export writes that is known to be wrong is left out, neither reproduced nor corrected: rvt-rs writes neither, and RE-235 §5 lists them. The census counts them apart.

## 3. What rvt-rs does

An opening a door or window cuts in a wall carries `Qto_OpeningElementBaseQuantities.Depth`, the wall's `Qto_WallBaseQuantities.Width` (its type's thickness, #1025), where rvt-rs writes that `Width` and the symbol that draws the door or window cuts an outline (RE-215). Revit's export draws such an opening as an extrusion of that outline, with its quantities; where the symbol cuts no outline (`opening_depth_list.py`: all 123 window openings of the office as built and 7 of its doors), it writes the opening without quantities. The wall is the one the door's native record names (RE-245): with the reference list's nearest wall, 3 of the office's doors would take a neighbouring wall's width. With `--no-base-quantities` the wall has none, and the opening none either. An opening in a curtain wall, or one nothing fills, gets none.

## 4. Measured

This change with RE-245 against `main` at 8bc87a5b, unfiltered (run 40322965161, `-f nordic=true -f r2025=true`, every scorer); only `qtos_vs_ifc` moves beyond what RE-245 moves:

| model | `Depth` equal | different | missing | quantities Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 184 of 300 | 0 | 116 | 0 |
| office at permit | 159 of 299 | 0 | 140 | 0 |
| R2025 `12_Rooms_Door_Windows` to `17_Rooms_Stairs_Spiral` | 4 of 4 each | 0 | 0 | 0 |

The 116 missing as built are the openings nothing fills (a floor's sketch holes, RE-151) and those of doors whose symbol cuts no outline. The census counts the door openings' `Width` and `Height` apart (191 each as built, 189 at permit, one on each R2025 model); the window openings' (3 on each R2025 model, two of them swapped) stay missing. Core Interior's and RE1 Architecture's exports, made with base quantities off (RE-242), gain 130 and 4 `Depth` values counted apart, and Core Interior's witness observations are refreshed. IfcOpenShell's validation finds nothing on any export.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-* r2025-12_Rooms_Door_Windows" -f scorers="qtos_vs_ifc opening_axes_list"
```
