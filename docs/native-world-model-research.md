# Native world-model capability

The native saved-record/world-model path runs alongside the legacy
walker/exporter. It provides opt-in, source-qualified projections with bounded
resource use and explicit refusal states. It is not a universal typed-model
reader or a drop-in IFC converter.

## Entry points

The public commands are [`rvt-native-document`](../src/bin/rvt_native_document.rs),
[`rvt-native-scene`](../src/bin/rvt_native_scene.rs),
[`rvt-native-world-model`](../src/bin/rvt_native_world_model.rs), and
[`rvt-native-saved-scene`](../src/bin/rvt_native_saved_scene.rs). They read
caller-provided RVT files and write bounded JSON or GLB outputs. Unsupported
versions, records, relationships, and geometry remain diagnostics; they are
not converted into fabricated objects or placeholder geometry.

## Projection boundaries

Saved spatial boundaries preserve the serialized room, level, circuit, segment,
and link relationships that are present in a file. A saved loop is not thereby
an evaluated three-dimensional boundary or canonical containment relation.
Transforms preserve their serialized basis, origin, frame, and units. Linked
placement refers to another document and does not import that document.

Native connector projections preserve owner and port identities, edge kinds,
target modes, resolution state, and unresolved references. A saved connector
does not automatically become a semantic `feeds`, `flows_to`, or
`connected_to` relationship.

Saved graphics remain distinct from operation-derived geometry. Supported
profiles can preserve trimmed faces, meshes, normals, transforms, materials,
and analytic surface parameters. Unsupported loops, seams, surfaces, cuts,
attachments, slanted or variable layers remain explicit refusals. Multiple
saved loops and complete rendered-image parity remain unsupported.

Parameter definitions, values, units, Extensible Storage entities, lifecycle
state, and revision differences remain separate projections. Unknown units,
ambiguous ownership, missing links, and incomplete graphs are retained as
diagnostics. A successful structural read does not establish complete semantic
coverage, cross-document identity, visibility parity, or render parity.

We have generated and analyzed many Revit files to discern the file structure and variation.
