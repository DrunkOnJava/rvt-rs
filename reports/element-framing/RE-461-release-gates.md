# RE-461: eight rules wrote values on releases their reports never measured: what each is held to, and what is gated (#1945)

**Date:** 2026-11-13
**Issues:** #1945 (found by the batch 9 export audit: eight rules whose release range is wider than the range their report measured)
**Artefacts:** Autodesk's sample projects of 2015 to 2027 (54 saves of `rac_basic`, `rac_advanced`, `rme_basic`, `rme_advanced`, `rst_basic` and `rst_advanced`; the pins are in `research/autodesk-sample-projects.tsv`) and the five Snowdon Towers 2024 files, exported by `main` at `f8e20536` and kept (Measure run [43456034222](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43456034222), artifact `measure-base-all`); the BIM4LCA models as the reports measured them.
**Tool:** `tools/re/release_rules_across_saves.py`
**Runs:** an unfiltered Measure of 146 exports (`-f samples=true -f nordic=true -f r2025=true -f racadv=true -f holdout=true -f bimcollab=true -f families=true`, run 43525326226, head 334c3a1b against base 6ad5efa9): every export and all 121 diagnostics equal, and the same-project gates pass.
**Status:** Positive for four rules (3, 4, 5 and 7), whose measurements are named below; four narrowed (1, 2, 6 and 8), none of which moves an export of any file in those corpora.

## 1. The rule, and what the experimental band changes

A release's reader is a decision, so a later release is not taken until a file of it is measured (the doc comment of `NEW_RULE_REVIT_VERSIONS`, RE-353). For Revit 2024 to 2027, the range of 1.0, a rule runs on a release only where Revit's own export measured it or where the same project's save reads the same as a measured save (`tools/re/bodies_across_saves.py` and `readers_across_saves.py`, RE-346). For 2015 to 2023 the readers of the neighbouring measured release run, labelled experimental (`docs/supported-profile.md`), and each rule's report says which releases it measured. Where a rule runs on a release no file in any corpus can exercise, a narrower list is the fail-closed answer; where a measurement exists, the rule is kept and the measurement named.

## 2. What the 59 exports hold

Per sample, by release (`tools/re/release_rules_across_saves.py`):

| What | 2015 to 2023 | 2024 | 2025 to 2027 |
|---|---|---|---|
| Hollow-section columns (rule 1) | 0 in all 34 saves | 1 (Snowdon structural, no Revit export) | 0 in all 15 |
| Wall sweeps typed from their record (rule 8) | 0 sweeps read | 258 of Snowdon architectural's, 249 typed; 8 of `rac_advanced`, none typed | 8 of `rac_advanced`, none typed |
| Ramps (rule 8) | 0 | 2 of Snowdon architectural's, typed | 0 |
| Doors that fill no opening (rule 5) | 17, 5, 17 and 17 | the same, 6 on Snowdon architectural | the same |
| Railings (rule 7) | named by ElementId, untyped (the rule starts at 2024) | 22, 10, 14, 14 and 131 | 22, 10, 14 and 14 |
| Pipes (rule 4) | 488 and 491 (2015, 2019), 491 (2023) | 488, 491 and 3,056 | 488 and 491 |
| Beams the RE-119 rule binds (rule 3) | `rst_basic`: 131 (2015), 41 (2016), 27 (2017 to 2023) | 0 | 0 |

No sample holds rme saves of 2016 to 2018 or 2020 to 2022: Autodesk publishes `rme_basic` for 2015, 2019 and 2023 to 2027 and `rme_advanced` for 2015, 2019 and 2024 to 2027.

## 3. Rule by rule

