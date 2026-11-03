# RE-317: an Area's boundary, from the area boundary lines around its point

**Date:** 2026-11-03
**Issues:** #763 (Areas as spaces); follows RE-191 (the Area records), RE-313 (every `IfcSpace` scored against Revit's) and RE-101 (a room's outline from its stored solid)
**Artefacts:** the BIM4LCA office, concrete structure, as built and at building permit (Revit 2024), with Revit's own IFC export of each (`research/bim4lca-measure.tsv`, CC BY-SA 4.0, read by byte range for a Measure run and never stored); the six reference models and puzzbobb's R2025 models (no Area in any); the held-out models, by counts only
**Probe and scorers:** `probes/examples/probe_re317_area_boundaries.rs` with `tools/re/area_boundaries_vs_ifc.py`; the exported bodies by `tools/re/space_profiles_vs_ifc.py` (RE-313)
**Runs:** the probe, [41705217941](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41705217941); the change against `main`, see §5
**Status:** positive on both office files. See §5 for the count of Areas drawn as Revit's export draws them.

## 1. Before

Revit's export draws each of the office's 30 Areas as an `IfcExtrudedAreaSolid` (RE-191 §3). rvt-rs wrote the 30 `IfcSpace` with number, name and storey and without a body, `SpaceOutlineSource` `none` (RE-313). The Area's chain record holds no box and no boundary.

## 2. Where the boundary is

The Area stores no boundary. What its record names, and what Revit draws, are these:

- **Its area boundary lines.** The record's id list names the Level, the area scheme (category `OST_AreaSchemes`, -2003201), and two elements of category `OST_AreaSchemeLines` (-2000079). Each of the office's 689 area boundary lines is a chain record of 217 to 241 bytes with the bounding-box marker, so the element-record scan reads it: its box is the line itself, since every one of the 689 is along an axis (346 horizontal, 343 vertical). Its reference list names its Level and its scheme, one of each on all 689, and its height is its Level's elevation to 1e-4 ft on all 689. The office's four schemes (`GFA 250mm`, `GFA`, `OFFICE`, `HEATED NETAREA`) hold 689 lines on 10 Levels, in 21 groups of a scheme on a Level.
- **A point.** Two `f64`s at byte 107 of the payload of the Area's data object (class 0x0e1c, RE-153) are a point in plan feet. It lies inside Revit's profile of the Area, the profile's holes taken out, on 30 of 30 Areas on both files (`area_boundaries_vs_ifc`, "point inside Revit's profile").
- **Its height.** Revit's exporter gives an Area no upper Level of its own and takes the height from its Level to the next Level that is a building story (`GetScaledHeight` and `LevelUtil.CalculateDistanceToNextLevel` in revit-ifc at `IFC_v24.3.40`, CLEANROOM.md source 5, read for what it writes, not copied). The extrusion depth of all 30 Areas is that distance to 0.0001 ft (11.1549 ft from Level_01 to Level_02a_Parking, 12.9593 from Level_02, 14.1732 from Level_04, 19.5866 from Level_05 to the Roof); the body's lowest height is its Level's elevation to 0.0001 ft on all 30 (the Area's base offset, which the file would hold if it were not zero, is read as zero); and `Qto_SpaceBaseQuantities.Height` is the same number.

## 3. The rule

The region of an Area is the one that the lines of its scheme on its Level enclose around its point. Revit's profile is exactly that on 30 of 30 Areas on both files (the symmetric difference of the two regions is 0.000 ft², checked with a prototype of the rule over the probe's lines and Revit's profile points, and then by the exporter in §5):

