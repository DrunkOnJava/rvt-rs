# RE-213: Revit 2025 frames a wall type's face angles with four more zero bytes

**Date:** 2026-10-24
**Issues:** #358, #227; follows RE-86 (tapered walls, Revit 2024) and RE-89 (window openings in tapered walls).
**Artefacts:** Autodesk's Snowdon Towers Sample Architectural as saved by Revit 2025 (local only, not redistributable: sha256 `8ac90a85bc3e2f3677b9978850bc90c50f79e488ab58b54e6da0efd53150d334`) and Revit 2025's own IFC4X3 export of it (Revit 25.4, "Export Revit property sets" on; local only, never committed: sha256 `18df1f114cf9921959567bba2668636a8c8165d1280b251aa3dbd9d7ce8df466`).
**Probe:** `examples/probe_re213_wall_face_angles_2025.rs`; `examples/probe_re86_tapered_walls.rs` for the walls' orientation words.
**Scorers:** `tools/re/tapered_walls_vs_ifc.py` and `tools/re/wall_bodies_vs_ifc.py`. Both now mesh a wall's `Body` representation by name, because Revit 2025's IFC4X3 export lists the `Axis` first.
**Status:** positive. Each of the 20 walls Revit tapers on the 2025 file is now drawn tapered, as RE-86 draws them on 2024.

## 1. The frame

RE-86 reads a wall type's three face angles after `u32 2`, eight zero bytes, an `f64` 0.7 and a count of 3. RE-86 found no 2025 file that stored the frame, so the reader ran only on Revit 2024. Snowdon Towers saved by Revit 2025 stores it with twelve zero bytes after the `u32 2`, not eight:

```text
2024: 02 00 00 00 · 00 × 8  · f64 0.7 · 03 00 00 00 · f64 · f64 · f64
2025: 02 00 00 00 · 00 × 12 · f64 0.7 · 03 00 00 00 · f64 · f64 · f64
```

`probe_re213_wall_face_angles_2025` looks for both frames in every element's data, from its element data header to the next:

| Snowdon Towers architectural (Revit 2025) | elements |
|---|---:|
| 2024 frame | 0 |
| 2025 frame | 58 |
| 2025 frame, angles [0, 10 degrees, 0] | 4: the three "Solar Wall" types 1345241 (Multi-Level), 1686275 (Parapet), 1686415 (L5), and 703553 |
| 2025 frame, angles [0, 0, 0] | 54 |

- Each of the 58 frames is in a different element's data, once, 516 to 997 bytes past its header.
- This is the census RE-86 measured on 2024: 58 elements store the frame, four of them non-zero, the same four.
- 10 degrees is stored as 0.17453292519943278 rad, as on 2024.

The orientation word (RE-54, RE-86) is read on 2025 already. `probe_re86_tapered_walls` gives 21 of the file's 1,111 walls word 2, 9 word 0 and 1,081 word 1, as on 2024.

## 2. The change

`partition_compound_structure::wall_face_angles_frame` gives the frame for the release (`WALL_FACE_ANGLES_FRAME` on 2024, `WALL_FACE_ANGLES_FRAME_2025` on 2025), and `wall_type_face_angles_after` reads the angles after it. `WALL_FACE_ANGLES_SUPPORTED_REVIT_VERSIONS` is now 2024 and 2025. Nothing else changes: RE-86's rule for a tapered wall, its body, and RE-89's window cut apply as they did on 2024.

2026 and 2027 are not admitted, as no file of either release with a tapered wall was measured.

## 3. End-to-end measurement

```bash
cargo run --profile ci --example probe_re86_tapered_walls -- MODEL.rvt > walls.jsonl
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/tapered_walls_vs_ifc.py walls.jsonl model.ifc REVIT_EXPORT.ifc
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc --json walls.json
```

**Snowdon Towers architectural (2025)**

`tapered_walls_vs_ifc.py` finds 44 of Revit's 1,037 straight walls wider at the base. 20 of them carry word 2, and on each, one face leans by its type's angle and the other is vertical. The 21st wall with word 2 is 1054762, which Revit trims at its base (1.04 ft across at 1 per cent of its height, 1.56 ft at 99), as on 2024. rvt-rs draws it tapered, with the other 20.

| the 20 walls Revit tapers under word 2 | main | RE-213 |
|---|---:|---:|
| `BodySource` | `partition_element_record_join_trimmed` (20) | `partition_wall_tapered_section` (20) |
| across the wall, within 0.001 ft of Revit's cut at all five heights | 0 | 19 |
| across, within 0.1 ft | 0 | 20 |
| largest difference across | 9.300 ft | 0.052 ft |
| along the wall, within 0.001 ft at all five heights | 1 | 11 |
| along, within 1 ft | 4 | 20 |
| largest difference along | 2.328 ft | 0.609 ft |

- On 2024, RE-86 measured 19, 20 and 0.052 ft across, and 8, 20 and 0.906 ft along.
- The wall that is not exact across is 1055516 again, 0.052 ft off on one cut.

`wall_bodies_vs_ifc.py` on the GLB, per wall against main:
- 24 walls change, and every one is better: 17 tapered walls, and 7 vertical walls whose ends join a tapered wall's body instead of its box (six of the type `Exterior - 12 5/8" Rainscreen w Insultation on Metal Stud` and the partition 1316989). RE-86 measured the same split on 2024.
- The other tapered walls are axis-parallel parapets, whose box already reached Revit's faces in plan. They score the same.

| 102 angled walls | main | RE-213 |
|---|---:|---:|
| both faces within 0.001 ft | 75 | 91 |
| both faces within 1 ft | 83 | 100 |
| both ends within 0.1 ft | 35 | 46 |
| both ends within 1 ft | 68 | 91 |
| largest face difference | 6.132 ft | 3.005 ft |

The 976 axis-parallel walls score the same.

**Window openings (RE-89).** The 20 windows hosted by Solar Walls are now cut as on 2024: a vertical box the host type's thickness deep, 2.039 ft in the Multi-Level walls and 1.794 ft in the L5 walls. On main each was 2.02 to 2.94 ft deep. No other opening changes.

**Time.** `rvt-ifc` takes 21.5 s on this file, 21.4 s on main (release build, median of three runs).

**Other files.** The IFC is byte-identical to main's, apart from the `FILE_NAME` and `IFCOWNERHISTORY` timestamps, on:
- the other six Snowdon Towers models (2025);
- rac_basic_sample_project (2025);
- both Golden Nugget models and the three サンプル models (2025);
- the 23 R2025 models of #356 (`02_2025.zip`, in CI's corpus as `r2025/`);
- the models of the project corpus CI fetches: 2024 Core Interior, the four RE1 models (2025) and Einhoven (2023).

## 4. Open

- **Slanted walls** (word 0) still keep their box, on 2025 as on 2024 (RE-86 §4). On the 2025 file, 9 walls carry word 0, 1618833 and 1483746 among them.
- **The bandstand parapet** (1996153, the in-place family "Parapet Cap Bandstand") is still drawn as its record box, 26.5 × 34.6 × 11.3 ft. Its body is the in-place family's geometry, which this does not read.
- **Doors in tapered walls**, the tapered walls' ends at joins, and their materials stay as RE-86 and RE-89 left them.
