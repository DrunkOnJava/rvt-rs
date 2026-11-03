# RE-317: an Area's boundary, from the area boundary lines around its point

**Date:** 2026-11-03
**Issues:** #763 (Areas as spaces); follows RE-191 (the Area records), RE-313 (every `IfcSpace` scored against Revit's) and RE-101 (a room's outline from its stored solid)
**Artefacts:** the BIM4LCA office, concrete structure, as built and at building permit (Revit 2024), with Revit's own IFC export of each (`research/bim4lca-measure.tsv`, CC BY-SA 4.0, read by byte range for a Measure run and never stored); the six reference models and puzzbobb's R2025 models (no Area in any); the held-out models, by counts only
**Probe and scorers:** `probes/examples/probe_re317_area_boundaries.rs` with `tools/re/area_boundaries_vs_ifc.py`; the exported bodies by `tools/re/space_profiles_vs_ifc.py` (RE-313)
**Runs:** the probe, [41705217941](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41705217941), and again at the final head with the point check, [41713925390](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41713925390); the Level settings of the office, [41721498984](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41721498984); the change against `main`, unfiltered, with the nordic, R2025 and held-out models, [41723183283](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41723183283) (the first version of the change, [41708150034](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41708150034)), and on Autodesk's 2024 samples, [41723200570](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41723200570)
**Status:** positive on both office files. All 30 Areas of each are drawn, and each equals Revit's in area (within 0.1%), in outline (within 0.01 ft) and in base and top (within 0.01 ft); no other export moved.

## 1. Before

Revit's export draws each of the office's 30 Areas as an `IfcExtrudedAreaSolid` (RE-191 §3). rvt-rs wrote the 30 `IfcSpace` with number, name and storey and without a body, `SpaceOutlineSource` `none` (RE-313). The Area's chain record holds no box and no boundary.

## 2. Where the boundary is

The Area stores no boundary. What its record names, and what Revit draws, are these:

