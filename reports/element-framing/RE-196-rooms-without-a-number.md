# RE-196 — A room with no number

**Date:** 2026-10-19
**Issues:** #35, #800; follows RE-117 (a room's number and name entries) and B78 (space types).
**Artefacts:** the BIM4LCA office, as built and at building permit, with Revit's own IFC export of each (`research/bim4lca-measure.tsv`); the reference models; Autodesk's sample projects and the R2025 models.
**Scorers:** `tools/re/room_names_vs_ifc.py` and `tools/re/psets_vs_ifc.py`, in runs [39232195456](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39232195456) and [39232199081](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39232199081).
**Status:** positive.

## 1. Before

RE-117 reads a room's number and name from two entries, `ROOM_NUMBER · u32 n · UTF-16 × n` followed by `ROOM_NAME · u32 m · UTF-16 × m`, and took them only when both were text of one or more characters. On the office, 43 of the 264 rooms Revit exports had neither. rvt-rs named their spaces `Room-<ElementId>`, with no `LongName`, no `IfcSpaceType` and no `Pset_SpaceCommon.Reference`. Revit's export names them `SHAFT:2367915`, `ELECT:3125549`, `CHASE:3127529`, `LIFT:1423872`, with `LongName` `SHAFT` and so on.

## 2. Revit's rule

Read in revit-ifc `IFC_v24.3.40` (CLEANROOM.md source 5; read and cited, no code copied):

- `IFCInstanceExporter.CreateSpace` takes a space's `Name` from `NamingUtil.GetNameOverride` of the room's `ROOM_NUMBER` (an `IfcName` parameter would override it), and its `LongName` from `ROOM_NAME`.
- Where that name is empty, `SetRoot` falls back to `NamingUtil.GetIFCName`. `GetIFCBaseName` makes it the element's name, here the room's display name, its name and number, then `:` and the element's id.
- The space's type is named the same base name (`STAIRCASE 001:1493779` for room 001 STAIRCASE), and Revit's export gives it as `Pset_SpaceCommon.Reference` without the id (`STAIRCASE 001`, or `CHASE` for a room with no number).

## 3. The rooms

The 43 rooms have an empty number: a `ROOM_NUMBER` entry of `n` 0, followed by their name. Read that way, every one gets Revit's name, `LongName`, type name and `Reference` (§5).

At building permit the same rooms have older copies in earlier partitions that hold a number and a Finnish name (`5322`, `TEKN.SÄH`). Revit's export follows the latest copy, whose number is empty and whose name is `ELECT`. RE-117 skipped the latest copy's empty number, so the older copy's pair won (#800).

## 4. What changes

- `scan_room_parameter_entries` reads an empty number or name, an entry of `n` 0, as an empty text. The latest partition's copy decides even where it is empty.
- `scan_text_parameter` (a room's Floor Finish, the Serial Number) reads an empty entry the same way and returns no empty value, so a value emptied in the latest copy is not taken from an older one.
- A space with a name and no number is named `<name>:<ElementId>`, and typed by an `IfcSpaceType` of the same name. Its `Pset_SpaceCommon.Reference` is its name. A room with neither keeps `Room-<ElementId>`, as nothing says what Revit's export names it.

## 5. Results

Measure A/B against `main` (e20a079):

| | main | RE-196 |
|---|---:|---:|
| As built: rooms with Revit's number and name, of 264 | 221 | 264 |
| At building permit: the same, of 264 | 221 | 264 |
| As built: property values equal to Revit's | 18,374 | 18,502 |
| At building permit: the same | 18,373 | 18,501 |

- The 43 rooms' spaces now pair with Revit's by name in `psets_vs_ifc`, so their `Pset_SpaceCommon.IsExternal` (43), `Reference` (43) and `HandicapAccessible` (42) count as equal instead of as values Revit does not write.
- They also carry the two sets rvt-rs gave every named room (`Pset_AirSideSystemInformation` and `Pset_ProductRequirements`, as Revit's IFC2X3 export of RE1 does), which the office's IFC4 export does not have. rvt-rs writes neither since #822.
- The reference models' exports are byte-identical, and so are all 64 samples' and 23 R2025 models' (run 39232199081), the Revit 2023 files among them.
- On one runner the exports take 0.97 to 1.04 times as long, the office files 1.02 and 1.03 (run 39232195456's timing job).

## 6. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true
```

`room_names_vs_ifc.txt` in each office model's results gives the number and name of every room against Revit's.
