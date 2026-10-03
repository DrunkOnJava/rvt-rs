# RE-153: a room's Floor Finish is BuiltInParameter -1006903

**Date:** 2026-10-03
**Issues:** #35; backlog item B21
**Artefacts:**
- RE1 Architecture (Revit 2025) with Revit's own IFC4 export;
- Core Interior (2024), Einhoven (2023), and RE1 Electrical, Mechanical and Plumbing (2025).

Measured on GitHub's hosted runners with the Measure workflow, run 37131189456.
**Probe:** `examples/probe_re153_room_finishes.rs`.
**Status:** positive.

## 1. The question

Revit's IFC4 export of RE1 Architecture gives each of its 11 rooms a `Pset_SpaceCommon` with two properties:
- `Reference`, an `IfcIdentifier`: the room's name and number (`Storage Narrow RE-9`);
- `FloorCovering`, an `IfcLabel`: the room's Floor Finish (`Wood`, `Concrete`, `Linoleum`, `Ceramic Tile`, `Tatami`).

RE-117 reads a room's number and name. Where is its Floor Finish?

## 2. Method

A text parameter is an entry `id · u32 n · UTF-16 × n`, with a BuiltInParameter id as wide as the release's ElementIds (RE-117). The probe finds every such entry whose id is within 20 of `ROOM_NAME` (-1006900), in every partition.

It joins each entry to its element through the data object holding it. That object is the last header `[i32 ElementId][i32 0][u32 Adler-32][i32 size][i32 class]` before the entry whose payload:
- encloses the entry;
- ends in its size again;
- verifies its Adler-32, taken over the class word and the payload less its last 4 bytes.

This is the method of the 2026-09-29 comment on #35. The object's ElementId is matched to the rooms (`OST_Rooms`), and each room is matched by number to the space's `Pset_SpaceCommon` in Revit's export.

A first version looked only in the bytes after a room's name entry. It found the next room's entries there, not the room's own, and was replaced.

## 3. Result

RE1 Architecture, 13 rooms:

| id | entries | in a verified object | on a room | equals Revit's |
|---|---:|---:|---:|---|
| -1006900 (`ROOM_NAME`) | 13 | 13 | 13 | |
| -1006901 (`ROOM_NUMBER`) | 13 | 13 | 13 | |
| -1006903 | 12 | 12 | 12 | `FloorCovering`, 11 of 11 |
| -1006907 | 13 | 13 | 13 | none |

- **-1006903 is the Floor Finish.** On every room Revit exports, its value is Revit's `FloorCovering`.
  - The twelfth room, a terrace (`RE-11`) that Revit does not export, holds `None`.
  - The thirteenth, `1 LDK Small` (number `RE`), has no Floor Finish. Revit does not export it either.
- **-1006907** holds a type of room: `Storage`, `Living Room`, `Dining Room`, `Terrace`, `Suite of Spaces`. It is probably the room's Occupancy or Department. Revit's export writes neither, so it is not ruled on.
- `Reference` is the room's name, a space, and its number, on 11 of 11.
- Core Interior holds no -1006903 entry: its rooms have no Floor Finish. Its 664 number and 665 name entries are all inside verified objects. Einhoven (2023) is outside the method, and the other RE1 models have no rooms.

## 4. What this changes

`partition_room_parameters::scan_room_text_parameter` reads a room's text parameter by this join, and the IFC export writes each room's `Pset_SpaceCommon` with Revit's two properties and value types. The test is `tests/space_common.rs`, against RE1 Architecture's export.
