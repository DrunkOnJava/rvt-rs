# RE-261 — Three bits of the element record's flag word, checked against Revit's exports

**Date:** 2026-10-28
**Issues:** #223 (the flag word at `+0x46`); follows RE-128 (bit 7 on type records, bit 12 unattributed), RE-135 (bit `0x10`) and RE-219 (no bit is `LoadBearing` or `IsExternal`)
**Claims checked:** puzzbobb's bit table on #223 (issuecomment-6427837684, 2026-10-26), measured on 63 models of their own and one-change saves in Revit 2025
**Artefacts:** Core Interior (Revit 2024, `2024_Core_Interior_slim.ifc`), the four RE1 models (Revit 2025, `Drshelden/IFC-ECS`) and puzzbobb's 23 R2025 models with Revit's IFC export of each, as Measure fetches them
**Runs:** Measure (`examples/probe_re261_flag_word_bits.rs`, `tools/re/flag_word_vs_ifc.py`) [40776705407](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40776705407) at 46d1d6df; [40775852776](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40775852776) at 906595c9 read first-prologue frames only and paired no space (IFC4 spaces have no `Tag`); its bit 7 counts are the same.
**Result:** positive on what these models can test. Bit 12 is exactly a room Revit exports as an `IfcSpace` (302 of 302 rooms) and exactly a wall with an end joined in Revit's export (8 of 8 walls, the only model whose export writes wall joins). Bit 7 is exactly a type record (all 144 records of five type classes, no other record). The room scan misses no room record that carries an ElementId. Nothing ships: no reader uses the word.

## 1. What the probe reads

For every frame carrying the file's bbox marker at `+0x50` with an ElementId (at `+0x00` for a first-prologue frame, from its enclosing record for a second-prologue one, RE-35), the probe prints the record's category, its class (`+0x4a` resolved in the file's own schema, RE-76), the word at `+0x46`, the placement kind and the container. 23,266 records over 28 models, 457 of them second-prologue. The scorer pairs:

- **walls** (`SWall`) with Revit's element of the same `Tag`, and reads each `IfcRelConnectsPathElements` by attribute name: an end is joined when the wall's own side of a relation is `ATSTART` or `ATEND`;
- **rooms** (`RoomElem`) with Revit's `IfcSpace` through the space's GlobalId: Revit's GlobalId for element `id` is its episode's GUID with `id` XORed into the last four bytes (`revit_global_ids.rs`), and every tagged element of the same file gives its episode's first twelve bytes and last word. Every space of every model pairs with exactly one room (116, 11 and 2 per R2025 room model).

## 2. Bit 12 (`0x1000`)

| Rooms | bit set | bit clear |
|---|---:|---:|
| exported as an `IfcSpace` | 139 | 0 |
| not exported | 0 | 163 |

Core Interior 116 and 161, RE1 Architecture 11 and 2, puzzbobb's six R2025 room models 2 and 0 each. Revit exports a room as a space only when it is placed (and enclosed), so this agrees with puzzbobb's "rooms: placed". Their 88 placed rooms with the bit clear are not in these models.

| Walls, RE1 Architecture | bit set | bit clear |
|---|---:|---:|
| an end joined in Revit's export | 5 | 0 |
| no end joined | 0 | 3 |

Wall 462510's only join is another wall's end meeting its middle (`ATPATH` on its side), and its bit is clear: the bit is an end join, not any join. Core Interior's slim export and every R2025 export write no `IfcRelConnectsPathElements`, so RE1 Architecture's eight walls are the whole oracle. puzzbobb's `18_WallJoins` model has 3 walls with the bit and 6 without, which nothing in its export can check. Pipes (bit 12 on connected pipes) are not checked: RE1's exports carry no connection to pair.

## 3. Bit 7 (`0x80`)

| Class | records with bit 7 |
|---|---:|
| `FamilySymbol` (Core Interior) | 57 of 57 |
| `SysPanelFamSym` | 52 of 52, 2 of 2 |
| `SysMullionFamSym` | 15 of 15, 9 of 9 |
| `RvtLinkSymbol` (RE1, four models) | 3, 3, 1, 1 of the same |
| `DirectShapeType` | 1 of 1 |

No record of any other class has it (23,122 records). This is RE-128's reading (a type record) on every model here. puzzbobb's row, "a `WallSweep` with `m_typeId == -1`, 411 of 411", is not testable: none of these models has a `WallSweep` record. Both readings hold together only if their sweeps are records Revit treats as types; their 3,138 `FamilySymbol` records under bit 5 would show whether bit 7 is on those too.

## 4. The room scan

puzzbobb reports that `scan_category_records_multi` misses `RoomElem` records whose computed block is empty (a room Not Enclosed or unplaced). On these models every `RoomElem` frame that carries an ElementId is decoded and returned by the scan (277 on Core Interior, 13 on RE1 Architecture, 2 per R2025 room model), including Core Interior's 161 rooms Revit does not export. Core Interior also has 266 frames with the room category and class but no ElementId (9 with the bbox marker, 257 without), which no reader attributes; whether any is a room record is not measured. Not reproduced; their one-change saves are not public.

## 5. Not checked

Bits 0, 1, 5, 14, 20 and 21 (puzzbobb's exact rows) and 4, 11, 16 and 22 (their likely or open rows) need member values from each element's native record (`m_structType`, `m_active`, `m_partitionSurrogateId`, group nesting) or saves that change one parameter. They stay recorded on #223 as puzzbobb's measurements, unverified here.

## Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=46d1d6dfdddb9a243cc4cb5855e7c67fcfdd0d54 -f base=none -f r2025=true \
  -f models="core-interior re1-* r2025-*" -f scorers=none -f probe=probe_re261_flag_word_bits -f probe_scorer=flag_word_vs_ifc
```

Each model's `probe.txt` and `probe_score.txt` are in the run's `measure-head` artifact.
