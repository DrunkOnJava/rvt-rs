# RE-103 — A steel beam's type stores its I section

**Date:** 2026-09-28
**Issues:** #94 (beam section profiles); follows RE-49 and RE-93
**Result:**
- A structural framing type stores its section's dimensions under public BuiltInParameters, as `f64 feet · ff×8 · i64 parameter` in its value block (RE-93):

  | parameter | BuiltInParameter |
  |---|---:|
  | Width | −1005502 |
  | Height | −1005503 |
  | web thickness | −1005525 |
  | flange thickness | −1005524 |
  | centroid, from the left | −1005508 |
  | centroid, from the bottom | −1005509 |

- Revit also stores the section's computed centroid, and that tells an I from a channel. A wide flange's centroid sits at its centre both ways. A channel stores the same four dimensions with its centroid off centre across: C8X11.5's is 0.572 in from its edge, against a 2.26 in width. An angle carries its own L-angle parameters.
- A beam whose type is an I section is now drawn as that I, extruded along its location line. The shape the section's own geometry parameter names (−1005501) is not read: its stored form is not measured.
- Placement:
  - Where the beam's body along its line (RE-49) is the section's width and depth, the I fills it.
  - A level beam set down from its line has a record box that runs from the line, on the box's top, to the steel's bottom. There the I sits on the box's bottom. On Snowdon most are set down 2.5 in, some up to 5.2 ft.

| Snowdon Towers structural, 942 structural-framing elements | main | RE-103 |
|---|---:|---:|
| drawn as their type's I section | 0 | 377 |
| of those the VIM can score, Revit's mesh is that I at that place | – | 271 of 271 |

-----

## 1. The type values

`examples/probe_re103_beam_sections.rs` reads the section of every framing type rvt-rs names:

| type | beams | width | height | web | flange | centroid across, up |
|---|---:|---:|---:|---:|---:|---|
| W12X26 | 41 | 6.49 in | 12.2 in | 0.23 in | 0.38 in | 3.245, 6.1 in: centred |
| W18x55 | 212 | 7.53 in | 18.1 in | 0.39 in | 0.63 in | centred |
| W8x10 | 97 | 3.94 in | 7.89 in | 0.17 in | 0.205 in | centred |
| C8X11.5 | 6 | 2.26 in | 8.0 in | 0.22 in | 0.39 in | 0.572, 4.0 in: off centre across |
| `_Size` (an angle) | 25 | 3.0 in | 3.5 in | 0.25 in | 0.25 in | L-angle parameters, not I |

Types W16X26 and W14X30 store W12X26's dimensions (6.4883 × 12.2188 in). They are drawn as stored.

Joists (16K2, 16K6, 20K6, 18LH04) and the concrete beams (CB…) store none of these and keep their box along the line.

## 2. Measured against the VIM

The VIM export of the sample's 2027 edition holds Revit's own mesh for each beam. A beam is scored only where the VIM holds it as the 2024 file does:
- its `Location` is the line's midpoint;
- its family and type are the ones rvt-rs names;
- its mesh is centred across the line where the body is.

For each scored beam, the mesh is projected on the plane square to the line. The probe looks for a vertex within 0.01 ft of each of the section's four outer corners, each of its four inner flange tips, and each web face. A solid box has no vertex at the tips.

| drawn beams | count |
|---|---:|
| the mesh is the I: outer corners, flange tips and web faces | 271 |
| moved in the VIM's edition | 78 |
| not in the VIM | 28 |

On every set-down beam the VIM scores, the mesh's bottom is the box's bottom, and its top sits 0.2083 ft (2.5 in), 0.4167, 2.3333, 4.9583 or 5.1667 ft below the box's top, where the line is.

## 3. The solid

The I is written as an `IfcExtrudedAreaSolid` of an `IfcIShapeProfileDef`, placed at one end of the line:
- the placement's Z axis runs along the line;
- its X axis runs level across it, so the flanges run across and the web stands upright.

An `IfcFixedReferenceSweptAreaSolid` with a level fixed reference would say the same by the IFC 4.3 definition, where the profile's x axis is the reference's projection. IfcOpenShell 0.8.5 lays that I on its side, so the extrusion is used.

`tools/re/beam_sections_in_ifc.py` meshes every I beam with IfcOpenShell 0.8.5. For all 377:
- the volume equals the I's area times the beam's `AxisLength` (worst 3e-10 relative);
- the mesh is the flange width across and the depth upright.

In the GLB, beam 608386 (W18x55) is 0.6275 ft across, 1.5083 ft deep and 17.33 ft long.

## 4. Other files

Only Snowdon Towers structural changes. Core Interior, the four RE1 models, Snowdon Towers architectural, Einhoven, Projeto1, `teste_export_2025`, Exemplo_data, modelo_bim and the MIT house (2024, 2025) are byte-identical to main's apart from the timestamp, so no witness observation changes.

Still open on #94:
- channels (C8X11.5) and angles (`_Size`), whose sections are stored but not drawn;
- `IfcMaterialProfileSet`;
- the 17 I-section beams with no body along their line (W12X26, W18x55, W18x86).

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Structural.rvt (local only) | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` |
| Snowdon.r2027.vim (local only) | `3e7de6364b4592f2281f7453053f7fa7b9f5ac0e703d41ff0cdd5c278eed387e` |

```bash
cargo run --profile ci --example probe_re103_beam_sections -- \
  "Snowdon Towers Sample Structural.rvt" Snowdon.r2027.vim "Snowdon Towers Sample Structural"
./target/ci/rvt-ifc "Snowdon Towers Sample Structural.rvt" -o structural.ifc
python3 tools/re/beam_sections_in_ifc.py structural.ifc
```
