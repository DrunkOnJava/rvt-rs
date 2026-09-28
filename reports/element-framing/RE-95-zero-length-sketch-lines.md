# RE-95 — Zero-length sketch lines, and Core Interior's 20 shading-device plates

**Date:** 2026-09-28
**Issues:** #233
**Result:**
- A sketch line whose record box is a single point is a zero-length segment. It carries no bounded line (RE-50) and adds no edge to its sketch. Where its point is an end of another of the sketch's recorded lines, it is now left out.
- Core Interior's 20 plates that Revit's IFC4 export writes as `IfcShadingDevice` each own 57 sketch lines. 56 carry a recorded line, and their ends close into a 28-vertex outer loop and a 26-vertex void. The 57th is a zero-length line on one of those ends. Before, it made each plate decline, and the plates kept their record box. They now carry their outline.

| Core Interior's 20 shading devices against `2024_Core_Interior_slim.ifc` (`plan_profiles_vs_ifc.py`) | main | RE-95 |
|---|---:|---:|
| upward-face plan area within 0.1% of Revit's | 0 | 20 (0.00%) |
| every upward-face vertex within 0.01 ft of Revit's, both ways | 0 | 20 (0.000 ft) |

Before, their area was 22,174 ft² against Revit's 3,489 ft²: the record box of a zigzag plate.

-----

## 1. The zero-length line

On each plate, record 21010 (for plate 20953) has a box from (167.0, 120.333, 91.0) to the same point, and no bounded line in its data. Two of the plate's recorded lines end at that point. RE-25 already rejected a zero-extent box in the box solve, and RE-50's line path required every line to be read, so the plates had no profile on either path. #233 had put their decline down to "21 diagonals with no axis-parallel anchor". Their diagonals' recorded ends (RE-50) settle that, and the zero-length line was the only thing left.

A zero-length line is left out only where:
- its box is a point (every extent within 1e-6 ft);
- it has no line of its own;
- its point is an end of another of the element's recorded lines.

Every other requirement of RE-50 still holds: every line level, its ends in its box, every vertex of degree two.

## 2. Other files

`examples/probe_re95_zero_length_sketch_lines.rs` lists every sketch whose boxes do not close:

| file | owners that close once zero-length lines are left out | of them exported |
|---|---:|---:|
| Core Interior | 21 | 20 (the plates) |
| Snowdon Towers structural (local only) | 2 | 2 slabs (722041, 773528) |
| MIT tutorial house 2024 (local only) | 1 | 1 slab (171319) |
| Snowdon Towers architectural | 0 | 0 |

No Revit export of Snowdon Towers structural or the MIT house is held, so their three slabs are not measured against Revit. Each outline spans exactly its record box. The MIT house's 2025 copy of slab 171319 keeps its box: its sketch does not close there, so the two releases cannot be compared.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| 2024_Core_Interior.rvt (MIT, magnetar-io/revit-test-datasets) | `c805df44…` |
| 2024_Core_Interior_slim.ifc (Revit's export, same repository) | `bfdf36ff…` |

```bash
cargo run --profile ci --example probe_re95_zero_length_sketch_lines -- 2024_Core_Interior.rvt
./target/ci/rvt-ifc 2024_Core_Interior.rvt -o model.ifc
python3 tools/re/plan_profiles_vs_ifc.py model.ifc 2024_Core_Interior_slim.ifc
```

`plan_profiles_vs_ifc.py` moves both exports into Revit's internal feet through each file's own site placement. It compares each element's upward-facing faces: their plan area, and the distance from every vertex to the nearest vertex of the other file's.

- **Core Interior:** only the 20 `IfcShadingDevice` bodies change. The 80 slabs are unchanged, 79 within 0.1% of Revit's area as before. Both witness observations are refreshed; the verdicts PASS with all replays matching.
- **Snowdon Towers structural, MIT house 2024:** only the three slabs above change.
- **Snowdon Towers architectural, the four RE1 models, Einhoven, MIT house 2025, Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp.
- `tests/iter_elements_typed.rs::core_interior_2024_slab_plan_profiles` now requires all 100 Core plates to close, the 20 shading devices as 28-vertex rings with one void.
