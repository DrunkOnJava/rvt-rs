# RE-477: a Revit 2015 to 2023 door or window the reference-list join leaves untyped takes Revit's type from its own native record

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows and 14 railings of the spent Revit 2018 project name no type); follows RE-475 (where the type is in the native record), RE-473 (the type material map before 2024), RE-109 and RE-110 (family and type names, type objects), RE-167 (the original symbol).
**Artefacts:** the spent Revit 2018 project and its held-out Revit 2018 save (counts only), a held-out Revit 2020 project (counts only), BIM Whale's held-out Revit 2021 `AdvancedProject` (counts only, its reference export is IFC2X3), the Autodesk sample projects of 2015 to 2024; Revit's own IFC2X3 export of the spent project, the oracle for the Name, ObjectType and type object rows.
**Commits:** `e0ae4afb861c` (Measure 43689445542, the first run), `d6890e26fa5b` (43690822430, the second, filtered to `spent-*` and `holdout-bimspeed-*`) and `bb829b7a17a4` (43692758207, the third: every scorer, 120 exports, `base` the merge base `5f442efc`), archived under `refs/archive/2026-11-15/cited/`. The group rule narrowed between the second and the third (section 2); nothing else in the reader changed.
**Status:** positive for the names and the type objects, with one registered row not met (row 5, on one held-out) and the gaps stated in section 5. A door or window of a Revit 2015 to 2023 file that the scan's list join leaves untyped now takes the type id, the type's name and the family's name its native record names, and so the Name, `ObjectType`, type object and type properties Revit's export gives it. Windows and doors of the spent project: 70 of 70 and 68 of 68 equal to Revit's.

## 1. The question

