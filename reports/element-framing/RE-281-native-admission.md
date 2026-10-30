# RE-281: saved parameters and the phase order on Revit 2015 to 2022, and what BIMcollab's footings name as their Level

**Date:** 2026-10-30
**Issues:** #1158, #879; follows RE-280 (the framing, the preamble and the graphs of 2015 to 2022), RE-184 (the phase order), RE-207 (saved parameters), RE-198 and RE-262 (BIMcollab's Revit 2015 example)
**Artefacts:** as RE-280: Autodesk's sample projects of 2015 to 2023 (`research/autodesk-sample-projects.tsv`, fetched by hash and never stored), BIMcollab's Revit 2015 structural example (`02_BIMcollab_Example_STR.rvt`, sha256 `8cd1f122…e193`, with Revit 2015's own export; no licence stated, so only counts left the runner), Einhoven (2023), and the reference, BIM4LCA and R2025 models of the unfiltered run.
**Runs:** Measure [41116945112](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41116945112) (the unfiltered run, `-f base=main -f nordic=true -f r2025=true -f samples=true -f bimcollab=true`, this change at `d2565618` against `main` at `b9e0ce71`; the 51 exports that differ are all 2024 to 2027 files, which this change does not read differently: they come from four commits `main` gained between this change's branch point `d1ef1906` and `b9e0ce71`, #1161 (a scorer), #1162 (walls' side areas and volumes: `qtos_vs_ifc` on every r2025 model, RE1 Architecture and Core Interior, and with the witness observation of Core Interior), #1152 (walls' tops cut at the floor that covers them: the two offices' frames, wall heights and psets) and #1169 (pipes' `InvertElevation`: the MEP samples' `schema.txt`); the run on this change's own base, 41122598346, is the one that compares this change alone), [41118620685](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41118620685) (parameters by name against the 2023 copies, and the Element base of every sample), [41118961781](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41118961781) and [41119973943](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41119973943) (BIMcollab's Levels), [41121536238](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41121536238) (the family corpus), [41122598346](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41122598346) (the unfiltered run of this change against its own base) and [41127674661](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41127674661) (`rvt-ifc` time, both sides on one runner).
**Probes and scorer:** `probes/examples/probe_re281_native_levels.rs`, `probes/examples/probe_re281_parameter_pairs.rs`, `tools/re/native_levels_vs_ifc.py`.
**Status:** positive for the two readers admitted, negative for #879's Level. A Revit 2015 to 2022 file now has its saved parameters read by name and its phases ordered. On Autodesk's 23 projects with a 2023 copy, 932,645 parameters read by name and value equal it and 243 differ, in the causes §3 lists, which are edits between the saves. Nothing in the default export moves on any file of any release measured: no element the export writes is left out of the last phase. Revit 2015 to 2023 records could not be read by their Element base at all before this, because `decode_base` took `Identifier` for Revit 2024's 8-byte `m_id64`; that also kept the phase order from being read on Revit 2023. #879's 16 footings name no Level in their Element base or in either Level parameter, so no saved Level places them, and the 16 are all in the lowest storey, below every storey's elevation, which cannot tell the rule apart.

## 1. What changed

- `native_element::decode_base` takes `Identifier` as `m_id64` (8 bytes, Revit 2024 to 2027) or `m_id` (4 bytes, 2015 to 2023). RE-280 §5 found `ElementId`, `Element` (20 fields) and the pointer fields the same on every release; the Element base then reads on every record that decodes, and its owner id equals the framed id.
- `native_document::group_records` accepts the declared body size of 2015 to 2017 (the bodies plus 4 bytes per record, RE-280 §4) as well as the bodies' own; a group that matches neither is refused as before.
- `native_document::Options::releases` says which releases the caller's reader is measured on. Its default is 2023 to 2027, the previous gate; saved parameters (`native_element_parameters::read`) and the phase order (`native_phases::compute_phase_filter`) name `NATIVE_RELEASES`, 2015 to 2027. A release outside `NATIVE_RELEASES` is refused as before.
- Every other native reader keeps 2023 to 2027: wall and element types, type and instance materials, stairs and landings, symbol cut loops, mullion hosts, connectors and ports, column joins, family forms, saved meshes. Those that read an ElementId through `/m_id/m_id64` (`native_json::element_id`) find none in a 4-byte `Identifier`, so on Revit 2023 they already miss every id they need, and on 2015 to 2022 they would too; each needs its own measurement against an export, and none of 2015 to 2023 but BIMcollab's has one. #1158 stays open for them.
- `native_family_forms::family_forms` frames groups through `group_records` itself, so it checks `Options::default().releases` (2023 to 2027) before it reads, as its only caller already did through `native_type_materials::geometry_materials`.
- The extensible-storage catalogue is unchanged. It does not read where a file holds schemas before Revit 2021 (`ESField` has `m_unitType` where 2021 has `m_specTypeId`, RE-280 §3), and a record whose graph needs an entity of it is refused, 7 records on each 2016 to 2020 `rst_basic` and the MEP projects' 561 and 555 `FamilyInstance`s on 2015. Neither is guessed. The `VarSketch` records of 2015 to 2017 (`VarParam.m_pSubst` has modifier 4) are refused the same way.

## 2. The phase order

`compute_phase_filter` reads every record's Element base (`graph_classes` empty but for `AllProjectPhases`) and applies RE-184's rule: the export phase is the last of `AllProjectPhases.m_phaseIds`, and an element created after it or demolished in or before it is left out. On every sample project of 2015 to 2023 and on Einhoven the order is read (2 phases, 3 on every `rac_basic`), and **no element is left out of the last phase**: 0 on all 34 files. On BIMcollab it reads 5 phases and 8 elements fall outside the last, none of which the export writes. So the filter changes no export measured, which is also why RE-184's note that the three 2023 files "return neither an order nor any phased record" is out of date: they return both. A file whose phases cannot be read (the preamble fails, or two project phase lists differ) writes every element and carries the diagnostics' warning, as before; no project measured warns (a family file does, §6).

## 3. Saved parameters by name and value

`probe_re281_parameter_pairs` reads each sample with `native_element_parameters::read`, and its 2023 copy, and compares every parameter of every ElementId both declare by (level, kind, name or built-in id, storage type). The name is the definition's `m_caption`, so a family, shared, project or global parameter is compared by name and a built-in one by its negative id.

| sample | parameters read, named | shared elements with parameters in both | lists identical | parameters equal / different | held by one file only (older / newer) | named parameters equal / different / in one only |
|---|---:|---:|---:|---:|---:|---:|
| 2015 `rac_advanced` | 85,334, 4,449 | 15,387 of 15,502 | 12,822 | 83,702 / 12 | 389 / 3,127 | 4,272 / 0 / 20 |
| 2015 `rac_basic` | 16,079, 2,916 | 7,467 of 7,514 | 7,124 | 15,618 / 33 | 233 / 625 | 2,857 / 1 / 42 |
| 2015 `rst_basic` | 19,391, 2,755 | 559 of 561 | 461 | 18 / 14 | 488 / 246 | 0 / 0 / 89 |
| 2016 `rac_advanced` | 85,334, 4,449 | 15,393 of 15,508 | 12,830 | 83,704 / 10 | 389 / 3,127 | 4,272 / 0 / 20 |
| 2016 `rac_basic` | 16,011, 2,864 | 7,485 of 7,532 | 7,174 | 15,642 / 31 | 190 / 623 | 2,857 / 1 / 0 |
| 2016 `rst_basic` | 19,075, 5,720 | 12,517 of 12,660 | 12,173 | 16,681 / 101 | 864 / 245 | 4,626 / 54 / 709 |
| 2017 `rac_advanced` | 86,113, 4,449 | 15,445 of 15,560 | 13,534 | 84,909 / 2 | 0 / 1,930 | 4,272 / 0 / 20 |
| 2017 `rac_basic` | 16,396, 2,864 | 7,613 of 7,660 | 7,593 | 16,316 / 1 | 0 / 19 | 2,857 / 1 / 0 |
| 2017 `rst_basic` | 17,711, 4,720 | 12,778 of 12,921 | 12,727 | 17,265 / 23 | 12 / 16 | 4,714 / 0 / 0 |
| 2018 `rac_advanced` | 86,124, 4,449 | 15,624 of 15,625 | 13,713 | 84,920 / 2 | 0 / 1,930 | 4,272 / 0 / 20 |
| 2018 `rac_basic` | 16,407, 2,864 | 7,723 of 7,725 | 7,703 | 16,327 / 1 | 0 / 19 | 2,857 / 1 / 0 |
| 2018 `rst_basic` | 17,710, 4,720 | 12,914 of 12,921 | 12,875 | 17,265 / 23 | 0 / 16 | 4,714 / 0 / 0 |
| 2019 `rac_advanced` | 86,131, 4,449 | 15,626 of 15,627 | 13,722 | 84,927 / 2 | 0 / 1,923 | 4,272 / 0 / 20 |
| 2019 `rac_basic` | 16,414, 2,864 | 7,758 of 7,760 | 7,746 | 16,335 / 0 | 0 / 12 | 2,858 / 0 / 0 |
| 2019 `rst_basic` | 17,489, 4,720 | 13,028 of 13,035 | 13,024 | 17,345 / 0 | 0 / 4 | 4,714 / 0 / 0 |
| 2020 `rac_advanced` | 86,223, 4,449 | 15,626 of 15,627 | 13,722 | 84,927 / 2 | 0 / 1,923 | 4,272 / 0 / 20 |
| 2020 `rac_basic` | 16,518, 2,864 | 7,801 of 7,803 | 7,801 | 16,439 / 0 | 0 / 0 | 2,858 / 0 / 0 |
| 2020 `rst_basic` | 17,581, 4,720 | 13,071 of 13,078 | 13,067 | 17,437 / 0 | 0 / 4 | 4,714 / 0 / 0 |
| 2021 `rac_advanced` | 87,840, 4,354 | 16,112 of 16,113 | 16,108 | 87,535 / 0 | 0 / 20 | 4,354 / 0 / 20 |
| 2021 `rac_basic` | 16,528, 2,864 | 7,875 of 7,877 | 7,875 | 16,449 / 0 | 0 / 0 | 2,858 / 0 / 0 |
| 2021 `rst_basic` | 17,591, 4,720 | 13,152 of 13,152 | 13,148 | 17,447 / 0 | 0 / 4 | 4,714 / 0 / 0 |
| 2022 `rac_advanced` | 87,866, 4,374 | 16,180 of 16,181 | 16,180 | 87,555 / 0 | 0 / 0 | 4,374 / 0 / 0 |
| 2022 `rac_basic` | 16,534, 2,864 | 7,944 of 7,946 | 7,944 | 16,449 / 0 | 0 / 0 | 2,858 / 0 / 0 |
| 2022 `rst_basic` | 17,601, 4,720 | 13,257 of 13,257 | 13,257 | 17,451 / 0 | 0 / 0 | 4,714 / 0 / 0 |

- **Parameters equal: 932,645; different: 243**, over the 23 projects that are the same project in 2015 to 2022 and 2023 (the 2015 `rst_basic` is another file under the same name, RE-280 §7; its 18 equal and 14 different are not counted). From 2021 on no sample has a different value. The 243 are 69 of `-1015043` (a double: 49.5456 on the 2016 to 2018 `rst_basic` and 49.2128 on its copy), 58 of `-1002550` (a material's colour, an integer: 660504 on the 2015 `rac_basic` and 2113664 on its copy), 27 each of a curtain wall's `PanelArrayLength` and `PanelWidth` (18.4646 ft and 18.4613 ft), 20 of `-1002001` (a type name: `88.9×5.0CHS` and `88.9x5.0CHS`), 18 of `-1002551` (a material's transparency, 8 and 80), 4 of `-1008602` (3.5 ft and 2.9528 ft) and 20 more of smaller causes, among them a text corrected between the saves ("Add a dor to a curtain wall."). They are edits to the model, or in Revit's upgrade, and the same values are read on the elements the edits left alone.
- **Named parameters: 90,130 equal, 58 different, 891 held by one file only**, over the same pairs. A parameter one file holds and the other does not is one the later release gives an element or its family that the earlier does not, mostly built-in ones, named by id only: the 2023 copy holds 3,127 more on the 2015 and 2016 `rac_advanced`, 1,930 more on the 2017 and 2018, 1,923 more on the 2019 and 2020, and 20 more from 2021 on; the older file holds at most 864 the copy does not (the 2016 `rst_basic`).
- **No Revit export of a 2015 to 2022 file saves a parameter to compare with**, as RE-207 compared 2024 and 2025 ones: BIMcollab's export is IFC2X3 with no Revit property sets, and Autodesk publishes no exports of the samples. So the 2015 to 2022 values rest on the 2023 copy, and `--revit-property-sets` on them is unmeasured: it is written there and the supported profile says so.

## 4. BIMcollab's Revit 2015 example

- **The export is byte-identical to `main`'s** (`ifc.sha256` equal; Measure 41117344693, head against base), so the admission changes nothing there. Its property set census is the same on both sides: Revit values on objects rvt-rs writes 1,045, equal 3, different 225, missing 817, rvt-rs values Revit does not write 475. The 817 missing are `Pset_*Common.IsExternal`, `LoadBearing`, `Span`, `Slope`, `ThermalTransmittance` and `ExtendToStructure` on beams, columns, slabs and walls, which RE-185, RE-193 and RE-228 read on 2024 and 2025 from native records through `/m_id/m_id64`; reading them on 2015 is the work that stays in #1158.
- **The Element base**: all 21,814 declared elements' records read; 285 name an associated Level and 427 a created phase. Of Revit's 241 exported elements, 210 name a Level (91 beams, 114 walls, 5 slabs): the 5 Levels they name are each in exactly one storey of Revit's export and each storey holds one Level's elements, so the map Level to storey is one to one, and **210 of 210 are in their Level's storey**. The 15 columns and 16 footings name none.
- **The three air terminals** Revit's export leaves out (RE-198, RE-262) are still written. They are not out of the last phase, since the export did not move, and RE-262's `IFC_EXPORT_ELEMENT` test, now possible on 2015, is not run here.

## 5. #879: the 16 footings

#879 expected the footings' storey from their Level parameter once the parameter store was read in the 32-bit layout. It is read (`probe_re281_native_levels` lists every ElementId parameter an element holds, 3,802 values on 1,416 of BIMcollab's 21,548 elements that decode), and it does not name one:

| source | elements that name a Level | footings that name one |
|---|---|---|
| Element base `m_assocLevelId` | 285 of 21,814; 210 of Revit's 241 (beams, walls, slabs) | 0 of 16 |
| the parameters `-1001352` and `-1002062`, which `m_assocLevelId` is projected to on a family instance (`native_metadata`) | 122 elements hold one: the 91 beams name a Level, in its storey 91 of 91 | 0 of 16: all unset (-1), as are the 15 columns' |
| any other typed ElementId parameter | none holds a Level an Element base names, on any element | 0 of 16 |

The 15 columns are unset too and Revit puts them on a storey, which the exporter reaches from their record's reference list (RE-27), so an unset Level does not mean Revit leaves an element off a storey. The saved parameters therefore add nothing to the Element base here: the two Level parameters are its projection. Two other ElementId parameters, `-1140362` and `-1140363`, are held by all 16 footings (and 106 other elements, 3 distinct values): they are not Levels any Element base names and their values do not follow Revit's storeys (71 of 122 in the majority storey of their value), so they are not reported.

All 16 footings are in one storey of Revit's export, the lowest of its five, and their tops lie below that storey's elevation (0 of 16 have their base at or above it; 16 of 16 have their top below it). So the storey at or below an elevation (the lowest where none is below), the nearest storey, and the lowest storey each give 16 of 16 by the placement, the base and the top, and a rule cannot be told from another on this model: every footing is below every storey. Revit's rule for an element with no Level is therefore not measured; the footings need a second model with foundations spread between storeys and Revit's export, which none of the scored models has (no other model's export holds an `IfcFooting`). #879 stays open, with the saved Level ruled out.

## 6. What moved: nothing in an export, 0.1 to 0.5 s of time on 2015 to 2023

- **Exports.** The unfiltered run of this change against its own base (41122598346; `-f nordic=true -f r2025=true -f samples=true -f bimcollab=true`, 91 models and samples) hashes every IFC the same on both sides: 91 identical, 0 moved, and no scorer, `info.json` or diagnostics file differs. That covers BIMcollab, Einhoven, the 30 samples of 2015 to 2022, the 3 of 2023, all of 2024 to 2027, the 6 reference models, the 2 offices and the 23 R2025 models. The first run (41116945112) compared against a `main` that had since gained #1161, #1162, #1152 and #1169 and showed 51 differences, all 2024 to 2027 files and all those commits' outputs; the run against the real base shows none.
- **Time.** On one runner, both sides in turn (41127674661), `rvt-ifc` takes the same time on Core Interior and the four RE1 models (0.96 to 1.01) and 0.04 to 0.5 s more on each file of 2015 to 2023: the 2015 `rme_basic` 1,257 to 1,753 ms, the 2015 `rac_basic` 628 to 815 ms, the `rst_basic` of 2015 to 2022 about 280 to 530 ms, Einhoven 127 to 163 ms. The cost is the two native passes (the phase order and every record's Element base; the saved parameters of every declared element) that these releases did not make before; 2024 and later make them already.

## 7. What this does not claim

- **Nothing here is measured against a Revit export of a 2015 to 2022 file except BIMcollab's**, and its export has no parameter to compare. The parameters rest on the 2023 copy of the same project.
- **`--revit-property-sets` and `--revit-set-placement` on 2015 to 2022** write sets from parameters that no export checks.
- **Every default export is unchanged on every model measured**, but a file with a phase list that leaves an element out would change: the rule is RE-184's, measured on 2024 and 2025 exports only. The extensible-storage catalogue failing (RE-280 §3) does not stop the preamble: `read_preamble` keeps its result, and only a record whose graph needs one of its entities is refused. The phase order is read on all nine such samples (2 phases each, §2); a project whose phase list sat in such a record would have its phases unread and every element written, with the diagnostics' warning, and none measured does.
- **The wall function, `IsExternal`, type, material and stair readers are not admitted** to 2015 to 2022, nor to 2023.
- **A family file is not read.** The Autodesk family of every release 2016 to 2027 fails the preamble at `Global/DocumentIncrementTable` (`invalid increment episode range`), as 2023 to 2027 families did before: no parameter or phase is read from a `.rfa`. On 2016 to 2022 families `rvt-elements` and Python now say so (`saved-parameters-unread`) and the export diagnostics carry the phases warning, where they were silent (Measure 41121536238).

## 8. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=<commit> -f base=main -f nordic=true -f r2025=true -f samples=true -f bimcollab=true
gh workflow run measure.yml --ref main -f ref=<commit> -f base=none -f samples=true -f sample_years="2015 2023" -f probe_pair_release=2023 \
  -f models=einhoven -f scorers=none -f probe="probe_re281_parameter_pairs probe_re281_native_levels" -f probe_scorer="- -"
gh workflow run measure.yml --ref main -f ref=<commit> -f base=none -f bimcollab=true -f models=bimcollab-str-2015 -f scorers=none \
  -f probe=probe_re281_native_levels -f probe_scorer=native_levels_vs_ifc
gh run download <run id>   # measure-head/sample-*/probe-pair-probe_re281_parameter_pairs.txt; bimcollab-str-2015/counts.txt
```
