# RE-374: a room solid's edges, and the floor outline they give

**Date:** 2026-11-09
**Issues:** #90 (rooms on their box); follows RE-101 (a room's outline from its stored solid's faces), RE-316 (the faces up to the solid's end) and RE-334 (round columns).
**Artefacts:** Autodesk's sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh` (Snowdon Towers Architectural 2024, sha256 `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1`); the BIM4LCA office at building permit, pinned in `research/bim4lca-measure.tsv` (CC BY-SA 4.0; the `.rvt` sha256 `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1`, Revit's IFC4 export `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d`) and as built; Core Interior, the four RE1 models and puzzbobb's Revit 2025 models. Never stored.
**Oracles:** Revit's own IFC4 export of the office at building permit (`tools/re/space_profiles_vs_ifc.py`); for Snowdon Towers, whose IFC4 export holds no `IfcSpace`, `Vim.Room.Area` of the model's later VIM edition (`tools/re/room_areas_vs_vim.py`, local only, never committed).
**Probe:** `probes/examples/probe_re374_room_solid_edges.rs`.
**Status:** positive. On Revit 2024 a room whose stored solid neither the walk of RE-101 nor the span of RE-316 closes takes the outline of the solid's floor, read from the solid's edges, where the solid is a prism of that floor up to 4 ft above it. Two rooms of the models measured are read so, one against Revit's IFC4 export and one against Revit's VIM area; on every room both rules read, the edges give the same outline.

## 1. Before

