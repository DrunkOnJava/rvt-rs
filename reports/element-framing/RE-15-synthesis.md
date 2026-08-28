# RE-15 synthesis — doors / slabs / layers / names toward real geometry

**Date:** 2026-08-28  
**Branch:** `cursor/re15-geometry-probes-44c9`  
**Scope:** Open P0/P1 RE-15 issues #87 (slab profiles), #88 (compound
layers), #89 (door dims + openings), #86 (Material/Level/Space names),
with optional recall baselines for #81–#84.  
**Non-goals:** Do **not** rewrite `src/ifc/mod.rs` ArcWall finish-line
geometry (owned by sibling branch `cursor/arcwall-finish-line-44c9`).

## Corpus

| File | Revit | Partition probed | Inflated size |
|---|---|---|---|
| `Revit_IFC5_Einhoven.rvt` | 2023 | `Partitions/5` | 587 060 B |
| `2024_Core_Interior.rvt` | 2024 | `Partitions/46` | 97 957 200 B |

Env: `RVT_PROJECT_CORPUS_DIR=_project_corpus/Revit` (magnetar-io/revit-test-datasets).

## Probes added

| Example | Issue | Purpose |
|---|---|---|
| `probe_re15_names` | #86 | Bucket partition UTF-16LE strings; list material/level/space-like |
| `probe_re15_rect_opening` | #89 | Filtered-hit envelopes for ArcWallRectOpening / VWallRectOpening |
| `probe_re15_opening_stride60` | #89 / #88 | 60 B opening-index column histogram + ArcWall thickness sweep |
| `probe_re15_compound_layers` | #88 | Trailer f64 / layer-run hunt on ArcWall + HostObjAttr |
| `probe_re15_slab_profiles` | #87 | Closed plan-polyline candidates + AnalyticalModelSlab hits |
| `probe_re15_recall_tags` | #81–#84 | Tag / filtered-hit inventory baselines |

## TL;DR (confidence-scored)

| ID | Finding | Conf. | Status |
|---|---|---|---|
| **F86** | Real Material + Level display names exist as partition string records on both corpora (`Concrete`, `Glass`, `Level 1`, `Ground floor`, …). | **0.90** | **Confirmed** — helper landed |
| **F89a** | 2024 `ArcWallRectOpening` (tag `0x01a7`) has a dominant **60 B fixed index** population (2927/4065 = 72%) with family marker `0x40088204`. | **0.92** | **Confirmed** — decoder landed |
| **F89b** | That 60 B index does **not** carry door width/height (0 door-plausible f64 pairs in-body). | **0.88** | **Confirmed negative** |
| **F89c** | Einhoven has **0** filtered `ArcWallRectOpening` (`0x019c`) hits — 2023 openings (if present) are not this envelope. | **0.85** | **Confirmed** on this sample |
| **F88a** | Standard ArcWall trailers on Einhoven contain **no** exact 4/6/8/10/12″ thickness f64 at record-absolute 8-align. | **0.80** | **H88-1 largely falsified** |
| **F87a** | Naïve closed-polyline scan hits ArcWall coordinate packs on Einhoven (false positives). | **0.75** | Filter required |
| **F87b** | 2024 yields ~998 plan-polyline candidates with building-scale spans; AnalyticalModelSlab has 945 filtered hits — promising but unbound to Floor ids. | **0.45** | Open |
| **F81–84** | Recall baselines recorded; current `main` IFC path emits ArcWall-only on 2023 and 0 typed elements on 2024 (version gate). | **0.70** | Inventory only |

## #86 — Real names (Material / Level / Space)

### Hypotheses

- **H86-1** (prior 0.55 → **posterior 0.90**): names are recoverable from
  `object_graph::string_records_from_partitions` via display-name filters.
- **H86-2** (prior 0.40 → posterior 0.35): names sit adjacent to
  HostObjAttr envelopes — not yet evidenced; deferred.

### Evidence

`probe_re15_names` on Einhoven (9276 strings) / 2024 (72551 strings):

- Display-name bucket: ~47.7% / ~36.2%.
- Material-like unique: 63 / 144 (includes `Concrete`, `Aluminum`,
  `Glass - Lime Window`, `Gypsum-Plaster`, `Masonry - Brick`, …).
- Level-like unique: 3 / 18 (includes `Level 1`, `Ground floor`,
  `Level 3 - Wall Layouts 1`, `Roof`).
- Space-like hits are noisier (OmniClass occupancy sentences mix with
  short labels like `Lobby` / `Office`).

### Decoder step landed

`src/partition_name_candidates.rs` — classify + collect helpers.  
**Not yet:** binding a candidate string to a Material/Level/Space
element id (needs ElemTable / parameter join).

### Next decoder steps

1. Join `Level 1`-class strings to Level element ids via ElemTable
   tuples + placement records.
2. Prefer Material names that co-occur with `HostObjAttr` /
   `m_renderStyleId` neighbourhoods.
3. For Space, filter OmniClass sentences (`len > 48`) — already started
   in the helper.

## #89 — Door dimensions + openings

### Hypotheses

- **H89-1** (prior 0.60 → **split**): filtered tag + family marker marks
  real opening records. True for 2024 **index** population with marker
  `0x40088204` (note: **not** ArcWall’s `0x00088004`). False for
  “geometry payload” expectation.
- **H89-2** (prior 0.50 → **0.15** on index pop.): width/height f64s in
  first 128 B — falsified for stride-60 index records.

### 2024 index envelope (landed)

