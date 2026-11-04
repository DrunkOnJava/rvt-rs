# rvt-rs-mcp

A local, read-only [Model Context Protocol](https://modelcontextprotocol.io)
server for Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`), over stdio. It
links the [`rvt`](https://crates.io/crates/rvt) crate in process, so an agent
can ask about a Revit file with no Revit install and no network: it opens no
socket, makes no network call, and writes only the export files a tool is asked
for.

```bash
cargo install rvt-rs-mcp --locked
claude mcp add rvt -- rvt-rs-mcp
```

Tools: `open_model`, `get_summary`, `list_elements`, `get_element`,
`list_rooms`, `list_levels`, `get_diagnostics`, `export_ifc`, `export_gltf`.
Every answer comes from the model the IFC exporter builds, so a tool answer and
the IFC export cannot disagree, and each element carries where its answer comes
from. The server reports what it did not read, the way the `rvt` CLIs do.

Needs Rust 1.88 or later to build (the MCP SDK `rmcp` 3.x declares it; the
`rvt` crate itself needs 1.87). The tools, the handle lifetime, the provenance
fields, the limits and what is not read are in
[docs/mcp.md](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/mcp.md).

Apache-2.0. Not affiliated with Autodesk.
