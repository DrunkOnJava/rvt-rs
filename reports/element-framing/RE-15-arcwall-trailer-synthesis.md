# RE-15 ArcWall trailer synthesis — ElementId / elevation / type recovery

**Date:** 2026-04-28
**Branch:** `cursor/arcwall-finish-line-44c9`
**Scope:** Decode the 177 B post-core trailer on Revit 2023 standard
ArcWall records (tag `0x0191`, variant `0x07fa`, singleton stride 292).
Corpus: Einhoven `Partitions/5` (24 standard walls).

Sibling RE-15 geometry probes (doors / slabs / openings) live on a
separate branch and must not be conflated with this ArcWall finish-line
slice.

## TL;DR

The trailer **does** carry a validated ElementId and a shared type-symbol
handle, plus a base-elevation f64 that mirrors the core start Z.
**Thickness is not in the trailer.** Level ElementId at `+0x116` does
**not** correlate with base elevation and is left undecoded.

| Field | Offset | Confidence | Notes |
|---|---|---|---|
| ElementId | `+0x10e` (+ echo `+0x11c`) | **High** | 22/24 equal echoes; 23/24 in ElemTable |
| TypeId candidate | `+0xfe` | **High (identity) / Medium (semantics)** | Constant `0x217a` on all 24; in ElemTable |
| Base elevation | `+0xf6` | **High** | Equals core start Z on every record |
| Height | core `\|ez−sz\|` | **High** | Always ≈6.562 ft (2 m) on Einhoven; not a trailer f64 |
| Thickness | — | **Absent** | No plausible width in trailer f64 sweep |
| Level ElementId | `+0x116` candidate | **Rejected** | Values `0x104`/`0x114` do not cluster by elevation |

## Wire map (singleton trailer)

```
+0x73 .. +0x9d   fixed padding / FF sentinels / schema-family echo
+0x9e            f64 1.0 (fixed)
+0xa6 / +0xb6    ±tiny denormal (not thickness)
+0xde            f64 1.0 (fixed)
+0xe6 / +0xee    plan point related to wall mid/end (not required for IFC)
+0xf6            f64 base elevation (= start Z)
+0xfe            u32 type-symbol candidate (0x217a on Einhoven)
+0x10e           u32 ElementId
+0x112           u32 hash-like (not an ElemTable id)
+0x116           u32 unstable candidate (not decoded as Level)
+0x11a           u16 tag-like 0x0819
+0x11c           u32 ElementId echo
```

## ElemTable linkage

For each validated trailer ElementId `E`, `Global/ElemTable` contains a
row with `id_primary == E`. Payload u32s at body+12 / body+16 commonly
read as `(4|5, 5)` — consistent with residence in `Partitions/5`, but
**not** a byte offset. The ElementId → `(partition, offset)` map is
therefore built from the partition scan (`partition_arc_walls`), then
joined to ElemTable by id.

## Product consequences

1. Shared `partition_arc_walls` API feeds IFC + diagnostics (no IFC-only scan).
2. IFC storeys come from distinct ArcWall base elevations; when RE-15/#86
   partition Level-like strings match confidently (`Level N`, `Roof`,
   `Ground floor`), those names replace elevation fallback labels
   (helper adapted from PR #117 `partition_name_candidates`).
3. IFC height uses core Z delta only — no invented 10 ft.
4. Thickness remains unresolved; RE-15/#88 (PR #117) falsified exact
   4/6/8/10/12″ trailer widths (conf. 0.80). IFC depth uses a named
   placeholder + diagnostics warning until WallType width join exists.
5. Openings / 2024 `ArcWallRectOpening` 60 B index stay on PR #117 — out of scope here.
6. 2024 ArcWall instance decode stays version-gated off the 2023 envelope.

## Cross-links from PR #117 (RE-15 geometry probes)

| Finding | Action on this branch |
|---|---|
| #86 Level/Material partition strings | Adapted `partition_name_candidates`; wired storey **names** |
| #88 no exact inch thickness in trailer | Kept unresolved-thickness diagnostics (no decode invent) |
| #89 openings are 60 B index, not W×H | Left to #117 — not expanded here |

## Artifacts

- `examples/probe_arcwall_trailer.rs` (histogram / f64 / u32 sweeps)
- `examples/probe_arcwall_trailer_decode.rs` (per-record field dump)
- `src/arc_wall_record.rs` trailer decode
- `src/partition_arc_walls.rs` shared iteration + storey helpers
- `src/elem_table.rs` `index_by_element_id` / `link_arcwall_element_ids`
