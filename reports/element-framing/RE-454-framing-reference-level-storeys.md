# RE-454: a beam or brace is in the storey of its Reference Level, which its record's second reference list holds

**Date:** 2026-11-13
**Issues:** #1842 (a Revit 2027 structural model: 104 elements on no storey, and 448 on a storey one to six below Revit's, 424 of them beams), #1864 (Autodesk's `rst_basic`: 12 framing members whose Level bind changes between its 2025 and 2026 saves); follows RE-27, RE-59, RE-174 and RE-175
**Artefacts:** the BIM4LCA office files that Measure reads with Revit's own IFC4 export of each (VTT's ARCH archive, CC BY-SA 4.0): the timber office as built and at building permit, the concrete office as built; the BIM4LCA timber housing as built; Autodesk's public samples `rac_advanced` and `rst_basic`, saved by Revit 2024 to 2027 (`research/autodesk-sample-projects.tsv`)
**Scorers:** `tools/re/storey_rules_vs_ifc.py` with `probes/examples/probe_1842_storey_rules.rs` (new); `tools/re/storeys_vs_ifc.py`
**Runs:** Measure [43440608938](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43440608938), [43443555682](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43443555682) and [43445936441](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43445936441) (the probe and its scorer)
**Status:** pending the unfiltered Measure of the change (section 5).

## 1. The question

On the two timber office files, Revit's export puts 63 beams in a different storey from rvt-rs's (`storeys_vs_ifc`, "IfcBeam 63" on each). A reporter's Revit 2027 structural model has 424 beams one to six storeys below Revit's, 470 of its framing records naming one Level at or below the box and one at or above it. Which Level is a beam's storey, and where does its record hold it?

## 2. What the records say

`probe_1842_storey_rules` lists, for every framing record, the Levels of its first counted reference list at `+0x88` (the list the storey bind reads) and of the second list that follows it (the list RE-174 reads a beam's `Reference Level` property from), with the record box's base and top and the storey rvt-rs puts the beam in; the scorer sets each rule against the storey of Revit's export.

On the timber office (as built and at permit, the same on both), 71 beams are listed by the Levels each list names:

| first list | second list | beams | storey of rvt-rs before | the second list's Level, else the first's (highest of several) |
| --- | --- | ---: | ---: | ---: |
| 1 Level | none | 8 | 8 of 8 equal Revit's | 8 of 8 |
| 1 Level | 1 Level | 16 | 0 of 16 | 16 of 16 |
| 2 Levels | none | 47 | 0 of 47 | 47 of 47 |

- **The 47** are the beams RE-59 binds to the base constraint, the lower of their two Levels. One Level lies at or below the box's base and one at or above its top; Revit's storey is the upper. The highest of the two is right for 47 of 47, the lower and the base constraint for none.
- **The 16** name Level 01 in the first list, lie 9 ft above it and just under Level 02, and Revit puts them in Level 02; the second list names Level 02. The 8 beams of the first row name Level 05 and lie 17.7 ft above it, just under the Roof, and Revit puts them in Level 05, not the Roof, so no rule on the first list and the box separates the two groups.
- **The concrete office as built** (8 beams naming one Level) and the timber housing as built (35) are right before and after; the 6 housing beams that name none are not touched.

A beam's storey in Revit's export is its Reference Level. rvt-rs read that Level for the `Reference Level` property (RE-174, RE-175: 83 of 83 beams of this office, and every beam it wrote on four Autodesk samples) and bound the storey from the first list alone.

## 3. Across saves

For every beam that the 2024 and a later save of the same Autodesk project list:

| project | beams in both | the second list's Level, else the first's: equal on 2025 / 2026 / 2027 | the first list's single Level: equal on 2025 / 2026 / 2027 |
| --- | ---: | --- | --- |
| `rac_advanced` | 38 | 37 of 38 on each (the other names none in both) | 38 / 38 / 38 |
| `rst_basic` | 370 | 370 / 370 / 370 | 370 / 360 / 360 |

So the second list holds on Revit 2025 to 2027, which RE-174 had measured on 2024 only, and the Level it gives is stable where the first list's is not: the 12 `rst_basic` tubes of #1864 gain `Top of Parapet` in the first list on the 2026 and 2027 saves, and 10 of them change their single first-list Level there, while the Level this reading gives them is the same on all four saves.

## 4. The change

A framing record takes its Level from the second reference list where that names any Level, else the first (RE-174). Naming one, the beam is bound to it; naming several, to the highest where that is unique (RE-175, `resolve_base_constraint_levels`, source `partition_element_record_reference_level`). The decoder keeps the second list (`PartitionElementRecord::second_references`). Every other class keeps RE-27 and RE-59.

## 5. Rows, registered on #1842 before the Measure of this change was dispatched

Pending.

## 6. Not changed, and what is left out

- Records of other classes that name three or more Levels (64 columns of the reporter's model): no open model with Revit's export holds one, so no rule is written.
- Foundations, floors and stair runs that name no Level (the reporter's 14 slabs, 3 footings and 14 flights on no storey).
- Revit 2015 to 2023 (RE-119 binds a beam to the Level at its box's top there): the second list is not read, and a record's first list that names two or more Levels now gives the highest instead of the base constraint.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha of this change> -f base=none -f nordic=true -f samples=true \
  -f sample_years="2024 2027" -f scorers=none -f probe=probe_1842_storey_rules -f probe_scorer=storey_rules_vs_ifc \
  -f models="nordic-office-timber-permit nordic-office-timber-as-built nordic-office-as-built nordic-housing-timber-as-built sample-20*-rstbasicsampleproject sample-20*-racadvancedsampleproject"
python3 tools/re/storey_rules_vs_ifc.py <probe.txt> <Revit's export>.ifc
```
