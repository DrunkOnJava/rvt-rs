# RE-86 — Tapered walls

**Date:** 2026-09-27
**Issues:** #358, #227
**Result:**
- A wall type's data stores three face angles, in radians, after a fixed frame. The frame is `u32 2`, eight zero bytes, an `f64` 0.7 and a count of 3, and it follows a word that varies by document.

  ```text
  02 00 00 00 · 00 × 8 · f64 0.7 · 03 00 00 00 · f64 · f64 · f64
  ```

  On Snowdon Towers all three are 0 on 40 of the 43 wall types Revit's IFC4 export holds. The three "Solar Wall" types store 10 degrees (0.174533 rad) in the middle one.
- The word after a wall's location-line setting (RE-54) is its cross-section: 1 for a vertical wall, 2 for a tapered one. Every Snowdon wall with 2 is a Solar Wall. Revit tapers each of them that it exports, except 1054762, which it trims at the base. The face that leans is the wall's exterior, by exactly the type's middle angle, wider at the base. The interior face is vertical. At the top the body is the type's thickness centred on the wall's line.
- A Solar Wall with 1 (1055653) is vertical in Revit's export, at its type's thickness.
- Each tapered wall is now drawn as that cross-section along its line: an `IfcExtrudedAreaSolid` of a trapezoid, `BodySource` `partition_wall_tapered_section`. Before, each was drawn as its record box, which around a wall at an angle is up to 9.3 ft wider than the wall.

| Snowdon Towers' 20 walls Revit tapers under word 2, `tapered_walls_vs_ifc.py` | main | RE-86 |
|---|---:|---:|
| across the wall, within 0.001 ft of Revit's cut at all five heights | 0 | 19 |
| across, within 0.1 ft | 0 | 20 |
| largest difference across | 9.300 ft | 0.052 ft |
| along the wall, within 0.001 ft at all five heights | 0 | 8 |
| along, within 1 ft | 7 | 20 |
| largest difference along | 2.328 ft | 0.906 ft |

- The ends along the wall are the line's own, trimmed at joins as a vertical wall's are. Revit trims some of these walls further where they meet the walls beside them, and that is the residual along.
- The two walls with 0 in the word (1618833, 1483746) are slanted: Revit leans their whole cross-section 22 degrees. Their angle is in the wall's own data, not its type's, and is not read here. They keep their box.

-----

## 1. The record

`examples/probe_re86_tapered_walls.rs` prints every element whose data holds the frame, with its three angles, and every wall's orientation word. `tools/re/tapered_walls_vs_ifc.py` joins them to Revit's export through its `IfcRelDefinesByType`.

A census of the frame across the element data of five files:

| file | elements storing it | non-zero |
|---|---:|---:|
| Snowdon Towers architectural (2024) | 58 | 4 |
| Snowdon Towers structural (2024) | 15 | 0 |
| Core Interior (2024) | 0 | |
| RE1 Architecture (2025) | 0 | |
| MIT house (2024 and 2025) | 0 | |

- On Snowdon architectural, the 58 include 42 of the 43 wall types Revit exports. The one without the frame is the in-place "Parapet Cap Bandstand".
- The four non-zero are the three Solar Wall types and 703553. 703553 stores the same angles, and no exported wall uses it.
- The frame's bytes also occur by chance in two other elements' data there, followed by subnormal numbers. The reader takes an angle only if it is 0, or between 1e-9 rad and a right angle either way, which leaves those two out.
- The files without the frame have no tapered walls. The 0.7 before the count is the same in every copy, and its meaning is not measured.

The word, on Snowdon Towers architectural's 1,112 walls with an orientation:

| word | walls | Revit's body |
|---:|---:|---|
| 1 | 1,089 | vertical; 23 are more than 0.05 ft wider at the base than at the top, which the word does not mark |
| 2 | 21 | tapered, 20; 1054762's base trimmed by Revit |
| 0 | 2 | slanted 22 degrees (1 of them also wider at the base) |

`tapered_walls_vs_ifc.py` cuts every straight Revit wall at 1 and 99 per cent of its height. A wall is tapered where its base cut is more than 0.05 ft wider than its top cut. It then measures each face's lean between the two cuts:

