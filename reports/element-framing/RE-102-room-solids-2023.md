# RE-102 — Revit 2023 stores room solids the same way

**Date:** 2026-09-28
**Issues:** #90, #421; follows RE-81 and RE-101
**Artefacts (local only, no licence):** `255ribeiro/intro_ifc` `Exemplo_data.rvt` (Revit 2023) with Revit's own IFC4 export.
**Result:**
- Revit 2023 writes a room's solid as Revit 2024 and 2025 do (RE-101), with two differences:
  - four `ff` bytes sit between the room's ElementId and the solid's tag;
  - its faces are 170 bytes apart where 2024's are 174.

  ```text
  u64 room id · ff ff ff ff · 04 90 08 00 · …
  ```

  The face fields sit at the same offsets.
- Rooms on Revit 2023 projects now take the outline their solids give. The same guard applies: the outline's extent must equal the room record's box within 0.001 ft.

| `Exemplo_data` against Revit's IFC4 (9 rooms) | main | RE-102 |
|---|---:|---:|
| area within 0.1% | 6 | 9 |
| outline within 0.01 ft | 6 | 9 |

-----

## 1. The header

Room 349972 is in Partitions/4:
- its ElementId is `14 57 05 00 00 00 00 00` at 0x6fc530;
- `ff ff ff ff` and `04 90 08 00` follow it;
- then `01 00 00 00 03 00 00 00` and the solid's box, as on 2024.

Its first face is at 0x6fce3d and the next at 0x6fcee7, 170 bytes on. The offsets RE-101 reads (uv box +20, `01` +85, origin +86, u +110, v +134) hold on every face.

## 2. Measured

`examples/probe_re101_room_solids.rs` now runs the full recovery, so it sees 2023's rooms. On `Exemplo_data` every room has one solid in its own partition, and all 9 close on their record box. Three of them, rooms 349975, 349984 and 350436, are not their box; main's 6 of 9 are the six rectangles.

Revit 2023 rooms do not carry Revit's GlobalId in rvt-rs's export, so `plan_profiles_vs_ifc.py --match centroid` pairs each room with the Revit one whose top surface's centroid is nearest. All 9 pair at 0.000 ft and equal Revit's area and outline.

`modelo_bim` has no rooms. Every other local file is byte-identical to RE-101's output apart from the timestamp.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |

```bash
cargo run --profile ci --example probe_re101_room_solids -- Exemplo_data.rvt
./target/ci/rvt-ifc Exemplo_data.rvt -o exemplo.ifc
python3 tools/re/plan_profiles_vs_ifc.py exemplo.ifc Exemplo_data.ifc --class IfcSpace --match centroid
```
