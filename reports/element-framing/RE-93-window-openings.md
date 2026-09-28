# RE-93 — Window openings from their type's Width and Height

**Date:** 2026-09-28
**Issues:** #227
**Result:**
- A family type's value block (RE-77) holds its length parameters as

  ```text
  f64 feet · ff × 8 · i64 parameter
  ```

  The parameter is a BuiltInParameter's negative id (Width −1001301, Height −1001300, Rough Width −1001305, Rough Height −1001304), or the declared ElementId of a family parameter. A family parameter's definition holds its name `u32 n · UTF-16` 0x56 bytes past its id (after `01 00 00 00`), followed by a `u32`-counted `revit.local.family:` id.
- Revit's IFC4 export cuts each window's opening:
  - the type's Width wide, centred on the window's origin (RE-87) along its X axis;
  - the type's Height high, starting 3.0 ft above the origin.

  On Snowdon Towers this holds for all 66 windows whose type stores Width and Height. rvt-rs cut each window's record box, which takes in the window's trim: 0.21 to 0.23 ft wider on each side, 0.26 ft lower and 0.19 ft higher.
- The 3.0 ft is the type's Default Sill Height on every type that stores one. A window whose type stores Width, Height and Default Sill Height now takes that opening. Revit also writes the type's Width and Height as the window's `OverallWidth` and `OverallHeight`, and rvt-rs now does too.

| Snowdon Towers' 68 windows against Revit's openings (`opening_boxes_vs_ifc.py`) | main | RE-93 |
|---|---:|---:|
| within 0.001 ft | 0 | 56 |
| within 0.26 ft | 2 | 2 |
| within 1.0 ft | 65 | 9 |
| over 1 ft | 1 | 1 |
| `OverallWidth` and `OverallHeight` equal to Revit's | 0 | 56 |

-----

## 1. The values

Each window type's value block on Snowdon Towers (Revit 2024) holds its Width and Height once, under their BuiltInParameters. Revit's opening is exactly that size, including where it is not the nominal size in the type's name: "50" x 98"" stores 4.125 × 8.125 ft, and Revit's opening is 4.125 × 8.125, not 8.167. The families also carry their own "Opening Width" and "Opening Height" parameters with the same values. The louver family (22" x 24") stores neither, and its opening is 1.833 × 2.0.

A block holding two values for one parameter gets neither (fail closed), as RE-77 does for text.

## 2. The sill

Every Snowdon window's opening starts 3.0 ft above its origin, and the origin is not at the window's level. The type's Default Sill Height is a family parameter, found by its name. It is 3.0 on all three types that store it: 39" x 76" (Solar Wall), 50" x 60" and 50" x 80".

The Double-Hung family's types store no Default Sill Height, yet their windows' openings also start 3.0 ft above the origin. So the value Revit uses is the instance's own sill height (INSTANCE_SILL_HEIGHT_PARAM, −1001361), which is not stored as a value entry in the instance's data and is not read. On every window here the type's default equals it. The guard below catches a window whose sill was moved off its type's default.

**Guard.** The opening is used only where the window's record box, its real geometry, holds it with at most 0.5 ft to spare at each side, top and bottom. A window whose sill was moved would have its box moved by the same amount, and would keep its box opening. No Snowdon window has a moved sill, so the guard declines none there, and its effect is not measured.

## 3. What this does not do

- Double-Hung windows (10), whose types store no Default Sill Height, keep their box opening.
- The louvers (2), whose family stores no Width or Height, keep their box opening.
- Doors are unchanged. Their openings are centred on the origin and start at its height. They are the type's Rough Width × Rough Height on 57 of 126 doors, and Width + 0.208 ft by Height + 0.104 ft on 66. Instances of one type fall both ways, and what decides it is not read.
- The one window oracle besides Snowdon on Revit 2024 or 2025 (`teste_export_2025`, one window) is unchanged; no type there is read.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re93_window_openings -- "Snowdon Towers Sample Architectural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/opening_boxes_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc"
```

`opening_boxes_vs_ifc.py` moves both exports' openings into Revit's internal feet through each file's own site placement, and compares their boxes per filling element's Tag.

- **Snowdon Towers:** only 56 `IfcOpeningElement` bodies change. The same 56 windows gain their opening's properties and `OverallWidth` / `OverallHeight`.
- **Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp.