```text
1037 straight walls in Revit's export; 44 wider at the base
  carrying word 2: 20; walls with word 2 not tapered: 1
  one face leaning by its type's angle, the other vertical: 20
```

Across the wall, measured from the wall's line in the direction the wall's flip gives as its exterior (RE-54), the face that leans is the exterior on all 21 walls with word 2. That includes the one flipped wall, 1054632.

## 2. The body

For a tapered wall of type thickness T, height H (its record box's) and exterior face angle a:

- at the top, the body spans T/2 either side of the line;
- the interior face is vertical;
- the exterior face is H · tan(a) further out at the base than at the top.

In plan the body covers the rectangle from the interior face to the exterior face at the base. That is a vertical wall T + H · tan(a) thick on the line moved H · tan(a) / 2 towards the exterior. It is checked against the record box exactly as RE-54 checks a vertical wall's rectangle. So a wall whose box does not close on it keeps its box, and the joins RE-26 and RE-70 decide set its ends. The solid is the trapezoid cross-section, placed in the frame of the line's direction and extruded along the body's run.

- A wall with word 2 is taken as tapered only where its type's first and third angles are 0 and its middle angle lies between 0 and 45 degrees. That is the one combination measured.
- A tapered wall on an arc is not measured, and keeps its box.
- The reader runs only on Revit 2024. No 2025 file measured stores the frame.
- A tapered wall is drawn whole, not in layers: a vertical layer band would not follow its leaning face. The GLB draws each as one mesh.
- As on main, a tapered wall has no material association in the IFC. Revit's export gives each an `IfcMaterialConstituentSet` of its type's layers (§4).

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re86_tapered_walls -- MODEL.rvt > walls.jsonl
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/tapered_walls_vs_ifc.py walls.jsonl model.ifc REVIT_EXPORT.ifc
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc
```

**Snowdon Towers**
- `tapered_walls_vs_ifc.py` gives the table above.
- On the one wall that is not exact across (1055516), a single cut differs by 0.052 ft.
- `wall_bodies_vs_ifc.py` on the GLB, per wall against main:
  - 24 walls change, and every one is better. They are the 17 tapered walls it scores as angled, plus 7 walls whose ends join a tapered wall's body instead of its box.
  - Angled walls with both faces within 0.001 ft of Revit's: 91 of 94 (75 on main).
  - Angled walls with both ends within 1 ft: 83 (69 on main).
  - The four axis-parallel tapered parapets score the same as on main. Measured in plan, the box already reached Revit's outer faces. Cut by height, they are now right across as well.
- The openings in these walls are unchanged: `openings_vs_ifc.py` gives 124 of 194 within 0.26 ft on main and here. Revit's opening in a tapered wall is not a box through the wall. On 1055806 it is 2.04 ft deep and set towards the exterior (§4).
- `rvt-ifc` takes 11.0 s (11.4 s on main).

**Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven and the MIT house (2024 and 2025):** IFC byte-identical to main's apart from the timestamp.

## 4. Open

- **Slanted walls.** Word 0 marks them (2 on Snowdon), and the slant is in the wall's own data: -0.383972 rad (22 degrees) at 3,024 and 2,820 bytes past the orientation anchor, on 1618833 and 1483746. Two walls are too few to fix its frame.
- **Openings in tapered walls** (#227). The windows these walls host are still drawn as boxes along the model's axes, not turned with their wall, so the cut does not apply to them.
  - Revit's openings are not boxes through the wall. On 1055806, the opening for window 1055807 spans 2.04 ft across, from 0.46 ft inside the line to 1.58 ft outside it, over the window's height.
  - The wall's interior face is 1.02 ft inside the line, so Revit's opening does not reach it. Its shape follows the leaning face and is not read here.
- **Materials.** Revit's export gives each tapered wall an `IfcMaterialConstituentSet`: one constituent per layer of its type, exterior first, named after the layer's material, with a repeated material's second constituent named "(2)". rvt-rs writes no material for them, on main or here. RE-82's constituent sets are one per material in name order, so this needs its own ordering and names.
- **Ends.** A tapered wall's ends are its line's, trimmed as a vertical wall's are. The largest residual is 0.906 ft (1055805, a parapet), where Revit trims further at a join.