- **Its area boundary lines.** The record's id list names the Level, the area scheme (category `OST_AreaSchemes`, -2003201), and two elements of category `OST_AreaSchemeLines` (-2000079). Each of the office's 689 area boundary lines is a chain record of 217 to 241 bytes with the bounding-box marker, so the element-record scan reads it: its box is the line itself, since every one of the 689 is along an axis (346 horizontal, 343 vertical). Its reference list names its Level and its scheme, one of each on all 689, and its height is its Level's elevation to 1e-4 ft on all 689. The office's four schemes (`GFA 250mm`, `GFA`, `OFFICE`, `HEATED NETAREA`) hold 689 lines on 10 Levels, in 21 groups of a scheme on a Level.
- **A point.** Two `f64`s at byte 107 of the payload of the Area's data object (class 0x0e1c, RE-153) are a point in plan feet. It lies inside Revit's profile of the Area, the profile's holes taken out, on 30 of 30 Areas on both files (`area_boundaries_vs_ifc`, "point inside Revit's profile", run 41713925390), and the two lines the Area's record names lie on the outline of Revit's profile on 30 of 30.
- **Its height.** Revit's exporter ignores an Area's own upper Level (`GetScaledHeight` in revit-ifc at `IFC_v24.3.40`, CLEANROOM.md source 5, read for what it writes, not copied) and takes the height from `LevelUtil.CalculateDistanceToNextLevel`: the distance to its Level's story above (`LEVEL_UP_TO_LEVEL`) where that is a building story, else the distance to the next building story. The story above is not read. The extrusion depth of all 30 Areas is the distance to the Level above it, which is the next building story, to 0.0001 ft (11.1549 ft from Level_01 to Level_02a_Parking, 12.9593 from Level_02, 14.1732 from Level_04, 19.5866 from Level_05 to the Roof); so on the office a Level's story above is the next Level up, or is not set. The body's lowest height is its Level's elevation to 0.0001 ft on all 30 (the Area's base offset, which the file would hold if it were not zero, is read as zero); and `Qto_SpaceBaseQuantities.Height` is the same number.

## 3. The rule

The region of an Area is the one that the lines of its scheme on its Level enclose around its point. Revit's profile is exactly that on 30 of 30 Areas on both files (the symmetric difference of the two regions is 0.000 ft², checked with a prototype of the rule over the probe's lines and Revit's profile points, and then by the exporter in §5):

1. Take every line of the Area's scheme on its Level. Lines meet at their ends; a line that ends on another, or crosses it, splits both there. The two groups of the office that have such lines are `GFA` on Level_01 (a line ends on the middle of another's edge in two places, so that the Area `1` and the Area `P` share part of the edge) and `HEATED NETAREA` on the Basement (a line crosses another and both end beyond the crossing).
2. A line with a free end bounds nothing and is left out, repeatedly (by a worklist of the points that have one line left, so the pass is linear in the lines). The Basement's `HEATED NETAREA` has two such stubs, 0.66 and 1.1 ft long.
3. The Area is the smallest loop that encloses its point, and its holes are the loops directly inside that loop, whether or not an Area sits in them. Six `OFFICE` Areas have three or four holes each, `GFA 250mm` has two on each of four Levels and one on Level_05.
4. A vertex where two lines continue in a straight line is not a vertex of the outline (the `OFFICE` Area `TSTO3` has one, where Revit's profile has 32 vertices and the lines 33); a vertex where a third line meets is, and Revit's profile has it (the `GFA` Areas `1` and `P`, six vertices each).

**The point decides the side.** An Area's two named lines are on its boundary, but not always on its outer loop: those of `5` (`GFA 250mm`, Level_05) are on a rectangle inside a larger loop, and Revit's Area is the larger loop with the rectangle as a hole (1857.7 ft²), not the rectangle (208.3 ft²). Choosing the loop that holds the named lines would draw 29 of 30. The point separates the two, and the named lines are then a check: they must be on the region's boundary.

## 4. What changes

`partition_area_boundaries::Arrangement` builds the arrangement of a set of straight lines along the plan axes: the splits, the free ends, the rings of the remaining edges with the region on their left, and which ring is nested in which. `face_at` returns the region around a point as an outer loop and its holes. It returns `None` where the lines are not all along the axes, overlap, are more than 2048, or where the point is on a line, in no loop or in two of one size, or where the loop or one of its holes touches itself.

`partition_area_records` reads, for each placed Area of a Revit 2024 file: its point; its Level, its scheme and its named lines from its record; the lines of that scheme on that Level; and the elevation of its Level and the distance to the Level above it. It draws a body only where every one of these holds, and none otherwise, saying why in `AreaBodyRefusal` and counting the reasons in `exported.area_body_refusals`:

- the Area's record names one Level, one scheme and at least one line, and every line it names is a declared line (from the chain records, not only one the scan returned) of that scheme on that Level whose copies agree (`named_lines_not_in_scheme_and_level` otherwise: a named line with no Level or scheme, with copies that disagree, or in a non-primary design option);
- every line of the scheme on the Level is along an axis at the Level's height, has one copy or agreeing copies, is a placed instance, and has a Level and a scheme in its reference list (a line that has not, or whose copies disagree, may belong to any scheme on its Level: the group is not read). Every declared area boundary line of the file, from the chain records, must also be one the scan returned: a declared line the scan did not return, or one whose height is not a number, may belong to any group, so no Area of the file is drawn (`boundary_line_not_read`);
- the point lies in exactly one region and the named lines bound it;
- the Area's Level and the Level above it are both read as building stories, and no other Level lies between them or at either elevation (`next_storey_not_proven` otherwise: the top storey, a Level that is not a story or whose setting is not read, or two Levels at one elevation);
- no other placed Area of the same scheme and Level has the same region, whatever stops that Area from being drawn: every Area that has a point and names a Level and a scheme gets its region, and an Area whose region another shares has none (`two_areas_one_region`), since Revit's second, redundant Area has no area of its own;
- no line of the group is in a non-primary design option: such a line is not skipped but makes its group unreadable (`boundary_lines_not_readable`). No area boundary line of the office or of Autodesk's 2024 samples has a design option (the probe's `line` rows), so how Revit treats one is not measured.

