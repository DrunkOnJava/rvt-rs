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

adds `decoded.element_provenance` to the sidecar. On the BIM4LCA office as built the table, with its value sources (#1360, #1368), is 6,128,625 bytes of compact JSON (what the library and Python write) against 38,640 bytes of diagnostics without it, about 159 times; the CLI's pretty-printed sidecar grows from 52,677 to 10,920,400 bytes (207 times) and Core Interior's from 27,588 to 1,500,700 (Measure run 41875078758). The `parts` of #1368 added 2.4% to the office's table (5,987,841 bytes before, measured on main in the same run) and 62% to Core Interior's (440,909 to 715,966), where the sketch-line profiles are listed line by line. RE1 Architecture's grew from 13,956 to 85,470 before the value sources and has not been measured since. Without the flag the sidecar has no such field and is the bytes it
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
`frame` (the last of the record's frames, which were not compared or do not
agree).
A reader notes its choice where it makes it, only while the export builds this
table, and the entry is listed only where the element's final `BodySource` is
drawn from that reader.

| `BodySource` | Reader | Rule | What `offset` is |
|---|---|---|---|
| `partition_element_record_bbox`, `partition_element_record_join_trimmed`, `partition_element_record_join_cut`, `partition_element_record_column_cut` | `selected_frame` | `selected` | The frame itself (a trim or cut also reads the joined elements' frames, which are not named). |
| The same bodies where the profile is read from a record no reader notes yet: a `profile_source` that is not the `body_source` | `selected_frame_box` | `selected` | The frame, for the box and the placement only. The profile's copy has no source yet: today that is every room (its outline is read from its solid's faces, `partition_room_solid_faces`, 116 elements on Core Interior and 234 on the office). |
| The same bodies where the profile is solved from sketch lines (`profile_source` `partition_element_record_sketch_lines`: floors, slabs and ceilings, and roofs whose body is the record's box; a roof drawn from its slope or extrusion has no source yet) | `sketch_lines` | `agreeing` or `current` | `parts`: the frame of each sketch line the solve was given, the first of its frames by frame order (`agreeing`), or the current partition's where its frames disagree (`current`, RE-216). The element's `selected_frame` names the frame for the box. |
| The same | `sketch_list` | `agreeing` or `current` | The element-data header of the Sketch's curve list that names the lines (RE-97), the lowest-numbered partition's or the current one. Where the element has one. |
| The same, where the outline is read from the lines' own ends rather than their boxes | `sketch_line_data` | `agreeing` or `current` | `parts`: the element-data header of each line's copy (as `bounded_line`). |
| The same bodies where the section is the type symbol's box (`profile_source` `family_type_symbol_envelope`: columns) | `type_section` | `agreeing` or `frame` | `parts`: the type symbol's frame, the last of its frames the column scan kept; `agreeing` (with `partitions`) where every frame of the symbol gives the same box, `frame` where they do not. |
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

No reader notes its copy yet for `partition_area_boundary_lines`,
`family_type_hollow_section`, `family_type_i_section`, `partition_pipe_axis`,
`partition_roof_slope`, `partition_roof_extrusion` and `family_extrusions`, nor
for the other values a body reads (a wall's layers, a roof's slopes, the
parameters of a type's section). An element of those bodies has no value
source; an element whose box is the body but whose profile is read from a
record no reader notes yet (a room's outline, read from its solid's faces)
has `selected_frame_box` and nothing for the profile. Neither is a claim that
it read the frame. `value_source_counts`
counts, by reader, the elements with a source, those whose value is held in
another partition than their frame, and those whose copy was chosen as the
current partition's (`current`), so a run on any model shows whether it
exercises that rule. On Core Interior and the BIM4LCA office as built no source
has the rule `current` (Measure run 41875078758: no `current` count). That does
not say every copy set agreed: a set that disagreed and could not be resolved is
dropped by its reader and leaves no source and no count.

`tests/value_sources_offsets.rs` reads the bytes at each recorded offset on Core
Interior, the RE1 models, Einhoven and the BIM4LCA as-built office. For the
element-data readers it finds the element-data header, then the element's own
id, then a record that decodes to the value the reader returns when asked again;
for a native record it finds the ElementId at the offset; for each record in
`parts` it reads what that record is: a sketch line's or a type symbol's frame
has the element's id at its first bytes, or, on a second-prologue frame, the id
of the record that encloses it (found from the bytes); a sketch line's data is
an element-data header and then the line's own id; a run type or a native type
record starts with its id; and a Sketch's curve list is a header followed by
the sketched element's id. A frame of a 32-bit release with no id at its first
bytes is counted as not read. It does not say the
copy taken is the right one: `agreeing` copies are equal, and `current` is
`current_partitions`, which it does not measure.

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
| `element_skipped` | `below_min_element_confidence`, `low_confidence_schema_scan_candidate`, `element_record_without_element_id`, `element_record_without_volume`, `element_record_in_non_primary_design_option`, `element_record_not_in_export_phase`, `room_not_in_export_phase`, `element_record_empty_curtain_panel`, `element_record_family_space_without_body`, `area_without_proven_body` | The element is not in the export. | the `skipped` item of the same reason |
| `body_not_drawn` | `unsupported_geometry_curve`, `unsupported_geometry_profile`, `unsupported_geometry_missing_dimensions` | The element is written with no body. | the `skipped` item |
| `opening_not_cut` | `unsupported_geometry_unresolved_host`, `opening_index_without_host_geometry` | A door or window has no host to cut, or an opening index row has no host geometry. | the `skipped` item |
| `storey_not_bound` | `unsupported_geometry_missing_level` | The element is written outside any storey. | the `skipped` item |
| `outline_not_read` | `floor_boundary_annotation_only` | A floor keeps its boundary annotation alone. | the `skipped` item |
| `outline_not_read` | `space_outline_is_record_box` | A room or space is its record's box. | `exported.bounding_box_spaces` |
| `outline_not_read` | `area_body_refused` (`detail`: why) | An exported Area has no body. | `exported.area_body_refusals` |
| `body_left_as_box` | `body_is_record_box` (`detail`: the `BodySource`) | The body is only the record's bounding box. | `exported.bounding_box_bodies` |
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
- `tests/export_decisions_counts.rs` re-reads the bytes at the entries of those
  seven frame reasons (the six above and `element_record_family_space_without_body`)
  on Revit 2024 and later, and finds the element's id there or a second-prologue
  frame inside its record. It does not read the offsets of Revit 2015 to 2023
  files or of `Global/Latest`, and it checks the entries placed from the
  provenance table only against that table. Those are not verified.

Why a switch of its own, not `--element-provenance`: the log is bounded (at most
5,000 entries a reason, whatever the model) where the provenance table lists
every element (about 159 times the rest of the diagnostics in compact JSON on the
BIM4LCA office as built), and it needs no table in the sidecar. Where an entry's
evidence is an element the walk decoded, its stream and offset are read from a
table built for the purpose and dropped. Someone asking why an element was left
out should not have to pay for the table, and someone checking offsets should not
get the log.

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
| `decoded.element_provenance` | object | Present only with `rvt-ifc --element-provenance` (see CLI above for the size: about 159 times the rest of the diagnostics in compact JSON on the BIM4LCA office). Where each decoded element was read from, by ElementId (#1325): `elements` maps an ElementId to its `carrier` (`partition_element_record_first_prologue`, `partition_element_record_second_prologue`, `global_latest_schema_walk`, `partition_arc_wall`, `partition_area_record`, `partition_derived` or `other`), its `stream` (`Partitions/46`, `Global/Latest`), its `offset` in that stream inflated, the `record_offset` where its ElementId is stored (a partition record header; for a second-prologue frame the enclosing record's, whose id is `enclosing_record_id`, RE-35), its `class_tag` and the `class_name` that resolves to in the file's own schema, its `builtin_category`, and its `native` record (`copies`, which `copy` is named, `stream`, `offset`). An offset indexes the stream as `RevitFile::inflated_partition` returns it, which `rvt-dump` writes as `Partitions_NN.decomp` (#291); for the schema walk it indexes `Global/Latest` inflated as the walk inflates it. `by_carrier` counts every element with an ElementId by carrier, with how many have their offset given (`located`) and how many do not (`unlocated`), whatever the cap of 250,000 listed entries (`omitted_entries` counts the highest ElementIds left out); `unkeyed_elements` counts decoded elements with no ElementId, which have no entry; `native` counts the native records of every element with an ElementId the same way; `layout_release`, `partition_count` and `record_id_bytes` (8 from Revit 2024, 4 before) describe the file. When an element has several partition element record frames, `offset` is the one the exporter selected: on Revit 2024 and later (`select_instance_records`) the frame with the greatest vertical extent and, on a tie, the higher-numbered partition and then the later offset; on Revit 2008 to 2023 (`select_newest_instance_records`, RE-107) the newest, with no extent test. A native record in several partitions names the current copy (RE-145, RE-216). A body reader may take another copy (the sketch-line readers take the current one), so `offset` and `native.stream` can name different partitions: the copy a reader took is in `value_sources`. The schema-walk carrier is unmeasured: no model of the offsets test has such an element. Not written in the IFC file; absent for a placeholder export and unless asked for. Each element also lists its `value_sources` (#1360), the copy each reader took a value of its body from, and the table its `value_source_counts`. |
| `exported.by_ifc_type` | object | Count of exported building elements grouped by STEP entity type. |
| `exported.building_elements_with_geometry` | integer | Exported elements with enough placement/body data for geometry. |
| `exported.building_elements_carried_by_parts` | integer | Exported elements with no body of their own because their aggregated parts carry it, as in Revit's export: stairs (RE-39) and curtain walls (RE-46). Not counted in `building_elements_with_geometry`. |
| `exported.storey_names` | array | Recovered building-storey display names, in emission order. |
| `exported.storey_elevations_feet` | array | Recovered storey elevations in feet, aligned with `storey_names`. |
| `exported.storey_bound_elements` | integer | Building elements contained in a specific storey. The rest are contained in the `IfcBuilding`, never in a named storey. |
| `exported.layered_element_count` | integer | Elements whose compound layers and layer colours were read (RE-53). The glTF export draws each in its layers where they add up to its body's thickness; the rest are drawn whole. |
| `exported.bounding_box_spaces` | integer | Exported `IfcSpace`s whose body is their record's bounding box (`RvtElementRecordGeometry.SpaceOutlineSource` is `record_box`): a room whose outline was not read from its stored solid, or a family instance written as a space and drawn as its box. The body's plan area is the box's, not the space's (RE-313, [rooms](rooms.md)). Each is also in `exported.bounding_box_bodies`, except a family instance turned or tilted off the model's axes. |
| `exported.area_body_refusals` | object | Of the exported Areas that have no body, why, by reason, counted from each Area's `RvtElementRecordGeometry.AreaBodyRefusal` (RE-317): `levels_not_recovered`, `boundary_line_not_read` (a declared area boundary line was not read or cannot be placed on a Level, so no Area of the file is drawn), `no_boundary_lines`, `point_not_read`, `level_or_scheme_not_named`, `named_lines_not_in_scheme_and_level` (a line the Area's record names is not a declared line of its scheme on its Level that was read: its copies disagree, it has no Level or scheme, or it is in a non-primary design option), `next_storey_not_proven` (the Area's Level and the one above are not both building stories, or another Level lies between or at them), `boundary_lines_not_readable` (a line of the group overlaps another, may be missing or has copies that disagree, or is in a non-primary design option; or a line that is not straight along an axis, or whose record box is wider than the line, lies where the Area's region does or is joined to a line that bounds it, RE-330), `point_in_no_single_region`, `named_lines_not_on_region` and `two_areas_one_region` (another placed Area of the same scheme and Level has the same region, whether or not that one is drawn). Empty when every Area is drawn or the export has no geometry. |
| `exported.resolved_outline_spaces` | integer | Exported rooms whose outline was read from the faces their stored solid lists, up to the solid's own end, where the walk of RE-101 did not close them (`RvtElementRecordGeometry.SpaceOutlineSource` is `stored_solid_faces`, RE-316, [rooms](rooms.md)). Equal to Revit's area on every such room measured, but bounded by the solid's list of objects and a gap of 1,024 bytes, so a closed loop of another element's faces inside the room's box would be read as the room's. |
| `exported.approximate_outline_spaces` | integer | Exported rooms whose resolved outline is the boundary of a region, a column that overlaps a wall having been taken off it or an end of one wall edge lying on another, or falls up to 0.02 ft short of their record box on a side (`SpaceOutlineSource` is `stored_solid_approximate`, RE-316, [rooms](rooms.md)). The area is approximate: up to 0.68% under Revit's floor area where Revit's export does not take that column off the room, and equal to it on 10 of the 15 BIM4LCA office room scores. |
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