# RE-117 — Rooms' numbers and names from their parameter entries, on every release

**Date:** 2026-09-28
**Issues:** #90 (rooms), #421 (Revit 2023); follows RE-29 and RE-102
**Artefacts:**
- `2024_Core_Interior.rvt` (2024) with `2024_Core_Interior_slim.ifc`;
- `RE1-Architecture.rvt` (2025) with Revit's IFC export;
- Autodesk's Snowdon Towers Architectural (2024, local only) with the VIM export of its 2027 edition. Its Revit IFC4 export holds no spaces.
- `Exemplo_data.rvt` (2023, local only) with Revit's IFC4 export.

**Result:** every room keeps its number and name as two parameter entries after its data object, on Revit 2023, 2024 and 2025. Read from there, every room rvt-rs exports on four models carries Revit's number and name. Before, only RE-29's 2024 parameter block named rooms: it named all of Core Interior's but 1 of Snowdon's 54, and that one wrongly, and no 2023 or 2025 room.

| model | release | rooms | number and name as Revit's, RE-117 | before |
|---|---|---:|---:|---|
| Core Interior | 2024 | 116 | 116 (against the IFC) | 116, from RE-29's block |
| Snowdon Towers Architectural | 2024 | 54 | 54 (against the VIM) | 1 named, wrongly (`Vinyl Base` / `Open` for `R103` / `Storage`) |
| RE1 Architecture | 2025 | 11 | 11 (against the IFC) | none |
| Exemplo_data | 2023 | 9 | 9 (against the IFC) | none |

The name is `IfcSpace.LongName` and the `RoomName` property; the number is the `RoomNumber` property. `IfcSpace.Name` stays `Room-<ElementId>` on every release, where Revit writes the number; that is left to its own change. Core Interior's IFC is byte-identical.

-----

## 1. The entries

A room's data is an object `01 00 00 00 · id` (a `u32` id on 2023, a `u64` on 2024 and 2025). Past it come two parameter entries, the second right after the first, their parameter ids as wide as the release's ElementIds:

```text
id   -1006901   BuiltInParameter ROOM_NUMBER (i32 on 2023, i64 on 2024 and 2025)
u32  n
UTF-16 × n     "101"
id   -1006900   BuiltInParameter ROOM_NAME
u32  m
UTF-16 × m     "Café"
```

**Where the pair sits.** It is the first such pair within 0x400 bytes of the room's object. Measured from the id:
- 377 or 389 bytes on Exemplo_data;
- 483 to 503 on RE1;
- 531 to 875 on Snowdon.

The window keeps out a pair 2,504 bytes on in RE1, which belongs to another element.

**Duplicates.** A room whose copies disagree gets nothing; here none disagree.

**Implementation.** `partition_room_parameters::scan_room_parameter_entries` reads the pair, with the id and parameter widths taken from `partition_names::element_data_layout` (RE-114).

**Clean room.** The parameter ids are the public BuiltInParameter values.

## 2. What is written

**Precedence.** A room's pair replaces the number and name RE-29's block gave it wherever they differ, on every release. Where they agree, the block stays the recorded source. RE-29's block still gives the room's Level where its record names none, and stands in for the number and name where no pair is read.

**Core Interior.** Both agree on all 116 rooms, so its rooms keep the block as their source and its IFC is unchanged.

**Snowdon.** The block framed one room, 2181766, with another element's values. The pair gives it the VIM's `R103` / `Storage`.

**The VIM oracle.** The VIM's room element Name is Revit's display name, `<name> <number>` (`Stair S1`), and its `Vim.Room.Number` is the number. A room counts as matched when both equal rvt-rs's.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Exemplo_data.rvt (local only) | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |

```bash
cargo run --profile ci --example probe_re117_room_names -- RE1-Architecture.rvt
./target/ci/rvt-ifc RE1-Architecture.rvt -o re1.ifc
python3 tools/re/room_names_vs_ifc.py re1.ifc RE1-Architecture.ifc
python3 tools/re/room_names_vs_ifc.py snowdon.ifc Snowdon.r2027.vim
```