The exporter writes the region as the profile of an extrusion at the Level's elevation of the distance to the Level above. `RvtElementRecordGeometry.SpaceOutlineSource` is `area_boundary`, `BodySource` and `ProfileSource` are `partition_area_boundary_lines`, `BodyHeightSource` is `area_level_to_next_storey`, and an Area without a body carries `AreaBodyRefusal`. An Area drawn as a body is not a room, so it is left out of the spaces that hold a room's furniture (`contain_in_spaces`); Revit puts a family instance in its room.

## 5. Results

Measure, unfiltered, the branch at 5a7e40ac against `main` (53e06c02; the change's merge base is 8664b94a, and the one commit between touches the viewer only), run 41723183283 (`-f base=main -f nordic=true -f r2025=true -f holdout=true`). `space_profiles_vs_ifc` on the two office files:

| | as built before | as built after | permit before | permit after |
|---|---:|---:|---:|---:|
| `IfcSpace` with a body (rvt-rs / Revit) | 278 / 308 | 308 / 308 | 278 / 308 | 308 / 308 |
| in both | 276 | 306 | 278 | 308 |
| only in Revit, rvt-rs writes it without a body | 30 | 0 | 30 | 0 |
| area within 0.1% | 221 | 251 | 222 | 252 |
| outline within 0.01 ft | 211 | 241 | 216 | 246 |
| boundary within 0.01 ft | 221 | 251 | 222 | 252 |
| lowest and highest vertex within 0.01 ft | 276 of 276 | 306 of 306 | 278 of 278 | 308 of 308 |
| area, outline and boundary within tolerance, the Areas | none drawn | 30 of 30 | none drawn | 30 of 30 |
| base and top within 0.01 ft, the Areas | none drawn | 30 of 30 | none drawn | 30 of 30 |

The `rvt-rs area area boundary` line reads "30, area within 0.1% 30, outline within 0.01 ft 30, both 30, boundary within 0.01 ft 30" on each file, and its height line "30, lowest and highest vertex within 0.01 ft 30". The off counts (55 and 56 in area, 65 and 62 in outline) are the rooms and family instances they were, and the lines for rooms read from their stored solid, rooms that keep their box and family instances are the ones `main` prints. The earlier run of the first version of this change, 41708150034 at febba58f, gave the same 30 of 30; the two later rules (every declared line read, and the Level above a building story) lose none of them.

- **No other export moved.** Of the 53 models the run exports, the hash of the IFC differs on the two office files only; Core Interior, RE1, the R2025 models, the terrain variants and every held-out model are byte-identical. The compare job's scorer diff is 62 lines: the space lines of the two office files (the old ones and the new height lines) and, in `schema.txt`, the count of swept solids that carry a Position distinct from their product placement, 12747 to 12777 and 12790 to 12820, the 30 new bodies. The property set, quantity, plan profile, class set, room name and storey, tagged space and WHERE-rule results print the same, so the Areas add no property value Revit does not write and no storey moved.
- **Diagnostics.** `exported.body_sources.partition_area_boundary_lines` is 30, `building_elements_with_geometry` rises by 30 (12757 to 12787 as built, 12803 to 12833 at permit) and the geometry element fraction goes from 0.9797 to 0.9820. `exported.area_body_refusals` is empty on both: every Area is drawn.
- **Held-out files, by counts.** Every held-out `space profiles:` count line is the same on both sides, the new height lines included. BIM Whale 2021, whose 8 spaces are Areas of a 'Rentable' scheme, still has none of them written (0 of 8), since 2021 is not read for Areas.
- **Gates.** Clippy, rustdoc, the MSRV build and fmt pass on 5a7e40ac; the full CI result is in the PR.
- **Autodesk's 2024 samples** (run 41723200570, base `main`, no Revit export to score them against): `rac_advanced` has 25 Areas of one scheme on three Levels, and 3 gain a body; `exported.area_body_refusals` there is 18 `boundary_lines_not_readable` and 4 `point_in_no_single_region`. `rst_basic`'s one Area is refused as `next_storey_not_proven`. No other 2024 sample has an Area; their exports are byte-identical, and the two that moved moved by the 3 bodies and by the new `AreaBodyRefusal` property. The probe on them (run 41714706290 at 730ffb7b): the 18 are on two Levels whose lines include 4 that are not along an axis (a slanted line or an arc, whose box is not the line), so those groups are not read, as intended. The 4 others are on the third Level, whose 38 lines are all along an axis: the group has three closed loops and one open chain of 18 points, and these 4 have their point in no loop. The 3 bodies are not measured against Revit's.

The prototype that fixed the rule (a Python version of §3 over the probe's lines and Revit's profile points) drew 30 of 30 with a zero symmetric difference before any Rust was written; the exporter's 30 of 30 above is the independent check. Vertex to vertex (`vertices`, the mesh's upward-face vertices), 29 of the 30 Areas agree on each file (86 to 115 and 84 to 113); the one that does not has Revit's area and outline, and why its vertices differ is not examined.

