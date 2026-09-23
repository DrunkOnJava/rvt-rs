# RE-34 — The second prologue's ElementId, from the reference list and the frame order

**Date:** 2026-09-22
**Result:**
- **Positive.** A second-prologue frame's ElementId can be recovered with no oracle and no wrong id on any measurement:
  - one slot of its reference list is the id (RE-30 §7);
  - frames sit in ascending ElementId order (RE-30 §8);
  - so an id forced by the ordering, whichever way ties are resolved, is the frame's own.
- **Hold-outs** (every `+0x00` id hidden, then scored): 3,458 correct and 0 wrong on `2024_Core_Interior.rvt`; 445 / 0 on Snowdon Towers; 41 / 0, 7 / 0 and 36 / 0 on the RE1 models.
- **Snowdon Towers Architectural** now exports **4,245 elements instead of 80**. 4,077 of its tagged elements are in Revit's own export. The other 115 are elements Revit's export omits, the same class as #309. None is a wrong id for an exported element.
- **Measured exclusion:** elements that own a sketch (floors, building pads, ceilings) get no id (§4).

Closes the open question of #295 for everything but sketch-owning categories.

**Oracles:** as in RE-33:
- `2024_Core_Interior.rvt` (magnetar, MIT), its own oracle once the ids are hidden;
- Snowdon Towers Architectural and Revit's IFC4 export of it (`bldrs-ai/test-models`, no licence, measured locally, `.rvt` sha256 `33271010…`, `.ifc` `ecfcb04e…`);
- the RE1 models (`Drshelden/IFC-ECS`, MIT, commit `9e8b371`).

**Probe:** `examples/probe_re34_holdout.rs` hides every declared `+0x00` id in the inflated partitions, runs the rule, and prints each wrong pick with its reference list. `tests/second_prologue_ids.rs` runs the same hold-out on Core Interior and requires zero wrong.

-----

## 1. The rule

For each partition, take every framed element in offset order: a frame whose `BuiltInCategory` at `+0x12` is in the band and whose `+0x00` is either a declared ElementId or no id at all.

- A **first-prologue frame** is fixed at its `+0x00` id.
- A **second-prologue frame that is a placed instance** (no container at `+0x32`, placement kind `0xffffef7f`, RE-33) may take any declared id in its reference list after the leading slot, except context ids (§3).
- Other second-prologue frames (container members, type symbols) take no part.

Find the longest chain in which the chosen ids rise strictly with offset. Do it twice: once resolving ties toward smaller ids, and once, by running the same search backwards on negated ids, toward larger ones. **A frame gets an id only when both chains give it the same one.** A frame the two chains disagree on stays unassigned.

The earlier attempt (RE-30 §8) kept a single chain that preferred smaller ids, plus guards. It made 32 wrong picks on the Core hold-out and got every Snowdon floor wrong. Requiring the two tie-breaks to agree is what removes the wrong picks: a pick the ordering does not force differs between them.

## 2. Hold-outs

Each file's first-prologue ids were overwritten with `0x1_ffffffff`, the value Snowdon's second-prologue frames hold, and the rule was run on the result.

| file | release | correct | wrong | unassigned placed instances |
|---|---|---:|---:|---:|
| `2024_Core_Interior.rvt` | 2024 | **3,458** | **0** | 1,416 |
| Snowdon Towers Architectural (first-prologue frames only) | 2024 | **445** | **0** | 113 |
| RE1 Architecture | 2025 | **41** | **0** | 94 |
| RE1 Mechanical | 2025 | **7** | **0** | 38 |
| RE1 Plumbing | 2025 | **36** | **0** | 93 |

On Core Interior the assigned ids by category are:

| category | ids |
|---|---:|
| sketch lines | 2,338 |
| walls | 382 |
| columns | 329 |
| doors | 252 |
| rooms | 146 |
| windows | 6 |
| analytical panels | 5 |

## 3. Context ids, and how sensitive the rule is

Without a context filter, the Core hold-out gives 3,448 correct and **6 wrong**. The six pick a Level (20303, 20304 or 20308) or a type (4166, 4705 or 4714) from the front of the list. Those ids are named by 66 to 260 frames. No placed instance's own id on Core is named by more than 34.

`SECOND_PROLOGUE_CONTEXT_REFERENCES = 56` removes ids named by at least that many frames across the whole file. Swept over every hold-out, with sketch-owning output suppressed:

