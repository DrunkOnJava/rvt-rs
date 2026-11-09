# RE-373: A type's value block ends where the next native record starts

**Date:** 2026-11-08
**Issues:** #1489; follows RE-77 and RE-93 (type value blocks), RE-187 (door and window sizes), RE-354 (the family type table's text form) and RE-346 (the gate that found the gap).
**Artefacts:** Autodesk's `rac_basic` sample saved by Revit 2026 (`racbasicsampleproject.rvt`, sha256 `bc547fe94274a17abbea89330fcf8800b85652e0b8e25562b153c102af3d99dc`) and 2027 (sha256 `c9ce4d013b630028973e4549d3be4b38b3640641948b035680eeea7306491a01`), pinned in `research/autodesk-sample-projects.tsv` and fetched by Measure, never stored.
**Probe:** `probes/examples/probe_1489_door_size_entries.rs`.
**Status:** positive: on the file that shows it, and on the one held-out Revit export the rule changes (counts only); no other export moves.

## 1. The question

On `rac_basic`'s 2027 save two `Single-Flush:800 x 2100` doors (431064 and 485679) write no `Qto_DoorBaseQuantities` Width and Height, which its 2025 and 2026 saves write (#1489, from #1454's same-project gate). Their cut loops are read, so `OverallWidth` and `OverallHeight` stay. The quantities are written only where the symbol that draws the instance and the instance's type hold the same Width and Height (`type_door_sizes`), so on 2027 one of the two was not read.

## 2. What the scan sees

`partition_type_parameters` gives a length entry (`f64 · ff × 8 · i64` parameter) to the value block whose mark (RE-77) comes last before it, and a block holding two values for one parameter gets neither.

- The doors' symbol 431054 holds Width 2.6247 ft and Height 6.8898 ft (800 × 2100 mm) on every save, 2,030 and 1,840 bytes past its block's mark, where the type's other symbols (232780, 235155, 490150) hold theirs.
- On the 2027 save the stretch from 431054's mark to the next mark (13,802 bytes) also holds three groups of four entries: Width and Rough Width 4.9213 ft, Rough Height and Height 8.8583 ft (1500 × 2700 mm), 11,053, 12,158 and 13,263 bytes past the mark. So 431054 holds two Widths and two Heights and `type_door_sizes` drops both.
- On the 2026 save the same three groups lie past symbol 420952's mark, 150 KB into its stretch, and 431054 reads cleanly.

Two bounds were tried first and fail. Every Width entry, owned or not, lies outside the partition's record chain (RE-35), so the chain cannot bound a block. And the 59-byte mark carries no length or count (56 bytes of `0xff` and three zeros).

## 3. Whose the entries are

The native record index (`native_document::RecordIndex`, #1032) holds where every record starts in its partition. Every length entry of these stretches lies inside exactly one record (Measure run 42557901914, head `84dfa9ba`, 2027 save, `Partitions/16`):

| entries | record | class | bytes |
|---|---|---|---|
| 431054's own | 431054 | `FamilySymbol` | 2537513..2546246 |
| the first 1500 × 2700 group | 457479 | `FamilyInstance` | 2547967..2549083 |
| the second | 457534 | `FamilyInstance` | 2549103..2550188 |
| the third | 457542 | `FamilyInstance` | 2550208..2551293 |

The three controls hold theirs inside their own `FamilySymbol` records too (232780 @2031539..2039636, 235155 @2056670..2065403, 490150 @2610547..2619280), each with its block's mark 68 bytes past the record's start. The 1500 × 2700 values are three family instances' own parameters, stored after 431054's record with no mark between.

## 4. The change

A length entry is a block's only while no native record starts between the block's mark and the entry (`bounded_value_blocks`): the length scans of the export path (`type_lengths_at`, behind the door and window sizes and openings and the I sections, and `type_named_lengths_at`, behind the hollow, rectangular and round column sections) end each owned block at the first record start after its mark, where that comes before the next mark. A type's block lies inside its own record and records do not nest, so the cut can drop only entries outside the owner's record.

- The record starts come from the handle's record index. Where no native pass has built it, the reader builds it with a pass that selects no element and decodes nothing, so whether it exists depends on the file alone, never on which reader ran first: a release or history the native reader refuses, or a partition over the stream limit, leaves no index, and the blocks then end at their marks as before.
- The length scans run on Revit 2024 to 2027 only (`supports_revit_version`), inside the native reader's default releases (2023 to 2027), so every file they read can be indexed unless its history or a partition over the stream limit refuses it. Revit 2015 to 2023 do not reach them.
- The RE-354 text scan (`scan_partition`, `FireRating` and `Manufacturer`) is not changed: its table entries lie up to 3.2 MB from their block's mark, far outside any `FamilySymbol` record.
- The public research scans (`scan_partition_lengths`, `scan_partition_length_places`, `partition_length_entries`) keep their mark-only blocks for the probes.

## 5. Measured

Measure run 42558908204, head `1e62821d` against `main` at `9c66e9bd`: the reference models, the BIM4LCA office, the Revit 2025 models, Autodesk's samples of every year and the held-out models, with timing. It had no `racadv` and no BIMcollab model, which are Revit 2015 to 2020 files that this change does not reach.

- **2 of 111 exports move.** `rac_basic`'s 2027 save: the same-project gate's door `Qto_DoorBaseQuantities.Width` and `Height` go from 9 equal and 2 only in the 2025 save to 11 equal and none, so the save's property values the gate compares go from 414 to 418, the 2026 save's. The `same-project gates` check passes on `1e62821d`.
- **The held-out AA house** (Revit 26.4's IFC4X3 export, counts only): `Qto_DoorBaseQuantities.Width`, `Height` and `Area` each go from 11 to 13 of 13 equal, 2 missing to none, so its quantities equal to Revit's go from 200 to 206 of 846. The MCP server's quantity answers equal to the export's go from 24 to 27. It is the one Revit export on which the rule changes a value, and it is out of sample for it: the rule was read off `rac_basic` alone.
- **Nothing else moves.** No property value or quantity on any model goes from equal to different or missing, so RE-354's `FireRating` and `Manufacturer` are unchanged; the other 109 exports are byte for byte `main`'s.
- **Cost.** Export time 0.98 to 1.01 times `main`'s on 68 timed files (0.993 in total) and peak memory 0.95 to 1.06 times (the highest ratio Snowdon Towers HVAC, 215 to 227 MiB). The viewer's wasm memory is not measured.

## 6. What this does not claim

- Whether symbol 420952's own type is 1500 × 2700: its record ends 8,877 bytes past its start, before the three instances' groups, so the change stops the 2026 save giving it their values, and which values its type holds has not been read.
- What the three family instances are beyond their class.
- A block whose owner's own entries lie in a record the index does not place (a group that is not the partition's own bytes): such a record cuts nothing, as before.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2026 2027
cargo run --profile ci -p rvt-probes --example probe_1489_door_size_entries -- samples/2027-racbasicsampleproject.rvt
```

On Measure: `gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f sample_years="2026 2027" -f models=- -f scorers=none -f probe=probe_1489_door_size_entries -f probe_scorer=-`.
