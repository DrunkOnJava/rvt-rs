# RE-19 synthesis — Door/Window discriminator + schema-field / 2024 Wall (negative)

**Date:** 2026-08-29  
**Branch:** `cursor/door-window-wall-research-67f9`  
**Issues:** #32 (Door/Window), #30 (schema-field Wall), #23 (2024 ArcWall)  
**Prior art:** RE-15 (opening index), RE-13 / RE-18 (tag drift / no literal `Wall`)  
**Corpus:** `RVT_PROJECT_CORPUS_DIR` → magnetar `Revit_IFC5_Einhoven.rvt` (2023),
`2024_Core_Interior.rvt` (2024).

**Probes:**

- `examples/probe_door_window_wall_research.rs`
- `examples/probe_door_window_wall_followup.rs`

## Verdict (fail-closed)

| Question | Result | Confidence |
|---|---|---|
| Reliable Door vs Window discriminator in 2024 opening-index / nearby partition data? | **No** — do not invent typed `Door`/`Window` | **0.90** |
| Recoverable schema-field / non-ArcWall `Wall` on these corpora? | **No** — literal `Wall` absent; VWall not 2023-envelope-decodable | **0.92** |
| Fail-closed 2024 ArcWall geometry decoder ready (#23)? | **No** — tag `0x019c` present but **not** the 2023 `(family_marker, 0x07fa)` envelope | **0.93** |

This is a **documented negative**. Production must keep class
`ArcWallRectOpening` for 2024 index rows and must not emit typed
`Door` / `Window` / schema-field `Wall` without new wire evidence.

## #32 — Door / Window

### What we already had

- 2024 `ArcWallRectOpening` 60 B index (`tag=0x01a7`, marker `0x40088204`)
  merges into `iter_elements` with ElemTable-confirmed `related_id_a/b`.
- Class stays `ArcWallRectOpening` (PR #140 / #141).

### Hypotheses tested

| ID | Hypothesis | Posterior | Evidence |
|---|---|---|---|
| H32-1 | Mid-body index bytes encode Door vs Window (bimodal enum) | **0.05** | 31/60 columns constant; leftover variance is 1–3 outlier bytes on 3000 samples, not a 50/50 class split. `+0x30` is `0x00040000` on 3132/3167 rows and `0x02040000` on 32 — not a door/window partition. |
| H32-2 | `related_id_a` vs `related_id_b` roles type the opening | **0.10** | `related_b == related_a+1` on 3164/3167; unique pairs ≪ openings (multiplicity up to 131). Pair is a dual-id handle, not Door vs Window. |
| H32-3 | ElemTable payload bytes discriminate type for those ids | **0.05** | 2024 ElemTable rows are 40 B marker+id echoes; no category / class field. |
| H32-4 | UTF-16 `Door`/`Window` / `OST_Doors` / `OST_Windows` co-locate with related ids | **0.10** | Partition has door-like (28) and window-like (11) strings; **0** `OST_Doors`/`OST_Windows`. 400 openings: **0** near-door-only / near-window-only string hits at ±128 of id occurrences. ±64B “id near string” hits collide across door+window and include tiny integers (noise). |
| H32-5 | `VWallRectOpening` (`0x01a8`) carries typed dims / discriminator | **0.20** | 717 filtered hits, irregular deltas; door-plausible f64 pairs **1/717**; window-plausible **0/717**. |
| H32-6 | Schema declares `Door`/`Window` with sill/host/flip fields | **0.02** | Both corpora: `Door`/`Window`/`FamilyInstance` **ABSENT**. No schema fields matching sill / flip_hand / host door-window hints on instance classes. |
| H32-7 | `Global/Latest` diagnostic scan yields `Door`/`Window` | **0.02** | Diagnostic top class is `HostObjAttr` only; `Door`/`Window` = 0 on both files. Production counts: 0 Door, 0 Window. |

### Production posture (locked)

- Keep emitting `ArcWallRectOpening` with provenance fields only.
- Do **not** map opening-index rows to `IfcDoor` / `IfcWindow` / host
  voids-fills until a **new** discriminator is evidenced (likely outside
  this 60 B index — e.g. family-symbol parameter blobs keyed by a still-
  unknown join).
- Regression: `tests/iter_elements_typed.rs` + RE-19 honesty tests.

### Einhoven 2023

- **0** filtered `ArcWallRectOpening` / `VWallRectOpening` hits on
  `Partitions/5`.
- **0** UTF-16 door/window-like strings in that partition.
- Honest zero Door/Window remains correct for this sample.

## #30 / #23 — Schema-field Wall + 2024 ArcWall

### Schema inventory (both corpora)

| Name | 2023 Einhoven | 2024 Core Interior |
|---|---|---|
| `Wall` | ABSENT | ABSENT |
| `ArcWall` | tag `0x0191`, **0 fields** | tag `0x019c`, **0 fields** |
| `VWall` | tag `0x0192`, **0 fields** | tag `0x019d`, **0 fields** |
| `BasicWall` / `CurtainWall` / `StackedWall` | ABSENT | ABSENT |

Confirms RE-18: there is no schema-field `Wall` instance path on these
files. Wire geometry (when present) is concrete tagged subtypes.

### 2024 ArcWall envelope (`Partitions/46`)

| Check | Result |
|---|---|
| Filtered `0x019c` hits | 919 |
| `SCHEMA_FAMILY_MARKER` (`0x00088004`) at +0x04 | **0 / 919** |
| Variant `0x07fa` / `0x0821` at +0x10 | **0** |
| 2023-style 6×f64 coords at +0x12 | coincidental f64s only; sample hex is **not** the 2023 layout |
| Filtered leftover `0x0191` | 742 hits, also **0** family markers — not live 2023 records |

**Conclusion:** Tag drift alone is insufficient. A new 2024 envelope must
be reverse-engineered from scratch (or a different stream/join). Shipping
a decoder that reuses the 2023 layout against `0x019c` would be a false
positive — version gate must stay closed (#23 remains open as research).

### VWall

- Einhoven: **0** filtered `0x0192` hits on `Partitions/5`.
- 2024: 794 filtered `0x019d` hits; **0** `SCHEMA_FAMILY_MARKER` at +4;
  only 20/2000 show that marker anywhere in +0..+64 (not a stable
  envelope). No production VWall path.

### Production posture (locked)

- Continue ArcWall-only wall geometry on **Revit 2023** standard variant.
- Report `schema_field_wall_instances` and 2024 ArcWall as unsupported
  features (already listed in IFC diagnostics).
- Do not invent schema-driven `Wall` rows from `HostObjAttr`.

## What would unblock these issues

1. **Door/Window:** Join from opening `related_id_*` (or a still-unknown
   third id) into family-symbol / type-parameter storage that carries
   category or symbol name — with a measured false-positive rate against
   walls/furniture. String co-location alone is **rejected**.
2. **2024 ArcWall:** Dedicated envelope RE from the 919 `0x019c` hits
   (alternate marker, stride, coord base) with FP measurement on a
   non-wall partition — do not lift the 2023 version gate until that
   lands.
3. **Schema-field Wall:** Unlikely on current schema dumps; pursue
   concrete tags (`ArcWall` 2024, `VWall`, drivers) instead of literal
   `Wall`.

## Log artifacts

Probe stdout captured during this research run:

- `/tmp/door_window_wall_research.log`
- `/tmp/door_window_wall_followup.log`