1. **A hollow-section column's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` (RE-247 section 8, #1773).** Measured on the BIM4LCA office, Revit 2024; no other file holds one a Revit export checks. **Gated to 2024** (`HOLLOW_SECTION_QUANTITIES_REVIT_VERSIONS`); the corpora's other 58 files have none, so no export moves.
2. **A wall that embeds a curtain wall, drawn from its saved mesh (RE-340).** The saved-mesh scope is 2023 to 2027, but the walls' records of 2023 name no embed step, so the rule found none there. **Written as 2024 to 2027** (`EMBED_HOST_REVIT_VERSIONS`); nothing moves.
3. **A beam bound to the Level at its record box's top (RE-119).** Measured against Revit's export on 8 beams of one 2023 file. Held to the 2024 save, which binds its beams by another rule (RE-454): the 41 beams of `rst_basic` 2016 and the 27 of each of 2017 to 2023 that this rule binds have the 2024 save's storey, 41 of 41 and 27 of 27 on every save. `rst_basic` 2015 shares no ElementId with the later saves (131 beams bound, none paired), and 2008 to 2014 have no sample. **Kept; the doc comment says what measured it** and that 2008 to 2015 are not held to anything.
4. **A pipe's type, centreline and radius from its `RbsPipeCurve` (RE-370).** Held to the 2024 save on 2015 (488 of 488 and 488 of 491 pipes: RE-370 section 4 explains the 3 by an edit the project had between those saves), 2019, **2023 (491 of 491 pipes, name, type and base quantities)**, 2025, 2026 and 2027, and to Revit's export on 2018 (counts, the Vitoria projects) and 2024 and 2025. 2016, 2017 and 2020 to 2022 have no file with a pipe in any corpus. **Kept; the doc comment says which releases are measured.**
5. **A door's placement origin from its stored transform (RE-361).** Measured against Revit's export on 2024 and 2025 (RE-361 section 5). Held to the 2024 save on 2025, 2026 and 2027: the origins of the 17 doors of `rac_advanced`, the 5 of `rac_basic` and the 17 of each `rme` save are the 2024 save's to 3 mm (offset 0.000 on every one). On 2015 to 2023 the reader gets no transform (`element_data_header` is measured from 2024), and the origin is the record box's, which is why those saves differ from 2024's by 0.14 to 0.48 ft. **Kept.**
6. **A shaft's outline cut from the elements its height passes through (RE-99, RE-345).** Revit 2024 only. `plan_profile_from_fields` also takes a stair landing's outline, which no shaft was measured to cut. **Limited to outlines from sketch lines** (floors, ceilings, roofs, as RE-99 measured); no export in these corpora moves.
7. **A railing's type from its payload (RE-66).** Measured against Revit's export on 2024 and 2025. Held to the 2024 save on 2025, 2026 and 2027: name, type and `Type Id` of the 22 railings of `rac_advanced`, the 10 of `rac_basic` and the 14 of each `rme` save are equal. **Kept.**
8. **A wall sweep's or ramp's type from its own record (RE-240).** Measured on the BIM4LCA office, Revit 2024 (9 sweeps, 5 ramps). On 2025 to 2027 the 8 sweeps of `rac_advanced` get no type from it and no other file holds a sweep or a ramp measured there. **Gated to 2024** (`SWEEP_RAMP_NATIVE_TYPE_REVIT_VERSIONS`).

## 4. Not measured, not changed

- Rules 3 and 4 run on releases no file measures (3: 2008 to 2015; 4: 2016, 2017, 2020 to 2022): the experimental band of section 1. A later file of any of them is the way to measure them, and a difference is the way to narrow them.
- The two rules the audit's second checker refuted stay as they were.
- `rac_advanced`'s 8 wall sweeps are named by their ElementId on 2024 (`192754`) and by class and ElementId on 2025 to 2027 (`WallSweep-192754`); neither is Revit's name, which no export of that file gives. Not changed here.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f samples=true -f keep_ifc=true -f scorers=none
gh run download <run> -n measure-head-all -D kept
uv run --no-project --python 3.12 --with ifcopenshell==0.8.5 python tools/re/release_rules_across_saves.py kept
```
