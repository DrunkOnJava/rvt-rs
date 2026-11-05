# IFC Export Diagnostics

`rvt-ifc --diagnostics path.json` writes a JSON sidecar next to the IFC
output. The same payload is available to Python via
`RevitFile.export_diagnostics_json()` / `rvt_to_ifc_diagnostics(path)` and to
WASM via `openRvtBytesWithDiagnosticsMode(bytes, mode)`.

The sidecar is meant for non-technical support, bug reports, and automated
export-readiness checks. It does not contain raw model bytes.

Terminology used by this file, `rvt-inspect`, Python, and the browser viewer is
defined in [Diagnostic Semantics](diagnostic-semantics.md).

## CLI

```bash
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json
```

Use `--mode` when automation needs to reject incomplete output instead of
accepting a scaffold IFC:

```bash
rvt-ifc model.rvt -o model.ifc --mode strict --diagnostics model.diagnostics.json
```

Supported quality gates are `scaffold`, `typed-no-geometry`, `geometry`, and
`strict`. These modes also shape *content*:

| Mode | Emission |
|---|---|
| `scaffold` | Framework + production elements; geometry when recovered. No HostObjAttr proxies. |
| `typed-no-geometry` | Mapped/typed elements only; geometry fields stripped. |
| `geometry` / `strict` | Mapped/typed elements; curves, loops, hosts and elevations when present. |

`strict` fails before writing IFC when required model data is missing;
when `--diagnostics` is present, the sidecar is still written for triage.
`scaffold` is an inspection/export-envelope success, not proof that Revit model
elements were converted.

Diagnostic proxy mode can also write the sidecar:

```bash
rvt-ifc model.rvt -o diagnostic.ifc --diagnostic-proxies \
  --diagnostics diagnostic.json
```