1. Take every line of the Area's scheme on its Level. Lines meet at their ends; a line that ends on another, or crosses it, splits both there. The two groups of the office that have such lines are `GFA` on Level_01 (a line ends on the middle of another's edge in two places, so that the Area `1` and the Area `P` share part of the edge) and `HEATED NETAREA` on the Basement (a line crosses another and both end beyond the crossing).
2. A line with a free end bounds nothing and is left out, repeatedly. The Basement's `HEATED NETAREA` has two such stubs, 0.66 and 1.1 ft long.
3. The Area is the smallest loop that encloses its point, and its holes are the loops directly inside that loop, whether or not an Area sits in them. Six `OFFICE` Areas have three or four holes each, `GFA 250mm` has two on each of four Levels and one on Level_05.
4. A vertex where two lines continue in a straight line is not a vertex of the outline (the `OFFICE` Area `TSTO3` has one, where Revit's profile has 32 vertices and the lines 33); a vertex where a third line meets is, and Revit's profile has it (the `GFA` Areas `1` and `P`, six vertices each).

**The point decides the side.** An Area's two named lines are on its boundary, but not always on its outer loop: those of `5` (`GFA 250mm`, Level_05) are on a rectangle inside a larger loop, and Revit's Area is the larger loop with the rectangle as a hole (1857.7 ft²), not the rectangle (208.3 ft²). Choosing the loop that holds the named lines would draw 29 of 30. The point separates the two, and the named lines are then a check: they must be on the region's boundary.

## 4. What changes

`partition_area_boundaries::Arrangement` builds the arrangement of a set of straight lines along the plan axes: the splits, the free ends, the rings of the remaining edges with the region on their left, and which ring is nested in which. `face_at` returns the region around a point as an outer loop and its holes. It returns `None` where the lines are not all along the axes, overlap, are more than 2048, or where the point is on a line, in no loop or in two of one size, or where the loop or one of its holes touches itself.

`partition_area_records` reads, for each placed Area of a Revit 2024 file: its point; its Level, its scheme and its named lines from its record; the lines of that scheme on that Level; and the elevation of its Level and the distance to the next building-story Level. It draws a body only where every one of these holds, and none otherwise:

- the Area's record names one Level, one scheme and at least one line, all in that scheme on that Level;
- every line of the scheme on the Level is along an axis at the Level's height, has one copy or agreeing copies, is a placed instance outside a non-primary design option, and has a Level and a scheme in its reference list (a line that has not is a line that may belong to any scheme on its Level: the group is not read);
- the point lies in exactly one region and the named lines bound it;
- a building-story Level lies above the Area's.

The exporter writes the region as the profile of an extrusion at the Level's elevation of the distance to the next storey. `RvtElementRecordGeometry.SpaceOutlineSource` is `area_boundary`, `BodySource` and `ProfileSource` are `partition_area_boundary_lines`, `BodyHeightSource` is `area_level_to_next_storey`. An Area drawn as a body is not a room, so it is left out of the spaces that hold a room's furniture (`contain_in_spaces`); Revit puts a family instance in its room.

## 5. Results

TODO: measured by the change, §5 of this report is filled from the Measure runs.

## 6. Not measured

- Revit releases other than 2024: no Area of another release is measured against an export, and nothing is read (fail closed).
- Boundary lines that are not along an axis (a slanted line, an arc): none on the office. A group that has one is not read, so its Areas keep no body, as before.
- An Area's base offset and its own "up to level": the office has none non-zero, and the file's fields for them are not read.
- An Area on the top storey, which Revit draws with a height of one unit: nothing on the office, and the Area is left without a body.
- Area polylines and the other Area kinds (`OST_AreaPolylines`, -2000830).
- Revit's phase filter, which skips a spatial element in an invalid phase (RE-184).
- The held-out BIM Whale 2021 project's 8 Areas ('Rentable' scheme): 2021 is not read for Areas, and the file is held out, so it was not examined. It is quoted by its counts only.

## Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-permit" \
  -f scorers=none -f probe=probe_re317_area_boundaries -f probe_scorer=area_boundaries_vs_ifc
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f nordic=true -f r2025=true -f holdout=true \
  -f scorers="space_profiles_vs_ifc tagged_spaces_vs_ifc room_names_vs_ifc"
python3 tools/re/area_boundaries_vs_ifc.py probe.txt revit-export.ifc
python3 tools/re/space_profiles_vs_ifc.py model.ifc revit-export.ifc --list 10
```
