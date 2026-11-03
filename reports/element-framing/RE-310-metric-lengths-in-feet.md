# RE-310 — How a metric length is stored in feet

**Date:** 2026-11-03
**Issues:** #155 (Discussion #112's warning that Revit computes feet as `mm / 25.4 / 12`, one unit in the last place from `mm / 304.8`); builds on RE-77 and RE-93
**Artefacts:** the reference models Measure fetches and pins (Core Interior, the BIM4LCA office, puzzbobb's R2025 models, the four RE1 models, Autodesk's sample projects); none is committed.
**Probe:** `probes/examples/probe_re310_metric_feet_ulp.rs`
**Runs:** Measure [41641337164](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41641337164) (the probe at 6bfb4667, metric lengths only; every figure below), after [41639952446](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41639952446) (12ec5cd4) and [41636931034](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41636931034) (2c67e876), which counted imperial values too
**Result:** the warning holds and the rule does not. Where the conversions give different bits, `mm * (1 / 304.8)` reproduces the most values, or ties for it, on 12 of the 13 files in the table below (32 of the 33 measured), and every value of RE1 Electrical and Snowdon Towers electrical; but every measured file with more than 300 such values also holds values only `mm / 25.4 / 12` or only `mm / 304.8` reproduces, and on the measured files with at least 100 such values 9% to 35% match none of the four. An exact-bit search for a millimetre value has to try several conversions and cannot take a miss as absence.

## 1. The question

A length in a type's value block is `f64 feet · ff×8 · i64 parameter` (RE-93). A length typed in millimetres is stored as some conversion of a whole or tenth millimetre. For most values every conversion rounds to the same double; for some, `mm / 304.8` and `mm / 25.4 / 12` differ by one unit in the last place, and a search for the bits of one misses the other.

## 2. The probe

`probe_re310_metric_feet_ulp` reads every `f64 · ff×8 · i64` entry after a declared element's value block (`partition_length_entries`, RE-93), once per owner, i64 and value. It keeps a value whose millimetres (value × 304.8) are whole or tenth to within 1e-6 mm, and leaves out a value that is a whole number of 1/256 inches: an imperial value, which `mm / 304.8` reproduces trivially (304.8 mm is 1 ft exactly). For each kept value it computes four candidates, `div` (`mm / 304.8`), `chain` (`mm / 25.4 / 12`), `mul` (`mm * (1 / 304.8)`) and `inch` (`mm * (1 / 25.4) / 12`), and where they do not all give the same bits counts which of them the stored bits equal. It counts these by what the entry's i64 is: a negative BuiltInParameter id, a declared ElementId (a family or shared parameter), or anything else (`kind other`, no parameter); and per parameter, the values matching `div` and not `chain` and the reverse.

The block mark the reader finds is the 2024 to 2027 one (`u64` owner, 56 × `ff`); RE-113's 2023 block (`u32` owner, 28 × `ff`) is not read, so no release before 2024 is measured here.

## 3. Measured

Values of a parameter (BuiltInParameter or declared) whose four candidates do not all give the same bits; a column counts the values the candidate reproduces, "alone" those it is the only one to:

