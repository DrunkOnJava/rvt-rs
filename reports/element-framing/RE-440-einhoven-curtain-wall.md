# RE-440: Einhoven's "ArcWalls" are its curtain wall's panels and grid lines

**Date:** 2026-11-12
**Probe:** `probes/examples/probe_re440_curtain_wall_records.rs`
**Files:** Revit 2023 `Revit_IFC5_Einhoven.rvt` (MIT, `magnetar-io/revit-test-datasets`), Autodesk's 2023 `rac_basic_sample_project` and `rst_basic_sample_project`, the 2023 `racbasicsamplefamily`, and every local 2015 to 2022 file for the comparison of section 6.

## 1. The question

The web viewer shows seven warnings on Einhoven:

- 24 "ArcWall records lack recovered thickness";
- 13 elements with no storey;
- two building storeys named from partition strings;
- two storeys left with elevation labels;
- the 2023 coverage summary;
- the suppressed `HostObjAttr` candidates;
- the unit warning, which RE-439 answers.

Its export held 26 `IfcWall`s, 9 `IfcPlate`s, 2 `IfcWindow`s and four storeys: `Level 1`, `Elevation 6.562 ft`, `Elevation 13.123 ft` and `Roof`. 24 of the walls came from the standard ArcWall byte pattern (RE-14.3, RE-15), which runs on Revit 2023 only. The question is what those 24 records are.

## 2. What the pattern matches

On Einhoven the pattern matches 24 records in `Partitions/5`. Their trailers give 22 ElementIds; 2 matches have none. Each id was looked up in the file's native records (`native_document::extract_graphs`, the record's class from its body's first tag):

| Native class | Matches | ElementIds |
|---|---|---|
| `FamilyInstance` (curtain panel) | 15 | 7301 to 7303, 8575, 8580, 8639 to 8643, 8682 to 8686 |
| `CurtainGridLine` | 7 | 7289, 7291, 7293, 7299, 8578, 8637, 8680 |
| no ElementId | 2 | Partitions/5 at 90346 and 109160 |
| `ArcWall` or `SWall` | 0 | |

None is a wall. Every id is in `Global/ElemTable` with owner 7187.

The pattern matches nothing on any other 2023 file: `rac_basic`, `rst_basic` or the sample family.

## 3. What the file holds

The file's wall records (`SWall`, `ArcWall`) are two:

- 2808 is a basic wall of type 1936.
- 7187 is of type 1937, a `NewCurtainWallType` named `Curtain Wall 1`.

Wall 7187 is 19.685 ft long (6 × 3.281) and 26.247 ft high (4 × 6.562). It carries a `CurtainGrid`, and its `CGridDivideFaceGStep.m_cellMap` lists 24 cells, each with a panel:

- 9 panels of type 8378: 7197, 7290, 7292, 7294, 7300, 8574, 8579, 8638 and 8681.
- 15 panels of type 8570: the 15 `FamilyInstance`s of section 2.

Both types are `FamilySymbol`s of family 7572, `Family1`. Every panel names 7187 as its host (`m_hostId`) and Level 1905.

The file's `Level` records are one: 1905, `Level 1`, a building storey. The four base elevations of section 1 (0, 6.562, 13.123 and 19.685 ft) are the four rows of the curtain wall's grid.

So the model is:

- one basic wall;
- one curtain wall of 24 panels on 8 interior grid lines;
- two windows;
- one Level.

## 4. What the old export did with them

The walker merged the 24 matches before the 2023 element records. So the 15 panels' records arrived with ElementIds already taken and were dropped ("held", #1389), and only the 9 other panels were written.

The base elevations made four storeys. The partition reader's lone-Level rule refused Level 1905 on 2015 to 2023 because "Einhoven decodes one Level while its walls stand at two or more base elevations" (`partition_level_records`). Those walls were the panels.

Wall 7187 was written as an `IfcWall` named `Wall-7187`: on 2023 nothing reads a curtain wall from its mullions, because Einhoven has none.

## 5. What changes

