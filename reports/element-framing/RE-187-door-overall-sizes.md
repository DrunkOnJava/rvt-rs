# RE-187 — A door's OverallWidth and OverallHeight from its type's Width and Height

**Date:** 2026-10-18
**Issues:** #759
**Result:**
- A door type's value block holds its Width (−1001301) and Height (−1001300) as `f64 feet · ff × 8 · i64 parameter` (RE-77), the BuiltInParameters RE-93 reads for windows. On Core Interior and RE1 Architecture they are Revit's `OverallWidth` and `OverallHeight` on every door where they are read: 136 of 136, none different.
- rvt-rs wrote those two attributes only from the opening it cuts (RE-93, RE-94), so all 143 doors and windows of the two models had `$`. A door now takes its type's Width and Height where no opening gives its size. A door whose opening is read (RE-94's rough-size doors) keeps that size, which is what Revit writes there.
- Windows are a measured negative, and keep failing closed: Core Interior's six windows hold 3.3136 × 3.3136 ft in their type's Width/Height and in its rough pair, while Revit writes 4.9213 × 8.8583 ft.

| Revit's `OverallWidth` and `OverallHeight` against rvt-rs's, by `Tag` (`door_window_sizes_vs_ifc.py`) | main | RE-187 |
|---|---:|---:|
| Core Interior doors (132): equal | 0 | 132 |
| Core Interior doors: `$` | 132 | 0 |
| Core Interior windows (6): `$` | 6 | 6 |
| RE1 Architecture doors (5): equal | 0 | 4 |
| RE1 Architecture doors: `$` | 5 | 1 |
| Different from Revit's | 0 | 0 |

-----

## 1. What the type holds

`probe_re187_door_window_sizes` prints each door's and window's candidate sizes: the Width/Height BuiltInParameters, the rough pair (−1001305, −1001304) and family parameters named `Width` and `Height`. Scored against Revit's export by `Tag` within 0.001 ft (Measure run 39116389615 on CI's reference models, reproduced locally):

| Model | Class | Matched | Width/Height | Rough pair | Family `Width`/`Height` |
|---|---|---:|---|---|---|
| Core Interior | IfcDoor | 132 | equal 132 | not read 132 | not read 132 |
| Core Interior | IfcWindow | 6 | differs 6 | differs 6 | not read 6 |
| RE1 Architecture | IfcDoor | 5 | equal 4, not read 1 | not read 5 | not read 5 |

The values agree with Revit's to about 1e-15 (Revit's own unit conversions: 2399.9999999999968 mm for a 2400 mm type), so they are compared within a tolerance, not bit for bit.

## 2. Left alone

- **Windows.** Core Interior's six windows (type 17335, "Standard") hold 3.3136 × 3.3136 ft (1010 mm) as Width/Height and as their rough pair. Revit writes 4.9213 × 8.8583 ft. Their record boxes are 6.3208 × 4.9213 × 8.8911 ft: Revit's width is the box's second extent and its height falls 0.033 ft short of the box. Where their size comes from (an instance-sized storefront or curtain-panel window, for example) is not found here, so a window keeps taking its size only from the opening RE-93 cuts.
- **A door whose type holds no size.** RE1's door 445975 (type 480990, "69" x 72"") holds none of the three candidates and keeps `$`. Revit gives it 6.0101 × 7.6767 ft, its record box's width and height; that is not used, since the box is not the type's size in general.
- **Opening-only doors (RE-84)** are written as their opening alone, not as an `IfcDoor`, and get no size.
- **Doors with an opening read** keep that opening's size: Revit writes the rough size on the 57 Snowdon Towers doors RE-94 cuts at it.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| `2024_Core_Interior.rvt` (magnetar-io/revit-test-datasets) | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| `2024_Core_Interior_slim.ifc` (Revit 2024's export) | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| `RE1-Architecture.rvt` (Drshelden/IFC-ECS) | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| `RE1-Architecture.ifc` (Revit 2025's export) | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |

```bash
cargo run --profile ci --example probe_re187_door_window_sizes -- 2024_Core_Interior.rvt > probe.jsonl
python3 tools/re/door_window_sizes_vs_ifc.py probe.jsonl 2024_Core_Interior_slim.ifc --list
./target/ci/rvt-ifc 2024_Core_Interior.rvt -o model.ifc
python3 tools/re/door_window_sizes_vs_ifc.py model.ifc 2024_Core_Interior_slim.ifc --list
```

Given an export, the scorer counts the `OverallWidth` and `OverallHeight` rvt-rs writes against Revit's; it runs with the other IFC scorers in `tools/ci/measure-reference-models.sh`. `tests/door_overall_sizes.rs` checks the same sizes on both models (through `RVT_PROJECT_CORPUS_DIR`), and fails on main.

- **Core Interior:** only the 132 doors' `IFCDOOR` lines change, from `$,$` to Revit's values, and each gains `TypeWidth` and `TypeHeight` in `RvtElementRecordGeometry`.
- **RE1 Architecture:** the same for 4 doors.
- **Einhoven and RE1 Electrical, Mechanical and Plumbing:** byte-identical to main's apart from the timestamp.
- **Snowdon Towers:** Measure reads it with `-f samples=true` (#766), but without Revit's export to score against, and that export is local only. Its 57 rough-size doors keep their opening's size. Its other doors now take their type's Width and Height, which is not measured here against Revit's.
