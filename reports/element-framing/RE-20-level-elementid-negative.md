# RE-20 — Level ElementId recovery (negative on magnetar corpora)

**Date:** 2026-08-29  
**Scope:** Recover Level `ElementId`s (and Floor/Room → level refs) so
`LevelStoreyBind` (#33 leftover after PR #144) can assign Floors/Rooms off
Unassigned. Secondary: AProperty host joins (#35).  
**Corpora:** `Revit_IFC5_Einhoven.rvt` (2023), `2024_Core_Interior.rvt` (2024)
via `RVT_PROJECT_CORPUS_DIR` (magnetar-io/revit-test-datasets).  
**Verdict:** **INSUFFICIENT** — fail closed; do not invent Level ids or
elevation-heuristic Floor/Room storey assignment.

## Context

PR #144 shipped fail-closed `LevelStoreyBind` plumbing and honest AProperty
surfaces. Production partition MVP Levels still decode with `id: None`;
Floors/Rooms lack `m_level_id`. Binding stays idle until both sides carry
matching ElementIds.

## Schema facts (new)

| Class | Einhoven 2023 | Core Interior 2024 |
|---|---|---|
| `Level` | **ABSENT** | **ABSENT** |
| `LevelType` | ABSENT | ABSENT |
| `DatumPlane` | ABSENT | ABSENT |
| `LevelAssociationCell` | present, **untagged**, 2 fields | same |
| `AnalyticalLevelAssociationCell` | tag `0x00ff`, 0 fields | tag `0x0100`, 0 fields |

`LevelAssociationCell` field layout (Formats/Latest):

```
m_levelOffset  Primitive f64 (kind 7, size 8)
m_levelId      ElementId
```

There is **no** schema-tagged `Level` instance class to decode on these
project files. Tagless Level recovery cannot use the RE-09/RE-11 tag-scan
path.

## Hypotheses tested

### H1 — Level-name UTF-16 proximity → ElemTable ids

Scan Partitions for storey-like strings (`Level 1`, `Roof`, …) and collect
nearby ElemTable ids.

- **Einhoven:** 139 (name,id) pairs; dominant ids are tiny noise (`1`,
  `255`, `256`). `unique_dominant_names=0/5`.
- **Verdict:** falsified (noise-dominated).

### H2 — Storey elevation f64 neighbourhood → ElemTable ids

- **Einhoven:** elev `0.0` alone hits 2592 distinct ids (`top=id 1` with
  millions of coincidences). No singleton elev→id clusters.
- **Verdict:** falsified.

### H3 — LevelAssociationCell-shaped `[f64 offset≈0][ElementId]` near ArcWalls

Per-elevation ArcWall groups voting for a shared `m_levelId`:

- **Einhoven:** “agreed” ids were `1` / `256` — appear across multiple
  elevations; not unique Level identities.
- **Core Interior:** no ArcWall elevations → probe inapplicable.
- **Verdict:** falsified (false-positive small ids).

### H4 — ElementId self-echo `[id][id]` / framed ElementId before Level names

- **Einhoven:** no self-echo hits; single framed ids are weak (`Roof→12`
  with 1 vote).
- **Core Interior:** some unique-looking hits (`Level 1→513`) but other
  names share small ids (`Roof`/`Ground floor`→`12`); family chrome
  (`Level Head - Upgrade→20`) contaminates.
- **Verdict:** falsified (unstable / colliding).

### H5 — Strict elev f64 + ElementId framing (`tag=0`, `id≥256`)

- **Einhoven:** resolved 1/4 elevations; top ids are powers-of-two noise.
- **Core Interior (IFC storey elevs −40…185.5):** resolved **0/15**; tops
  are `16368`, `3220176896`, `256`, `65536` — not Level ElementIds.
- **Verdict:** falsified.

### H6 — Floor plan-loop neighbourhood ids

- **Einhoven:** 22 distinct nearby ElemTable ids; no stable shared Level id
  across the 2 floor loops (`m_elem_table_bound=false` already).
- **Verdict:** no Floor→Level join evidence.

### H7 — Companion IFC ElementId properties

`2024_Core_Interior.ifc` emits `IfcBuildingStorey` names/elevations but
**zero** `Element Id` / Revit-id property values usable as ground truth.

### H8 — AProperty host joins (#35 secondary)

Production `iter_elements`:

| File | elements | AProperty* | `parameters≠[]` |
|---|---:|---:|---:|
| Einhoven | 74 | 0 | 0 |
| Core Interior | 3353 | 0 | 0 |

`Global/Latest` `scan_candidates` (min_score=0): **0** AProperty*/Parameter*
hits on both files. Host↔AProperty joins have nothing to join on these
corpora.

## Production honesty (unchanged)

- `LevelStoreyBind` remains empty when Levels lack ElementIds.
- Floors/Rooms stay **Unassigned · no Level ElementId bind**.
- ArcWalls continue elevation-based storey grouping (pre-existing).
- Do **not** invent Door/Window/Wall (#31 thickness still without evidence).

## Artifacts

- `examples/probe_level_elementid_recovery.rs` — multi-hypothesis probe
- `examples/probe_level_elementid_bind.rs` — earlier ArcWall proximity probe
  (also INSUFFICIENT on Einhoven)
- This report

## Recommended next steps (outside this negative)

1. Locate a **tagged** Level/Datum subtype or a ContentDocuments /
   Directory category discriminator that isolates Level ElementIds from
   ElemTable.
2. Decode real `LevelAssociationCell` envelopes (length/magic), not
   free-scan proximity — RE-14.2 kept-ratio work may help.
3. Seek additional corpora where Formats schema includes `Level` /
   `DatumPlane` class entries.
4. For #35: find AProperty* instance envelopes in Partitions (not only
   Global/Latest tag scan) before attempting host joins.