- `partition_arc_walls`: a pattern match is kept only where the native record of its ElementId is an `ArcWall` or `SWall`. A match with no ElementId, or whose record cannot be read, is left out. On Einhoven none is kept; on every other local file there was none to keep.
- `partition_level_records`: on 2015 to 2023 a single recovered Level is the storey set when the file's native `Level` records are that Level alone. On 2024 to 2027 the broader id scan still decides, as before.
- `partition_schema_mvp/records_2023`, on Revit 2023 only (`CURTAIN_WALL_2023_REVIT_VERSIONS`, the release measured): a wall whose native record's type (`m_WallAttributesId`) is a `NewCurtainWallType` is a curtain wall, named by that type's `SymbolInfo.m_name` where its record holds one (the class alone makes it a curtain wall, as on 2024). The walls' types come from the wall-record read `read_wall_records` shares (`WallReads::type_ids`, #1335), and the types' own records from one read of those types (`native_walls::curtain_wall_types`). Each panel or mullion whose reference list names exactly one curtain wall is its part, as on 2024 (RE-46).
- `ifc::geometry_gap_skipped_items`: a whole whose aggregated parts carry its body is not counted as missing dimensions or profile; it is counted in `building_elements_carried_by_parts`. Both read one rule, `wholes_carried_by_parts`: a whole is carried only where at least one of its parts has a body, so a whole whose parts have none is still a gap. This removes a false gap on every bodiless curtain wall whose panels are drawn, RE1 Architecture's (2025) included.

## 6. Results

Einhoven, `main` (`3abb42f0`) and this change:

| | `main` | RE-440 |
|---|---|---|
| `IfcWall` | 26 (24 pattern matches, 2808, 7187) | 1 (2808) |
| `IfcCurtainWall` | 0 | 1, `Curtain Wall:Curtain Wall 1:7187`, aggregating 24 panels |
| `IfcPlate` | 9 | 24, the 24 cells' panels |
| `IfcWindow` | 2 | 2 |
| storeys | 4, from the matches' base elevations | 1, `Level 1` at 0 |
| elements on a storey | 24 of 37 | 28 of 28 |
| warnings | 7 | 4 |

The warnings that remain are:

- the 2023 coverage summary;
- the units warning, which RE-439 removes;
- the note that all 28 elements reached their storey through their own record;
- the suppressed `HostObjAttr` candidates.

Every local 2015 to 2023 file was exported by `main` and by this change (30 files):

- 28 are identical apart from `FILE_NAME` and `IFCOWNERHISTORY`.
- On 2023 `rac_basic` the only changes are the names and type properties of its 9 curtain walls. They were `Curtain Wall:p` on 4 and unnamed on 5, and are now `Curtain Wall:SH_Curtain wall` on all 9, the name the 2024 and 2025 saves of the same model give the same ElementIds.
- 2023 `rac_basic` also loses the false missing-body gap on those 9 curtain walls.

RE1 Architecture's IFC is unchanged, and its diagnostics lose the same false gap on its one curtain wall.

## 7. Limits

- No Revit export of Einhoven exists to compare with. The witness is the file's own records: the classes of the matched ids, the curtain wall's cell map and its type's class. The same model saved by 2024 and 2025 is the witness for `rac_basic`'s names.
- The ArcWall pattern stays in place for a 2023 file whose matches are walls, but no local file has one, so the half of `native_walls_only` that keeps a match has no test, and neither has `elem_table::link_arcwall_element_ids`. What keeps that half from being wrong is the test it applies: it keeps only a match whose ElementId's own native record is an `ArcWall` or `SWall`, the class test `native_walls` uses on 2015 to 2027. A 2023 file with a real ArcWall match would check it.
- A wall that is now a curtain wall is no host for `bind_opening_hosts`, so a door or window whose host it was loses that host, as on 2024 (#439). On Einhoven neither window is in it: both void wall 2808 (Measure run 43509948454, `keep_ifc`). Whether a door of 2023 `rac_basic`'s 9 curtain walls moves is what the unfiltered Measure on the final head shows.
- The 2023 coverage summary lists what the 2023 reader does not read: join lists, end steps, profile openings and `Length`, parameters, "Export to IFC As" overrides and components nested in doors and windows. That is unchanged.
