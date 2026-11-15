# RE-475: the 68 windows of the spent Revit 2018 project name Revit's own type in their native record, which the scan's reference-list join cannot see

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows and 14 railings of the spent Revit 2018 project name no type); follows RE-473 (section 5, "the next row"), RE-109 (name entries), RE-280 (the id shape before 2024), RE-240 (which record names an untyped element's type).
**Artefacts:** the spent Revit 2018 project and its held-out Revit 2018 save (counts only), a held-out Revit 2020 project (counts only), the Autodesk sample projects of 2018, 2023 and 2024; Revit's own IFC2X3 export of the spent project, the oracle.
**Probes:** `probe_1603_unjoined_lists_pre2024` and `probe_1603_native_instance_types_pre2024`, scored by `tools/re/unjoined_types_vs_ifc.py` and `tools/re/untyped_types_vs_ifc.py`. The commits are archived under `refs/archive/2026-11-15/cited/`: `d1a2b67cc60d` (Measure 43682867776), `16dbf242c69f` (43684406836), `e80cb1b5332d` (43685039364), `bebbe901a03a` (43686411026) and `82a5e79c3f39` (43687761375).
**Status:** negative for the list rule RE-473 asked about, positive for another route. The window-only id in a window's reference list is not its type: it is the type's master symbol. Revit's own type id, the type's name and the family's name are all in the window's native record and the records it names, by member name, on Revit 2018. Nothing in the output changes.

## 1. The question

RE-473 section 5 left one row open: whether a window's type can be recovered from its reference list "by another rule (the one named id without a record that other windows of the file share, with the family told apart from the type)". 68 of the spent project's 70 windows name a named id with no record of its own, and the type join (`type_and_family_names_2023`) finds no symbol record for them.

## 2. What the probes count

- `probe_1603_unjoined_lists_pre2024`: per category, the instances with and without a symbol record, how many ids that no scanned record carries and that only this category's records hold each holds, how many instances hold each, and, for each instance without a symbol record, its whole list with what the scan knows of every id (a name entry, a record). Counts for held-out files; the per-instance lines are dropped by the scrub.
- `probe_1603_native_instance_types_pre2024`: for each window, door, railing, stair and furniture the decode gives no type, every ElementId its native record names (both id shapes, `native_json`'s), then the same for the symbols those records name and the strings of the families they name.
- The scorers hold both against Revit's export of the same file, by the element's Tag.

## 3. Measured

**The list rule is wrong (run 43682867776, 43684406836, 43685039364).** On the spent project and on the held-out save of the same project (72 windows, 4 with a symbol record, the same 68 without):

- each of the 68 windows holds exactly one id that only window records hold and that has a name entry and no record; 43 distinct ids, 29 held by one window, 14 by 2 to 9, none by 10 or more, at most 4 by one id;
- none is held by a window with a symbol record, and none is the family of a symbol record;
- the id's name equals Revit's type name for 68 of 68 windows, and the id is Revit's type id (the Tag of the type in Revit's export) for **0** of 68;
- Revit's own type id is in the window's list for 68 of 68, as an id with neither a name entry nor a record. The scan knows nothing of it.

So a rule that took that id as the type would name the type right and give it the wrong ElementId, and it would be a guess from this project's habit of naming a type as its master.

**The native record names all of it (runs 43686411026, 43687114757, 43687761375).** Read by member name with the releases widened to 2015 to 2027 (the default refuses Revit 2018):

| Piece | Where | Windows (70) |
| --- | --- | ---: |
| Revit's type id | `FamilyInstance`'s `InstanceInfo.m_symbolId` | 70 |
| The type's name | `SymbolInfo.m_name` of the symbol `FamilyInstance.m_masterSymbolId` names | 70 equal to Revit's (the scorer counted 68, see below) |
| The type's own `SymbolInfo.m_name` | the symbol at `m_symbolId` | empty on 70 of 70 |
| The family | `FamilySymbol.m_familyId`, a `Family` record with `Family.m_name` | 104 of 104 symbols name one |
| The family's name | `Family.m_name`, such as `OB_1H` for Revit's `OB 1H` | read on the 9 families printed |
| The type names of a family | `FamilyTypeTable.m_pairs[i].name` | present |

Every window names a master symbol with a name (70 of 70), and the name is Revit's type name on all 70. `untyped_types_vs_ifc.py` counted 68 because it reads an underscore in the probe's output as a space, and two of the names hold a real underscore (`T16_75x90`, the master's and Revit's alike). The two doors the probe lists read the same way (`m_masterSymbolId` is the Tag). Railings (14) and stairs (6) do not name their type's id in their native record: 0 of 20.

**Where else the single id is the type.** For one other category of the spent project (Revit category id -2000171, 206 instances) the one id only its records hold is Revit's type Tag for 206 of 206, a named id with no record. That is a system type, which the scan never needed a symbol record for, and it is a separate rule.

## 4. The registered rows

Registered in RE-473 section 5 as "the next row", and here as the questions the runs answered:

1. **The shared id is the type or the family.** Not met: it is the master symbol, 0 of 68 equal to Revit's type id.
2. **Revit's type id is recoverable.** Met by the native record, not by the list: 70 of 70.
3. **The type's name.** Met for 70 of 70 through the master symbol (68 by the scorer's reading of underscores).
4. **The family's name.** Read from the `Family` record on the 9 families printed; not yet held to Revit's name for every window.

## 5. What this decides and does not

- **Decided.** A reader for a Revit 2015 to 2023 family instance that no symbol record joins would take the type id from `InstanceInfo.m_symbolId`, the type's name from its master symbol and the family's name from the family record. Each piece is equal to Revit's for the windows of one project (the family's name on the families printed).
- **Not decided.** The output: the Name `Family:Type:Id`, the `ObjectType`, the type object and the material set a type gives. They need a reader change scored against Revit's export of the spent project and its held-out save, and against the other pre-2024 models with exports. One project is the evidence for the table; the rule is not general until other projects hold it.
- **Railings and stairs** are another route: their type is not named in their record (0 of 20).

## 6. Reproduce

```bash
gh workflow run measure.yml --ref research/1603-unjoined-lists-pre2024 -f ref=82a5e79c3f394d894f4e450080463394b760e75c -f base=none \
  -f samples=true -f sample_years='2018 2023 2024' -f holdout=true -f models='spent-* holdout-bimspeed-*' -f scorers=none \
  -f probe=probe_1603_native_instance_types_pre2024 -f probe_scorer=untyped_types_vs_ifc
# after this lands, `--ref main` holds the probes, the scorers and the scrub patterns
# the lists: the same with -f ref=e80cb1b5332d1ce44816dee798e307fd6f1c618e -f probe=probe_1603_unjoined_lists_pre2024 -f probe_scorer=unjoined_types_vs_ifc
```
