# RE-92 — A stair run's "End with Riser" setting, and monolithic runs

**Date:** 2026-09-28
**Issues:** #357
**Result:**
- A stair run's element data holds the fields of the schema class StairsRun, in schema order:

  ```text
  f64 × 7   bottom elevation, top elevation, extend below base,
            extend below tread base, run width, left and right stringer width
  u32       top riser index
  u8 × 3    centre mark visible, begin with riser, end with riser
  ```

  It is read as the first such record in the run's first 4 KB whose bottom elevation and run width equal the run's sketch (RE-52), to 1e-6 ft. On Snowdon Towers it is read on all 41 runs with a sketch.
- "End with Riser" is 0 on exactly the three runs whose riser lines outnumber their risers: 1563804, 1647253 and 2234674. It is 1 on the other 38. A run ending with a tread has one riser line more than risers, and its last tread runs to the last riser line.
- A monolithic run is one cast body. Its risers lean from each step's inner corner up to the nosing, and its underside lies the type's structural depth below the line through the inner corners, square to the pitch. The floor cuts it off, and the last riser line closes it with a vertical face.

| Snowdon Towers' 43 stair runs | main | RE-92 |
|---|---:|---:|
| drawn as their steps | 34 | 38 |
| every drawn vertex on Revit's mesh, and every Revit vertex on or in the drawn run | 30 | 33 |
| as above, except the nosing's lower front edge (Revit's nosing profile) | 3 | 4 |
| as above, except the first riser below a landing (RE-52 §4) | 1 | 1 |

The four runs drawn now:

| run | type | ends with | riser lines, risers drawn | against Revit's mesh (ft) |
|---:|---|---|---|---|
| 1563804 | 1565035, upright risers | a tread | 3, 2 | drawn 0.083 (nosing edge), Revit 0 |
| 2234674 | 168815, monolithic | a tread | 13, 12 | 0, 0 |
| 2061630 | 168815, monolithic | a riser | 17, 16 | 0, 0 |
| 1647253 | 951855, monolithic "Solid Base" | a tread | 3, 2 | 0, 0 |

-----

## 1. The record

StairsRun's fields in `Formats/Latest` are seven `f64` (`m_bottomElevation`, `m_topElevation`, `m_extendBelowBase`, `m_extendBelowTreadBase`, `m_actualRunWidth`, `m_leftStringerWidth`, `m_rightStringerWidth`), then a `u32` `m_topRiserIndex`, then three `bool`s: `m_centerMarkVisible`, `m_beginWithRiser` and `m_endWithRiser`. A run's element data holds them in that order with no tags between them. The first place in the data's first 4 KB where:
- seven finite `f64` have the sketch's base elevation first and its width fifth;
- the top elevation is above the bottom;
- a `u32` below 300 follows;
- three bytes of 0 or 1 follow that,

lies 665 to 2953 bytes into the data. On all 41 sketched runs the centre mark and begin-with-riser flags are 1. The two spiral runs have no straight sketch and are not read.

Revit's built-in parameters `STAIRS_RUN_BEGIN_WITH_RISER` (−1151316) and `STAIRS_RUN_END_WITH_RISER` (−1151317) name the same settings. Their values are not stored next to those ids.

## 2. The profiles

Riser lines are `u₀ = 0 < u₁ < … < u_L` along the climb, with riser height `h`, tread thickness `t`, riser thickness `r` and nosing `N`, all in feet.

**Separate treads and upright risers, ending with a tread (1563804).** The run has `R = L` risers. Riser `k` stands at `u_{k−1} + N` as RE-52 draws it. The top tread spans from `u_{R−1}` to `u_R`, from `R·h − t` to `R·h`, over the last riser's back. A slanted run ending with a tread is not measured and is not drawn.

**Monolithic, slanted risers (types 168815 and 951855).** Every monolithic run is drawn with `R = L` risers, whichever way it ends:
- riser `k` leans from the inner corner `(u_{k−1} + N, (k − 1)·h)` to its nosing `(u_{k−1}, k·h)`;
- tread `k` runs from the nosing to the next inner corner, and the top one to `(u_R, R·h)`;
- the underside is `z(a) = (a − u₀ − N)·s − D·√(1 + s²)`, with pitch `s = R·h / (u_R − u₀)` and structural depth `D`;
- the underside ends on the floor at `a₀ = u₀ + N + D·√(1 + s²) / s`, and the floor runs back to the first inner corner;
- the last riser line is a vertical face from the top tread down to the underside, or to the floor where the underside at `u_R` is below it.

Run 2061630's setting is "End with Riser", and its stair records 17 risers of 0.375 ft (RE-47). Yet Revit's body of the run stops at 6.0 ft = 16 × 0.375 and ends with a tread at the 17th riser line, as the two monolithic runs ending with a tread do. So a monolithic run's body does not follow the setting, and the reader only requires it to be read. The "Solid Base" type 951855 has a structural depth of 2 ft, so on 1647253's two risers the underside lies below the floor throughout, and the body closes straight down to the floor.

## 3. What this does not do

- Monolithic runs are measured on two types, and runs ending with a tread on three runs, all in one file. No other local file has a stair.
- A slanted run ending with a tread, a monolithic run without slanted risers, runs without risers (3 on Snowdon) and spiral runs (2) keep their record box.
- The reader runs on Revit 2024 only, like RE-47 and RE-52.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re92_run_ends -- "Snowdon Towers Sample Architectural.rvt" --json runs.json
python3 tools/re/stair_runs_vs_ifc.py runs.json "Snowdon Towers Sample Architectural_IFC4.ifc"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/stair_runs_vs_ifc.py runs.json model.ifc
```

- `stair_runs_vs_ifc.py` takes Revit's `IfcStairFlight` with each run's Tag, meshed by IfcOpenShell 0.8.5 and moved into internal feet through the export's site placement. It measures every drawn vertex against Revit's nearest vertex, and every Revit vertex against the drawn outline, in the run's side-view plane. It also checks that every Revit vertex lies within the run's width.
- rvt-rs's own IFC of the four new runs, meshed the same way, equals the drawn profiles to 1e-5 ft.
- **Snowdon Towers:** only those four `IfcStairFlight` bodies change (`partition_stair_run_sketch`, 34 to 38).
- **Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven and the MIT house (2024, 2025):** byte-identical to main's apart from the timestamp.
