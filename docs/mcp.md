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
  an answer and the IFC cannot disagree.
- **Honest about gaps.** It reports what it left out the way the CLIs do:
  unread streams, skipped items, diagnostics. A field the model does not have is
  absent from an answer, never guessed.

It is new. It is not in v0.13.0; it ships from the next release (see
[Install](#install)).

## Install

From a checkout of `main` (needs Rust 1.88 or later, one above the other
tools, because the MCP SDK `rmcp` 3.x declares 1.88):

```bash
cargo install --path rvt-mcp --locked
rvt-rs-mcp --version
```

From the next release, the prebuilt archives on
[GitHub Releases](https://github.com/DrunkOnJava/rvt-rs/releases)
(`rvt-rs-mcp` sits beside the CLIs) and the container image
`ghcr.io/drunkonjava/rvt-rs` carry it, and `cargo install rvt-rs-mcp --locked`
installs it once the crate's first version is on crates.io (RELEASE.md,
step 8). In the image the model's folder is mounted at `/data`:

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
| `list_elements` | Building elements, filtered by `category` (IFC entity: `IFCWALL`, `IfcWall` and `wall` are the same), `class` (Revit class: `Wall`, `Room`) and `level` (storey name), paged by `limit` (100 by default, at most 1000) and `offset`. Each row has the ElementId, name, level, `predefined_type` and its provenance. |
| `get_element` | One element by Revit ElementId or IFC GlobalId: its property sets and quantities as the IFC export writes them (quantities in the IFC's SI units, each property with its IfcValue type), its trust markers, its provenance, the file's, its type object's sets and any other entity with the same ElementId. |
| `list_rooms` | Rooms and spaces: ElementId, number, name, level, area and provenance. The area is present only where the outline was read exactly. |
| `list_levels` | Storeys with elevation (feet and metres) and element count. |
| `get_diagnostics` | The export diagnostics, as `rvt-ifc --diagnostics` writes them ([`export-diagnostics.schema.json`](schemas/export-diagnostics.schema.json)): what was decoded, exported, skipped, approximated and left unread, and why. |
| `export_ifc` | Writes IFC4 to `out_path` and returns a summary and a `resource_link` to the file. `options` takes `rvt-ifc`'s switches: `mode`, `saved_meshes` (`doors_and_windows`, `all`, `off`), `revit_property_sets`, `revit_set_placement`, `no_base_quantities`, `allow_partial`. |
| `export_gltf` | Writes binary glTF to `out_path` and returns the same kind of summary and link. |

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

Every element in an answer carries a `provenance` object. These fields exist
today, each present only where the model has it:

| Field | What it says |
|---|---|
| `element_id` | The Revit ElementId, which the IFC writes as the `Tag`. |
| `global_id` | The GlobalId the IFC export writes: Revit's own where the file yields it, otherwise one derived from the document's identity and the ElementId. |
| `category` | The IFC entity the export writes (`IFCWALL`). |
| `class` | The Revit class the element's record names (`Wall`). |
| `body_source` | How the element's body was derived. `partition_element_record_bbox` means only the element record's bounding box: placed and sized, shape not decoded. |
| `profile_source` | Where the plan profile of the body came from. |
| `space_outline_source` | For a room or space: `stored_solid`, `stored_solid_faces`, `stored_solid_approximate`, `record_box`, `saved_mesh`, `family_body`, and for an Area `area_boundary` or `none`. |
| `area_body_refusal` | For an Area with no body, why. |

`get_element` and the summary add the file's: the Revit release and build from
`BasicFileInfo`, the number of classes in the embedded schema, and the record
layout of `Global/ElemTable`. `get_element` also lists every `...Source`,
`...Resolved` and `...Refusal` property of the element's record as
`trust_markers`.

### Not provided yet

- Where an element's record sits (the stream and offset) and which carrier it
  was read from (#1325), and the per-element export decisions behind an
  `explain_element` tool (#1326). The provenance object leaves room for them;
  they are not faked.
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
test matrix and again in Corpus health, where Einhoven runs in both lifecycles.