## 6. Not measured

- Revit releases other than 2024: no Area of another release is measured against an export, and nothing is read (fail closed).
- Boundary lines that are not along an axis (a slanted line, an arc): none on the office. A group that has one is not read, so its Areas keep no body, as before.
- An Area's base offset: read as zero, which the office measures.
- A Level's story above (`LEVEL_UP_TO_LEVEL`): Revit's exporter takes it first for an Area's height, and rvt-rs never reads it. On the office the height equals the distance to the next Level up on all 30, so a Level whose story above is another Level is unmeasured. The Area's own upper Level is ignored by Revit's exporter for an Area and is not read here either.
- Levels that are not building stories: an Area has a body only where its Level and the Level above are both read as building stories and no other Level lies between them or at either elevation. All ten of the office's Levels are building stories (the probe prints the setting of each, run 41721498984), so a Level that breaks this is not measured.
- The top storey: an Area on the highest Level has no body, where Revit draws it one unit high. Nothing on the office.
- Areas bounded partly by walls. Autodesk's `rac_advanced` is the one file with Areas whose lines do not all close: of the 7 Areas on its third Level, whose 38 lines are all along an axis, 3 sit in closed loops and are drawn, and the other 4 have their point in no loop of the lines (the group has three closed loops and one open chain of 18 points). That is what an Area bounded partly by walls would look like, and it is the main gap in this rule; what bounds them is not examined, because Revit's export of the sample is not available to compare with. The free-end rule above is measured on the office's two stubs only; on `rac_advanced` the open chain is one that the rule prunes, and whether Revit ignores its free ends there is not measured either.
- Area polylines and the other Area kinds (`OST_AreaPolylines`, -2000830).
- Revit's phase filter, which skips a spatial element in an invalid phase (RE-184).
- The held-out BIM Whale 2021 project's 8 Areas ('Rentable' scheme): 2021 is not read for Areas, and the file is held out, so it was not examined. It is quoted by its counts only.

## Reproduce

```bash
# once landed, --ref main (the scorers are on main then); before, --ref feat/area-spaces-bodies
gh workflow run measure.yml --ref main -f ref=5a7e40ac6a078d19548399b9a5248d72c0024f2d -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-permit" \
  -f scorers=none -f probe=probe_re317_area_boundaries -f probe_scorer=area_boundaries_vs_ifc
gh workflow run measure.yml --ref main -f ref=5a7e40ac6a078d19548399b9a5248d72c0024f2d -f base=main -f nordic=true -f r2025=true -f holdout=true
gh workflow run measure.yml --ref main -f ref=5a7e40ac6a078d19548399b9a5248d72c0024f2d -f base=main -f samples=true -f sample_years="2024 2024" -f models="zzz-no-model"
# the probe on Autodesk's 2024 samples (run 41714706290 at 730ffb7b; the probe has no scorer there)
gh workflow run measure.yml --ref main -f ref=730ffb7b457c9cb282e3d317c42980b123bda184 -f base=none -f samples=true -f sample_years="2024 2024" -f models="zzz-no-model" -f probe=probe_re317_area_boundaries
python3 tools/re/area_boundaries_vs_ifc.py probe.txt revit-export.ifc
python3 tools/re/space_profiles_vs_ifc.py model.ifc revit-export.ifc --list 10
```

The first version of the change, which `main` does not hold, is 46783fe9 (run 41708150034 at febba58f).
