# MCP server: rvt-rs-mcp

`rvt-rs-mcp` is a local [Model Context Protocol](https://modelcontextprotocol.io)
server for Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`). An agent opens a file
and asks about its elements, levels, rooms and diagnostics, or has it exported
as IFC4 or glTF, with no Revit install and no network.

- **Local.** It links the `rvt` crate in process and speaks MCP on stdin and
  stdout. It starts no subprocess, opens no socket and makes no network call.
- **Read-only on the model.** The one thing it writes is an export file a tool
  is asked for, at a path the caller names. Reading is not confined: a tool
  opens any file the user running the server can read, wherever its `model`
  path points, and `--output-dir` confines the writes only.
- **One source of truth.** `open_model` runs the decode `rvt-ifc` runs, up to
  the model its exporter builds and the diagnostics that go with it, and keeps
  that model. Every query tool reads it, and the exports are written from it, so
  an answer and the IFC cannot disagree. The decode is one library function,
  `rvt::ifc::run_export`, which `rvt-ifc`, `rvt-gltf`, the Python package and
  the viewer call as well, so the steps are written once.
- **Honest about gaps.** It reports what it left out the way the CLIs do:
  unread streams, skipped items, diagnostics. A field the model does not have is
  absent from an answer, never guessed.

It is new in v0.14.0 (see [Install](#install)).

## Install

From crates.io (needs Rust 1.88 or later, one above the other tools,
because the MCP SDK `rmcp` 3.x declares 1.88):

```bash
cargo install rvt-rs-mcp --locked
rvt-rs-mcp --version
```

From a checkout of `main`, `cargo install --path rvt-mcp --locked`. The
prebuilt archives on
[GitHub Releases](https://github.com/DrunkOnJava/rvt-rs/releases)
(`rvt-rs-mcp` sits beside the CLIs) and the container image
`ghcr.io/drunkonjava/rvt-rs` carry it from v0.14.0. In the image the model's
folder is mounted at `/data`:

```bash
docker run --rm -i -v "$PWD:/data" ghcr.io/drunkonjava/rvt-rs rvt-rs-mcp
```

## Configure a client

The server is a stdio server whose command is `rvt-rs-mcp`.

Claude Code:

```bash
claude mcp add rvt -- rvt-rs-mcp
```

Claude Desktop (`claude_desktop_config.json`) and any client that reads
`mcpServers`, such as a project's `.mcp.json`:

```json
{
  "mcpServers": {
    "rvt": {
      "command": "rvt-rs-mcp",
      "args": ["--max-open-models", "2"]
    }
  }
}
```

Give the full path to the binary as `command` when the client does not search
your `PATH`. Add `--large-model` to `args` for models with a partition of
hundreds of megabytes, and `--output-dir <folder>` to confine the exports to one
folder.

## Protocol

The server targets MCP 2026-07-28, the stateless revision, and also answers the
older `initialize` handshake (2024-11-05 to 2025-11-25) with the same tools. A
2026-07-28 client probes with `server/discover`; an older one sends
`initialize`. Neither needs anything from the server beyond the tools: it holds
no session, and it never asks the client for anything (no sampling, elicitation
or roots), so it works with a client that offers none of them.

- `tools/list` is sorted by name and fresh for an hour (`ttlMs`) for a
  2026-07-28 client, since the inventory is fixed per binary.
- Every tool declares an `outputSchema` and returns `structuredContent`, with
  the same JSON as a text block. A tool error has `isError` set and a text
  message. Bad arguments (a missing field, a value of the wrong type) are tool
  errors too, not JSON-RPC errors, so the model can correct them: the tools read
  their arguments themselves, because rmcp's `Parameters` extractor answers a
  malformed argument object with a protocol error.
- The input and output schema of every tool, and their descriptions, are in
  [`mcp-tools.md`](mcp-tools.md), rendered from [`mcp-tools.json`](mcp-tools.json),
  which is what the built server reports for `tools/list` (the end-to-end test
  fails when the committed file differs, and CI fails when the Markdown differs
  from the JSON). This page does not restate them.
- A request that carries a progress token gets `notifications/progress` while the
  model decodes (the stages are those of `WalkerControl`: schema parse, candidate
  scan, element decode, partition scan, saved graphics and saved meshes), and a
  last one when the call ends. A cancelled call is cancelled by the client's
  `notifications/cancelled`, which rmcp turns into a cancelled `context.ct`
  (`rmcp` 3.8 `src/service.rs`, the serve loop: it removes the request from its
  table and cancels its token, and `context.ct` is a child of that token), and
  the server hands that to `WalkerControl`. The decode and the saved-mesh pass
  stop at their next checkpoint, and an export being written stops at its next
  chunk: nothing is left under the target's name, and the temporary file is
  removed. rmcp sends no response to a cancelled request, as MCP says. A decode
  already running when the cancellation arrives is not killed mid-step: the
  walk checks between steps.
- stdout carries only JSON-RPC. Logs go to stderr (`--log-level`). The server
  exits with status 0 when the client closes stdin, whenever that happens: after
  a session, after a lone `server/discover` probe (answered before any session
  exists) or `initialize`, or before anything was sent. A first message that is
  not a request is a failed start and exits 1.

## Handles

`open_model` decodes the file once and returns an opaque `model_id` (`rvt-` and
sixteen hex digits, a hash of the file's canonical path, size and modification
time, with no path in it). Every other tool takes the `model_id` or the path in
its `model` parameter (`model_id` and `path` are accepted as names for it), and
a path that is not open yet is opened.

A Revit project can hold several phases, and an export shows one: the last, as
Revit's own does, unless told otherwise. Every tool that takes a `model` path
also takes `phase`, a phase's name (exactly) or its ElementId, as
`rvt-ifc --phase` does. The model is then read at that phase: an element that
does not exist in it, and a room whose own phase is not it, are left out, and
`get_diagnostics` counts them. A model opened at a phase is a different model
from the same file with no `phase`: it has its own `model_id` (the hash covers
the phase's ElementId, so a phase named by its name and by its number is one
model), and the server keeps both. Naming the project's last phase is a model
of its own too, since the server does not know a file's last phase until it
reads it. A `model_id` names the phase it was opened at, so passing a different
`phase` with one is a tool error; pass the path with `phase` instead. A phase
the file does not have is a tool error that lists the phases it does; the
server remembers the answer, so asking again does not read the file again.

The handle lives while the server process runs and while the model is among the
most recently used ones the server keeps (2 by default, `--max-open-models`):
opening another model drops the least recently used one, and a decoded large
model needs several GB. A `model_id` this server did not issue, one it evicted,
one from before a restart, or one whose file changed on disk is a tool error
that says to call `open_model` again. The tool descriptions say the same.

## Tools

| Tool | What it returns |
|---|---|
| `open_model` | Decodes the file and returns the `model_id` with the summary: file kind, Revit release and build, element counts by IFC entity and Revit class, levels, room count, what the export left out and the file-level provenance. |
| `get_summary` | The same summary for a `model_id` or a path. |
| `list_elements` | Building elements, filtered by `category` (IFC entity: `IFCWALL`, `IfcWall` and `wall` are the same), `class` (Revit class: `Wall`, `Room`) and `level` (storey name), paged by `limit` (100 by default, at most 1000) and `offset`. Each row has the ElementId, `name` and `long_name` (the IFC `Name` and `LongName`: an IfcSpace's are its room number, or `<room name>:<ElementId>` for a room with a name and no number, and its room name; `name` is absent only for a room with neither whose name is exactly the exporter's `<Revit class>-<ElementId>` placeholder, and the placeholder names of other elements are still served as `name`), level, `predefined_type` and its provenance. |
| `get_element` | One element by Revit ElementId or IFC GlobalId: its `name` and `long_name` as `list_elements` gives them, its property sets and quantities as the IFC export writes them (quantities in the IFC's SI units, each property with its IfcValue type), its trust markers, its provenance, the file's, its type object as the export writes it (`element_type`: the IFC entity as `ifc_type`, the `name` and the `element_id`, the type object's `Tag`, and the sets it holds, `Other` and `Identity Data` among them, or the `IfcSpaceType` made for a room; absent where the export writes none) and any other entity with the same ElementId. |
| `explain_element` | Why one element is in the export as it is, or is not in it (#1326). `element` is a Revit ElementId, whether or not the export holds the element, or the GlobalId of an exported one. Returns the element's provenance and trust markers where it is exported, and every fail-closed decision recorded for it: a `stage` (`element_skipped`, `body_not_drawn`, `body_left_as_box`, `opening_not_cut`, `outline_not_read`, `storey_not_bound`), a `reason` (the `reason` of the `skipped` item in `get_diagnostics`, or `body_is_record_box`, `space_outline_is_record_box`, `area_body_refused`, `slab_shape_refused`, `wall_base_cut_refused`, `wall_end_extension_refused`), the `stream` and byte `offset` its record was read from where known, and a `detail`. |
| `list_rooms` | Rooms and spaces: ElementId, number, name, level, area and provenance. The area is present only where the outline was read exactly. |
| `list_levels` | Storeys with elevation (feet and metres) and element count. |
| `get_diagnostics` | The export diagnostics, as `rvt-ifc --diagnostics` writes them ([`export-diagnostics.schema.json`](schemas/export-diagnostics.schema.json)): what was decoded, exported, skipped, approximated and left unread, and why. |
| `export_ifc` | Writes IFC4 to `out_path` and returns a summary and a `resource_link` to the file. `options` takes `rvt-ifc`'s switches: `mode`, `saved_meshes` (`doors_and_windows`, `all`, `off`), `revit_property_sets`, `revit_set_placement`, `no_base_quantities`, `allow_partial`. |
| `export_gltf` | Writes binary glTF to `out_path` and returns the same kind of summary and link. |

### Elements and levels as the export writes them

The lists, counts and levels are the elements the IFC export writes, in the
storey it puts them in (`rvt::ifc::step_writer::placements`, which the writer
writes with). A part of a stair, a railing or a curtain wall has the level of
its whole, an element in a room the level of the room, and an opening the
level of its host, as the IFC's containment and aggregation say. A further
piece of an element of separate loops, which the export writes as an item of
the owner's body and not as an element, is not a row and not in `total` or
`by_ifc_type`; the summary counts it by IFC entity in `folded_into_bodies`
(always present, empty where none is folded), and the diagnostics' `exported`
counts leave it out too, holding it apart in `exported.body_pieces_folded`
(the same counts). A part of several wholes takes the storey of the first,
which is not measured. The diagnostics' `storey_bound_elements` and the rooms
schedule (`room_rows`) read the same placement, so a part counts as bound to
its whole's storey in both.

### Names

A row's `name` is the `Name` the export writes (its `long_name` the `LongName`
of a space). The exporter names an element nothing named
`<Revit class>-<ElementId>`: a room with neither a number nor a name has that
as its `Name`, and the server leaves it out (`name` is absent), but the same
placeholder on any other element is still served as its `name` (a wall nothing
named is `Wall-<ElementId>`). On the models Measure runs, 23 of Einhoven's 37
rows, 3 of RE1 Electrical's 49 and 52 of the BIM4LCA office's 13,021 as built
carry one. The path where a row has no `name` is not measured: no row of the
32 models' dumps lacks one. `explain_element`'s `name` follows the same rule
(it was the model's name, placeholder included, before).

### Exports

An export goes to the path the caller names. The path must end in `.ifc` or
`.glb`, must not be the model being read, must not exist unless `overwrite` is
true, and must be inside `--output-dir` when the server is started with one (a
relative path is taken from it, otherwise from the server's working directory).
The file is written to a uniquely named temporary file in the target's folder,
created exclusively (so a link planted at a guessable name is not followed, and
two exports to one target do not share it), and put in place when whole: with
`overwrite` it replaces the target, without it it is linked there and fails if the
target exists, so a file that appeared meanwhile is never replaced. A failed or
cancelled export leaves no truncated file under the target's name and removes its
temporary file. On a file system that cannot link files, an export without
`overwrite` fails with the system's error; pass `overwrite` there. The export is
an ordinary file (mode 0644 on Unix).

The IFC is the file `rvt-ifc` writes from the same model with the same switches,
byte for byte (the end-to-end test compares them), and the glb is `rvt-gltf`'s.
`SOURCE_DATE_EPOCH` in the server's environment fixes the time in the IFC
header, as for `rvt-ifc`. The result never carries the file: it has the
entity counts, the diagnostics (IFC), the unread streams, the notes the
decode made, the size and the SHA-256 of the file, and a `resource_link`
(`file://`) to it.

If a resource limit kept part of the file from being read, an export refuses
(`rvt-ifc` exits 3) unless `allow_partial` is true, and the result lists the
unread streams either way.

## Provenance

Every element in an answer carries a `provenance` object. These fields exist,
each present only where the model has it:

| Field | What it says |
|---|---|
| `element_id` | The Revit ElementId, which the IFC writes as the `Tag`. |
| `global_id` | The GlobalId the IFC export writes: Revit's own where the file yields it, otherwise one derived from the document's identity and the ElementId. |
| `category` | The IFC entity the export writes (`IFCWALL`). |
| `class` | The Revit class the element's record names (`Wall`). |
| `body_source` | How the element's body was derived. `partition_element_record_bbox` means only the element record's bounding box: placed and sized, shape not decoded. |
| `profile_source` | Where the plan profile of the body came from. |
| `space_outline_source` | For a room or space: `stored_solid`, `stored_solid_faces`, `stored_solid_approximate`, `record_box`, `saved_mesh`, `family_body`, and for an Area `area_boundary`, `area_boundary_joined` or `none`. |
| `area_body_refusal` | For an Area with no body, why. |
| `carrier` | The kind of record the element was read from: `partition_element_record_first_prologue` (a frame that starts its partition record, the ElementId at its first bytes), `partition_element_record_second_prologue` (a frame inside its element's partition record, RE-30 and RE-35), `global_latest_schema_walk`, `partition_arc_wall`, `partition_area_record`, `partition_derived` (a row with no frame of its own, such as a material) or `other`. |
| `stream` | The stream the record is in: `Partitions/NN`, or `Global/Latest` for the schema walk. |
| `offset` | Where the record or frame starts in that stream inflated: the bytes `rvt-dump` writes as `Partitions_NN.decomp`, so the offset indexes that file directly (#291). It is the frame the exporter selected among the element's frames, not necessarily the copy a body came from (see below). For the schema walk it indexes `Global/Latest` as the walk inflates it, which is unmeasured. |
| `record_offset` | Where the ElementId is stored: the partition record header whose first bytes (8 from Revit 2024, 4 before) are the id. The frame's own `offset` for a first-prologue frame, the enclosing record's start for a second-prologue one. Absent for the schema walk, whose id is a field of the instance. |
| `enclosing_record_id` | For a second-prologue frame, the ElementId of the partition record that encloses it, which is the element's (RE-35). |
| `class_tag`, `class_name` | The class tag the record names (for an element record the `u16` at `+0x4a`) and the class it resolves to in the file's own `Formats/Latest`. `class` is the Revit class the export names the element by. |
| `native_record` | The element's native record, where the export's native passes indexed one: `copies` (in how many partitions it has one), `copy` (`only`, `current` of several, or `unresolved`, RE-145 and RE-216), `stream`, `offset` (where the record header starts in that partition inflated; the id is its first bytes) and `class_name`. |

`get_element` and the summary add the file's: the Revit release and build from
`BasicFileInfo`, the number of classes in the embedded schema, the number of
`Partitions/NN` streams, the release whose record layout the file takes
(`layout_release`) and the record layout of `Global/ElemTable`. `get_element`
also lists every `...Source`, `...Resolved` and `...Refusal` property of the
element's record as `trust_markers`.

The record fields come from the element provenance table (#1325), which the
export builds only when asked (`rvt-ifc --element-provenance`) and which the
server asks for in the decode it makes when it opens a model. It is large: its size
on the models measured is in
[Provenance table size](export-diagnostics.md#provenance-table-size). The server
keeps it in memory beside the model and does not return it: `get_diagnostics`
and the exports' summaries are the diagnostics without it, as `rvt-ifc` writes
them by default. Building it is one pass over the walk the decode already made
and a re-read of the first bytes of each partition record. Its time on each
model is printed by `tests/element_provenance_offsets.rs` (the `build` line of
its report, Corpus health's `element-provenance` artifact); no figure is stated
here yet. The fields are never written into the IFC.

When an element has several partition element record frames, `offset` is the
one the exporter selected. On Revit 2024 and later
(`select_instance_records`) that is the frame with the greatest vertical extent
and, on a tie, the higher-numbered partition and then the later offset;
on Revit 2008 to 2023 (`select_newest_instance_records`, RE-107) it is the
newest, the highest-numbered partition and then the latest offset, with no extent
test. A body reader may take another copy (the sketch-line readers take the
current one, `current_partitions`, RE-216), and `native_record.stream` names the
current native copy. So `offset` and `native_record.stream` can name different
partitions. `get_element` and `explain_element` list, in `value_sources`, the
copy each reader took a value of the body from (its reader, the rule that chose
the copy, its stream and offset, and whether it is in another partition than the
frame; #1360, #1368): the bounded line of a wall or beam, a family instance's
transform, a saved mesh, the sketch lines and curve list a floor's or roof's
profile is solved from, a column's type symbol, a stair run's lines, ends and
type, a landing's outline lines and native record. A source made of several
records lists them in `parts`, at most 64, with the rest counted in
`parts_left_out`. `list_elements` does not. A
body whose reader does not note its copy yet has none; see
`docs/export-diagnostics.md` for the readers.

What the offsets have been checked on: Core Interior, the RE1 models, Einhoven
and the BIM4LCA as-built office, by reading the bytes at each offset
(`tests/element_provenance_offsets.rs`). Einhoven (Revit 2023) confirmed 13 of 13
first-prologue frames and 71 of 71 native records. Not measured: Revit 2008 to
2022, and the second-prologue path of 2023. The schema-walk carrier
(`global_latest_schema_walk`) has no element on any of those models, so its
offsets have not been checked: the walk inflates `Global/Latest` with
`inflate_stream_auto` and `rvt-dump` with its own routine, and the two are not
compared.

The table lists at most 250,000 elements. An element past that has no record
fields and says `provenance_omitted: true`; the file's `omitted_entries` counts
them.

### Decisions

`explain_element` answers from the log of per-element decisions (#1326) that the
server asks the export for when it opens a model, as it asks for the provenance
table. The log is bounded: at most 5,000 entries a reason, in ElementId order,
and `capped_reasons` names the reasons that dropped some, whose decisions about
the element may be missing. The `skipped` counts `get_diagnostics` returns are
made from the same decisions, and, like the table, the log is not part of
`get_diagnostics`. A left-out element has decisions and no provenance
(`exported: false`); a frame with no ElementId has no entry. The values Revit's
exporters disagree about are left out by a rule for a whole class and have no
per-element decision to return. A decision's `stream` and `offset` for an
element-record reason are the element's last frame in scan order (partition
number, then offset), which can differ from the frame `provenance` names; only
those frames on Revit 2024 and later have been re-read from the bytes. The log
holds at most 5,000 entries a reason
whatever the model; its size in JSON is printed per model by
`tests/export_decisions_counts.rs` (Corpus health's `export-decisions`
artifact), and no figure for it, or for the time it adds to a decode, is stated
here yet.

### Not provided yet

- A Measure scorer for the server's answers (#1327).
- Lookup by the Revit Mark parameter: nothing in rvt-rs decodes it.
- The Tasks extension (`io.modelcontextprotocol/tasks`). A long export is one
  call with progress and cancellation; a client moves a call that runs long to
  the background by itself.

## What is not read

The server answers from what rvt-rs decodes, which is a subset of a Revit file.
[`status.md`](status.md) states what is read per Revit release and category; the
summary's `not_read` and every tool description say the short version:

- An element's body is often only its record's bounding box (`body_source`
  says which). Family files give metadata, previews and types but no family
  geometry.
- An element's property sets hold the parameters the export writes. Revit's own
  parameter sets are written only with `export_ifc`'s `revit_property_sets`.
- Files saved by Revit 2015 to 2022, 2026 and 2027 are experimental, and 2008 to
  2014 more so; the summary's `release_support` says how a file's release is
  read.
- `get_diagnostics` and the summary's `diagnostics` list what the export
  skipped or approximated, and `unread_streams` what a resource limit kept
  unread.

## Limits

The resource limits are `rvt-ifc`'s, with the same flags: `--large-model`,
`--max-stream-bytes`, `--max-inflate-bytes` and the five `--max-walker-*`
limits (`rvt-rs-mcp --help`). A stream the limits keep from being read is
reported, never read partially without saying so. A model's decode peaks at
several GB for the largest files, which is why only the two most recently used
models are kept by default.

## Privacy

Nothing leaves the machine. The server's dependency tree carries no HTTP client
or server and no TLS stack, and tokio's `net` feature (mio, socket2) is off: the
server serves stdin and stdout through tokio's `io-std` and rmcp's
`transport-async-rw`, not rmcp's `transport-io`, which turns `net` on
(`tools/ci/audit-mcp-network.sh` fails the build if any of these appears). The
end-to-end test asserts, on Linux, that the running server holds no TCP, UDP or
raw socket (the test build links tokio's `process` feature for its client, whose
signal driver opens a local socketpair; the shipped binary has neither the
feature nor the socketpair). What the client does with the answers is the client's
to say: an answer can name the file, the project, its levels and its elements.
The model's path appears in no successful answer (the `model_id` is a hash and
the diagnostics carry the file name only); an export's result names the path
the caller gave.

## Checks

`rvt-mcp/tests/mcp_stdio.rs` starts the built binary and drives it over real
stdio with rmcp's client in both lifecycles (`server/discover` at 2026-07-28,
`initialize` at 2025-11-25) on the committed synthetic fixture under
`corpus/tier1`, and on Einhoven when `RVT_PROJECT_CORPUS_DIR` is set. It
compares the exports with `rvt-ifc` and `rvt-gltf` byte for byte, looks an
element's GlobalId up in the IFC the export wrote, checks that bad arguments are
tool errors, the unsupported version error (-32022), progress notifications,
that cancelling an export stops it with nothing written or left over (on
Einhoven the cancellation always lands during the decode), that stdout carries
nothing but JSON, and that the server exits cleanly when stdin closes (after a
session, after only `server/discover` or `initialize`, and at once). Another test holds
`mcp-tools.json` to what the server reports for `tools/list`. CI runs it in the
test matrix and again in its "MCP server on Einhoven" job, where Einhoven runs
in both lifecycles. That job also keeps every tool's `outputSchema` and every
structured answer the test is given, through rmcp's client or a raw JSON-RPC
request (`RVT_MCP_ANSWERS_DIR`), and `tools/ci/validate-mcp-answers.py --require-all` checks each
answer against its tool's schema, failing too when a tool gave no answer:
a field the schema requires and an answer leaves out fails CI, as it would fail
a client that validates `structuredContent` (#1415). The answers and schemas are
the job's `mcp-answers` artifact.
