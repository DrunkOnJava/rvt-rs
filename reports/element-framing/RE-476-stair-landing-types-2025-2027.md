# RE-476: a stair landing of a Revit 2025 to 2027 save was written with no type object because its type name was only read on Revit 2024

**Date:** 2026-11-15
**Issues:** #2046 (Revit 2027: stair landings are written with no `IfcSlabType` where Revit types them `LANDING`, CHL, 14 landings); follows RE-65 (stair component type names, Revit 2024), #323 and #1977.
**Artefacts:** Autodesk's `rac_advanced_sample_project` and `rac_basic_sample_project` saved by Revit 2024 to 2027, Snowdon Towers Architectural (2024), the 24 Revit 2025 models and the reference models Measure fetches; the confidential Revit 2027 model CHL as counts from its owner only.
**Probe:** `probe_2046_landing_types` (commits `986271bbf3e4` for the first run and `dd16af982701` for the last, archived under `refs/archive/2026-11-15/cited/`).
**Status:** positive. A landing of a non-monolithic type on Revit 2025 to 2027 now gets the type name and family Revit 2024's gets, and so its `IfcSlabType`. What Revit's own export of a 2025 to 2027 model writes for a landing is not scored by any model Measure fetches.

## 1. The question

HWT-25 ran 0.24.0 on CHL (Revit 2027): its 14 stair landings are `IfcSlab` parts of their `IfcStair` with no type object, where Revit's export types each by an `IfcSlabType` `LANDING`. Every other slab agrees. On Revit 2024 the landings' types match Revit's. Where does the join stop on 2027?

## 2. What the code did

A type object is made for an element only when the decode gives it a type id, a family name and a type name (`step_writer::type_objects`). For a stair part:

- Revit 2024: `scan_component_types` reads a landing type's family and name from byte offsets measured on Revit 2024 (RE-65, `partition_stairs`: "Nothing here reads a release other than 2024"). On any other release it returns nothing.
- Revit 2025 to 2027: `native_stair_parts` names a stair, its flights and its stringers from their native records, and its doc left a landing out ("none is measured").

So a 2025 to 2027 landing had a type id and no family or type name. Not the cause: the multistory copies of RE-463, which `MULTISTORY_REVIT_VERSIONS = [2024]` keeps to 2024.

## 3. The change

`native_stair_parts` takes the landings (id, stair, type) that `attach_native_stair_names` reads as it reads a run's: the type is the one `OST_StairsLandings` type definition the landing's element record names, the stair the one it is aggregated under or the one stair that record names. The type's name is its `SymbolInfo.m_name`. The family is `Non-Monolithic Landing` where `StairsLandingType.m_hasMonolithicSupport` is false, as the type byte of Revit 2024's landings gives (RE-65). A monolithic landing type's family is not measured on any release and stays out. A landing then gets the Name `<stair> Landing N` as on 2024.

## 4. The registered rows and what they gave

Registered in the draft PR before any run was read.

1. **Landings typed.** Met on the samples that hold landings. `rac_advanced` 2025: 5 of 5 landings typed `Non-Monolithic Landing` (0 of 5 on `main`, 5 of 5 on its 2024 save). Measure 43685123916 on the head and 43684535165 on `main`.
2. **Held to the same project's 2024 save.** Met: the 5 landings of `rac_advanced` 2025 carry the `Family:Type` of their 2024 save's, by ElementId, 5 of 5 (`Non-Monolithic Landing:Non-Monolithic Landing`; Measure 43685809366).
3. **Held to Revit.** Not scored. None of the 24 Revit 2025 models holds a stair landing, and no Revit export of a 2025 to 2027 model with one is in Measure. The check is HWT-25's counts on CHL (the family word and the number of landings of a monolithic type), which are asked for on #2046.
4. **Nothing else moves.** Met: the full Measure 43685120427 against `main` has 117 of 120 exports equal, the three that move are the `rac_advanced` saves of 2025, 2026 and 2027, 102 of 102 diagnostics equal and no scorer output moved. A line-by-line diff of the kept exports of 2025 and 2026 (ids set aside, Measure 43687905412) differs in the creation time and in the five landings alone: each landing's Name (`StairsLanding-358449` to `Assembled Stair:Stair:358313 Landing 1`) and, on 2025, its `ObjectType`; one `IfcSlabType`, one `IfcRelDefinesByType` and their property sets. On 2026 the `ObjectType` stays `$`, as Revit's export of that release writes it (#1472).
5. **Reported.** `rac_basic`'s one landing stays untyped on 2024 and on 2025 (its type is not one the rule names: a monolithic type or one whose name is not read); it is unchanged.

## 5. What this decides and does not

- **Decided.** The landings of a non-monolithic type on 2025 to 2027 get the type and family 2024's get, the same on the same project.
- **Not decided.** The landing's Name and `ObjectType` on 2025 to 2027 against Revit's own export (none exists to score them); the family of a monolithic landing type; CHL's 14.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=dd16af98270104c02aca9ab9c1c5facb877006ad -f base=none -f samples=true \
  -f sample_years='2024 2025 2026 2027' -f scorers=none -f probe=probe_2046_landing_types
# one `landing types:` line per file with a count of the landings and how many are typed;
# the `landing <id> type <name>` lines pair a save with the same project's other saves
```