RE-475 found where Revit's type is for the spent project's windows: `InstanceInfo.m_symbolId` (the type's Tag), the master symbol's `SymbolInfo.m_name` (its name) and `Family.m_name` (the family's). It did not write them. Does a reader that takes those three pieces for every door and window the list join leaves untyped give Revit's Name `Family:Type:Id` and type object, and does it leave everything else where it was?

## 2. The change

`native_instance_types::instance_types` reads, in one pass, the instances, every `FamilySymbol` and every `Family` record, on Revit 2015 to 2027 and in either id shape (RE-280). For an instance whose record is a `FamilyInstance` with an `InstanceInfo.m_symbolId` and an `m_masterSymbolId`, whose master symbol is a `FamilySymbol` with a `SymbolInfo.m_name` and an `m_familyId`, and whose family is a `Family` with a `Family.m_name`, it returns the three pieces. Any link missing and the instance is left as it was.

`recover_2023_records` calls it for the doors and windows it keeps that have no entry from the list join, and adds the type id, type name and family name fields. It also marks them `m_type_from_native_record`, which `attach_type_materials` reads (section 4).

Two measured details:
- **The type id.** `InstanceInfo.m_symbolId` for all 70 windows. For the spent project's 2 doors it is not Revit's Tag: both are members of a model group (`ElementGroupMembership.m_groupId`), their `m_symbolId` is a copy of the master made for the group's instance (`FamilySymbol.m_masterId` names the master; the copy holds `FamilyElemRef` objects and no name), and Revit's Tag is the master, `m_masterSymbolId`. An instance with a nonzero group id takes its master symbol where its `m_symbolId` is that master or a symbol whose `m_masterId` names it, and is left untyped otherwise (a grouped window is not measured); the rest take `m_symbolId`.
- **The materials.** The scanned material map of the symbol that draws a window is not the type's: in the first run it gave 55 windows a set, 53 of them apart from Revit's. A type read from the native record takes none until the native map is read (RE-473 section 6).

## 3. The registered rows

Registered on #1603 before each run was read (comments of 2026-11-15, the second the amendment). Rows 1 to 4, 7 and 8 were read from the second run (`d6890e26`, 27 exports) and confirmed on the third (`bb829b7a`, 120 exports); rows 3 and 5 are stated with the third, which held a held-out project the second did not.

1. **Windows.** Met. On the spent project the 70 windows' Name and `ObjectType` are equal to Revit's, 70 of 70 (0 of 70 on `main`); on the held-out Revit 2018 save 70 of 70; on the held-out Revit 2020 project the 4 that were different are equal.
2. **Doors.** Met. The spent project's 2 doors are equal (66 of 68 on `main`, 68 of 68 now).
3. **Held-out.** Met. On the held-out Revit 2018 save windows 70 of 70 and doors 68 of 68; on the Revit 2020 project 29 of 29 windows and 27 of 27 doors, where 25 and 18 were equal; on BIM Whale's Revit 2021 `AdvancedProject` (third run only) the 115 doors that differed are equal, 126 of 126 (11 of 126 on `main`). The other held-out projects (Revit 2021) are byte-identical.
4. **Type objects.** Met as registered, not in the first run. `type_objects_vs_ifc` on the spent project: typed in ours 1,005 to 1,077 of 1,280, entity agrees 1,077 of 1,077, Tag agrees 938 to 1,010 of 1,077 and no Tag disagrees, untyped windows and doors 72 to 0, the 8 typed in ours and not in Revit's unchanged. The first run had 2 Tag disagreements, the grouped doors (row 7).
5. **Nothing else moves.** Not met on one held-out; met on the rest. In the third run 13 of 120 exports move: the spent project, the three held-out projects with such elements (Revit 2018, 2020 and BIM Whale's 2021) and the nine saves of `rac_basic` from 2015 to 2023, where the same two doors take the name `Entrance door:Entrance door:<id>` that the 2024 save gives them (they were `Door-<id>`; each save gains exactly 4 entities and 4 parameter values). The 107 others are byte-identical. `psets_vs_ifc`: different unchanged everywhere (0 on the spent project and the Revit 2018 and 2020 held-outs, 2 on BIM Whale's), rvt-rs values Revit does not write unchanged on the spent project and the Revit 2018 and 2020 held-outs (57, 54 and 18), equal 3,496 to 3,640 of 5,207 on the spent project (`Pset_WindowCommon.Reference` 0 to 70 of 70, `Pset_DoorCommon.Reference` 66 to 68 of 68) and 4,050 to 4,194 of 6,030 on the held-out Revit 2018 save. **The row fails on BIM Whale's Revit 2021 project:** rvt-rs values Revit does not write go from 13 to 23, all `Pset_DoorCommon.Reference` on doors (1 to 11 of the 126, with `Pset_WindowCommon.Reference` unchanged at 6). Of the 115 doors newly typed there, 105 take a `Pset_DoorCommon.Reference` equal to Revit's and all 115 a `Pset_EnvironmentalImpactIndicators.Reference` equal to Revit's (1,343 to 1,458 equal, different unchanged at 1); for 10 of them Revit's export has no `Pset_DoorCommon.Reference`. The reader does not decide this: `main` already writes the door-set `Reference` for the elements the join types and shows the same residual there (1 door of 11, and 6 windows). That project's reference export is IFC2X3, which removes no value by the standing rule (an IFC2X3 view never removes one), and the held-out is counts only, so the 10 doors cannot be listed to find what separates them (filed as #2053).
6. **Where it would be wrong.** Met by absence. The run does not count the instances it leaves out; on the spent project no door or window is left untyped, and on the held-out projects no name differs.
7. **Group members** (amendment). Met: 0 Tag disagreements, 72 of 72 new type objects with Revit's Tag.
8. **No scanned materials** (amendment). Met: `element_materials_vs_ifc` of the spent project is as on `main` (different sets 0, no materials from rvt-rs for the 70 windows).

## 4. What the first run showed

Run 43689445542 (`e0ae4afb`) gave the names and psets of rows 1 to 3, and two things not registered: 2 Tag disagreements on the grouped doors, and `element_materials_vs_ifc` different sets 0 to 53 (`IfcWindow`, all "apart"). The second run (`d6890e26`) is the one the rows above are read from.

## 5. What this decides and does not

- **Decided.** A door or window of Revit 2015 to 2023 that the list join leaves untyped is typed from its native record as above, with the group rule for the type id.
- **Evidence.** The windows: 70 on the spent project and held-out save, 4 more on the held-out Revit 2020 project (Name and `ObjectType` only). Doors: 68 on the spent project and held-out save, 9 on the Revit 2020 project, 115 on BIM Whale's Revit 2021 project (Name and `ObjectType`). The group rule rests on 2 doors of one project; it is listed in `docs/one-project-rules.md`.
- **Not decided.**
  - **The door-set `Reference` where Revit writes none.** 10 of BIM Whale's 126 doors take a `Pset_DoorCommon.Reference` that Revit's IFC2X3 export does not write (section 3, row 5, unmet). It is the existing `Reference` rule's residual, which `main` shows on 1 door and 6 windows of the same project; what separates those elements is not known, and the held-out project gives counts only. Narrowing the rule for the elements typed here would give up 105 equal door values on that project and the 70 and 68 on the spent project, and leave the residual that `main` already has. #2053 holds it.
  - **The MCP answers.** `mcp_answers_vs_ifc` counts 5 more property values Revit's export does not write on the spent project (69 to 74, with 105 to 110 equal) and 4 more on BIM Whale's (38 to 42); `psets_vs_ifc` shows none on the spent project. The answers are a sample of elements, and the 5 are not attributed.
  - **The type objects' GlobalIds.** The 72 new type objects' GlobalIds are not Revit's (`global_id agrees` stays 414 of 1,077): before Revit 2024 the type GlobalId is not read from the original symbol (RE-167 reads it from 2024), as for 591 of the project's 1,005 existing type objects.
  - **The type objects' Names** differ from the IFC2X3 export's on every type of the project (`name agrees` 0 of 1,077), as before.
  - **The windows' and doors' materials** from the type map: the native map in the older id shape, for the right symbol, is the next reader (RE-473).
  - **Railings and stairs** name no type in their native record (0 of 20, RE-475) and are unchanged.

## 6. Reproduce

```bash
# the full run, every scorer (the third; rows 3 and 5 and the 13 of 120 exports)
gh workflow run measure.yml --ref main -f ref=bb829b7a17a4735319a733b38ae7693b97cbcb65 -f base=5f442efce856ee7b06c8fe796f3a646ac37fffb8 \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true

# the second run, filtered (rows 1 to 4, 7 and 8)
gh workflow run measure.yml --ref main -f ref=d6890e26fa5b318df1a4d6c0a4cfa2b92c77ddbf -f base=1320cd23c99c5e7752281c00db4c57fd22e81095 \
  -f samples=true -f sample_years='2018 2023 2024' -f holdout=true -f models='spent-* holdout-bimspeed-*'
# names_vs_ifc, type_objects_vs_ifc, psets_vs_ifc and element_materials_vs_ifc of the spent project, and the counts of the held-out ones
```
