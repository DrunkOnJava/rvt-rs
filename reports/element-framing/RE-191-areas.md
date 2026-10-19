# RE-191: Revit 2024 Areas from their partition chain records

**Date:** 2026-10-18
**Issues:** #763; follows RE-21 (the instance words), RE-35 (the partition record chain), RE-40 (design options), RE-117 (room number and name entries) and RE-145 / #800 (the latest copy).
**Artefacts:** the BIM4LCA office, concrete structure, as built and at building permit, with Revit's own IFC export of each (`research/bim4lca-measure.tsv`, CC BY-SA 4.0, read by byte range for a Measure run and never stored); the six reference models; Autodesk's sample projects.
**Probe and scorers:** `examples/probe_area_records.rs` and `tools/re/area_spaces_vs_ifc.py` (branch `re/763-area-records`); `tools/re/room_names_vs_ifc.py`, which now also compares each space's storey.
**Status:** positive on both office files. All of Revit's 30 area spaces are written with Revit's GlobalId, number, name, storey and space type, and without a body (run 39182294745).

## 1. Before

Revit's export of the office has 308 `IfcSpace`: 234 rooms, 30 Areas and 44 family instances (#763). rvt-rs wrote the 234 rooms. An Area's partition chain record carries `OST_Areas` (-2003200) at `+0x12` but not the bounding-box marker at `+0x50`, and the element-record scan finds records by that marker.

## 2. The record

Every Area has a chain record of 183 bytes (RE-35). Its frame holds RE-21's instance words:

- at `+0x32`, the container ElementId, unset (`ff` × 8) on every Area;
- at `+0x42`, the placement kind: `0xffffef7f`, the placed-instance kind, on the 30 Areas Revit exports and `0xffff8020` on the permit file's 2 others.

After those words come two ascending lists of ElementIds:

- six ids: the Area's Level, two ids of its area scheme, two more, and the Area's own id;
- two ids: the Level and 85092.

The Level is therefore the one recovered Level id the record names, not a fixed slot. On the as-built file the list order puts it at `+0x5a` on 16 of the 30 Areas and at `+0x6a` on the other 14.

The number and name are RE-117's `ROOM_NUMBER` and `ROOM_NAME` entries in the Area's data object (class `0x0e1c`), as for a room. Its data object also holds its own "Export to IFC As" `IfcSpace` on 29 of the 30.

`probe_area_records` against Revit's export, matched by Revit's GlobalId (run 39175563031):

| file | Areas | placed | in Revit's export | number and name | storey |
|---|---:|---:|---:|---:|---:|
| as built | 30 | 30 | 30 | 30 | 30 |
| building permit | 32 | 30 | 30, and neither unplaced one | 30 | 30 |

The permit numbers and names are from each Area's copy in the latest partition, `Partitions/398`. The older copies carry Finnish names (#800).

## 3. What Revit writes

`area_spaces_vs_ifc` lists Revit's attributes on the 30 paired spaces (run 39177901496). On all 30:

- `PredefinedType` `.SPACE.` and `CompositionType` `.ELEMENT.`;
- a `Body` `SweptSolid` `IfcExtrudedAreaSolid`;
- an `IfcSpaceType` `.NOTDEFINED.` of its own, named `<name> <number>:<ElementId>` and tagged with the ElementId, as Revit types a room (B78);
- `Pset_SpaceCommon` with `IsExternal` false and `Reference` `<name> <number>`;
- `Pset_SpaceHeaterTypeCommon.Reference` `<name> <number>`;
- `Qto_SpaceBaseQuantities.Height`, 3,400 to 5,970 mm.

Revit's exporter collects Areas with the other spatial elements, through the same design-option filter as every element, and does not export an Area that has no location (`ElementFilteringUtil.GetExportFilter` and `SpatialElementExporter` at revit-ifc `IFC_v24.3.40`, CLEANROOM.md source 5).

## 4. What changes

`partition_area_records::areas_from_partition_chain_records` reads each declared Area's chain record, from its latest partition. It keeps an Area that has no container, has the placed-instance kind, and is not in a non-primary design option. Each kept Area becomes a decoded `Area`: its number and name, and the Level its record names. The exporter writes it as `IfcSpace .SPACE.` under Revit's GlobalId, with:

- `Name` the number and `LongName` the name;
- the storey of its Level;
- its own `IfcSpaceType`;
- `Pset_SpaceCommon.Reference`.

It has no body. Revit 2024 only, the one release measured.

## 5. Results

Measure A/B against `main` (086b05b), run 39182294745. Both office files give the same counts:

| | before | after |
|---|---:|---:|
| `IfcSpace` of Revit's 308 written | 234 | 264 |
| spaces with Revit's GlobalId | 234 of 234 | 264 of 264 |
| spaces in the storey Revit aggregates them in | 234 | 264, none in another |
| spaces with Revit's number and name | 191 | 221 |
| `Pset_SpaceCommon.Reference` equal to Revit's | 191 | 211 |

- **Storeys.** `room_names_vs_ifc` now compares each paired space's storey. On `main` it gives the rooms 234 of 234 on both office files, 116 of 116 on Core Interior and 11 of 11 on RE1 Architecture. An earlier commit of this change wrote the Areas in no storey, until the Area's record set carried its Level.
- **No extra values.** rvt-rs writes no property value Revit does not write, unchanged from `main`.
- **Reference models.** The six export byte-identically: none has an Area.
- **Samples.** Run 39182297020 hashes each export. Of 64, two change: `IfcSpace` goes from 91 to 116 on Autodesk's 2024 `rac_advanced` and from 0 to 1 on its 2024 `rst_basic`. There is no Revit export of either to compare with.

## 6. What is not shown

- **A body.** The Area's boundary is not decoded, so its space has no body, where Revit's has an extrusion of the boundary.
- **The values not derived from what is read:** `Pset_SpaceCommon.IsExternal`, `Pset_SpaceHeaterTypeCommon.Reference` and `Qto_SpaceBaseQuantities.Height`.
- **The 44 family instances** Revit writes as `IfcSpace` (#763), which rvt-rs writes as proxies.
- **Releases other than 2024.** No Area of another release is measured against an export.
- **Phases.** Revit's exporter skips a spatial element in an invalid phase. rvt-rs reads no phases (#328).

## 7. Reproduce

```bash
gh workflow run measure.yml -f ref=main -f base=none -f nordic=true
gh workflow run measure.yml -f ref=re/763-area-records -f base=none -f nordic=true -f probe=probe_area_records -f probe_scorer=area_spaces_vs_ifc
```
