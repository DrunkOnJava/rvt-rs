# RE-354: door types' Fire Rating and family types' Manufacturer, and a door type's Heat Transfer Coefficient left out

**Date:** 2026-11-05
**Issues:** [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-77 (type text parameters), RE-185 section 3.3 (a family type table's entries), RE-286 (type objects' common sets) and RE-289 (a Manufacturer negative that this report overturns)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, `research/bim4lca-measure.tsv`), as Measure fetches them
**Probes:** `probes/examples/probe_re354_door_type_parameters.rs` (Measure run [42112543949](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42112543949), when it was named `probe_re352_door_type_parameters`, commit 1d4ecaae on a scratch branch) and `probe_re354_door_type_blocks.rs` (runs [42115439931](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42115439931), as `probe_re352_door_type_blocks` before the Manufacturer texts were added, commit 38d9e426 on the same branch, and [42118085660](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42118085660), commit 42707c3a)
**Status:** positive for two values, on one building (the office), with the held-out and reference models measured (section 5): a door type's `FireRating` and six family types' `Manufacturer` are written, equal to Revit's export on all 12 type values on both office files, and on the AA house's 15 and 16, and the door `FireRating` on Snowdon Towers Architectural's 128 of 128 against its local Revit export. A door type's `ThermalTransmittance` is found (section 3) and left out (section 6): Revit's exporters disagree on it. What else stays out is `IsExternal` on type objects, by the maintainer's decision, and one value on the tank RE1 Plumbing writes and Revit's export does not hold (section 6).

## 1. What Revit's export writes

On type objects of the office, beside what RE-286 wrote: `Pset_DoorCommon.FireRating` on six door types, `Pset_DoorCommon.ThermalTransmittance` on one, and `Pset_ManufacturerTypeInformation.Manufacturer` on six family types. The two doors of one of the rated types write the same `FireRating` as their type, which is `-`; the doors of the other five carry a value of their own, the shared parameter `Pset_DoorCommon.FireRating` that RE-194 reads (`EI60` on every door of type 2803674, whose own is `EI30`), so a type's value and its doors' differ.

| door type (Tag) | name | `FireRating` |
|---|---|---|
| 2803674 | O-1.12x21 | EI30 |
| 2324339 | O-17.12x21 | EI30 |
| 1110711 | O-16.1200x2000_180° | EI60 |
| 1106810 | LU-3.VSS.700x1200_90° | EI60 |
| 1106812 | LU-3.VSS.700x1200_180° | EI60 |
| 2391770 | O-14 | - |

`ThermalTransmittance` 3.7021 is on door type 3261757 (UO-6.30x24) and on none of its doors. The six `Manufacturer` types are 1414255 (`IDO`), 2575395 (`Ido`), 1611986 (`Väinö Korpinen Oy`), 3365322 and 3376911 (`Thorlux Lighting`) and 1533300 (`Korpinen Oy`); none of their instances carries it.

## 2. Where they are not

The saved parameters of the doors and their types (RE-207, `probe_re354_door_type_parameters`) list the doors' own `Pset_DoorCommon.FireRating` (1877482, shared) and `Paloluokka` (1105117, shared) and, on the types, family and shared parameters of a door's dimensions and finishes, but no fire rating, no manufacturer text (`Valmistaja` is empty on type 2803674) and no heat transfer coefficient. RE-77's reader takes `i64 BuiltInParameter · u32 n · UTF-16 × n`; the three `EI60` texts it finds (RE-289 section 2) are in the number of the three wall types Revit's export gives that rating.

## 3. Where they are

In the type's own value block, the entries are in the form of a family type table's (RE-185 section 3.3), the text first and the parameter after it:

```text
u32 n · UTF-16 × n · (u32 0 | ff ff ff ff · u16) · f64 · i64 -1 · i64 BuiltInParameter
```

`probe_re354_door_type_blocks` lists every `EI30`, `EI60`, `EI90` and `Manufacturer` text of the partitions with the bytes after it and the value block it lies in. On the office as built:

