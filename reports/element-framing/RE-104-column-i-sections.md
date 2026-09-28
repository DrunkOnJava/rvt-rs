# RE-104 — A steel column's flanges follow its own X axis

**Date:** 2026-09-28
**Issues:** #94; follows RE-87 and RE-103
**Result:**
- A structural column whose type stores an I section (RE-103) runs its flange width along the X axis of its instance transform (RE-87) and its depth along the Y axis, centred on its record box.
- Such a column is now drawn as that I, extruded through its record box's height and turned to its X axis. It is drawn only where the section, turned that way, fills the record box in plan within 0.01 ft.
- The box alone could not decide the turn. W10x49 is 10.0 × 9.98 in, and four columns whose box has the flange width along model Y are turned 90°.
- The type-value readers (RE-77, RE-93, RE-94, RE-103) now take a value block's owner from every ElementId `Global/ElemTable` declares, its secondary ids included (RE-41), not only its primary ids.
  - Before, a type whose block opens with a secondary id lent its values to the block before it. W8x31's block (type 709888) is one.
  - No exported value changed on any local file, but W8x31's section is now read.

| Snowdon Towers structural, 74 structural columns | main | RE-104 |
|---|---:|---:|
| drawn as their type's I section | 0 | 40 |
| of those the VIM can score, Revit's mesh is that I at that turn | – | 24 of 24 (5 turned off the model's axes) |

-----

## 1. Measured against the VIM

`examples/probe_re104_column_sections.rs` scores every column rvt-rs draws against the VIM export of the sample's 2027 edition. A column is scored where the VIM holds it as the 2024 file does:
- the same family and type;
- its `Location` and its mesh centred where the record box is.

In the column's own axes, the probe looks for a mesh vertex within 0.01 ft of each of the section's four outer corners and four inner flange tips.

| type | drawn | the mesh is the I | moved in the VIM's edition |
|---|---:|---:|---:|
| W14x109 | 17 | 7 | 10 |
| W10x49 | 14 | 8 | 6 |
| W8x31 | 7 | 7 (5 turned) | 0 |
| W18x119 | 2 | 2 | 0 |

The other 34 are not I sections and keep their box:
- 20 SC_Reference Column;
- 13 CC24x24 concrete;
- one HSS6x6x5/8 tube.

## 2. The solid and the IFC

The I is an `IfcExtrudedAreaSolid` of an `IfcIShapeProfileDef`, placed at the record box's base centre and turned by the element's placement. The body source is `family_type_i_section`.

`tools/re/beam_sections_in_ifc.py` now checks columns too. With IfcOpenShell 0.8.5, all 40 have:
- a vertex at each outer corner and inner flange tip of the section in their own placement's axes;
- a volume equal to the I's area times their height.

The 377 beams of RE-103 are unchanged.

## 3. Other files

Only Snowdon Towers structural changes. Core Interior, the four RE1 models, Snowdon Towers architectural, Einhoven, Projeto1, `teste_export_2025`, Exemplo_data, modelo_bim and the MIT house (2024, 2025) are byte-identical to RE-103's output apart from the timestamp, so no witness observation changes.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Structural.rvt (local only) | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` |
| Snowdon.r2027.vim (local only) | `3e7de6364b4592f2281f7453053f7fa7b9f5ac0e703d41ff0cdd5c278eed387e` |

```bash
cargo run --profile ci --example probe_re104_column_sections -- \
  "Snowdon Towers Sample Structural.rvt" Snowdon.r2027.vim "Snowdon Towers Sample Structural"
./target/ci/rvt-ifc "Snowdon Towers Sample Structural.rvt" -o structural.ifc
python3 tools/re/beam_sections_in_ifc.py structural.ifc
```
