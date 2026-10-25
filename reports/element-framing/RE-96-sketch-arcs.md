# RE-96 — Arc sketch lines

**Date:** 2026-09-28
**Issues:** #31 (slab profiles, closed); follows RE-50 and RE-75
**Result:**
- A sketch line's curve can be an arc. Its first curve record (`04 00 08 01`) then holds RE-75's arc layout: start and end angle, a unit X and Y axis, a radius and a centre. It is preceded by the same bytes as a straight line's record, not the `u64` 1 that marks a curved wall's arc. Read as a line, as RE-50 read it, the record gives ends outside the sketch line's own box, so these sketches did not close.
- A sketch line now counts as an arc where its record reads as one, the arc lies in a horizontal plane, and every point of it (drawn as chords within 0.0005 ft, as RE-75 draws a curved wall) lies in the line's record box. A record that also holds as a line in the box is ambiguous and closes nothing.
- An outline with an arc is kept only where its plan extent is its element's record box, within 0.001 ft. On Snowdon Towers one arc sketch (slab 1424071) closes into an outline 9 ft wider than the slab; it is left out.

| Snowdon Towers' 199 slabs against Revit's IFC4 (`plan_profiles_vs_ifc.py`) | main | RE-96 |
|---|---:|---:|
| with an outline | 132 | 149 |
| upward-face plan area within 0.1% of Revit's | 73 | 87 |

The 17 slabs that gain an outline:

| slabs | against Revit's upward-face area |
|---|---|
| 14 | within 0.1% (13 within 0.02%; 1405438 and 1406264 0.06%, where IfcOpenShell draws Revit's arcs with fewer chords) |
| 2 (1423969, 2060617) | half of Revit's: Revit writes each as two solids, and the area counts both |
| 1 (1406326) | 0.8% larger: Revit writes it as a tessellated face set |

Before, each was its record box, up to 14.7 times Revit's area.

-----

## 1. The record

Sketch line 2060015 (slab 2060014) holds two curve records, at +0x1f1 and +0x2a5 into its data. The first reads as neither a line nor an arc. The second reads as an arc:
- angles −1.8012 to −1.6158 rad;
- X (1, 0, 0) and Y (0, −1, 0);
- radius 50 ft, centre (−8.333, −59.471, 52.667).

Its ends are (−19.75, −10.79) and the end of the next straight line. Read as a line, the same bytes give "ends" at x = 1.0, the X axis's first component, which is what RE-50 read. The 40 bytes before the tag are the same on the arc and on a straight sketch line of the same slab, so nothing marks the curve's kind. A record read as an arc needs unit, orthogonal axes and a positive radius, which a line's origin and direction almost never form. The record box decides between the two readings.

## 2. Other files

`examples/probe_re96_sketch_arcs.rs` counts sketch-line owners with an arc in its box: Snowdon Towers 53 of 532, Snowdon Towers structural 4 of 113, Core Interior and the MIT house none.
- **Snowdon Towers structural:** one slab (935216) gains an arc outline spanning its record box. No Revit export of it is held.
- Outlines of straight lines alone are unchanged: the box check applies only to outlines with an arc. On Snowdon Towers structural, 8 such slabs' outlines stop 0.3 to 0.75 ft inside their record boxes, where the slab edge may reach, and they stay as they were.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re96_sketch_arcs -- "Snowdon Towers Sample Architectural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/plan_profiles_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc" --class IfcSlab
```

- **Snowdon Towers:** only the 17 `IfcSlab` bodies above change.
- **Snowdon Towers structural:** only slab 935216 changes.
- **Core Interior, the four RE1 models, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp. No witness observation changes.

## 4. Addendum (2026-10-25): a quarter arc closed by its chord (#824)

A quarter arc's ends are two corners of its sketch line's record box. RE-25's box solve, which runs before the lines' own curves are read, takes that diagonal as a straight edge, and on the BIM4LCA office it closes roof 1461758's sketch with its two corner arcs drawn as their chords: 10 vertices and 2,760.07 ft², against Revit's 2,812.60 ft². The 52.53 ft² missing are the two circular segments of radius 9.596457 ft, π r²/4 − r²/2 each. Read from its curves, as `probe_824_slab_arcs` lists them on the research branch `re/824-slab-arcs`, each of the two lines is an arc whose every point lies in its record box and whose line reading does not, with Revit's centre and radius.

`sketch_plan_profiles` now re-reads, from its lines' curves, any element whose box-solved outline uses a line whose record box is not flat in plan and whose curve reads as an arc and not as a line. That element then goes through the exact path, where an outline with an arc must still span its record box.

Measure run [40170355982](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40170355982), this change against `main` (eb369d9f), default models plus `-f nordic=true -f r2025=true`: `plan_profiles_vs_ifc` slabs with Revit's outline within 0.01 ft go from 60 to 61 of 63 as built and from 58 to 59 at building permit, and with Revit's area within 0.1% from 60 to 61 and 59 to 60. Only the two office files' IFC changes, and no other scorer's output. Snowdon Towers is not re-measured (local only).

