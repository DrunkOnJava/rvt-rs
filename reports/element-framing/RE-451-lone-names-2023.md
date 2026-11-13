# RE-451: on Revit 2023 the element before a furred ceiling reads as a one-character name and hides the ceiling's type: the two furred ceilings of `rac_advanced` get it back

**Date:** 2026-11-13
**Issues:** #1899 (two furred ceilings of Autodesk's `rac_advanced` saved by Revit 2023 lose their parameter sets); follows RE-111 (a 2023 system type is the named id its element's record names that is the most specific) and RE-180 section 5 (an element-data name of one character is dropped on 2019 to 2022 and kept on 2023)
**Artefacts:** Autodesk's sample projects `racadvancedsampleproject.rvt` of Revit 2022 (sha256 `430176be888c85e1af36e4477cde23b73dc83f70ea48315a52e3ab93083fefe6`, 16,330,752 bytes), 2023 (`75e8738dcc776950d1f08f8cf8c7ea6da1710c3a9aca17fb1e5e405aa7388d83`, 16,486,400) and 2024 (`ff791052979d8f0acd51c35da1f9976f3357c10495b96fa322dca7555dfe5afc`, 17,039,360), and the 2023 `racbasicsampleproject.rvt` (`94f98cc1a38f4f6c4799d62076dbdb14590543f9737863411ba68069a840df89`, 18,898,944), `rstbasicsampleproject.rvt` (`091ba54f47a2a84b45f925715ef4f4e3f910a16345b7d370150388cdd23c5391`, 6,344,704) and `rmebasicsampleproject.rvt` (`5b2331579140cc621d23b5642d24b2b7bbb6727e26610156962909d725e44e6a`, 30,797,824), all public and redistributable (`research/autodesk-sample-projects.tsv`); `Revit_IFC5_Einhoven.rvt` (MIT, `magnetar-io/revit-test-datasets`). No Revit export of any of the 2023 saves exists: the 2023 `rac_advanced` is held to the same project's saves of 2015 to 2022, which write 19,979 parameter values to its 19,973.
**Probes:** `probes/examples/probe_1899_covering_parameters.rs` and `probe_1899_ceiling_type_names.rs` (PR #1905)
**Runs:** Measure [43416338575](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43416338575) (`probe_1899_covering_parameters`), [43418834070](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43418834070) (`probe_1899_ceiling_type_names`, the 40 models of Autodesk's 2015 to 2023 samples and Einhoven) and [43420558633](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43420558633) (this change, unfiltered, against `0c58ea68`)
**Status:** Positive on the probe: the two ceilings regain their type and nothing else of the system types of the 40 models read moves. The export's rows (section 4) are read on run 43420558633, which was still running when this was written; they replace the note in section 4 before the pull request is marked ready.

## 1. The question

The 2022 save of `rac_advanced` writes its two furred ceilings 247617 and 247677 as `Compound Ceiling:Furred Ceiling:<id>` with a `Pset_CoveringCommon` and a `Pset_EnvironmentalImpactIndicators`; the 2023 save writes `Ceiling-247617` and `Ceiling-247677` with neither, which is the 6 parameter values by which its 19,973 fall short of the 19,979 of every save from 2015 to 2022 (#1899). Why?

## 2. What the bytes say

**The records are the same.** `probe_1899_covering_parameters` finds both ceilings' native records on the 2022 and 2023 saves identical in size and shape (13,220 and 5,407 bytes, graphs of 56 and 42 members, 5 saved parameters each, status `complete_bounded_graph`). What differs is the name: the 2023 export found no type for them.

**A 2023 type is chosen from the ids a record names.** A system type has no record and no name entry on 2023 (RE-109); its name is in its element data, and an exported wall, floor, roof or ceiling takes as its type the one named id its record names that has no record or name entry of its own and whose naming records' categories are a strict subset of every other candidate's (RE-111).

**Each ceiling has two candidates and neither is more specific.** `probe_1899_ceiling_type_names` prints them:

| ceiling | candidate | name | records naming it |
| --- | --- | --- | --- |
| 247617 | 242823 | `Furred Ceiling` | Lighting Fixtures, Ceilings |
| 247617 | 247616 | `퉻` (U+D27B) | Lines: <Sketch>, Ceilings |
| 247677 | 242823 | `Furred Ceiling` | Lighting Fixtures, Ceilings |
| 247677 | 247676 | `퉼` (U+D27C) | Lines: <Sketch>, Ceilings |

The element before each ceiling (247616, 247676) is named by the ceiling's record and by its sketch lines, has no record and no name entry, and its data opens with a list of one ElementId, which reads as a name of one character (RE-180 section 5: `퉼` is the low half of id 250492). That makes it a candidate beside the type, and its categories are no subset of the type's: no candidate is the most specific, and the ceilings have no type.

**RE-180 called 247676 a ceiling type; it is not.** Its section 5 says the 2019 to 2022 `rac_advanced` "gave ceiling type 247676 the name `퉼`". The type of the furred ceilings is 242823; 247676 is the element before the ceiling 247677. The name was right and the label was not.

**2015 to 2022 do not have it, and 2023 is the one release that keeps it.** The probe repeats the choice with and without the names that are one non-ASCII character (`lone_non_ascii`) on every 32-bit file of the run:

| files | lone names left to drop | system-type elements | gain a type | change | lose one |
| --- | ---: | ---: | ---: | ---: | ---: |
| 30 files, Revit 2015 to 2022 (every one-character name is already dropped) | 0 | 6 to 267 each | 0 | 0 | 0 |
| 2023 `rac_advanced` | 17 | 267 | **2** (247617, 247677) | 0 | 0 |
| 2023 `rac_basic` | 30 | 68 | 0 | 0 | 0 |
| 2023 `rme_basic` | 30 | 230 | 0 | 0 | 0 |
| 2023 `rst_basic` | 54 | 14 | 0 | 0 | 0 |
| Einhoven (Revit 2023) | 0 | 2 | 0 | 0 | 0 |

The two that gain are the two ceilings, each getting 242823 `Furred Ceiling`.

## 3. The change

`find_element_data_names_32` on Revit 2023 refuses a name that is one non-ASCII character, the rule RE-180 already applies to name entries on every 32-bit release (`name_entries_2023`). Every other release keeps dropping every one-character name. A one-letter ASCII name stays on 2023, as RE-180 left it: dropping it would give five `rac_basic` walls a type their 2024 copies do not give, and `rac_basic` 2023 itself names four walls' type `p` today.

## 4. Rows

Registered on #1899 before the run:

1. The 2023 `rac_advanced` export names 247617 and 247677 `Compound Ceiling:Furred Ceiling:<id>` as the 2022 and 2024 saves do, with their `Pset_CoveringCommon` and `Pset_EnvironmentalImpactIndicators`, and its `parameter_value_count` goes from 19,973 to 19,979, the value of every save from 2015 to 2022.
2. No other export of the 146 changes and no other diagnostics: the probe measured only the choice of a system type, and the one other reader of these names, the materials with no name entry of their own (`partition_materials`), is not measured by it.
3. The held-out models change in none of their counts.
4. The `same-project gates` check passes on the pull request's head.

Read: not yet (run 43420558633 in progress).

## 5. Not changed

- `rac_basic` 2023's four walls whose type is `p` (an ASCII one-letter name), as RE-180 recorded.
- No Revit export of the 2023 `rac_advanced` exists, so the two ceilings are held to the same project's other saves, not scored against Revit's.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha of this change> -f base=<its merge-base with main> \
  -f samples=true -f nordic=true -f r2025=true -f racadv=true -f holdout=true -f bimcollab=true -f families=true
gh workflow run measure.yml --ref main -f ref=<sha of probe_1899_ceiling_type_names> -f base=none -f scorers=none \
  -f probe=probe_1899_ceiling_type_names -f probe_scorer=- -f samples=true -f sample_years="2015 2023"
```