`src/rect_opening_index.rs` — version-gated to Revit 2024:

```
+0x00 u16 tag = 0x01a7
+0x02 u16 pad = 0
+0x08 u32 index (0,1,2,…)
+0x10 u32 family_marker = 0x40088204
+0x14 u32 related_id_a (even, +2 per record)
+0x18 u32 = 0x0546
+0x32 u32 = 4
+0x36 u32 related_id_b
+0x3a u16 = 0x0248
```

Stride = 60. Tests in `tests/re15_geometry_invariants.rs` require
≥1000 decodable records on Core Interior Partitions/46.

### Next decoder steps

1. Resolve `related_id_a` / `related_id_b` through ElemTable → host wall
   / family instance.
2. Hunt door width/height in **FamilySymbol** / type-parameter blobs
   referenced by those ids (not in the index row).
3. Re-probe `VWallRectOpening` (`0x01a8`) variable-sized hits — hex
   shows denser f64 content than the index population.
4. On 2023, inspect compound ArcWall (`variant 0x0821`) sub-markers
   `21 08` / `70 08` from RE-14.3 for embedded openings.

## #88 — Compound-layer thicknesses

### Hypotheses

- **H88-1** (prior 0.55 → **0.20**): total thickness is an f64 in the
  standard ArcWall trailer. **Falsified** for exact common imperial
  widths at record-absolute 8-byte alignment on Einhoven (0 hits for
  4/6/8/10/12″).
- **H88-2** (prior 0.35 → 0.30): layer runs of 2–6 f64s — **no hits** in
  standard/compound ArcWall windows probed so far.

### Evidence

- Current IFC placeholder depth = `8/12` ft (`src/ifc/mod.rs`
  `arcwall_geometry_from_record`) — unchanged here on purpose.
- HostObjAttr “real record + marker” filter yielded 0 hits with the
  narrow window used; RE-14.1 shared-suffix search needs a dedicated
  revisit.

### Next decoder steps

1. Decode `WallType` / compound-structure parameter storage (likely
   outside ArcWall instance records — type-level).
2. Expand HostObjAttr real-record filter using RE-14.1 suffix
   `76 05 00 00 … 04 80 08 00` rather than requiring marker in +4..+32.
3. Do **not** fold thickness into ArcWall core types on the finish-line
   PR; keep a separate version-gated type decoder.

## #87 — Slab boundary profiles

### Hypotheses

- **H87-1** (prior 0.45 → 0.35): closed plan polylines in partition
  bytes. **Contaminated** on Einhoven by ArcWall coord packs
  (e.g. points `(26.486, 6.562)` match RE-14.3 wall endpoints).
- **H87-2** (prior 0.40): CurveDriver / AbsCurveGStep adjacency — not
  yet probed in depth.
- **H87-3** (prior 0.30 → 0.40): AnalyticalModelSlab tags are abundant
  on 2024 (`0x0132`, 945 filtered hits) but hex looks like table /
  sentinel structure, not loops.

### Evidence

- Einhoven polyline “loops” often 4-point repeats of wall geometry.
- 2024: ~998 candidates with spans like 45×45 ft, 110×65 ft — worth a
  **dedup + ArcWall-exclusion** pass before claiming Floor profiles.

### Next decoder steps

1. Exclude any polyline whose vertices match an ArcWall
   `coords`/`coords_dup` pair within epsilon.
2. Require ≥5 unique vertices and signed area above a floor-plate
   threshold.
3. Bind surviving loops to Floor ElemTable ids via spatial containment
   of placement points.

## #81–#84 — Recall baselines (optional)

`probe_re15_recall_tags` inventory (filtered hits):

| Class | Einhoven tag / hits | 2024 tag / hits |
|---|---|---|
| ArcWall | `0x0191` / 32 | `0x019c` / 919 |
| ArcWallRectOpening | `0x019c` / 0 | `0x01a7` / 4065 |
| VWallRectOpening | `0x019d` / 0 | `0x01a8` / 717 |
| AnalyticalModelSlab | `0x0128` / 3 | `0x0132` / 945 |
| HostObjAttr | `0x006b` / 324 | `0x006b` / 15200 |

Current exporter on this tree: Einhoven → 24 `IFCWALL` (standard
ArcWall decode); 2024 → 0 walls (version gate). Placement-linker recall
percentages cited in the issues are **not** implemented on `main` yet —
these tag counts are the partition-side baseline any future recall lift
must reconcile.

## Small decoders landed (ArcWall-safe)

1. `rect_opening_index` — 2024-only, no interaction with
   `arc_wall_record` core fields.
2. `partition_name_candidates` — pure string heuristics.

No changes to `src/ifc/mod.rs` ArcWall emission path.

## How to reproduce

```bash
git clone --depth 1 https://github.com/magnetar-io/revit-test-datasets _project_corpus
export RVT_PROJECT_CORPUS_DIR="$PWD/_project_corpus/Revit"

cargo run --release --example probe_re15_recall_tags
cargo run --release --example probe_re15_names
cargo run --release --example probe_re15_rect_opening
cargo run --release --example probe_re15_opening_stride60
cargo run --release --example probe_re15_compound_layers
cargo run --release --example probe_re15_slab_profiles

RVT_PROJECT_CORPUS_DIR="$RVT_PROJECT_CORPUS_DIR" \
  cargo test --release --test re15_geometry_invariants -- --nocapture
```

Artifacts from this session: `/opt/cursor/artifacts/re15_*.log`.
