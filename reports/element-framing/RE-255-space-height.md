# RE-255 — A space's Height

**Date:** 2026-10-27
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-101 (room outlines from their stored solids), RE-207 (saved parameters) and RE-251 (the same search for walls)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports) and puzzbobb's R2025 models (Revit 2025), as Measure fetches them; Core Interior and RE1 Architecture, whose exports have base quantities off (RE-242); held out, the flowbim.ee house (Revit 2026, Revit 26.4's IFC4X3 export) and 255ribeiro's two projects, counts only
**Runs:** Measure (`examples/probe_re255_space_numbers.rs` with `tools/re/space_numbers_vs_qtos.py`) [40461695764](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40461695764); the change against `main`, [40462323338](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40462323338) (unfiltered) and, held out, [40464217800](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40464217800) (Revit 2024 and 2025 only) and [40464541603](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40464541603) (with Revit 2026)
**Result:** positive, narrowed: a room's native `RoomElem.m_height` is Revit's `Qto_SpaceBaseQuantities.Height` on every room whose record holds its stored solid's faces, and is written there; a room's areas and volumes are not in its record or its saved parameters, and are not written.

## 1. What the record holds

The probe prints every number of each room's native record and of its saved parameters, as RE-251's does for walls, with the `Name` rvt-rs gives the room's `IfcSpace` (its number, else `<name>:<ElementId>`), which keys a space in the quantity census. On the office as built, of the 254 spaces Revit's export gives a `Height`:

- `RoomElem.m_height` is Revit's `Height` on 234, and `m_upperOffset` on 50;
- every one of the 234 holds two `Face` objects or more in its record, the faces of the room's stored solid (RE-101), and none of the other 20 does: the second `Face` tells them apart exactly, and no other field is needed;
- on R2025 12 and 13 `m_height` is Revit's on both rooms of each.

revit-ifc's `NetVolumeCalculator` (CLEANROOM.md source 5) takes a space's `NetVolume` from a parameter before its geometry. No number of the room's record or saved parameters is it: `MassInstUserData.m_nVolume` matches on 2 of the office's 278, and no field matches a space's areas or volumes on the R2025 rooms.

## 2. What rvt-rs does

`partition_schema_mvp::attach_room_heights` reads each room's native record in one pass and gives a room whose record holds two `Face` objects or more its `m_height` (`ROOM_HEIGHT_FIELD`), on Revit 2024, 2025 and 2026; the exporter writes it as `Qto_SpaceBaseQuantities.Height`.

## 3. Measured

This change against `main` (Measure 40462323338, unfiltered, `-f nordic=true -f r2025=true`, every scorer):

| model | `Height` equal | different | missing | quantities equal |
|---|---:|---:|---:|---:|
| office as built | 0 → 234 of 254 | 0 | 20 | 20,831 → 21,065 of 39,721 |
| office at permit | 0 → 234 of 254 | 0 | 20 | 20,746 → 20,980 of 40,013 |
| R2025 12 to 17 | 0 → 2 of 2 each | 0 | 0 | |

Held out, on a model the rule was not derived on: the flowbim.ee house (Revit 2026) gets Revit's `Height` on all 13 of its spaces, none different (Measure 40464541603); 255ribeiro's projects have no space heights in their exports.

Core Interior's 116 room heights and RE1 Architecture's 11 are counted apart, their exports having base quantities off; Core Interior's witness observations gain 116 each of `IfcElementQuantity`, `IfcQuantityLength` and `IfcRelDefinesByProperties` (both verdicts PASS, unchanged). Every other scorer's output is as on `main`.

## 4. What stays open

The 20 office rooms whose records hold no faces, and every space's areas and volumes (`NetVolume` on 278 office spaces, `GrossVolume` on 136): Revit computes them from the room's geometry, which rvt-rs reads as an outline (RE-101) but not as Revit's volume.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=COMMIT -f base=none -f nordic=true -f r2025=true \
  -f models="nordic-office-as-built r2025-12_Rooms_Door_Windows r2025-13_Rooms_Stairs" -f scorers=none \
  -f probe=probe_re255_space_numbers -f probe_scorer=space_numbers_vs_qtos
gh workflow run measure.yml --ref main -f ref=COMMIT -f base=main -f nordic=true -f r2025=true
```