- 49 `EI30` and `EI60` entries have the parameter -1001206 (the built-in Fire Rating RE-77 named), in the blocks of 25 owners. The blocks of 2803674, 2324339, 1110711, 1106810 and 1106812 hold the text Revit writes for those types. The other 20 owners are none of the 41 door types of Revit's export; what they are was not read. The `-` of 2391770 was not searched for, and its type and both its doors are equal in the census, so the scan reads it.
- The six `Manufacturer` texts are entries of this form under -1010108, in the blocks whose owners are the six type Tags above. Other blocks hold the same texts too. Ido's type and Thorlux's two use the second flag form (`ff ff ff ff · u16`) and the other three the first.
- `ThermalTransmittance`: 3.7021 is the `f64 · ff × 8 · i64 -1005430` entry (RE-93's form) of block 3261757, and it is in four other blocks (3261276, 3261284, 3262425 and 35764), none a door type of Revit's export.

## 4. What rvt-rs does

`partition_type_parameters::scan_partition` reads, beside RE-77's entries, the table form for the Fire Rating (-1001206) and the Manufacturer (-1010108), both flag forms, in the block of a declared owner. Two rules keep a table entry from a type that is not its owner:

- **No mark between the owner's and the entry.** An entry belongs to the nearest preceding block mark only where no value block mark of any id, declared or not, lies between that mark and the entry. `value_blocks` keeps only the marks of declared ids (and 0), so a mark of an undeclared id opens another element's block without ending the one before it, so a scan that took the nearest declared mark with no bound would give an entry in that block to the last declared type before it. On the office as built, probe run [42118085660](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42118085660) found 33 of the 88 `Manufacturer` entries more than 100 KB from the nearest preceding declared mark, up to 3.2 MB (blocks 745201, 1413592 and 1600160), almost certainly in loaded families' documents; the 12 type values that match Revit's export lie within 8.4 KB of theirs. RE-77's id-first entries keep their rule (nearest declared block), so Type Mark, Description and the id-first Fire Rating read as before.
- **Copies agree.** Two copies of a type's entry that disagree give the type neither, within a partition and across partitions, and so do a table entry and an id-first entry that disagree. RE-77's id-first copies keep their rule that a later partition's value replaces an earlier one's. A type left out so is counted in the export diagnostics (the warning of the type text parameters left out for want of agreeing copies). Unexercised on the office, whose table entries are all in `Partitions/425`: no type there has two partitions' copies. It fires once on Core Interior and once on Snowdon Towers Architectural (section 5).

The text then goes where RE-77's goes: onto each element of the type as the type parameter `Fire Rating`, which an element's own `Pset_DoorCommon.FireRating` replaces (the two doors of type 2391770 take the type's `-`), and onto the type object (`TypeCommon`). A `Manufacturer` goes in `Pset_ManufacturerTypeInformation` on the type object alone, and only where IFC4 applies that set to the type: `Pset_ManufacturerTypeInformation` applies to `IfcElementType`, so a type object of an entity that is no `IfcElement` (the `IfcSpaceType`) gets none (`standard_property_sets::applies_to`, as the `FireRating` is gated by the set's definition). Nothing reads the `-1005430` entry (section 6).

## 5. Measured

Measure run [42135050819](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42135050819), unfiltered, on the final head e07349c0 against `main` at daf36102 (`-f base=main -f nordic=true -f r2025=true -f holdout=true -f samples=true -f keep_ifc=true`, no `ifc_flags`, every scorer; the office as built and at permit, the 18 held-out models (counts only), the 24 R2025 models, the four RE1 models, Core Interior, Einhoven and the sample projects of 2008 to 2027). `psets_vs_ifc`, the default export, the office:

| | office as built | office at permit |
|---|---:|---:|
| Revit values on objects rvt-rs writes | 45,808 | 45,967 |
| equal, `main` / this change | 44,413 / 44,427 | 44,562 / 44,576 |
| missing, `main` / this change | 1,394 / 1,380 | 1,404 / 1,390 |
| different | 1, unchanged | 1, unchanged |
| values rvt-rs writes that Revit does not | 2, unchanged | 7, unchanged |
| `Pset_DoorCommon.FireRating` equal | 47 to 55 of 55 | 47 to 55 of 55 |
| `Pset_ManufacturerTypeInformation.Manufacturer` equal | 0 to 6 of 6 | 0 to 6 of 6 |
| `Pset_DoorCommon.ThermalTransmittance` equal | 18 of 19, unchanged | 18 of 19, unchanged |

The two doors and six door types, and the six family types, are the 14 gained on each file. The quantities are unchanged. Of the 182 values on type objects as built, 46 are written equal (34 by RE-286, 12 by this report), 135 are `IsExternal` and 1 is the left-out `ThermalTransmittance`. The earlier run [42123166507](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42123166507) (head 40948685, before the attribution rule of section 4 was narrowed, scored with `--revit-set-placement`) gave the same 55 of 55 and 6 of 6 and the same gains of 14, so the narrowing lost no value the scorer sees.

**The rule's reach, building by building** (the IFC hashes of 111 exports against `main`'s: 99 are the same and 12 moved; the kept IFCs of the 12, ids normalised, differ only in these properties):