| file | differ | `mul` | `div` | `chain` | `inch` | `mul` alone | `div` alone | `chain` alone | none |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior (2024) | 104 | 90 | 60 | 26 | 66 | 6 | 1 | 0 | 12 |
| BIM4LCA office, as built (2024) | 2,484 | 1,799 | 1,269 | 842 | 1,572 | 32 | 40 | 241 | 346 |
| R2025 12_Rooms_Door_Windows (2025) | 8 | 2 | 0 | 2 | 2 | 0 | 0 | 0 | 6 |
| RE1 Architecture (2025) | 57 | 43 | 12 | 28 | 40 | 4 | 0 | 0 | 13 |
| RE1 Electrical (2025) | 12 | 12 | 0 | 12 | 12 | 0 | 0 | 0 | 0 |
| RE1 Mechanical (2025) | 193 | 172 | 17 | 155 | 168 | 0 | 0 | 0 | 21 |
| RE1 Plumbing (2025) | 113 | 101 | 94 | 9 | 95 | 0 | 0 | 2 | 10 |
| rac advanced (2024 sample) | 871 | 544 | 417 | 242 | 398 | 35 | 116 | 3 | 177 |
| rme advanced (2024 sample) | 4,819 | 4,115 | 1,072 | 3,090 | 3,881 | 66 | 51 | 16 | 610 |
| rst basic (2024 sample) | 369 | 233 | 151 | 92 | 196 | 0 | 3 | 0 | 127 |
| Snowdon Towers architectural (2024 sample) | 17 | 2 | 10 | 0 | 10 | 0 | 0 | 0 | 7 |
| Snowdon Towers electrical (2024 sample) | 8 | 8 | 8 | 0 | 6 | 0 | 0 | 0 | 0 |
| rme advanced (2027 sample) | 4,768 | 4,068 | 1,072 | 3,043 | 3,834 | 66 | 51 | 16 | 606 |

- **The most common match** is `mul`: of the values some candidate reproduces, 84% on the BIM4LCA office, 98% on rme advanced and RE1 Plumbing, all 172 on RE1 Mechanical. Snowdon Towers architectural is the exception, 10 `div` and 2 `mul`.
- **No one candidate covers a large file.** On the office, 241 values only `chain` reproduces and 40 only `div`; on rac advanced, 116 only `div`; on rme advanced, 66, 51 and 16 only `mul`, `div` and `chain`.
- **One parameter, several forms.** Across the types that hold it, the office's built-in Height (−1001300) matches `div` and not `chain` on 31 values and the reverse on 117, Width (−1001301) 15 and 50, Rough Width (−1001305) 76 and 12. These pool the parameter's values over every type that holds it, so they say the form is not fixed by the parameter, not that it varies within one type.
- **Neither.** 346 of the office's 2,484 such values, 610 of rme advanced's 4,819, 127 of rst basic's 369. The probe's sample lines (the first 25 per file, every kind) show such a value one unit in the last place below `div` (139 of 383 samples), most others 3 to 999 units from it (192), and one recurring value about 3×10⁸ units from it, which rounds to 6 mm within the probe's 1e-6 mm: not a conversion of 6 mm at all. A value computed from others (a formula, a derived length) would do this, but nothing here shows which.
- **Counts.** On the office as built, 646,371 entries, 633,461 imperial-exact, 6,765 metric, 3,109 differing, of which 2,484 are parameters' values (460 built-in, 2,024 declared) and 625 `kind other`.

## 4. What it changes

No reader in rvt-rs searches for a length by its bits converted from millimetres; the RE-93 readers read the stored double. A probe or reader that does has to try `mul`, `div` and `chain` at least, and must not treat a miss as absence.

## 5. Not claimed

- What decides the form: the unit a value was typed in, a formula, the family document's units, a load into the project.
- That every kept value was typed in millimetres. The selection is numeric: Autodesk's samples are mostly imperial projects whose metric-looking values may come from metric families, and a millimetre value that is also a whole number of 1/256 inches (127 mm, 5 in) is left out.
- Anything before Revit 2024 (§2).
- Discussion #112's other claims (two record shapes, extrusion and revolution BuiltInParameters, shared-parameter names through PartAtom GUIDs): RE-77 and RE-93 read two entry shapes, the text `i64 · u32 n · UTF-16` and the length `f64 · ff×8 · i64`; the rest is not measured here.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=6bfb4667 -f base=none -f nordic=true -f r2025=true \
  -f samples=true -f sample_years="2024 2027" -f probe=probe_re310_metric_feet_ulp -f scorers=none \
  -f models="core-interior re1-* nordic-office-as-built r2025-00_Blank r2025-12_Rooms_Door_Windows sample-2024-* sample-2027-*"
```

Each model's `probe.txt` in the `measure-head` artifact has the `census`, `disagree`, `kind`, `param` and `sample` lines; the table sums the `kind builtin` and `kind declared` lines.
