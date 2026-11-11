# RE-429: LoadBearing from a wall's Structural box and usage, and from a floor's or stair's Structural box

**Date:** 2026-11-11
**Issues:** #1193 (W7: the BIM4LCA office's remaining property values), #1636 (the rules rest on the BIM4LCA models); follows RE-285 (structural beams' and columns' `LoadBearing`), RE-194 (parameters named for a property) and RE-158 (a type's Function)
**Artefacts:** the BIM4LCA housing and office, concrete and timber, as built and at building permit (Revit 2024), and Revit's IFC4 exports of them; the 24 R2025 models; RE1 Architecture; the 18 held-out models as counts only; Vitoria's 2018 model and its spent pair (Revit's IFC2X3, counted apart)
**Probe and scorer:** `probes/examples/probe_re285_flag_candidates.rs`, `tools/re/native_flags_vs_ifc.py`
**Runs:** Measure [43025706580](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43025706580) (the probe on `main` at `a0dd7813`: saved parameters of floors and family instances), [43028230598](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43028230598) (the same, with the 2024 walls' class named), [43030521879](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43030521879) (the integer members of a wall's root record), [43032705128](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43032705128) (the wall rule held against Revit's value on every model) the change before its Revit 2024 gate [43033314345](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43033314345) (unfiltered, `keep_ifc`, head `b6072615` against `main` at `b647c60b`) and the change with it [43044696450](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43044696450) (unfiltered, `keep_ifc`, head `b02c1ac9` against `main` at `8ec2ac98`)
**Status:** Positive on Revit 2024 and later, for walls, floors and stairs, and nothing is written before Revit 2024. The census of the 42 models with a Revit export reads `LoadBearing` equal on 4,006 of 4,253 walls (from 2,372), on 294 of 313 slabs (from 171) and on all 168 stairs (from 52), none different; 1,634 + 123 + 116 values added, each equal to Revit's. The 247 walls left without are Vitoria's pair of 2018 (IFC2X3, counted apart).

## 1. The question

In the property census of the BIM4LCA models `Pset_WallCommon.LoadBearing` was missing on all 1,464 walls of the six housing files and on 25 walls of the offices, `Pset_SlabCommon.LoadBearing` on 131 slabs and `Pset_StairCommon.LoadBearing` on 116 stairs (Measure run 43018018669), and on the 23 R2025 models' walls and RE1 Architecture's. rvt-rs wrote the property only for the structural column and framing categories (RE-285) and where a parameter named for it is read (RE-194). Where does Revit's export take the other values from?

## 2. What Revit's source says

Autodesk's exporter (CLEANROOM.md source 5, read and cited, not quoted: `Source/Revit.IFC.Export/Exporter/PropertySet/Calculators/LoadBearingCalculator.cs`, class `LoadBearingCalculator`, the same logic at the tags `IFC_v24.2.20` and `IFC_v27.4.0.30`) calculates `LoadBearing` in this order: for a wall, whether its `StructuralUsage` is `Bearing` (not `Shear` or `Combined`); for the structural column, framing, framing system and truss categories, true; for any element that holds the built-in parameter `FLOOR_PARAM_IS_STRUCTURAL` as an integer, whether it is not 0; otherwise a parameter named for the property on the element or its type. A parameter named for the property in the file replaces the calculation, as it does every standard property (RE-194).

## 3. Where a 2024 file keeps it

**Floors and stairs.** `FLOOR_PARAM_IS_STRUCTURAL` is the built-in parameter -1001954, a saved integer of `Floor` records. On the elements Revit's IFC4 export writes `LoadBearing` for (run 43025706580):

| model | slabs | hold the parameter | value 1 | value 0 |
|---|---:|---:|---|---|
| housing, timber, as built and at permit | 34 | 30 | true on 19 | false on 11 |
| housing, concrete, as built | 35 | 30 | true on 23 | false on 7 |
| housing, concrete, at permit | 31 | 27 | true on 20 | false on 7 |
| the two terrain housing files | 2 each | 2 | true on 2 | |
| RE1 Architecture | 2 | 2 | | false on 2 |
| the 18 held-out models (counts only) | 110 | 110 | true on 12 | false on 98 |

Every slab that holds the parameter and no parameter named for `LoadBearing` has Revit's value in it. The slabs that do not hold it (4 or 5 per large housing file) already read equal through a parameter named for the property. The offices are the one place the parameter and Revit's value part: their 31 slabs that hold it (value 1 on 18 slabs Revit writes false for) also hold a parameter named `Pset_SlabCommon.LoadBearing`, which has Revit's value on all 31 (true 8, false 23), and the named parameter wins in Revit's exporter as it does in rvt-rs. The stairs of the four large housing files (29 each) hold the parameter with the value 0 and Revit writes false on all 116. No stair on any model holds a checked box.

**Walls.** The 2024 walls are class `SWall` (455 in the timber housing hold saved parameters), and none holds an integer built-in parameter: the first two probe runs printed no wall line, because the RE-285 probe named the 32-bit releases' wall classes and no integer parameter of a 2024 wall is saved. The members of the wall's root record hold it: `m_isStructuralSignificant`, the Structural box, and `m_wallStructuralUsage`, the usage (1 is Bearing). Every wall of the housing holds usage 1 whether or not its box is set (the box is 0 on 344 of the timber housing's 455 walls), so Revit's `StructuralUsage == Bearing` is the box set and the usage Bearing. The scorer prints that as one derived member, `wall_load_bearing_rule`, and holds it against Revit's value (run 43032705128):

| models | walls | Revit true | the rule 1 and Revit true | the rule 0 and Revit false | other |
|---|---:|---:|---:|---:|---|
| BIM4LCA housing, the six files | 1,464 | 431 | 431 | 1,033 | none |
| the 10 held-out models of Revit 2024 to 2027 (counts only) | 138 | 35 | 35 | 103 | none |
| the 8 held-out models of Revit 2018 to 2023 (counts only) | 416 | 0 | | 416 | none |
| the R2025 models and RE1 Architecture | 145 | 0 | | 145 | none |
| BIM4LCA offices, the six files | 2,397 | 306 | 126 | 2,075 | 196: parameters named for the property decide |

The housing and one held-out house hold both states and the rule separates them exactly. The older models hold only the false state: the 416 walls of the held-out models of Revit 2018 to 2023 are false in Revit's export and the box reads 0 on all of them, as it does on Vitoria's 247 (Revit 2018, IFC2X3), so the rule agrees there but no model of those releases shows it telling a true wall from a false one. The true state is measured on Revit 2024 files alone, and the 2025 to 2027 models Measure holds (the R2025 models, RE1 and the held-out files) read false on every wall. The offices' walls hold a parameter named `Pset_WallCommon.LoadBearing` (605 of 607 as built), which has Revit's value and replaces the rule; where the rule alone is held against Revit's value it disagrees on 196 of the 2,397 walls, 180 with the rule false where Revit writes true and 16 with the rule true where it writes false, and the census after the change reads all 2,397 equal, so each of those walls holds a parameter named for the property with Revit's value, and the 25 walls of the offices that hold none read the rule's value, which is Revit's. The usage values 2 and 3 (Shear, Combined) are held by no model, and a wall whose usage is not Bearing reads false.

## 4. The rule written

- `attach_wall_load_bearing` (`src/research/partition_schema_mvp/walls.rs`) gives a wall `LOAD_BEARING_FIELD` from `native_walls::wall_load_bearing`: true when `m_isStructuralSignificant` is 1 and `m_wallStructuralUsage` is 1, false otherwise, where the record holds both members, on Revit 2024 and later (`WALL_LOAD_BEARING_REVIT_VERSIONS`) and in the 64-bit record pipeline alone; the 2015 to 2023 pipeline and the pass `read_wall_records` makes there are as on `main`. The records are read with the facts the wall readers already take (`WallReads`, which builds only the listed fields of a wall's graph, #1335): the members come from the records the pass `native_walls_and_top_joins_into` already decodes, so the walls cost no further read there, and a wall that pass does not reach is read by one pass over its record that builds five fields of it (`WallReads::load_bearing`). Curtain panel walls are read on 2024 and later too.
- Nothing is written for a wall before Revit 2024 because only the false state is evidenced there (section 3): the box reads 0 on every wall of the older models Measure holds, so a rule written there would return false for every wall without having been seen to separate a true one. Revit 2025 to 2027 are in the gate, with the same members, on the evidence that the 2025 models read false on every wall and that the record layout is the 2024 one; the true state is measured on 2024.
- `attach_structural_load_bearing` (`src/research/partition_schema_mvp/parameters.rs`) reads `FLOOR_PARAM_IS_STRUCTURAL` from the element's own data object on Revit 2024 and later (`scan_integer_parameter`, as a type's Function is read, RE-158) for slabs and products. 0 is false for any element; 1 is true for a floor alone.
- An element that has a `LoadBearing` from its category keeps it, and a parameter named for the property replaces the value when the sets are written (`add_named_parameter_sets`).

## 5. What the registered rows say

Registered on #1636 before the runs were read (walls https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6796564165, slabs https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6797098215, stairs https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6797146458), read in https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6798941132:

| row | the bar | result |
|---|---|---|
| walls 1 | one member separates the housing's walls | **Held**, by two members, not the usage alone: the box separates them and the usage is 1 on every wall |
| walls 2 | it holds on every IFC4 export with walls | **Held** on the R2025 models and RE1 (false on all 145) and the held-out models of 2024 to 2027 (138 walls); the offices part where a named parameter decides |
| walls 3 | Shear and Combined | **No model holds one**; the rule requires usage 1, so such a wall reads false, as the source says, unmeasured |
| walls 4 | a named parameter still wins | **Held**: the offices read 2,397 of 2,397 equal (from 2,372), none different |
| walls 5 | held-out, counts only | **Held** on the 138 walls of Revit 2024 to 2027 (the rule is Revit's on all); the 416 walls of 2018 to 2023 are false in Revit's export and the box reads 0 on all, which shows only the false state, so nothing is written for them (section 4) |
| walls 6 | IFC2X3 views count apart | Vitoria's spent pair: its 247 walls and 9 slabs are not written (Revit 2018), so they stay missing and counted apart |
| slabs S1 | the parameter is the flag | **Held** on every model that holds it and no named parameter (table above) |
| slabs S2 | slabs with no parameter | none written; the 4 or 5 per housing file already read equal |
| slabs S3 | exporters | no IFC4 export disagrees; BIMcollab and the IFC2X3 views count apart |
| stairs T1 | the parameter on stairs | **Held for 0**: 116 of 116 false; a checked stair is held by no model and gets none |
| what moves | only `LoadBearing` properties are added | **Held**, section 6 |

The first reading of the walls, run 43033314345, wrote the rule on every release and read equal on 4,253 of 4,253 walls; the 247 of Vitoria's pair that run added are the ones the gate takes back, and rvtReview's review of the pull request asked for the rule to be written only where the true state is measured.

## 6. What Measure found

Run 43044696450 (126 exports each side). The property census of the 42 models that have a Revit export, base against head: Revit's values 213,756 each side; equal 199,261 to 201,134 (+1,873); missing 14,434 to 12,561; **different 61 to 61**. By property:

| property | values | equal before | equal after | missing after |
|---|---:|---:|---:|---:|
| `Pset_WallCommon.LoadBearing` | 4,253 | 2,372 | 4,006 | 247 |
| `Pset_SlabCommon.LoadBearing` | 313 | 171 | 294 | 19 |
| `Pset_StairCommon.LoadBearing` | 168 | 52 | 168 | 0 |

| family | walls | slabs | stairs |
|---|---|---|---|
| BIM4LCA housing, six files | 0 to 1,464 of 1,464 | 17 to 138 of 138 | 0 to 116 of 116 |
| BIM4LCA offices, six files | 2,372 to 2,397 of 2,397 | 154 of 164 | 52 of 52 |
| the 24 R2025 models | 0 to 138 of 138 | | |
| RE1 Architecture | 0 to 7 of 7 | 0 to 2 of 2 | |
| Vitoria's spent pair (IFC2X3, Revit 2018) | 0 of 247 | 0 of 9 | |

The 18 held-out models, as counts: equal 13,654 to 13,813 (+159, the 138 walls and 21 slabs of the models of Revit 2024 to 2027), different 24 to 24, the values rvt-rs writes that Revit does not 116 to 116; 10 of the 18 exports move and the 8 of Revit 2018 to 2023 are byte-identical.

70 of the 126 exports move and 56 are byte-identical: the held-out models of 2018 to 2023, the six `rac_advanced` saves, BIMcollab's export, Einhoven, Vitoria's, the blank R2025 model, RE1's MEP models and 35 of Autodesk's samples (the 33 saves of Revit 2015 to 2023 and the two Snowdon Towers MEP saves of 2024, which hold no wall, slab or stair). The 70 that move are Core Interior, the BIM4LCA housing and offices (12), 23 R2025 models, RE1 Architecture, 10 held-out models and the 23 Autodesk samples saved in Revit 2024 to 2027, which no Revit export scores. `tools/re/ifc_element_diff.py` on the 60 moved pairs that keep their exports: no element is only in one file, 6,041 elements differ, and every difference is one property added to a set the element already holds, with nothing removed or changed: 5,003 `IfcWall` gain `Pset_WallCommon.LoadBearing`, 922 `IfcSlab` `Pset_SlabCommon.LoadBearing` and 116 `IfcStair` `Pset_StairCommon.LoadBearing`; the only entity count that moves is `IfcPropertySingleValue`, by 6,041. Core Interior, whose Revit export writes no standard property set, gains the property on 360 walls and 79 slabs, scored by no Revit value. `same-project gates` pass on the measured head, and `diagnostics.decoded.parameter_value_count` rises by the values added (439 on Core Interior, 401 to 432 on a large housing file, 18 on a terrain housing file, 2 to 20 on an office, 9 on RE1 Architecture, 5 to 16 on a R2025 model).

The values rvt-rs writes that Revit's export does not rose by 5, 358 to 363, none on a model where the rule is wrong: one wall of the concrete housing as built that rvt-rs writes as an `IfcWall` and Revit's export writes as an `IfcCovering` (`Basic Wall:AK-2.otsa`) and the two slabs of each terrain housing file whose other `Pset_SlabCommon` values were rvt-rs's alone before (their `IsExternal`, `PitchAngle` and `Reference`). Each is an element-class mismatch of the kind #792 and RE-417 track, and the value follows the element. The 4 on Vitoria's IFC2X3 pair that the first reading added are gone with the gate.

**Cost.** Measure [43040952174](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43040952174) (base and head timed in turn on one runner, three rounds, 26 files: Core Interior, the BIM4LCA housing and offices, RE1, the 2024 samples) timed the change at `b6072615`: time 0.96 to 1.04 of `main`'s (23 of the 26 within 1.01; the 1.04 is RE1 Mechanical, 546 to 570 ms) and peak memory 0.95 to 1.00 (1.00 on 24). All 26 files are Revit 2024, and the gate leaves the 2024 path as that run measured it: the members come from the records the wall readers already decode, the box of a floor or stair is one scan of the partitions for an 8-byte key (as a type's Function is read), and the 2015 to 2023 pass is now as on `main`. The run was not repeated at the gated head. Where the pass that reads the walls' joins does not run, `WallReads::load_bearing` makes one pass over the walls' records that builds five fields of each, which these files do not time.

## 7. What is not done

- **Walls, floors and stairs on Revit 2015 to 2023.** A floor's box is read as a saved parameter on those releases and the held-out 2018 to 2023 models hold the mapping (110 held-out slabs of all releases: 12 true on value 1, 98 false on value 0), and the wall members are in the older records too, but the rule reads the 2024 layout, and for walls only the false state is evidenced there (section 3); Vitoria's pair keeps its 247 walls and 9 slabs missing.
- **The 10 slabs of the timber office** (1 as built, 9 at permit) that hold neither the parameter nor a parameter named for the property: Revit writes a value, and its source is not found (a type's parameter named for the property is the one the source names that rvt-rs does not read).
- **A checked stair, and Shear and Combined walls:** no model holds one.

## 8. Reproduce

```bash
# the probe and the wall rule on the BIM4LCA models, the R2025 models, RE1 and the held-out ones (counts)
gh workflow run measure.yml --ref research/1193-wall-load-bearing -f ref=5883ece0f45c422eaa91212b29c3f940fd0ffe69 -f base=none -f nordic=true -f r2025=true -f holdout=true -f models="nordic-* r2025-* re1-* core-interior holdout-*" -f scorers=none -f probe=probe_re285_flag_candidates -f probe_scorer=native_flags_vs_ifc
# the change against main
gh workflow run measure.yml --ref feat/1193-structural-load-bearing -f ref=b02c1ac9e5aaab8d3758b81fb0b48e431704e787 -f base=8ec2ac98d3c982b83d0cdefe378d8c0e4a2df50d -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
```

The commits the runs measured are kept under `refs/archive/2026-11-11/cited/<first 12 hex>`.