- **Office, as built and at permit** (Revit 2024): `FireRating` on 9 door type objects and 2 doors, `Manufacturer` on 11 type objects, the same on both files; scored equal, 55 of 55 and 6 of 6.
- **AA house** (Revit 26.4's IFC4X3 export, held out): `FireRating` 15 of 15 and `Manufacturer` 16 of 16 equal (0 before), equal 378 to 409 of 624, different 0, the values Revit does not write unchanged. Its IFC is not kept.
- **RE1 Electrical, Mechanical and Plumbing** (Revit 26.2, IFC2X3, counted apart): `Manufacturer` equal 11, 3 and 5 more, none different; Architecture's IFC is `main`'s. One extra on Plumbing, below.
- **Core Interior** (its export writes no property set, RE-239 section 5): 4 door type objects and 50 doors gain a `FireRating`, and the 50 doors their `Fire Rating` copy. `IFCPROPERTYSET` goes 6,510 to 6,514 and `IFCPROPERTYSINGLEVALUE` 27,962 to 28,066 (the pins of `tests/fixtures/project-counts/2024-core-interior*.json`, which already hold 6,514, and the witness observations' counts and hash: replay matches, none moved). `decoded.parameter_value_count` goes 4,126 to 4,180 and the values counted apart 4,126 to 4,177, three apart as before; `tests/parameter_diagnostics.rs` passed in this head's full CI.
- **Snowdon Towers Architectural** (Revit 2024, no Revit export in Measure; its local Revit IFC4 export scored apart): the door types of 12 types (28 type objects, one per original symbol) and 100 doors gain a `FireRating`. Against that export, `Pset_DoorCommon.FireRating` goes 16 to 128 of 128 equal, `main`'s 112 missing to 0; different (44) and the values rvt-rs writes that Revit does not (512) are unchanged.
- **The four `rac_basic` samples** (2024 to 2027; no Revit export): the same 23 type objects gain a `Manufacturer` in each release (66 values with the instances' copies), the 2024 file and its three later saves alike; `same-project gates` passes.
- **Unchanged:** the 24 R2025 models, the 17 other held-out models (their counts are `main`'s byte for byte), Einhoven, RE1 Architecture, the other 8 samples of 2024, the other 12 of 2025 to 2027 and the 33 of 2015 to 2023.

The rules do not reach a window's `FireRating`: R2025's `0.5h` are table entries under the family-parameter ids 12482 and 12220 (RE-185 section 3.3), not -1001206, so `Pset_WindowCommon.FireRating` stays missing with the same counts before and after (4 on each of `r2025-12` to `r2025-17`, 13 on each of the held-out 2024 to 2027 models, 11 on each of the held-out office 2024 to 2027 models, and 71 on Snowdon Towers Architectural; section 6).

**Which copies disagree.** The disagreement rule of section 4 is not exercised on the office, whose entries are all in `Partitions/425`. It leaves one type's text parameter out on Core Interior and one on Snowdon Towers Architectural (the diagnostics' warning, in no other model's); on Snowdon the door `FireRating` is 128 of 128 equal and the wall `FireRating` 710 of 710, so the type left out is none Revit's export holds a rating for there.

**Diagnostics against the census.** On the office the diagnostics' `decoded.parameter_value_count` goes 54,764 to 54,786, +22, against the census's +14. From the kept IFC: the 6 door types that hold a `FireRating` are 9 type objects (one Tag on 4 type objects, so 3 more than the Tags) and the 6 family types that hold a `Manufacturer` are 11 type objects (5 Tags on more than one, 5 more than the Tags), and 2 doors gain theirs: 9 + 11 + 2 = 22 values written, against 6 + 6 + 2 = 14 Tags that the scorer, which keys a type object by its Tag, counts. `type_object_pset_values` counts once per type-object group, which `step_writer::type_objects` keys by the type's id, the entity, the predefined type and the original symbol's GlobalId (Revit writes a type object per original symbol).

## 6. What stays out

- **`ThermalTransmittance` on door type 3261757** (3.7021; one value on each office file, missing 1 of 19): found in section 3 and left out, because Revit's exporters disagree on it (the maintainer's standing rule of 2026-10-31). Run [42122005108](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42122005108) (head c714d2cc, which wrote it from every door type holding the `-1005430` entry above zero) was equal on the office, 19 of 19, but wrote one value Revit's export does not on a door type of each of `r2025-12` to `r2025-17` (puzzbobb's models, Revit 25.0.2's IFC4 exports), and on RE1 Architecture (an IFC2X3 export by 26.2) 2 equal and 1 more where Revit writes none. Nothing in the model separates the office's type from those, so no rule narrows it. The AA house and the other held-out models had none.
- **One `Manufacturer` on RE1 Plumbing: its extras go from 2 to 3, on the one type Revit's export does not hold.** The type is 442362, `224200-FIXT-Rough In Water Closet Tank:Water Closet Tank_M` (`Generic`), the type of the water closet tank 442378. rvt-rs writes the tank; Revit's export holds neither the tank nor its type (RE-160, RE-169 section 4, RE-262), so every property written on it is a value Revit does not write: the two already counted, `Pset_EnvironmentalImpactIndicators.Reference` and `Pset_SanitaryTerminalTypeCommon.Reference`, and now this. It is not the class difference of #792 (the office's fixture 3095244): the object is absent. RE1's export is IFC2X3, which the standing scope counts apart and never removes a value for, and the defect is the tank being written, not its `Manufacturer`. The tank is tracked by [#1496](https://github.com/DrunkOnJava/rvt-rs/issues/1496) (known before as B38 in `docs/archive/BACKLOG.md`, blocked on B46, which is done) and pinned in `tests/element_records_2025.rs`.
- **`IsExternal` on type objects** (135 values as built): the maintainer's decision of 2026-10-31, because Revit's newer exporters write none (RE-286 section 3).
- RE-286 section 6's other leave-outs are unchanged: a curtain panel's door type (RE-269), a ramp's, railing's and curtain member's type, and `Pset_BuildingElementProxyCommon.IsExternal` on the building.
- **`Pset_WindowCommon.FireRating`, a measured residual ([#1507](https://github.com/DrunkOnJava/rvt-rs/issues/1507)).** R2025 models 12 to 17's windows hold their `0.5h` as table entries under the family-parameter ids 12482 and 12220 (two definitions named `Fire Rating` in the group `identityData`, RE-185 section 3.3), not under the built-in -1001206 that RE-354 reads, so nothing here reaches them. The value stays missing, with the same counts before and after this report: 4 on each of `r2025-12` to `r2025-17`, 13 on each of the held-out 2024 to 2027 models, 11 on each of the held-out office 2024 to 2027 models and 71 on Snowdon Towers Architectural (66 on its windows and 5 on its window types, against its local Revit IFC4 export).
- A `Manufacturer` on an element's own parameters: Revit's export writes none, so the instance gets none in any `Pset_` set. The type's Fire Rating and Manufacturer are also copied as Text onto every element of the type by `attach_type_text_parameters`, as the type parameters `Fire Rating` and `Manufacturer` of rvt-rs's own non-`Pset_` set (`identity_property_set`, `export_content.rs`): that is where the 50 non-`Pset_` values of Core Interior's +104 `IFCPROPERTYSINGLEVALUE` are. "The instance gets none" is about `Pset_` sets only, the ones Revit's export holds and the census scores.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=42707c3a -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
  -f probe=probe_re354_door_type_blocks -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=40948685 -f base=main -f nordic=true -f holdout=true -f r2025=true \
  -f models="nordic-office-as-built nordic-office-permit holdout-* r2025-* re1-* core-interior einhoven" \
  -f scorers="psets_vs_ifc qtos_vs_ifc" -f ifc_flags=--revit-set-placement
gh workflow run measure.yml --ref feat/1185-door-type-values -f ref=e07349c0 -f base=main -f nordic=true \
  -f r2025=true -f holdout=true -f samples=true -f keep_ifc=true
```
