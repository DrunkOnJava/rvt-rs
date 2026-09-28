# RE-128: the element record's flag word at +0x46

**Date:** 2026-09-28
**Issue:** #223; follows RE-21 (element records) and RE-76 (+0x4a class)
**Artefacts:** `2024_Core_Interior.rvt` (MIT, sha256 `c805df44…`) with `2024_Core_Interior_slim.ifc` (IFC4 ReferenceView), and the four RE1 models (Revit 2025, MIT: Architecture, Electrical, Mechanical, Plumbing), each with its IFC2X3 CoordinationView export. Measured on GitHub's hosted runners, Measure run 36494666177.
**Probe:** `examples/probe_re128_record_flags.rs`; scored by `tools/re/record_flags_vs_ifc.py`.
**Status:** partial. Three bits are attributed. Bit 12 is not, and the reference exports cannot attribute it.

## Method

For every element record of the categories rvt-rs exports (14,580 on Core Interior, 98 on RE1), the probe reads the `u32` at `+0x46`, Revit's `m_abFlags4Bytes` (Discussion #112). It also re-reads `+0x4a` from the same bytes: that equals the decoded class on 14,580 of 14,580 and 98 of 98 records, so the offsets are right. The scorer joins each record to Revit's export by Tag.

## Values

| file | value | class (+0x4a) | records | in Revit's export |
|---|---|---|---:|---:|
| Core | `0x0928` | FamilyInstance (1894), Floor (2017), 4124 | 12,563 | 737 |
| Core | `0x1929`, `0x0929` | SWall (3669) | 1,487 | 638 |
| Core | `0x916a`, `0x816a` | RoomElem (3612) | 399 | (rooms export as IfcSpace) |
| Core | `0x09a8`, `0x2009a8` | FamilySymbol (1928), 4064, 4066, 1238 | 123 | 0 |
| RE1 | `0x0928` | FamilyInstance (1951), 733, Floor (2073) | 65 | 65 |
| RE1 | `0x1929`, `0x0929` | SWall (3762) | 8 | 8 |
| RE1 | `0x916a`, `0x816a` | RoomElem (3704) | 13 | (IfcSpace) |
| RE1 | `0x2009a8` | 4161, 4163 | 11 | 0 |

Floors and one foundation also carry `0x1928`, and one generic model `0x400928`.

## Attributed

- **Bit 7 (`0x80`): an element type.** It is set on every record of the type classes (FamilySymbol and the curtain panel and mullion types) and on no instance: 123 records on Core Interior and 11 on RE1, none in Revit's export. Bit 21 (`0x200000`) occurs only with it.
- **Bits 1, 6 and 15 set, bit 11 clear: a room.** Exactly the RoomElem records (399 and 13).
- **Bit 0: an element drawn along a curve.** It is set on exactly the wall, pipe and duct records: 1,487 walls on Core Interior, 8 on RE1 Architecture, 6 pipes and 25 ducts on RE1 Mechanical, and 63 pipes on RE1 Plumbing. No other record sets it.

These restate what `+0x4a` already says (RE-76) in a form that needs no schema, so nothing in rvt-rs changes.

## Not attributed

**Bit 12 (`0x1000`)** is set on:
- Core Interior: 1,457 of 1,487 walls, 2 of 146 floors and 238 of 399 rooms;
- RE1 Architecture: 5 of 8 walls;
- RE1 Mechanical and Plumbing: every record (ducts, pipes, fittings, terminals, equipment);
- RE1 Electrical: 91 of 145 records.

It is not LoadBearing: RE1's walls say false on both sides of it. On RE1 Electrical alone, every record with bit 12 or bit 22 (`0x400000`) is in Revit's export and none of the others are. Those others are the symbols nested in switches and receptacles, which RE-79 already leaves out. On Architecture, family instances without bit 12 are exported, so bit 12 does not mark export in general. Revit's usual per-element flags (Room Bounding, Structural) are not written by either export, which carries only `Pset_*Common` properties. So these oracles cannot attribute it. It needs an export with Revit's own property sets, or a model made to vary one parameter.
