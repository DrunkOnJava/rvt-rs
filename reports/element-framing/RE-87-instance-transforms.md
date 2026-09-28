# RE-87 — Family instance transforms

**Date:** 2026-09-28
**Issues:** #227, #358
**Result:**
- A family instance's element data holds its transform as twelve `f64`. The first nine are a 3 x 3 rotation stored row by row, and the last three are the origin in model feet. The rotation's columns are the instance's X, Y and Z axes.

  ```text
  f64 Xx · f64 Yx · f64 Zx · f64 Xy · f64 Yy · f64 Zy · f64 Xz · f64 Yz · f64 Zz · f64 ox · f64 oy · f64 oz
  ```

  It is the first place in the data where nine `f64` are orthonormal with determinant 1.
- On Snowdon Towers, 1,956 elements of Revit's IFC4 export hold one whose Z axis is the model's. On 1,951 of them its X axis is Revit's placement X (1,845), its reverse (57, mostly doors) or a right angle from it (49). The other 5 are walls, which RE-87 does not use. All 49 on RE1 Architecture (Revit 2025) agree.
- A family instance turned off the model's axes is now drawn as the rectangle at its angle whose axis-aligned box is its record box. Before, it was drawn as that box, a median 1.47 ft off Revit's body along the instance's own axes. Its opening turns with it.

| Snowdon Towers' 181 turned family instances, `turned_instances_vs_ifc.py` | main | RE-87 |
|---|---:|---:|
| extents along the instance's own axes within 0.01 ft of Revit's body | 0 | 55 |
| within 0.5 ft | 11 | 98 |
| within 1 ft | 49 | 174 |
| median difference | 1.473 ft | 0.482 ft |
| better / worse than main | | 181 / 0 |

- What is left is the record box's own error, not the turn. On the family instances square to the model's axes, which RE-87 leaves alone, the record box differs from Revit's body by a median 0.80 ft (windows), 0.81 ft (furniture), 0.88 ft (proxies), 1.91 ft (light fixtures) and 2.79 ft (doors).

-----

## 1. The record

`examples/probe_re87_instance_transforms.rs` prints every element whose data holds a transform. `tools/re/instance_transforms_vs_ifc.py` moves each of Revit's placements into Revit's internal frame through the export's site placement, and compares them.

```text
Snowdon Towers (2024):  11,252 elements hold a transform, 8,524 upright
  elements of Revit's export with a transform: 3,944; upright 1,956;
  X axis the same, reversed or a right angle: 1,951
  X axis against Revit's placement X: same 1,845, reversed 57, right angle 49, other 5 (4 walls, 1 curtain wall)
  origin against Revit's placement origin, upright: within 0.001 ft 1,255; within 2 ft 400; further 301
RE1 Architecture (2025): 87 elements hold a transform, 78 upright
  elements of Revit's export with a transform: 58; upright 49; the same, reversed or a right angle: 49
```

- **Reversed.** 49 of the 57 are doors. Revit's export points a door's placement X the other way.
- **Right angle.** 29 of the 49 are walls (their data holds some other rotation) and 10 are coverings.
- **Origins.** On Snowdon the origin is Revit's placement origin to 0.001 ft on 1,255 elements. Doors and windows are not among them: Revit's export moves their placement. RE1's export places every element away from the origin read here.
- The exporter uses only the X axis, so none of this changes what it draws.
- The bytes before the transform are not a fixed tag. On Snowdon, 3,812 of 3,850 occurrences end in `ff ff ff ff 6b 07`, but 0x076b is `FamilyInstancePatternHelper` in the file's schema, not a transform class. The reader relies on the rotation's own shape instead.

## 2. The turned box

A rectangle `w` x `d` turned by `a` has an axis-aligned box `w |cos a| + d |sin a|` wide and `w |sin a| + d |cos a|` deep. With the box and the angle, `w` and `d` follow, and the rectangle is centred on the box.

- Applied to records whose Revit class is `FamilyInstance` (RE-76) and whose Z axis is the model's.
- Not applied to columns, beams, curtain-wall panels and mullions, or anything else a type section, sketch, stair run or line already draws.
- Not applied within 1e-4 rad (0.0057 degrees) of the model's axes. RE1 Architecture holds 34 instances off the axes by less than 0.001 degrees, which is round-off, not a turn. Snowdon's 628 turned upright transforms are all 0.01 degrees or more off.
- Not applied where the angle is within about 3 degrees of 45 degrees, since `w` and `d` cannot be told apart there. RE1's only two turned instances are at exactly 45 degrees and keep their box.
- Not applied where either side would come out non-positive.
- `BodySource` `partition_family_instance_turned_box`. The GLB draws the same boxes: plan extents equal the IFC body's on all 181.

## 3. Openings

A door's or window's opening takes its filler's placement. Measured in the filler's own frame against Revit's opening, for the 27 turned doors and windows with one:

| | main | RE-87 |
|---|---:|---:|
| along the filler, within 0.26 ft of Revit's opening | 0 | 27 |
| across, 4 windows in vertical angled walls | 1.28 to 1.54 ft | 0.00 ft (RE-83's cut now applies) |
| across, 8 windows in tapered "Solar Wall (L5)" walls | 1.52 ft | 0.13 ft |
| across, 12 windows in tapered "Solar Wall (Multi-Level)" walls | 2.17 ft | 0.77 ft |
| across, 3 doors | 5.85 ft | 4.63 ft |

`openings_vs_ifc.py` compares world axis-aligned boxes. A turned rectangle's axis-aligned box is the record box by construction, so that score is unchanged (124 of 194 within 0.26 ft). It cannot see the turn.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re87_instance_transforms -- MODEL.rvt > transforms.jsonl
python3 tools/re/instance_transforms_vs_ifc.py transforms.jsonl REVIT_EXPORT.ifc
./target/ci/rvt-ifc MODEL.rvt -o new.ifc       # this branch
./target/ci/rvt-ifc MODEL.rvt -o old.ifc       # main
python3 tools/re/turned_instances_vs_ifc.py new.ifc old.ifc REVIT_EXPORT.ifc
```

- **Snowdon Towers:** 213 products change, all others identical per placement and shape. They are 181 turned family instances (119 proxies, 31 furniture, 24 windows, 4 doors, 3 light fixtures) and 32 openings: the 27 turned doors' and windows' openings, and 5 turned doors and windows exported as their opening alone (RE-84). `rvt-ifc` takes 11.5 s (11.9 s on main).
- **Snowdon Towers structural:** 5 footings are turned. No Revit export of that file is held to check them against.
- **Core Interior, the four RE1 models, Einhoven and the MIT house (2024 and 2025):** identical to main's.
  - Core Interior, RE1 Electrical and Mechanical, Einhoven and the MIT house are byte-identical apart from the timestamp.
  - RE1 Architecture and Plumbing are identical per placement and shape.

## 5. Open

- The record box is still the extent: a family's own geometry is not read (#255, and RE-78's saved meshes behind an option). The turned box is as good as the record box is along the instance's axes.
- Instances whose Z axis is not the model's (1,988 in Snowdon's export: mullions, wall- and ceiling-hosted fixtures) keep their box.
- A door's box along its depth (4.63 ft residual on its opening) is the record box, which includes more than the door leaf.
