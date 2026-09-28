# RE-127 — Joined-wall lists decide the L joints the join lists leave open

**Date:** 2026-09-28
**Issue:** #238 (RE-26 follow-up); follows RE-70 (join lists)
**Artefacts:** `2024_Core_Interior.rvt` (MIT, sha256 `c805df44…`), measured against `2024_Core_Interior_slim.ifc` (Revit 24.0.20.20, IFC4 ReferenceView), and RE1 Architecture (Revit 2025, MIT) against its IFC2X3 CoordinationView export. Every measurement here ran on GitHub's hosted runners through the Measure workflow; the runs are cited below.
**Probe:** `examples/probe_re127_joined_wall_lists.rs`; scored by `tools/re/joined_wall_lists_vs_ifc.py`.
**Status:** positive.

## 1. The lists

Beside RE-70's join lists (`07 00 00 00 · tag · 02 · (0) · count · count × (u32 · u64 ElementId · u32)`), a wall's element data holds lists opening `01 00 00 00` with the join lists' tag + 2 (`0x0b9f7f` beside `0x0b9f7d` on Core Interior and RE1, as on the Revit 2026 house, RE-126), the same `02 · (0) · count`, and variable-length entries `u64 ElementId · u32 n · n × u32`. They name every wall a wall is joined to: 545 of them on Core Interior's 310 walls with element data, 6 on RE1.

## 2. What they say at L joints

At an L joint (a wall end where exactly one other wall overlapping it in height ends its centreline), Revit's body reaches half the partner's thickness past the joint (through), stops that far short (stopped), or neither. Run 36493176602, Core Interior:

| Revit's body | RE-70 decides | both walls' joined lists name each other | ends |
|---|---|---|---:|
| through | through | yes | 122 |
| stopped | stopped | yes | 122 |
| stopped | undecided | yes | 9 |
| ends at the joint (reach 0) | undecided | yes | 9 |
| other | undecided | yes | 12 |
| other | undecided | no | 8 |

Naming is symmetric, so the lists' presence cannot say which wall runs through. The entries' words can: at the 244 ends RE-70 decides, a wall's entry for its partner is one run of consecutive values (such as `417 418 419 420`) at 0 of the 122 through ends and at 111 of the 122 stopped ones.

The 18 undecided ends are #238's nine remaining walls and their partners, one per storey of the building's core (20816, 29873, 55754, 59334, 60215, 61096, 61977, 62858 against 20800 and its copies, and 80743 against 80747). The same corner on the top storey (63739) carries an RE-70 join list, and the eight below do not. At each, one wall's entry for the other is consecutive (`417 418 419 420`, or `229 230` for 80747) and the other's is not (`33 36 146 147`, `77 79`). Revit stops the consecutive one at the other's near face and ends the other at the joint, its line's end: 18 of 18. The rule predicts nothing at the 20 other undecided ends, where both entries are consecutive or neither wall names the other.

## 3. The rule and what it changes

`butt_joins` now decides a perpendicular L joint the join lists leave open from the two walls' joined-wall entries: where exactly one is consecutive, that wall stops at the other's near face with every layer and the other ends at the joint. The entries come from `partition_compound_structure::scan_wall_joined_entries`.

Run 36493826658, this branch against `main`, Core Interior against Revit's export:

| | `main` | RE-127 |
|---|---:|---:|
| walls with both ends within 0.001 ft (`wall_bodies_vs_ifc.py`) | 351 of 360 | **360 of 360** |
| ends within 0.001 ft (`wall_ends_cv_vs_ifc.py`) | 711 of 720 | **720 of 720** |

No other scorer's output changes. The IFCs of Einhoven and the four RE1 models are identical to `main`'s, apart from the export time.

## 4. Not measured

Snowdon Towers and the Revit 2026 house carry the same lists but have no redistribution licence, so they are not measured on the hosted runners. The rule fires only at perpendicular L joints the join lists leave undecided.