| threshold | Core | Snowdon | RE1 Arch | RE1 Mech | RE1 Plumb |
|---:|---|---|---|---|---|
| 48 | 3,458 / 0 | 445 / 0 | 41 / 0 | 7 / 0 | 36 / 0 |
| **56** | 3,458 / 0 | 445 / 0 | 41 / 0 | 7 / 0 | 36 / 0 |
| 64 | 3,458 / 0 | 445 / 0 | 41 / 0 | 7 / 0 | 35 / 0 |
| 80 | 3,452 / **1** | 445 / 0 | 41 / 0 | 7 / 0 | 36 / 0 |
| 96 | 3,452 / **1** | 445 / 0 | 41 / 0 | 7 / 0 | 36 / 0 |

Zero wrong holds from 48 to 64, and 56 is the middle of that band.

The count must span the whole file:
- counted per partition, the Core result swings between 0 and 3 wrong across the same thresholds;
- taking sketch-owning frames out of the chain, rather than only suppressing their ids, gives 2 wrong at 64.

The rule is therefore fixed as described: a whole-file count, and sketch-owning frames kept in the chain. It is a measured, bounded result, not a proof. A file whose Levels or types are named by fewer than 48 frames is outside what was measured, and the fail-closed answer there is the agreement requirement itself.

## 4. Sketch-owning elements are excluded

A floor's reference list names its sketch, created one ElementId before the floor. On Snowdon, floor 1423899's list reads `[…, 1423898, 1423899, …]`, and with the context filter on, the one floor the chains agree on takes 1423898, the sketch. RE-30 §8 recorded the same pattern (629118 / 629119).

Nothing in `Global/ElemTable` separates the two: both rows' owner field is a self-reference. `SKETCH_OWNING_CATEGORIES` (`OST_Floors`, `OST_BuildingPad`, `OST_Ceilings`) therefore never receive an id. Their frames stay in the chain and still constrain their neighbours.

No ceiling or railing frame on Snowdon is assigned even without this exclusion.

## 5. Snowdon Towers, end to end

`rvt-ifc` on Snowdon Architectural, against Revit's IFC4 export by `Tag`:

| entity written | exported | not in Revit's export |
|---|---:|---:|
| `IfcWall` | 981 | 7 |
| `IfcMember` (mullions) | 1,407 | 0 |
| `IfcPlate` (curtain panels) | 511 | 47 |
| `IfcBuildingElementProxy` (specialty equipment, wall sweeps) | 494 | 45 |
| `IfcFurniture` (furniture, casework) | 329 | 0 |
| `IfcSanitaryTerminal` | 145 | 15 |
| `IfcColumn` | 94 | 0 |
| `IfcDoor` | 58 | 0 |
| `IfcWindow` | 39 | 0 |
| `IfcSlab` | 1 | 1 (#309) |
| **total with a `Tag`** | **4,192** | **115** |

It also writes 133 `IfcOpeningElement`, the voids of hosted doors and windows, all in the export.

The 115 were checked one by one against the frame's reference list:
- **45 specialty equipment, 15 fixtures and 47 panels:** the list holds no exported `Tag` of the expected entity at all, so there was no exported element to pick instead;
- **7 walls:** bleacher-seating walls in the 2,062,3xx range beside the #309 slab, whose geometric oracle match (RE-30 §7) is a different exported wall that is not in their list;
- **1 slab:** #309 itself.

They are real elements the rule identifies and Revit's exporter leaves out (for example through design options, phases or "Export to IFC: Don't export"). rvt-rs does not model that filter yet (#309). Apart from the #309 slab, a first-prologue record, each of them carries the `element_id_from_reference_order` provenance warning.

**Diagnostics:**
- the unattributed count on Snowdon falls from 4,886 to 684, and the readiness score rises from 36% to 91%;
- Snowdon Structural goes from 26 exported elements to 60;
- RE1 Mechanical and Plumbing gain their second-prologue ducts, pipes and fittings (48 and 39 exported, all in Revit's export).

## 6. Cost

`RevitFile::second_prologue_ids` computes the assignment once per file: one pass over the partitions for the context count, one per partition for the chains. Core Interior, which has no second-prologue frame, exports in 2.75 s as before, and its IFC body is byte-identical. Snowdon Architectural exports in 7.3 s.

## 7. What this does not claim

- Floors, building pads and ceilings in the second prologue stay unassigned and counted.
- Elements Revit's export omits are exported (#309).
- 684 of the 4,929 placed instances of recovered categories on Snowdon (14 %), and 1,416 of 4,874 in the Core hold-out, stay unassigned because the ordering does not force their id. They remain counted as `element_record_without_element_id`.
- 2023 and 2026 stay unsupported.