Where each element was read from is a list as long as the file has elements,
so it is off unless asked for (#1325):

```bash
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json --element-provenance
```

adds `decoded.element_provenance` to the sidecar. It is large: how large is in
[Provenance table size](#provenance-table-size). Without the flag the sidecar has no such field and is the bytes it
was before the flag existed. `ExportOptions::with_element_provenance(true)`
and Python's `export_diagnostics(element_provenance=True)` (and
`rvt_to_ifc_diagnostics(..., element_provenance=True)`) do the same, and the MCP
server turns it on for the models it opens. The IFC is the same file either way.

What an element's `offset` is, and what has been measured:

- `offset` is the frame the exporter selected for the element's record. Among an
  element's frames, Revit 2024 and later (`select_instance_records`) take the
  one with the greatest vertical extent and, on a tie, the higher-numbered
  partition and then the later offset; Revit 2008 to 2023
  (`select_newest_instance_records`, RE-107) take the newest, the
  highest-numbered partition and then the latest offset, with no extent test. A
  body reader may take another copy: the sketch-line readers take the current
  one (`current_partitions`, RE-216), which is also the copy `native.stream`
  names. So `offset` and `native.stream` can name different partitions, and the
  copy a reader took a value from is in `value_sources`, below.
- `tests/element_provenance_offsets.rs` reads the bytes at every reported
  offset on Core Interior, the RE1 models, Einhoven and the BIM4LCA as-built
  office. Einhoven (Revit 2023) confirmed 13 of 13 first-prologue frames and 71
  of 71 native records. Not measured: Revit 2008 to 2022, and the
  second-prologue path of 2023.
- The schema-walk carrier (`global_latest_schema_walk`) has no element on any of
  those seven models, so its offset check has not run on any. Its offsets index
  `Global/Latest` as the walk inflates it (`inflate_stream_auto`); `rvt-dump`
  inflates that stream with its own routine, and the two are not compared.
- `native` and `by_carrier` count every element with an ElementId, the
  omitted ones included.

#### Value sources

`value_sources` (#1360) says which copy a body was drawn from, where it is not
the frame `offset` names. Each entry is one reader's: `reader` (what was read),
`rule` (how the copy was chosen), `copies` (in how many partitions the
element's data named it; two headers in one partition count once), `stream` and
`offset` (where the copy starts), `partitions` (for `agreeing`, the partitions
that hold a copy) and `differs_from_frame` (the value is held in another
partition than the frame; for `agreeing`, in none of the partitions that hold a
copy, so the lowest-numbered partition's being named does not make it differ;
for a source made of `parts`, none of them in the frame's partition, and for a
profile's sketch lines also not the Sketch's curve list, which names them: the
lines are in the partitions of the lines, so the profile is held in the frame's
partition too when the list has a copy there). A value made of several records,
or of another element's, lists them in `parts` (`element_id`, `stream`,
`offset`), at most 64; `parts_left_out` counts the rest (#1368). A source with
`parts` has no `stream` or `offset` of its own. A profile's `copies` is the
number of sketch lines it is made of; a source of one record (a type symbol, a
run type) counts the partitions that held that record. Its `rule` is how that
record's copy was chosen: `agreeing` (every copy gives the same value),
`current` (the copies disagreed and the current partition's was taken) or
`frame` (one frame or partition of the record, without its other copies being
compared: the last frame of a type symbol whose frames differ, a room's solids
read in the room's own partition).
A reader notes its choice where it makes it, only while the export builds this
table, and the entry is listed only where the element's final `BodySource` is
drawn from that reader.

| `BodySource` | Reader | Rule | What `offset` is |
|---|---|---|---|
| `partition_element_record_bbox`, `partition_element_record_join_trimmed`, `partition_element_record_join_cut`, `partition_element_record_column_cut` | `selected_frame` | `selected` | The frame itself (a trim or cut also reads the joined elements' frames, which are not named). |
| The same bodies where the profile is read from a record no reader notes yet: a `profile_source` that is not the `body_source` | `selected_frame_box` | `selected` | The frame, for the box and the placement only. The profile's copy has no source yet. |
| The same bodies where the profile is solved from sketch lines (`profile_source` `partition_element_record_sketch_lines`: floors, slabs and ceilings, and roofs whose body is the record's box; a roof drawn from its slope or extrusion has the `roof_*` rows below) | `sketch_lines` | `agreeing` or `current` | `parts`: the frame of each sketch line the solve was given, the first of its frames by frame order (`agreeing`), or the current partition's where its frames disagree (`current`, RE-216). The element's `selected_frame` names the frame for the box. |
| The same | `sketch_list` | `agreeing` or `current` | The element-data header of the Sketch's curve list that names the lines (RE-97), the lowest-numbered partition's or the current one. Where the element has one. |
| The same, where the outline is read from the lines' own ends rather than their boxes | `sketch_line_data` | `agreeing` or `current` | `parts`: the element-data header of each line's copy (as `bounded_line`). |
| The same bodies where the section is the type symbol's box (`profile_source` `family_type_symbol_envelope`: columns) | `type_section` | `agreeing` or `frame` | `parts`: the type symbol's frame, the last of its frames the column scan kept; `agreeing` (with `partitions`) where every frame of the symbol gives the same box, `frame` where they do not. |
| The same bodies where the profile is a room's outline read from its stored solid (`profile_source` `partition_room_solid_faces`, `partition_room_solid_faces_resolved` or `partition_room_solid_faces_approximate`, RE-101, RE-316) | `room_solid_faces` | `frame` | `parts`: the header of each stored solid of the room that gave the outline (the ElementId, then the solid's tag), in the room's own partition; where more than one solid gave one, every one gave the same. Only that partition is read: the room's solids in other partitions are not compared, so the rule is not `agreeing`. |
| `family_type_hollow_section` | `type_hollow_section` | `agreeing` | `parts`: the entries of the column type's parameters `b`, `Ht`, `t` and `kr` (`f64`, `ff` x 8, `i64` parameter id), in the type's value block; the first copy of each, every copy holding the same value. A parameter whose value block holds two different values, or whose copies in two partitions differ, is skipped (the type then has no section): that is not counted as a disagreement among copies. The element's `selected_frame` names the frame whose box the section is held to. |
| `family_type_i_section` | `type_i_section` | `agreeing` | `parts`: the six entries of the type's I section (width, height, web and flange thickness, the centroid both ways), as for the hollow section. `instance_transform` (as below) gives the plan angle of the column's X axis. On Core Interior, the BIM4LCA office and the RE1 models no element has this body (Measure run 41887789579), so this reader is untested on real files. |
| `partition_pipe_axis` | `pipe_connectors` | `agreeing` | `parts`: the two connector entries (`u64` element, `u32` index, `u32` 1, the point) whose points are the pipe's ends, in the partition of its record. An entry is found by where its point falls, never by whose it is (RE-131): at a connected end it is keyed by the element connected there, so a part's `element_id` may be a neighbour of the pipe (a fitting), not the pipe. The radius is the record box's, held in the element's `selected_frame`. Where the pipe's body came from two partitions they agree. |
| `partition_roof_slope` | `roof_slope_edges` | `agreeing` | `parts`: the element-data header of the slope block of each sketch line the slope was made from: the defining edges of a roof with some (its flat edges are not listed), the slope arrow's line alone for a roof with one arrow, and for a roof with several arrows (RE-307) the arrows and the boundary lines they rise from. The first copy of each; every copy decodes to the same slope (the reader compares the decoded values, not the bytes) (RE-56, RE-199, RE-307). |
| `partition_roof_slope`, `partition_roof_extrusion` | `roof_line_data` | `agreeing` or `current` | `parts`: the element-data header of the copy of each of those sketch lines (a roof by extrusion: all of its lines) the line or spline was read from, as `sketch_line_data` (RE-211). |
| `partition_roof_slope` | `roof_fascia_depth` | `agreeing` | The element-data header of the roof's own data that gave its fascia depth, where it was read (RE-199). |
| `partition_roof_slope` | `roof_rafter_cut` | `native` | The header of the roof's current native record, which gives its rafter cut, where it was read (RE-199). |
| `partition_roof_slope`, `partition_roof_extrusion` | `roof_type_layers` | `agreeing` | `parts`: the element-data header of the roof type's layers, whose widths sum to the roof's thickness, read once for the type and noted for each roof of it. |
| `partition_roof_slope`, `partition_roof_extrusion` | `sketch_lines`, `sketch_list` | as above | The roof's outline (a sloped roof) as for a floor; for a roof by extrusion `sketch_lines` lists the first frame of each of its sketch lines, rule `frame` (the other frames are not compared), and `sketch_list` is listed only when the roof's Sketch lists its curves (RE-97): `11_RoofByExtrusion` has none, its lines are found by their owner reference. The element's `selected_frame` names the frame whose box the section is held to. |
| `family_extrusions` | `family_forms` | `frame` | Only with `--element-provenance` and `--family-extrusions` both on. `parts`: the records of the family's own embedded document that the instance's extrusions were rebuilt from: each visible extrusion (`ExtrusionElem`), its sketch plane (`SketchPlane`) and the curves of its sketch (`CurveElem`), at the place the document's group puts the record in its partition (the record's id, then its length and body). The ids are the family document's, not the model's, so a part's `element_id` can equal another element's. The document is read once, as it is embedded. A record the file holds in several partitions is named at its copy in the highest partition (partitions taken in number order), the copy the rebuild reads; the other copies are not compared. An instance one of whose records has no place (its group's bytes are not the partition's own consecutive members) gets no `family_forms` source, because a part list that left a record out would not be the records the body was rebuilt from; it keeps its other sources. The instance's own `selected_frame` and `instance_transform` are listed beside it: the transform places the extrusions, and the frame is the location and rotation they are drawn at. The saved mesh the extrusions are checked against is not a source of the body. |
| `partition_wall_centreline`, `partition_wall_centreline_layer_joins`, `partition_wall_tapered_section`, `partition_beam_axis`, `partition_beam_box_along_axis` | `bounded_line` | `agreeing` | The element-data header of the lowest-numbered partition's copy of its location line. Every copy holds the same line; a wall or beam whose copies differ is drawn as its box instead. |
| `partition_wall_arc` | `bounded_arc` | `agreeing` | As `bounded_line`, for a curved wall's arc. |
| `partition_family_instance_turned_box`, `partition_family_instance_oriented_box` | `instance_transform` | `agreeing` or `current` | The element-data header of the copy: the lowest-numbered partition's where the copies agree, the current partition's where they differ (RE-216). |
| `saved_graphics_mesh` | `saved_graphics` | `native` | The header of the element's current graphics record. |
| `native_spiral_run_treads` | `native_run` | `native` | The header of the run's current native record. |
| `native_landing_tread` | `native_landing` | `native` | The header of the landing's current native record, which gives its outline and top. Its thickness is its stair's run types' tread thickness, which has no source yet. |
| `partition_stair_run_sketch` | `stair_run_lines`, `stair_run_ends` | `agreeing` | The element-data header of the first copy of the run's data that gave the sketch lines, or the ends; `partitions` lists the copies that did. |
| `partition_stair_run_sketch` | `stair_run_type` | `agreeing` | `parts`: the run type's id object (where its id is stored), read once for the type and noted for each run of it; `copies` is the partitions that held the id object, `partitions` those that gave a run type. |
| `partition_stair_run_sketch` | `native_run` | `native` | A Revit 2025 run: the header of the run's current native record, which gives its sketch, riser height and ends (RE-217). |
| `partition_stair_run_sketch` | `native_run_type` | `native` | A Revit 2025 run: `parts`: the header of the run type's current native record, which gives the type's construction and name. |
| `partition_stair_run_sketch` | `stair_run_base_cuts` | `native` | The header of the run's current native record, which gives where its underside is cut; listed only for a run whose record gave a cut. |
| `partition_landing_slab` | `landing_lines` | `agreeing` | The element-data header of the first copy of the landing's data that gave its outline lines. |
| `partition_landing_slab` | `native_landing` | `native` | The header of the landing's current native record, which gives its top. |
| `partition_landing_slab` | `native_landing_type` | `native` | `parts`: the header of the landing type's current native record, which gives the thickness (monolithic types only). |

No reader notes its copy yet for `partition_area_boundary_lines` and
`native_slab_shape_edit` (a floor drawn along the plane of its shape-edited
corners, #1384), nor for the other values a body reads (a wall's layers, a beam's I section, the
parameters of a rectangular column's section). An element of those bodies has
no value source; an element whose box is the body but whose profile is read
from a record no reader notes yet has `selected_frame_box` and nothing for the
profile. Neither is a claim that it read the frame. `value_source_counts`
counts, by reader, the elements with a source, those whose value is held in
another partition than their frame, and those whose copy was chosen as the
current partition's (`current`), so a run on any model shows whether it
exercises that rule. On Core Interior and the BIM4LCA office as built no source
has the rule `current` (Measure run 41890994176: no `current` count). That does
not say every copy set agreed: a set that disagreed and could not be resolved is
dropped by its reader and leaves no source and no count.

`tests/value_sources_offsets.rs` reads the bytes at each recorded offset on Core
Interior, the RE1 models, Einhoven and the BIM4LCA as-built office, and, where
`RVT_R2025_DIR` names puzzbobb's Revit 2025 models (CI's Corpus health fetches
them), on those: the only models of the gate whose roofs are drawn from their
slopes or from an extrusion. For the element-data readers it finds the element-data header, then the element's own
id, then a record that decodes to the value the reader returns when asked again;
for a native record it finds the ElementId at the offset; for each record in
`parts` it reads what that record is: a sketch line's or a type symbol's frame
has the element's id at its first bytes, or, on a second-prologue frame, the id
of the record that encloses it (found from the bytes); a sketch line's data is
an element-data header and then the line's own id; a room's solid is the room's
id and the solid's tag, and read again (`room_outline_at`, or
`resolved_outline_of_solids` for a resolved outline) it gives an outline with
the vertex count the room holds; a type's parameter entry is `f64`, `ff` x 8 and
an `i64`, in the value block whose owner (declared, as the reader's blocks
are) is the type, and a section's parts are distinct entries of the expected
parameters (an I section's six; a hollow section's four, whose captions the
`ParamElemFamily` elements name `b`, `Ht`, `t` and `kr`); a roof's slope block
decodes to a slope or a slope arrow, and its sketch line to a line, an arc or a
spline; a pipe's connector
entry is the keyed element's `u64`, an index of at most 255, the `u32` 1 and a
point, and the two parts' points make a cylinder (`pipe_body`) with the pipe's
record box; a form record of a family's document is the record's id, a length, a
body of that length that opens with the tag of an extrusion, a curve or a sketch
plane, and the length again; a run type or a native type record starts with its
id; and a
Sketch's curve list is a header followed by the sketched element's id. A frame of a 32-bit release with no id at its first
bytes is counted as not read. It does not say the
copy taken is the right one: `agreeing` copies are equal, and `current` is
`current_partitions`, which it does not measure.

#### Provenance table size

The only place the table's size is stated. Every other document, doc comment
and schema description links here and gives no number, so a change to the table
(a new reader, say) updates this table and its run, and nothing else (a
changelog fragment of unreleased work may state its own figure). Measured by
Measure runs 41890994176 (the head of #1373; the first four rows) and
41902619175 (the head of #1379; the Revit 2025 rows), with
`-f ifc_flags=--element-provenance` against the same models without it.
"Compact" is the JSON the library and Python write; "pretty" is the CLI's
sidecar.

| Model | Diagnostics without the table (compact) | Table (compact) | Table over diagnostics | Sidecar without the flag (pretty) | Sidecar with the flag (pretty) | Sidecar over sidecar |
|---|---|---|---|---|---|---|
| BIM4LCA office as built | 38,640 | 6,202,130 | 160.5 | 52,677 | 11,098,081 | 210.7 |
| Core Interior | 20,563 | 733,326 | 35.7 | 27,588 | 1,549,080 | 56.2 |
| RE1 Mechanical | 11,809 | 43,745 | 3.7 | 13,935 | 91,938 | 6.6 |
| RE1 Plumbing | 11,568 | 71,361 | 6.2 | 13,583 | 147,585 | 10.9 |
| R2025 02_1Slope (a shed roof) | 7,899 | 11,131 | 1.4 | 9,549 | 30,336 | 3.2 |
| R2025 11_RoofByExtrusion | 7,983 | 11,214 | 1.4 | 9,663 | 30,149 | 3.1 |
| R2025 20_T_U_Roof (T and U hip roofs) | 7,835 | 18,855 | 2.4 | 9,445 | 45,915 | 4.9 |

The table has an entry for every element, so it grows with the model, and it is
off unless asked for. The value sources of #1368 added 3.6% to the office's
compact table (5,987,841 bytes after #1360) and 66% to Core Interior's
(440,909 bytes), where the sketch-line profiles are listed line by line.
The roof sources of #1368 (run 41902619175) added 16% to the shed roof's
compact table (9,614 bytes), 6% to the roof by extrusion's (10,541) and 21% to
the T and U roofs' (15,602), and nothing to the office's or Core Interior's,
which have no roof drawn from its slopes or an extrusion.
With `--family-extrusions` as well, the sources of the family instances drawn
from their families' extrusions (run 41907345226, against main with the same
flags) added 1.1% to the office's compact table (6,187,946 to 6,255,099 bytes),
10% to Core Interior's (729,071 to 802,767), 17% to RE1 Architecture's (56,627
to 66,305), 15% to RE1 Electrical's (31,875 to 36,614) and 13% to each terrain
file's (41,891 to 47,354 for the concrete one).
RE1 Architecture's sidecar grew from 13,956 to 85,470 bytes before the value
sources and has not been measured since.

### Export decisions

The `skipped` section counts what the exporter left out by reason and class,
with sample names. It cannot say why one door has no opening or one room is a
box. `--export-decisions` (#1326) adds `export_decisions`, one entry per element
for each fail-closed decision:

```bash
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json --export-decisions
```

An entry has the ElementId (`element_id`, absent for a frame that carries none),
a `stage`, a `reason` code, the `class` the aggregate counts it under, the
`stream` and byte `offset` its record was read from where known (the bytes
`rvt-dump` writes as `Partitions_NN.decomp`), and a `detail`. The `reasons`
object gives each reason's `stage`, its `total` and how many were `recorded` and
`dropped`; the log keeps at most 5,000 entries a reason, in ElementId order
(then partition number, so `Partitions/99` comes before `Partitions/100`), and
counts the rest. `ExportOptions::with_export_decisions(true)` and Python's
`export_diagnostics(export_decisions=True)` do the same, and the MCP server turns
it on for the models it opens and returns an element's entries from
`explain_element`. Without the flag the field is absent and the sidecar is the
bytes it was. The IFC is the same file either way. `--anonymize` scrubs the log
as it scrubs `skipped`: the phase named in a left-out room's `detail` becomes
`<redacted>`, as `skipped[].phase` does, and any `detail` that is not a code
rvt-rs writes is replaced; reason codes, classes, streams, offsets and
ElementIds, which are the file's structure, stay.

The `skipped` items are counted from the same decisions, so the two agree: for
every reason of `skipped` the log's `total` is the sum of the items' counts and,
for a reason that dropped none, the entries by class are the item's `classes`.
`tests/export_decisions_counts.rs` checks this on Core Interior, the RE1 models
(the default export, and the first phase, which leaves the 11 rooms out),
Einhoven and the BIM4LCA as-built office. The one item it leaves out is
`stream_not_read_resource_limit`, which counts streams, not elements. Core
Interior has nothing left out (its `skipped` section is empty and the three
`exported` counts are 0), so its log is empty and the test checks that; for the
other models it names the reasons their diagnostics count and requires entries
for each, so an empty log cannot pass.

| Stage | Reason | What was decided | Counted in |
|---|---|---|---|
| `element_skipped` | `below_min_element_confidence`, `low_confidence_schema_scan_candidate`, `element_record_without_element_id`, `element_record_frame_stopped` (`class`: the stage), `element_record_without_volume`, `element_record_in_non_primary_design_option`, `element_record_not_in_export_phase`, `room_not_in_export_phase`, `element_record_empty_curtain_panel`, `element_record_family_space_without_body`, `area_without_proven_body` | The element is not in the export. | the `skipped` item of the same reason |
| `body_not_drawn` | `unsupported_geometry_curve`, `unsupported_geometry_profile`, `unsupported_geometry_missing_dimensions` | The element is written with no body. | the `skipped` item |
| `opening_not_cut` | `unsupported_geometry_unresolved_host`, `opening_index_without_host_geometry` | A door or window has no host to cut, or an opening index row has no host geometry. | the `skipped` item |
| `storey_not_bound` | `unsupported_geometry_missing_level` | The element is written outside any storey. | the `skipped` item |
| `outline_not_read` | `floor_boundary_annotation_only` | A floor keeps its boundary annotation alone. | the `skipped` item |
| `outline_not_read` | `space_outline_is_record_box` | A room or space is its record's box. | `exported.bounding_box_spaces` |
| `outline_not_read` | `area_body_refused` (`detail`: why) | An exported Area has no body. | `exported.area_body_refusals` |
| `body_left_as_box` | `body_is_record_box` (`detail`: the `BodySource`) | The body is only the record's bounding box. | `exported.bounding_box_bodies` |
| `body_left_as_box` | `slab_shape_refused` (`detail`: why) | A floor, roof or ceiling whose corner vertices are shape-edited keeps its record's box (RE-336). | `exported.slab_shape_refusals` |
| `body_left_as_box` | `wall_base_cut_refused` (`detail`: why) | A wall whose base join names such an element keeps its base (RE-336). | `exported.wall_base_refusals` |
| `value_left_out_exporters_disagree` | none | Not recorded: such a value is left out by a rule for a whole class of element, in the reader that would write it, and nothing counts them; there is no per-element decision to log until a reader does. | |

A frame with no ElementId (`element_record_without_element_id`) has an entry with
its stream and offset and no ElementId. An entry about a geometry gap or a body
or outline decision takes its stream and offset from the element's provenance
(see `--element-provenance`), when the walk decoded the element.

What an entry's `offset` is, and what has been measured:

- For the reasons made from the element-record frames
  (`element_record_without_element_id`, `element_record_without_volume`,
  `element_record_in_non_primary_design_option`,
  `element_record_not_in_export_phase`, `room_not_in_export_phase`,
  `element_record_empty_curtain_panel`) it is the frame the scan read. A scan that
  finds an element framed in several partitions keeps the last frame in scan
  order, partition number and then offset, with no test of vertical extent. The
  frame `decoded.element_provenance` names for an exported element is the one
  the exporter selected (`select_instance_records`: on Revit 2024 and later the
  greatest vertical extent), so the two can name different frames of the same
  element.
- On Revit 2008 to 2023 files `element_record_frame_stopped` is made from the
  frames of the marker scan that did not become an element (see
  [Element record stages](#element-record-stages)): its `class` is the stage the
  frame stopped at (a stage code, not a Revit class), and its `offset` is where
  the frame's ElementId is stored, 52 bytes before the marker (48 on Revit 2008).
  A frame stopped before its record has no ElementId; a frame with no volume has
  its record's.
  `tests/element_record_stages.rs` re-reads the bytes at those entries on
  Revit_IFC5_Einhoven (Revit 2023): a marker 52 bytes on and, where the entry
  has an ElementId, that id at the offset.
- `tests/export_decisions_counts.rs` re-reads the bytes at the entries of those
  seven frame reasons (the six above and `element_record_family_space_without_body`)
  on Revit 2024 and later, and finds the element's id there or a second-prologue
  frame inside its record. It does not read the offsets of Revit 2015 to 2023
  files or of `Global/Latest`, and it checks the entries placed from the
  provenance table only against that table. Those are not verified.

Why a switch of its own, not `--element-provenance`: the log is bounded (at most
5,000 entries a reason, whatever the model; 1,000 of each stage for
`element_record_frame_stopped`, whose counts in `skipped` are complete) where the provenance table lists
every element ([Provenance table size](#provenance-table-size)), and it needs no table in the
sidecar. Where an entry's
evidence is an element the walk decoded, its stream and offset are read from a
table built for the purpose and dropped. Someone asking why an element was left
out should not have to pay for the table, and someone checking offsets should not
get the log.

### Element record stages

A Revit 2008 to 2023 file takes another element-record layout than 2024 and
later: an ElementId 52 bytes before the record marker (48 on Revit 2008), not the
frame's first bytes. The rejected-frame reasons above (`element_record_without_element_id`,
`element_record_without_volume`) read the 2024 layout and count nothing on these
files, so a file that exported no element showed only
`low_confidence_schema_scan_candidate` and nobody could tell which stage lost
its elements (#1389, from the files reported on #421). `decoded.element_record_stages`
counts what became of the frames at the file's markers, per partition stream
(`streams`) and in total (`total`), where the scan and the recovery already decide:
no further pass over the file. It is present for any export of a file that takes
that layout, which includes a file saved by Revit 2024 or later whose schema is an
older release's, even when each count is zero (that is what says where the records
stopped), and absent otherwise and for a placeholder export, which writes none of
what was decoded. A family file (`.rfa`, `.rft`) reads its elements from
`Global/Latest`, so its project records are none and mean nothing: its report is
there, and `rvt-ifc` prints no line for it. The file name decides that, not the
file's streams: the synthetic project fixtures carry a `PartAtom` stream, as a
family does. It is in the diagnostics only; the IFC is not changed.

The counts are integers added where the scan already decides, whatever the export
asks for: the default export lists no frame and allocates no name for one. Only
`--export-decisions` lists the frames that stopped (below), by asking the
recovery for them before the walk is made.

| Count | What it is |
|---|---|
| `markers` | Places that hold the file's marker: the tags of `Outline`, four `ff` bytes and `ElementParents`. |
| `stream_not_read` | Partition streams that did not inflate, so that their markers are not counted: 1 on the entry of such a stream. It enters no step; when it is all a file has, `stopped_at` is `stream_not_read`, not `no_markers`. |
| `declared_id` | Of the markers, those with a `u32` ElementId that `Global/ElemTable` declares 52 bytes before them and the tag of `ElementHeader` 8 bytes after it. Counted beside the steps: a placed instance whose frame lies inside its element's record gets that record's id (`id_from_enclosing_record`, RE-315). |
| `frame_outside_stream`, `category_out_of_range`, `box_not_valid`, `no_declared_id` | Markers that gave no record, by where they stopped: a field of the frame lies outside the stream; the `BuiltInCategory` is out of range; the bounding box is not finite or ends before it starts; the frame has no declared id of its own and none from a record around it (this one can include frames inside the documents of loaded families, RE-135, whose ids no ElemTable record declares: it is not a count of lost elements). |
| `records` | Markers that gave a record. |
| `not_placed_instances`, `placed_instances` | Records that are container members or type symbols, and those that pass the RE-21 instance rule. |
| `without_volume`, `with_volume` | Placed instances whose box has no volume (Revit's export leaves them out, #309), and those whose box has. These are frames of every category, and an element can have a frame of each. |
| `superseded`, `nested_components`, `selected` | Of the frames with volume, those a newer frame of the same ElementId replaces, the nested MEP components the export leaves out (#96), and the newest frame of each ElementId. |
| `in_non_primary_design_option`, `not_in_export_phase`, `nested_in_door_or_window`, `category_not_exported`, `handed_to_export` | Of the selected frames, those left out for their design option, their phase, being nested in a door or window, or a category no class of the export has, and those that became elements handed to the export. |

A frame is counted once, at the first check it fails, and the checks run in the
order of the scan: a field outside the stream stops it at the first one read; the
category is checked before the container, the placement kind, the class tag and
the box are read; the box is checked before the ElementId is settled. A frame with
an out-of-range category and no declared id is therefore a `category_out_of_range`
stop, not a `no_declared_id` one, and a frame with a bad box and no declared id is
a `box_not_valid` stop.

Each step counts exactly what the one before it passed on and left, so the
counts add up, per stream and in total: `markers` is the four stops plus
`records`, `records` is `not_placed_instances` plus `placed_instances`, and so on
down to `selected`, which is the four ways it is left out plus `handed_to_export`.
The elements handed to the export then add up to what became of them:
`written_to_model` counts, in the IFC model, the elements the model holds, one for
each ElementId, and the rest were set aside after the recovery, each for one
cause. `id_already_held` counts those the walk dropped because an element of an
earlier reader (an ArcWall, a `Global/Latest` instance) already held the
ElementId; `below_min_confidence` those below the minimum confidence, which the
default export hides; `not_written_by_class` the others, by the Revit class they
were handed as, which are elements the exporter writes nothing for. That last
bucket is counted by class and not attributed to a cause beyond it.
`handed_to_export` is `written_to_model` plus `id_already_held` plus
`below_min_confidence` plus the sum of `not_written_by_class`.
`exported.building_elements` also holds the elements other readers write (Areas,
ArcWalls, the parts of stairs and curtain walls), so `written_to_model` is checked
to be no more than it, not equal to it. When `written_to_model` is 0,
`stopped_at` names the stage the file's records stopped at:
`elem_table_not_read` or `schema_has_no_element_marker` (the scan could not
start), `stream_not_read`, `no_markers`, the stop that took most of the markers,
`no_placed_instances`, `without_volume`, `nested_components`, the way that left
out most of the selected frames, or, of the elements handed to the export, the
cause that set aside most (`id_already_held`, `below_min_confidence`,
`class_not_exported`). `rvt-ifc` prints a line of its own with it on stderr.

The frames that stopped before they were records, and the placed instances with
no volume, are also in `skipped` as `element_record_frame_stopped`: `count` is
their number, counted from the integers above, and `classes` counts them by stage
(`frame_outside_stream`, `category_out_of_range`, `box_not_valid`,
`no_declared_id`, `without_volume`). They are frames, not elements: they are of
every category, an element with a frame of no volume and a newer one with volume
is written, and a count such as 1,837 is not 1,837 lost elements (the MCP server's
`diagnostics_summary` says the same of its `count`). The item does not feed
`unexported_element_records` or the warning of `element_record_without_element_id`.
With `--export-decisions` the log of decisions has an entry for each frame, with
the stream and the offset of the place where the frame's ElementId is stored. The
counts are complete and the log is capped: it keeps at most 5,000 entries of a
reason, and of this reason 1,000 of each stage, in ElementId order with the frames
that have no ElementId first, so that a stage with many frames does not leave the
others out; `export_decisions.reasons` gives the total, the number recorded and the
number dropped.

Measured on Autodesk's 33 sample projects of 2015 to 2023 by Measure run
41932213967 (head f44b028d), before the set-aside causes above were added: every
count added up, per stream and in total (`element_stage_counts.txt` ended `ok` on
all 33), and the IFC was byte-identical to main's. Together the 33 projects (41
partition streams) have 197,463 markers, 196,482 of them with a declared
ElementId; 4,477 stopped at the category and 175 at the id, none at the box or
outside the stream, and 192,811 gave a record; 167,702 records are placed
instances, 66,250 of them with no volume, and 101,452 with; 98,119 frames were
selected (3,333 superseded, on one project saved with two partitions, and none
nested), and 87,366 were handed to the export, of which the models hold 87,315.
The 51 not written (1 to 6 on each of 22 projects) were not attributed to a cause
in that run; the causes above count them, and are not yet measured on this head.

`tests/element_record_stages.rs` checks it on Revit_IFC5_Einhoven (Revit 2023):
the markers, and the markers with a declared id, are counted again from the
partitions' bytes, stream by stream; every step adds up; the elements handed to
the export are the elements written and those set aside; the item and the log
have the stages' counts, and the default export has the same counts and item; each
log entry points at a marker 52 bytes on. On the committed synthetic
`corpus/tier1/structural-2023` project, whose partitions hold no record, nothing
is written, `stopped_at` names a stage before any marker and `rvt-ifc` prints it;
on the committed family `tests/fixtures/families/empty.rfa` `rvt-ifc` prints no
line. `tools/re/element_stage_counts.py` makes the same checks on a
`diagnostics.json`; Measure runs it on each of Autodesk's sample projects
(`-f samples=true`, `element_stage_counts.txt`).

## Schema

`schema_version` is currently `1`. Additive fields may appear in a future
minor release; incompatible changes must increment this number.

| Field | Type | Meaning |
|---|---:|---|
| `schema_version` | integer | Diagnostics schema version. |
| `mode` | string | `placeholder`, `default`, or `diagnostic_proxies`. |
| `input` | object | Revit version and build, the file name of the path Revit last saved the file at (`original_path`, without its folders), the export's project name, which required streams are present, and `release_support`: how the release's element records are read, as the `status` (`verified`, `partial`, `experimental`, or `unsupported` where none are read) of the `docs/support-matrix.json` row named in `capability` (#871). An `experimental` release also adds a warning. |
| `decoded` | object | Counts recovered by reader-side scanners before IFC emission. |
| `exported` | object | IFC model entity counts, geometry count, materials, units, storeys. |
| `skipped` | array | Suppressed items, grouped by reason and class. |
| `unsupported_features` | array | Known exporter gaps that affected this output. |
| `warnings` | array | User-facing caveats for this specific export. |
| `confidence` | object | Coarse export-readiness level and booleans for metadata/elements/geometry. |
| `source_coverage` | object (optional) | Measured coverage from export/decode counts. `status` is `measured` when at least one fraction has a trustworthy denominator; otherwise `unset` with null fractions. Never invents ratios. |
| `formats_latest_integrity` | object (optional) | `Formats/Latest` page-boundary integrity under the narrow gate of #151 (the page strip stays **disabled**). Multipage streams report `integrity_status: uncertain` and `diagnostic_code: RVT_FORMATS_MULTIPAGE_UNVERIFIED` without claiming completeness. `schema_scan_truncated` / `schema_scanned_bytes` report whether schema parsing stopped at the 65536-byte scan limit — a "we stopped looking" signal, not a claim about the unscanned tail. |
`skipped.reason` is stable enough for automation. Geometry-related reasons use
the `unsupported_geometry_*` prefix and currently include
`unsupported_geometry_curve`, `unsupported_geometry_profile`,
`unsupported_geometry_unresolved_host`, `unsupported_geometry_missing_level`,
and `unsupported_geometry_missing_dimensions`. A single element can appear in
more than one geometry bucket because these are condition counts, not a
deduplicated element total.

`stream_not_read_resource_limit` lists the streams a resource limit kept from
being read whole (#1075): `classes` maps each stream's name (`Partitions/189`)
to how many limits stopped it, `count` is the number of streams, and one
warning per stream, beginning `Not read whole:`, gives its stored size, the
limit and which one it was (the stream limit, one inflated member, or the
stream's inflated total). Every reader went on without those streams, so the
export is missing what they hold. `rvt-ifc` and `rvt-gltf` write no model and
exit with status 3 when this item is present, unless `--allow-partial`.

`element_record_without_element_id` counts the element records of every
recovered category whose frame is in place (the `BuiltInCategory` at `+0x12`
and the bounding-box marker at `+0x50`) but whose `+0x00` holds no ElementId,
keyed by class in `classes`. The recovered categories are walls, doors,
windows, columns, floors, building pads and rooms, plus the RE-33 product
categories: furniture, casework, plumbing fixtures, specialty equipment,
ceilings, curtain-wall mullions and panels, railings, wall sweeps, ducts,
duct fittings, pipes and pipe fittings, and the RE-36 structural categories:
structural framing, structural columns, structural foundations and generic
models, and the RE-37 categories: lighting fixtures, air terminals,
food-service equipment, planting, parking, entourage, hardscape, vertical
circulation and ramps. Only placed instances count (no
container reference, placement kind `0xffffef7f`), so container members and
type symbols do not inflate it. Only frames inside the partition's leading
record chain count: records after it belong to loaded families' own documents,
whose frames are not model elements. A second-prologue frame whose enclosing
record carries a declared ElementId is exported under it and not counted
(RE-35). The fail-closed decode cannot attribute the frames that remain, so
none is exported, and a matching warning says the model is incomplete. It is
absent on `2024_Core_Interior.rvt`, on the RE1 models, on every
project-count fixture and, since RE-41 declares the ElemTable's second id,
on Autodesk's Snowdon Towers 2024 architectural sample, where it was 37
(16 walls, 4 columns and 17 generic models, all in Revit's own export) (see
`reports/element-framing/RE-30-snowdon-generalisation.md`,
`reports/element-framing/RE-33-product-categories.md` and
`reports/element-framing/RE-35-partition-record-wrapper.md` and
`reports/element-framing/RE-36-structural-categories.md`).

`element_record_without_volume` counts placed instances of a recovered
category whose record bounding box is flat on some axis. These are 2D symbol
families (a floor drain drawn in plan, a wheelchair turning circle, an
accessible clearance zone) with no 3D body. Revit's own IFC export leaves them
out, and so does rvt-rs. They are omitted by design, not missing, so they do
not count toward `unexported_element_records`. On Snowdon Towers all 60 of
them (45 specialty equipment, 15 plumbing fixtures) are absent from Revit's
export, and no element Revit does export lacks volume (#309).

`element_record_in_non_primary_design_option` counts placed instances of a
recovered category that belong to a design option other than their set's
primary one. Revit's own IFC export writes the main model and each option
set's primary option by default, and leaves the other options out; rvt-rs
reads the primary from the set's name entry and does the same. These are
omitted by design too, and do not count toward `unexported_element_records`.
A set whose primary cannot be read keeps all its options. On Snowdon Towers
the count is 27 (9 floors, 9 slab edges, 8 walls, one generic model), none of
which Revit's export holds (RE-40). On Revit 2016 to 2023 the item is
reported too (RE-315): Autodesk's `rst_basic_sample_project` of each of those
releases counts one `Mass`, which Revit does not export, so its `Mass` leaves
the production class counts and the exported IFC does not change.

`element_record_not_in_export_phase` counts placed instances of a recovered
category that do not exist in the export phase: created after it, or
demolished in it or before it. With no phase chosen, Revit's own IFC export
shows the project's last phase and leaves these out; rvt-rs does the same,
and they do not count toward `unexported_element_records`. `rvt-ifc --phase`
names another export phase, by its name or ElementId. A room is not counted
here: Revit's exporter skips it by its own phase, not by its status. On
Snowdon Towers Architectural (2024) the count is 6 (Floor), the slabs of #328,
which are created and demolished in the Legends phase (RE-184). On Revit 2016
to 2023 the same filter applies (RE-315); no Autodesk sample has an element
outside its last phase, and the held-out Vitoria Project's counts are the only
evidence that it is right on a 32-bit file. A read of the phases that fails on
a Revit 2015 to 2027 file is reported in `warnings` ("The project's phases
could not be read"), and the export then writes every element.

`room_not_in_export_phase` counts placed rooms whose own phase
(`RoomElem.m_phaseId`, Revit's `ROOM_PHASE`) is a phase of the project other
than the export phase, which Revit's exporter leaves out and rvt-rs does too
(RE-326). There is one item for each phase the rooms sit in: `count` and
`classes.Room` are the rooms, and `phase` is that phase's name (`ElementId <n>`
where the file names none; `--anonymize` replaces it). A room whose phase is
unset or unread is written and not counted, and so is an Area or an MEP space,
which Revit's exporter does not check against a phase. Autodesk's
`rac_basic_sample_project` has 14 rooms in Working Drawings, the second of
three phases, so the default export counts 14 for that phase and
`--phase "Working Drawings"` counts none (Measure runs 41784415396 and
41784419457). Revit 2016 to 2023 apply it as they apply the
element filter; Revit 2015 and earlier write every room, as every element
(#1332).

On Revit 2024 to 2027, a door or window is written as its opening alone,
with no door or window, only where its type's native record was read with
an empty geometry map (RE-84, RE-299). Where the type's parameter block
holds no map and its native record was not read (a large file over its
limits, #1247), the element is written as a door or window drawn as its
record box, and `warnings` gives their number ("door(s) and window(s) are
written as doors and windows though nothing read tells whether their type
draws geometry"). One that is a cased opening in Revit is its opening alone
in Revit's export.

`element_record_empty_curtain_panel` counts placed curtain panels of the
"Empty System Panel" family, the panel type that fills a grid cell with
nothing. Revit's own IFC export leaves every one of them out, and so does
rvt-rs: all 24 on Snowdon Towers. They are recognised by that family's name,
which is Revit's English one, so a file saved in another language keeps its
empty panels. As a cross-check, an empty panel type stores no material (the
`u64` after its type-object tag is unset), where Snowdon's glazed and solid
panel types name "Glass" and "Default". An empty-family type that carries a
material is reported in `warnings`. The material alone isn't used, because a
panel set to "By Category" may store no material either (#309).

`area_without_proven_body` counts the Areas of a Revit 2015 to 2023 file that are not written, because the lines of their area scheme prove neither a region around their point nor a body (RE-324). `classes` gives the count by the reason their body has none, the reasons of `exported.area_body_refusals` (RE-317): `boundary_lines_not_readable` (the group's lines cannot be read as a whole, or a line off the plan axes lies where the Area's region does, RE-330), `point_in_no_single_region`, `named_lines_not_in_scheme_and_level`, `two_areas_one_region`, `next_storey_not_proven`, `point_not_read`, `level_or_scheme_not_named`, `boundary_line_not_read`, `no_boundary_lines`, `levels_not_recovered` and `named_lines_not_on_region`. It does not say that Revit's export leaves them out: writing every placed Area wrote spaces Revit's export lacks on a held-out project (RE-324), and Revit's exporter makes no space for an Area with no enclosed boundary, which rvt-rs does not read. Revit 2024 writes an Area without a body and counts it in `exported.area_body_refusals` instead.

`element_record_family_space_without_body` counts the family instances written as `IfcSpace` through their "Export to IFC As" that are left out because their family draws no model form for their parameter values (every model form is a void, or hidden by its `Visible` or by the parameter it is bound to). Revit's own IFC export writes a family instance that is not a door or a window only when it has a 3D body, and leaves these out: the 89 bicycle bays of the BIM4LCA terrain office, whose "Stripe Y/N" is 0 (RE-312). An instance whose family document is not found, holds no form, nests a family instance, or has a form bound to a parameter whose value was not read is written. When the check cannot run (a Revit release outside 2023 to 2027, or a read that fails), `warnings` gives how many instances were not checked and why, and all of them are written.

`unsupported_features` carries exactly one geometry-coverage code:
`real_file_element_geometry` when **no** exported building element has a
recovered body, and `partial_element_geometry` when some do and some do not
(Core Interior after #204: 256 `IFCCOLUMN` with bodies, 82 slabs/spaces
without). Neither appears once every exported element carries geometry.

It also carries one parameter code (#35):
`revit_element_parameters_to_ifc_property_sets` when the export writes no value
into an IFC common property set (`Pset_*`), and
`partial_revit_element_parameters` when it writes some. Revit's exporter fills
those sets from element parameters, and rvt-rs writes the ones it reads, such as
`Reference`, `SerialNumber`, a door's `IsExternal` and a pipe's
`InvertElevation`; the rest of an element's parameters are not read yet.

Important nested fields:

| Field | Type | Meaning |
|---|---:|---|
| `decoded.production_walker_elements` | integer | Elements accepted by the conservative production walker path. |
| `decoded.diagnostic_proxy_candidates` | integer | Low-confidence candidates available to diagnostic export. |
| `decoded.arcwall_records` | integer | Version-gated ArcWall records exported as `IFCWALL`. |
| `decoded.parameter_value_count` | integer | Property values the export writes in `Pset_*` sets: each element's, and the storeys' and the building's. It equals the values of the IFC file's `Pset_` sets. |
| `decoded.recovered_unit_identifiers` | array | Revit `autodesk.unit.*` identifiers selected for IFC unit assignment. |
| `decoded.unknown_unit_identifiers` | array | Revit unit identifiers observed but not mapped to IFC units. |
| `decoded.wall_curve_lengths_feet` | object | The length of each exported wall's native curve in feet, by ElementId (RE-252): one of the two lengths Revit's export may write as the wall's `Qto_WallBaseQuantities.Length`. `tools/re/qtos_vs_ifc.py` reads it to count apart a wall whose two lengths differ. It is not written in the IFC file, and is absent when no wall has one. |
| `decoded.walls_type_confirmed` | array | The ElementIds of the exported walls whose native `SWall` record names the type the export gives them (RE-253). Where Revit's export gives such a wall another type, `tools/re/qtos_vs_ifc.py` counts its differing `Width` apart. Not written in the IFC file; absent when there is none. |
| `export_decisions` | object | Present only with `rvt-ifc --export-decisions` (see Export decisions above). Each fail-closed decision behind `skipped` and the `exported` body and outline counts, one entry per element, with a stage, a reason code and where the evidence was read; bounded at 5,000 entries a reason with the rest counted in `dropped`. The `skipped` counts are made from the same decisions. Not written in the IFC file. |
| `decoded.element_provenance` | object | Present only with `rvt-ifc --element-provenance` (see [Provenance table size](#provenance-table-size)). Where each decoded element was read from, by ElementId (#1325): `elements` maps an ElementId to its `carrier` (`partition_element_record_first_prologue`, `partition_element_record_second_prologue`, `global_latest_schema_walk`, `partition_arc_wall`, `partition_area_record`, `partition_derived` or `other`), its `stream` (`Partitions/46`, `Global/Latest`), its `offset` in that stream inflated, the `record_offset` where its ElementId is stored (a partition record header; for a second-prologue frame the enclosing record's, whose id is `enclosing_record_id`, RE-35), its `class_tag` and the `class_name` that resolves to in the file's own schema, its `builtin_category`, and its `native` record (`copies`, which `copy` is named, `stream`, `offset`). An offset indexes the stream as `RevitFile::inflated_partition` returns it, which `rvt-dump` writes as `Partitions_NN.decomp` (#291); for the schema walk it indexes `Global/Latest` inflated as the walk inflates it. `by_carrier` counts every element with an ElementId by carrier, with how many have their offset given (`located`) and how many do not (`unlocated`), whatever the cap of 250,000 listed entries (`omitted_entries` counts the highest ElementIds left out); `unkeyed_elements` counts decoded elements with no ElementId, which have no entry; `native` counts the native records of every element with an ElementId the same way; `layout_release`, `partition_count` and `record_id_bytes` (8 from Revit 2024, 4 before) describe the file. When an element has several partition element record frames, `offset` is the one the exporter selected: on Revit 2024 and later (`select_instance_records`) the frame with the greatest vertical extent and, on a tie, the higher-numbered partition and then the later offset; on Revit 2008 to 2023 (`select_newest_instance_records`, RE-107) the newest, with no extent test. A native record in several partitions names the current copy (RE-145, RE-216). A body reader may take another copy (the sketch-line readers take the current one), so `offset` and `native.stream` can name different partitions: the copy a reader took is in `value_sources`. The schema-walk carrier is unmeasured: no model of the offsets test has such an element. Not written in the IFC file; absent for a placeholder export and unless asked for. Each element also lists its `value_sources` (#1360), the copy each reader took a value of its body from, and the table its `value_source_counts`. |
| `exported.by_ifc_type` | object | Count of exported building elements grouped by STEP entity type. |
| `exported.building_elements_with_geometry` | integer | Exported elements with enough placement/body data for geometry. |
| `exported.building_elements_carried_by_parts` | integer | Exported elements with no body of their own because their aggregated parts carry it, as in Revit's export: stairs (RE-39) and curtain walls (RE-46). Not counted in `building_elements_with_geometry`. |
| `exported.storey_names` | array | Recovered building-storey display names, in emission order. |
| `exported.storey_elevations_feet` | array | Recovered storey elevations in feet, aligned with `storey_names`. |
| `exported.storey_bound_elements` | integer | Building elements contained in a specific storey. The rest are contained in the `IfcBuilding`, never in a named storey. |
| `exported.layered_element_count` | integer | Elements whose compound layers and layer colours were read (RE-53). The glTF export draws each in its layers where they add up to its body's thickness; the rest are drawn whole. |
| `exported.bounding_box_spaces` | integer | Exported `IfcSpace`s whose body is their record's bounding box (`RvtElementRecordGeometry.SpaceOutlineSource` is `record_box`): a room whose outline was not read from its stored solid, or a family instance written as a space and drawn as its box. The body's plan area is the box's, not the space's (RE-313, [rooms](rooms.md)). Each is also in `exported.bounding_box_bodies`, except a family instance turned or tilted off the model's axes. |
| `exported.area_body_refusals` | object | Of the exported Areas that have no body, why, by reason, counted from each Area's `RvtElementRecordGeometry.AreaBodyRefusal` (RE-317): `levels_not_recovered`, `boundary_line_not_read` (a declared area boundary line was not read or cannot be placed on a Level, so no Area of the file is drawn), `no_boundary_lines`, `point_not_read`, `level_or_scheme_not_named`, `named_lines_not_in_scheme_and_level` (a line the Area's record names is not a declared line of its scheme on its Level that was read: its copies disagree, it has no Level or scheme, or it is in a non-primary design option), `next_storey_not_proven` (the Area's Level and the one above are not both building stories, or another Level lies between or at them), `boundary_lines_not_readable` (a line of the group overlaps another, may be missing or has copies that disagree, or is in a non-primary design option; or a line that is not straight along an axis, or whose record box is wider than the line, lies where the Area's region does or is joined to a line that bounds it, RE-330), `point_in_no_single_region`, `named_lines_not_on_region` and `two_areas_one_region` (another placed Area of the same scheme and Level has the same region, whether or not that one is drawn). Empty when every Area is drawn or the export has no geometry. |
| `exported.slab_shape_refusals` | object | Of the exported floors, roofs and ceilings whose corner vertices are shape-edited (`SlabShapeEditGStep`) and that keep their record's box, why, by reason, counted from each element's `RvtElementRecordGeometry.SlabShapeRefusal` (#1384, RE-336): `interior_edge_vertices_or_creases`, `corners_not_in_one_plane`, `first_three_corners_collinear`, `fewer_than_three_corner_vertices`, `shape_edit_dormant_or_unmarked`, `shape_edit_steps_disagree`, `shape_edit_fields_not_read`, `corner_vertex_not_finite`, `record_copies_disagree`, `record_copy_not_read`, `roof_or_ceiling_edit_not_drawn`, `plan_outline_not_read`, `plan_outline_in_pieces`, `corners_not_the_outline_vertices`, `plane_does_not_reproduce_record_box`. Empty when no such element is kept as its box. |
| `exported.wall_base_refusals` | object | Of the exported walls whose base join names a floor, roof or ceiling with shape-edited corners and that keep their base, why, by reason, counted from each wall's `RvtElementRecordGeometry.BaseCoverRefusal` (#1384, RE-336): `slab_shape_not_read`, `wall_not_one_plain_extrusion`, `named_element_body_not_read`, `no_element_covers_the_wall_plan`, `several_elements_cover_the_wall`, `top_crosses_the_wall_base_or_top`, `top_at_or_below_the_wall_base`, `top_at_or_above_the_wall_top`. A wall that names only unedited elements and is not cut carries none. |
| `exported.resolved_outline_spaces` | integer | Exported rooms whose outline was read from the faces their stored solid lists, up to the solid's own end, where the walk of RE-101 did not close them (`RvtElementRecordGeometry.SpaceOutlineSource` is `stored_solid_faces`, RE-316, [rooms](rooms.md)). Equal to Revit's area on every such room measured, but bounded by the solid's list of objects and a gap of 1,024 bytes, so a closed loop of another element's faces inside the room's box would be read as the room's. |
| `exported.approximate_outline_spaces` | integer | Exported rooms whose resolved outline is the boundary of a region, a column that overlaps a wall having been taken off it or an end of one wall edge lying on another, or falls up to 0.02 ft short of their record box on a side (`SpaceOutlineSource` is `stored_solid_approximate`, RE-316, [rooms](rooms.md)). The area is approximate: up to 0.68% under Revit's floor area where Revit's export does not take that column off the room, and equal to it on 10 of the 15 BIM4LCA office room scores. |
| `exported.wall_cut_openings_left_out` | object | The steps of the walls' native records that name a rectangular wall opening, a fire hose cabinet, a door in a second wall or a curtain wall and write no opening for it, by reason (RE-321, RE-338). A wall's record lists the elements that cut it; the step is written as an `IfcOpeningElement` that nothing fills where its box is proven, and counted here where it is not: `rectangular_opening_not_read` and `rectangular_opening_not_in_wall` (the opening's saved rectangle is missing or does not lie in its wall's plane), `cabinet_cut_solid_not_read` and `cabinet_not_reaching_wall`, `door_cut_flag_not_read`, `door_opening_box_not_read` (the door has no own opening box of a read cut loop), `door_wall_axis_not_read`, `door_second_wall_parallel_to_door_wall` (Revit cuts the opening ten feet across, about the plane the door's symbol cuts in, which is not read) and `door_opening_not_in_second_wall`, `curtain_wall_box_not_read`, `curtain_wall_not_in_wall`, `curtain_wall_within_half_foot_of_wall_end` and `curtain_wall_box_beside_another_wall` (Revit cuts a curtain wall's opening back where the curtain wall reaches the wall's end or another wall lies beside it, by an amount nothing read gives), `opening_extent_not_positive`, and `wall_body_not_an_axis_aligned_prism` (the wall the step is in is not drawn as a prism along the model's axes: curved, turned, stepped otherwise than RE-320's, of several pieces, or one whose ElementId an earlier element holds, so no opening of any kind is written in it), `shadow_cutout_step_not_read` (a `WallShadowCutoutGStep` names the cutter, a step whose box is not read) `door_cuts_a_wall_it_is_not_hosted_in_as_its_own` (a door's step with `m_bCutSomething` 1 in a wall other than the one it is hosted in, a door with no host included), `cabinet_cut_flag_not_set` (a cabinet's step whose `m_bCutSomething` is not 1, the only value measured), `opening_outside_wall_height` (a cabinet's, door's or curtain wall's box rises more than 0.05 ft above its wall's top or falls below its base) and `curtain_wall_record_copies_disagree` (the curtain wall's element record is framed in several partitions and the copies' boxes differ by more than 0.01 ft). `cabinet_cut_solid_not_read` includes a symbol that cuts several solids whose boxes do not touch. A counted step is not a statement that Revit's export has no opening for it: it writes one for most of them. Empty when every step is written or the export has no geometry. |
| `confidence.level` | string | `scaffold`, `typed_no_geometry`, `geometry`, `diagnostic_partial`, or `proxy_only`. |
| `confidence.score` | number | Heuristic 0..1 readiness score for UI sorting and dashboards. Its element terms (elements, typed elements, geometry) are scaled by the exported share when `unexported_element_records` is non-zero. |
| `confidence.unexported_element_records` | integer | Element records of a recovered category (walls, doors, windows, columns, floors, rooms and the RE-33 product categories) the partition scan found but could not export because no ElementId is attributable (RE-30). Non-zero means the model is incomplete. Zero is not a completeness claim. |

## Storey provenance

`storey_count` / `storey_names` / `storey_elevations_feet` describe storeys
whose elevation the file gives. A file where no Level's elevation is read
(family files, and projects of releases whose Level records are not read)
has no storeys: `storey_count` is 0 and its elements are contained in the
`IfcBuilding`. Level-like strings alone are not written as storeys. Until
0.4.0 they were, at elevation 0, and on a Revit 2026 house none of the three
(`Ground floor`, `Level 1`, `Roof`) was one of its Levels (RE-124).

- **Measured elevations.** Base elevations were recovered — from 2023 ArcWall
  trailers, or on Revit 2024 from the partition element-record bounding boxes
  (#213) — so `storey_elevations_feet` carries distinct values and
  `storey_bound_elements` counts the elements that matched one. Storeys whose
  elevation was measured but whose name was not are labelled
  `Elevation N ft`; a Level name is applied only when there is exactly one
  recovered name per measured elevation, because any other pairing would be a
  guess about which Level sits where. The unpaired names remain visible in
  `decoded.production_class_counts.Level`, and a warning states how many were
  left unplaced.

A third recovery sits in front of both and is the only one the file *states*
rather than infers. On Revit 2024 a partition element record's counted
reference list names the `Level` that hosts the element, and when it names
exactly one recovered Level the element is contained in that Level's storey
(#219, RE-27). Such an element carries `LevelBindResolved = true`,
`LevelElementId`, and `LevelBindSource =
partition_element_record_reference_list`. A record that names no Level, or two
— a column and a wall each carry a base *and* a top constraint, with nothing
in the bytes saying which slot is which — resolves to nothing and keeps the
elevation join.

Every element that reached a storey says how, in `StoreyBindSource`:

| value | join |
|---|---|
| `record_level_reference` | the Level ElementId the record names (#219) |
| `record_base_elevation` | the record's base face equals a storey elevation (#213) |
| `record_top_elevation` | the record's top face equals one — plates hang below their level (#212) |

The property's *absence* is the honest unbound state. An unbound element is
contained in the `IfcBuilding`, which says "in this building, storey unknown";
it is never written into the first storey, because that would state a
containment nothing measured.

`LevelBindResolved` still stays `false` for every element whose storey came
from an elevation match, and for the typed `Floor` / `Room` `m_level_id` join,
which remains unrecovered (#86 / RE-20).

## Geometry placement

`exported.building_elements_with_geometry` counts elements that carry a body.
Where that body sits is fixed by one rule: **the element translation is carried
exactly once**, by the element's own `IfcLocalPlacement`. The swept solid's
`Position` is the project-level identity placement, and the profile is authored
in the element-local frame.

That matters to anyone comparing a diagnostics count against what a viewer
draws. Until #232 the writer put the element's own `IfcAxis2Placement3D` in
both slots, so a consumer that composes `ObjectPlacement × Position` — which
IFC4 requires — placed every element at twice its translation, while viewers
that ignore `Position` drew it correctly. Two tools could therefore disagree
about the same file with neither of them wrong about what it read. A geometry
count that looks right is not evidence that the geometry is where it belongs;
compose the two and check, as `tools/ci/ifc_schema_arity.py` now does on every
export.

## Example

```json
{
  "schema_version": 1,
  "mode": "default",
  "input": {
    "revit_version": 2024,
    "build": "20230308_1635(x64)",
    "original_path": "model.rvt",
    "project_name": "Sample",
    "stream_count": 24,
    "has_basic_file_info": true,
    "has_part_atom": true,
    "has_formats_latest": true,
    "has_global_latest": true,
    "release_support": {
      "status": "partial",
      "capability": "typed-project-elements"
    }
  },
  "decoded": {
    "production_walker_elements": 0,
    "diagnostic_proxy_candidates": 9,
    "arcwall_records": 0,
    "class_counts": {
      "HostObjAttr": 9
    },
    "recovered_unit_identifiers": [
      "autodesk.unit.unit:meters-1.0.0"
    ],
    "unknown_unit_identifiers": []
  },
  "exported": {
    "total_entities": 1,
    "building_elements": 0,
    "building_elements_with_geometry": 0,
    "by_ifc_type": {},
    "classification_count": 0,
    "unit_assignment_count": 1,
    "material_count": 0,
    "storey_count": 0,
    "storey_names": [],
    "storey_elevations_feet": [],
    "storey_bound_elements": 0
  },
  "skipped": [
    {
      "reason": "low_confidence_schema_scan_candidate",
      "count": 9,
      "classes": {
        "HostObjAttr": 9
      },
      "sample_names": ["HostObjAttr-12345"]
    }
  ],
  "unsupported_features": ["real_file_element_geometry"],
  "warnings": ["No building elements were exported; output is scaffold-only."],
  "confidence": {
    "level": "scaffold",
    "score": 0.25,
    "has_project_metadata": true,
    "has_typed_elements": false,
    "has_geometry": false,
    "has_diagnostic_proxies": false,
    "warning_count": 1,
    "unexported_element_records": 0
  },
  "source_coverage": {
    "status": "unset",
    "notes": "Export source-coverage fractions are unset: no trustworthy denominator was available; fields stay null."
  }
}
```

Important `source_coverage` fields:

| Field | Meaning when measured |
|---|---|
| `decoded_element_fraction` | `production_walker_elements /` the distinct ElementIds `Global/ElemTable` declares, when walker ≤ that count. (Before 2026-09-22 the denominator was the header's `element_count`, which is a per-release constant, not a count.) |
| `exported_element_fraction` | `building_elements / production_walker_elements` |
| `geometry_element_fraction` | `building_elements_with_geometry / building_elements` |

These are inspection ratios from counts already in the sidecar — not a claim that the file was fully converted.