The walk and the span draw each wall of a room from its face's uv box. That box is the face's parameter range: where a wall face is not a rectangle (a wall that steps down, a face cut by another room's), its box runs past the wall's real end, and the room's outline does not close, or closes in the wrong place, and the room keeps its record box. Snowdon Towers had 5 of its 54 rooms on their box, and the office at building permit 1 of 234: 3095831, a corridor 100 ft long whose 14 wall ends are not joined (RE-316 §4).

## 2. The solid's records

After the solid's header (`u64` ElementId · `04 90 08 00`) comes its list of objects (RE-316): `04 c0 08 00 · u32 n · u32 4 · n × (u16 class · u32 token)`.

- **Faces** take the indices 4 to 4 + n - 1. The tokens run from 5, but the list's last entry repeats one, so the faces are not read token by token: they are the face records (`04 00 08 00`, RE-101) after the list whose index (a plane's `u32` at +8, a longer record's `u16` at +10 after `46 05`) is one of those, in order, until each has been seen. The next element's records restart their indices at 4, which is where the faces stop.
- **Edges** are the records between the list and the first face: `04 82 08 00`, 113 bytes, the two faces' indices at +4 and +8, and eight `f64` from +32, each face's (u, v) at two points of the edge: the first face's at the first point, the second's, the first's at the second point, the second's.

Bounded so, every edge names two of its solid's faces on all 2,952 room solids of the 51 models with rooms. Read up to the next solid's header instead, Snowdon Towers' room solids took the records of the element after them: 1,897 edges whose two faces disagree, which the list bound removes.

**Where both faces are planes** each gives the edge's ends, and the two agree to 0.001 ft on 51,847 of 51,851 such edges. The other 4 are on the office as built (the probe names the rooms); a solid with any such edge is refused (`edge_faces_disagree`).

**On a cylinder**, an edge along the axis stores the same angle at both points, one end of the face's angle (11,488 such sides). An edge round the cylinder stores two points that are not its ends: on Snowdon Towers, measured locally on all 114 such sides, they lie 1/k and 2/k of the face's angle in from one end, k a whole number from 3 to 20, and on the 27 where the face across the edge is a plane that was read, the plane gives the same two points. The reader therefore takes a curved wall's arc over the face's whole angle, from its uv box (RE-334), never from the edge. The run of §6 tallies that form on every model.

## 3. The rule

`edge_outline_at` (`src/research/partition_room_boundaries.rs`) draws the floor's outline and refuses, with a reason, a solid:

- with a face that is neither vertical nor horizontal, or a face whose record is not read unless all its edges lie at one height (`face_neither_vertical_nor_horizontal`, `unread_face_not_placed`, `unread_face_not_horizontal`);
- with an edge its two planes place differently (`edge_faces_disagree`);
- with no single floor face, the face all of whose edges lie on the record box's floor (`floor_not_one_face`);
- with any point of any edge, other than along a vertical edge, above the floor and below 4 ft over it (`ROOM_PRISM_HEIGHT_FEET`, `solid_not_a_prism_up_to_the_prism_height`): a raised floor, a step, a pocket or a bulkhead below that height. Every edge of an accepted solid is then vertical or at one height, so the solid is a prism of its floor up to 4 ft;
- whose floor meets a wall that was not read or does not stand on the floor (`floor_wall_not_read`, `floor_wall_not_on_the_floor`).

Each edge of the floor is drawn as the wall it meets: a plane between the edge's ends on it, a vertical cylinder as the chords of its arc (RE-334). The ends are snapped at 0.0001 ft; the lines must close as the walk reads them, in one piece, with no line crossing another (`floor_edges_do_not_close`), and the outline must span the record box exactly (`outline_not_on_the_record_box`). Where a room has several solids, every one that gives an outline must give the same.

It runs only where neither the walk nor the span reads the room, and only on Revit 2024 (`ROOM_SOLID_EDGES_REVIT_VERSIONS`), the one release where it adds a room on the models measured. The room's `SpaceOutlineSource` is `stored_solid_edges`, its profile source `partition_room_solid_edges`, and `exported.edge_outline_spaces` counts it.

## 4. Why 4 ft

Snowdon Towers' room 2181757 has a floor of 5172.857 ft², a 1-inch pocket in it and two features 3.93 ft up. Revit's area of it (`Vim.Room.Area`, 5177.492 ft²) is neither the floor nor the record box: it is the solid's horizontal section 4.0 ft above the floor (5177.493). At 3.9 ft the section is 5177.449. Across the stepped rooms the same holds where a height decides: 828537 matches Revit's area at any section from 2 ft up, 2181762 at 6 ft, and 828548 at no height (both local, against the VIM).

So the floor's outline is the room's area only where the solid does not change between the floor and that height, which is what the rule requires. 4 ft is measured on 2181757 only: where the height a room's area is taken at is stored, and the elevation it is measured from, are not decoded. Until they are, a room the rule would need that height for keeps its box. The four stepped Snowdon Towers rooms do: each is refused first because a face whose record is not read has edges at more than one height, and 2181757's features 3.93 ft up would refuse it again at the prism check. On the `rme` samples the prism check is the one that refuses 46 of the 49 rooms on their box on each save.

## 5. What it reads

Measure run [42580514237](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42580514237) ran the probe at `dbe56298` on every Autodesk sample of 2015 to 2027, the office as built and at permit, the terrain offices, Core Interior, Einhoven, the RE1 models and the Revit 2025 models: 2,952 rooms with a solid on 51 models. It compares what the edges give with what `main` reads (`walk`, the span's `faces` or `approximate`, or the box). Two outlines agree where their areas are within 0.01 ft² and their extents within 0.001 ft.

| `main` reads | edges agree | edges refuse | edges differ |
|---|---:|---:|---:|
| the walk | 2,175 | 146 | 0 |
| the span, exact | 0 | 9 | 0 |
| the span, approximate | 2 | 25 | 0 |

| `main` keeps the box | edges add | edges refuse |
|---|---:|---:|
| | 2 | 593 |

The edges never give an outline that differs from one `main` reads. The two they add are both on Revit 2024:

- **The office at building permit, room 3095831** (CORRIDOR 030): 1864.737 ft², against 1864.74 ft² for the same space in Revit's IFC4 export of that file (`space_profiles_vs_ifc.py`; the box was 6642.03).
- **Snowdon Towers, room 830902**: 304.059 ft², against Revit's 304.058 (`Vim.Room.Area`, a later save, local only; the box was 808.665).

Of the refusals, 564 are rooms whose solid has a point between its floor and 4 ft, 179 have a face whose record is not read and whose edges do not lie at one height, 28 have floor edges that do not close, and 2 have a sloped face. On the 2015 to 2023 and 2025 to 2027 releases the edges add no room on the models measured, so the rule is not read there.

## 6. The export

[TODO: the unfiltered Measure run on the PR's head against `main`: the exports that move, the office at permit's space scores, and the probe's tally of the cylinder edges' two points and of the 4 plane edges.]

## 7. Not measured, not read

- Rooms that need a section above the floor (§4): 2181757, 828537, 828548 and 2181762 keep their box. Decoding the height a room's area is taken at is a follow-up of #90.
- 593 rooms that neither `main` nor the edges read: 49 rooms on each of the 12 `rme` saves (46 not a prism up to 4 ft, 3 with an unread face whose edges are at more than one height), Snowdon Towers' four stepped rooms and 1 of the terrain concrete office's (an unread face, as Snowdon's).
- Revit 2025 to 2027 and 2015 to 2023: the layout holds there (no edge names a face outside its solid, plane pairs agree), but no room is added, so nothing there is checked against an export.
- A curved wall's arc is taken from its face's angle, not from the edge, whose two points are not its ends (§2).

## 8. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2024
cargo run --profile ci -p rvt-probes --example probe_re374_room_solid_edges -- \
    samples/2024-snowdon_towers_sample_architectural.rvt
gh workflow run measure.yml --ref main -f ref=dbe56298 -f base=none -f samples=true \
    -f nordic=true -f r2025=true -f probe=probe_re374_room_solid_edges -f probe_scorer=- -f scorers=none
```
