# RE-43 — Frames with the invalid ElementId, and sketches of separate pieces

**Date:** 2026-09-23
**Result:**
- **Positive.** Some element frames hold `u32::MAX` at `+0x00`, the 32-bit form of Revit's invalid ElementId −1, where RE-30's second prologue holds `0` or a value above `u32::MAX`. They carry no id of their own either, and now take their enclosing partition record's id (RE-35).
- On Snowdon Towers Architectural this recovers the last element of Revit's export missing from an exported entity: stair run 1644693, "Assembled Stair:Stair:1644692 Run 1".
- Of the elements Revit's export holds, in the entity types rvt-rs writes, none is now missing.
- The same frames include sketch lines, and 18 slabs gain their sketched plan profile. 16 have exactly Revit's area per solid.
- **Fixed on the way:** a sketch of separate pieces was read as an outer loop with the other pieces as voids outside it, which is invalid geometry. It now gives no profile, and the slab keeps its record box (#331).

**Probe:** `examples/probe_re43_invalid_id_frames.rs` counts the placed frames holding `u32::MAX` per category, inside and outside the chain.

**Oracle:** Snowdon Towers Sample Architectural (`bldrs-ai/test-models`, local only, `.rvt` sha256 `33271010…`) and Revit's IFC4 export of it (`ecfcb04e…`). No licensed file has such a frame; their exports are byte-identical.

-----

## 1. The frames

Placed-instance frames whose `+0x00` is `0x0000_0000_ffff_ffff`:

| file | sketch lines | stair runs | other line categories |
|---|---:|---:|---:|
| Snowdon Architectural | 99 | 1 | 73 (`OST_Lines` 35, `OST_RailingRailPathExtensionLines` 31, `OST_RoomSeparationLines` 7) |
| Snowdon Structural | 8 | 0 | 0 |
| Core Interior, RE1 x4, `teste_export_2025`, Projeto1 | 0 | 0 | 0 |

Every one sits inside a chain record. `partition_element_records::carries_no_element_id` now counts `0`, `u32::MAX` and anything above as "no id". `assign_second_prologue_ids` gives these frames their record's id, and the unattributed count treats them the same way.

Run 1644693's frame is a placed `OST_StairsRuns` instance with no design option. Its reference list names its stair 1644692, so it joins that stair's `IfcRelAggregates` (RE-39). Revit's export has all 43 flights, and so does rvt-rs now.

## 2. Sketches

The 99 sketch lines now attributed close the plan profiles of 18 slabs that exported as record boxes before. Profile area against the same `Tag` in Revit's export:

| result | slabs |
|---|---:|
| equal to Revit's area | 12 |
| equal to each of Revit's two stacked solids | 4 |
| outer loop equal, Revit also cuts a 0.445 m² void rvt-rs does not have | 1 (2114707) |
| different loop (48.12 m² against 40.46 m²; the box was 61.80 m²) | 1 (2113951) |

## 3. Separate pieces

One more slab closed its sketch as two separate rectangles (1402063). `plan_profile_from_segments` takes the largest loop as the outer boundary and every other loop as a void. It made the second rectangle a void outside the first, and the net area was 0.

Four slabs on `main` already had the same defect (1388180, 1388290, 1402277, 1404309): `IfcArbitraryProfileDefWithVoids` with voids outside the outer curve and near-zero area.

A loop that is not inside the outer loop is now a second piece, not a void, and the sketch gives no profile. Those five slabs export their record box with `ProfileResolved` false.

Revit writes such a slab as one `IfcSlab` per piece, all with the element's `Tag` (1402063 is two 0.377 m² slabs). Representing that is #331.

**Addendum (#331, 2026-09-23).** A plan profile now keeps every separate loop as a piece with the voids inside it, and the exporter writes each further piece as another element with the element's `Tag`, named `…:2`, `…:3`. The five slabs against Revit's pieces (m²):

| slab | rvt-rs | Revit |
|---|---|---|
| 1388180 | 1.60, 1.60 | 1.60, 1.60 |
| 1388290 | 1.99, 1.99 | 1.99, 1.99 |
| 1402063 | 0.38, 0.38 | 0.38, 0.38 |
| 1404309 | 9.02, 9.76 | 9.02, 9.76 |
| 1402277 | 0.40, 0.40, 3.35 | one `IfcSlab` with three tessellated bodies |

IfcOpenShell reports no issue on the export, and Core Interior and the RE1 models export as before.

## 4. Other files

Core Interior, RE1 Architecture and RE1 Electrical export byte-identically apart from timestamps. None of the licensed files has a frame with the invalid id or a sketch of separate pieces.

## 5. What this does not claim

- Why Revit writes −1 rather than the second-prologue value in these frames is not established.
- Slab 2113951's loop differs from Revit's, and 2114707 lacks one void. Both are closer to Revit's area than their record boxes were. It is not a stale sketch: keeping each sketch line's newest frame instead of its oldest leaves the Snowdon export byte-identical.
