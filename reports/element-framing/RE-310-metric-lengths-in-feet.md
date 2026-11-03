# RE-310 — How a metric length is stored in feet

**Date:** 2026-11-03
**Issues:** #155 (Discussion #112's warning that Revit computes feet as `mm / 25.4 / 12`, one unit in the last place from `mm / 304.8`); builds on RE-77 and RE-93
**Artefacts:** the reference models Measure fetches and pins (Core Interior, the BIM4LCA office, puzzbobb's R2025 models, the four RE1 models, Autodesk's sample projects 2016 to 2027); none is committed.
**Probe:** `probes/examples/probe_re310_metric_feet_ulp.rs`
**Runs:** Measure [41641337164](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41641337164) (the probe at 6bfb4667, metric lengths only; the figures below), after [41639952446](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41639952446) (12ec5cd4, by parameter) and [41636931034](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41636931034) (2c67e876, every entry on 66 files)
**Result:** the warning holds, the rule does not. Both conversions are stored, in every file with enough metric lengths, and within one parameter of one file: neither is the way Revit stores a metric length. An exact-bit search for a millimetre value has to try both, and from 13% to 40% of the metric lengths whose candidates differ, by file, match neither.

## 1. The question

A length in a type's value block is `f64 feet · ff×8 · i64 parameter` (RE-93). A metric model's lengths are typed in millimetres, so the stored double is some conversion of a whole or tenth millimetre. For most values every conversion rounds to the same double; for some, `mm / 304.8` and `mm / 25.4 / 12` differ by one unit in the last place, and a search for the bits of one misses the other.

## 2. The probe

`probe_re310_metric_feet_ulp` reads every `f64 · ff×8 · i64` entry after a declared element's value block (`partition_length_entries`, RE-93), once per owner, parameter and value. It keeps a value whose millimetres (value × 304.8) are whole or tenth to within 1e-6 mm, and leaves out a value that is a whole number of 1/256 inches: an imperial value, which `mm / 304.8` reproduces trivially (304.8 mm is 1 ft exactly). For each kept value it computes four candidates, `div` (`mm / 304.8`), `chain` (`mm / 25.4 / 12`), `mul` (`mm * (1 / 304.8)`) and `inch` (`mm * (1 / 25.4) / 12`), and where they do not all give the same bits counts which of them the stored bits equal. It counts by what the entry's i64 is: a negative BuiltInParameter id, a declared ElementId (a family or shared parameter), or anything else; and per parameter, the values matching `div` and not `chain` and the reverse.

The first run counted imperial values as well and was dominated by them (1.0 ft is 304.8 mm). It is kept only as the census that Revit 2016 to 2023 files have no such entry: all 28 such files Measure fetches report 0, as does Einhoven.

## 3. Measured

Metric lengths where the four candidates do not give the same bits, by which candidates the stored bits equal ("both" is `div` and `chain` equal to each other but not to `mul` or `inch`):

| file | metric lengths | candidates agree | differ | `div` only | `chain` only | both | `mul` or `inch` only | none |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior (2024) | 307 | 142 | 165 | 77 | 27 | 0 | 6 | 55 |
| BIM4LCA office, as built (2024) | 6,765 | 3,656 | 3,109 | 1,418 | 939 | 17 | 41 | 694 |
| R2025 00_Blank (2025) | 18 | 8 | 10 | 2 | 2 | 0 | 0 | 6 |
| R2025 12_Rooms_Door_Windows (2025) | 31 | 18 | 13 | 3 | 2 | 0 | 0 | 8 |
| RE1 Architecture (2025) | 210 | 98 | 112 | 28 | 35 | 0 | 4 | 45 |
| RE1 Electrical (2025) | 85 | 57 | 28 | 4 | 14 | 0 | 1 | 9 |
| RE1 Mechanical (2025) | 473 | 276 | 197 | 19 | 157 | 0 | 0 | 21 |
| RE1 Plumbing (2025) | 296 | 175 | 121 | 101 | 10 | 0 | 0 | 10 |
| rac advanced (2024 sample) | 2,307 | 1,262 | 1,045 | 518 | 285 | 6 | 41 | 195 |
| rme advanced (2024 sample) | 16,347 | 10,398 | 5,949 | 1,305 | 3,680 | 34 | 167 | 763 |
| rst basic (2024 sample) | 1,163 | 706 | 457 | 182 | 116 | 3 | 4 | 152 |
| Snowdon Towers architectural (2024 sample) | 87 | 44 | 43 | 17 | 0 | 0 | 0 | 26 |
| rme advanced (2027 sample) | 16,032 | 10,132 | 5,900 | 1,307 | 3,633 | 34 | 167 | 759 |

- **Both forms in one file.** Every file with more than a few dozen metric lengths holds both. The share varies by file: the BIM4LCA office and RE1 Plumbing mostly `div`, RE1 Mechanical and Autodesk's MEP samples mostly `chain`.
- **Both forms in one parameter.** On the office as built, built-in Height (−1001300) is `div` on 31 values and `chain` on 117, Width (−1001301) 15 and 50, Rough Width (−1001305) 76 and 12. Family parameter 343423 is `chain` on all 102 of its values that differ and 343424 is `div` on 86 and `chain` on 33. On rme advanced, family parameters such as 358415 (69) and built-in −1001320 (60) are `chain` only.
- **Neither.** 694 of the office's 3,109 and 763 of rme advanced's 5,949 match no candidate. In the probe's sample lines such a value is most often one unit in the last place below `div`, others tens of units from it, and some about 3×10⁸ units, which is still within the 1e-6 mm the probe rounds to: those are not a conversion of the millimetre value they round to at all. A value computed from others (a formula, a derived length) would do this, but nothing here shows which.
- **Totals.** On the office as built, the probe kept 6,765 metric lengths out of 646,371 entries (633,461 imperial-exact); on Core Interior 307 of 376,328.

## 4. What it changes

No reader in rvt-rs searches for a length by its bits converted from millimetres; the RE-93 readers read the stored double. A probe or reader that does has to try at least `div` and `chain`, and must not treat a miss as absence: 13% to 40% of these values match neither.

## 5. Not claimed

- What decides the form: the unit the value was typed in, a formula, the family document's units, a load into the project. The per-parameter split says it is not the parameter.
- That the 1/256-inch filter separates metric from imperial input exactly: a millimetre value that is also a whole number of 1/256 inches (127 mm, 5 in) is left out.
- Discussion #112's other claims (two record shapes, extrusion and revolution BuiltInParameters, shared-parameter names through PartAtom GUIDs): RE-77 and RE-93 read two entry shapes, the text `i64 · u32 n · UTF-16` and the length `f64 · ff×8 · i64`; the rest is not measured here.
- That every `f64 · ff×8 · i64` entry is a parameter: entries whose i64 is neither a BuiltInParameter nor a declared ElementId are counted in the totals (on the office as built, 625 of the 3,109 that differ; 460 are built-in parameters and 2,024 declared ones) and apart in the probe's `kind other` lines.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=6bfb4667 -f base=none -f nordic=true -f r2025=true \
  -f samples=true -f sample_years="2024 2027" -f probe=probe_re310_metric_feet_ulp -f scorers=none \
  -f models="core-interior re1-* nordic-office-as-built r2025-00_Blank r2025-12_Rooms_Door_Windows sample-2024-* sample-2027-*"
```

Each model's `probe.txt` in the `measure-head` artifact has the `census`, `disagree`, `kind`, `param` and `sample` lines.
