# Native format reference

This is a portable implementation reference for the Revit native format. It
collects version-qualified rules that guide `rvt-rs` maintenance.

The rules cover multiple file families and releases. They are not an Autodesk
normative specification. A structural rule establishes how a particular stream
or record is framed; it does not establish that every class, field, or semantic
API object has the same encoding in every release.

The runtime keeps three statements separate:

1. bytes were read and structurally decoded;
2. a native object graph or field was projected with its source location;
3. an evaluated semantic result was qualified for a supported profile.

The first does not imply the second, and the second does not imply the third.
See [native projection status](../native-provenance.md) for the common status
and source-location contract.

## Outer container

Modern RVT, RFA, RTE, and RFT files use Microsoft Compound File Binary Format
(CFB/OLE Structured Storage). CFB handles the FAT, DIFAT, MiniFAT, directory,
sector, and stream boundaries. Revit-specific decoding begins after those
boundaries have been validated.

The CFB layer is comparatively stable across supported releases. Inner
stream contents, schema tags, partition organization, and application-level
serialization evolve independently. A parser must therefore record the CFB
version and sector layout but must not use a CFB version as a complete Revit
format version gate.

Known stream names include `Formats/Latest`, `Global/Latest`,
`Global/ContentDocuments`, `Global/ElemTable`, and `Partitions/*`. Stream
presence is informative, not sufficient evidence that the stream is complete
or semantically understood.

## Compression and framing

Several Revit streams contain concatenated DEFLATE/gzip-like members. A member
scanner must stop at the actual DEFLATE end and continue at the next Revit
framing boundary. Treating one stream as one gzip member can truncate later
records or consume framing bytes as compressed data.

Some streams carry page trailers associated with storage integrity. The
page layout is stream-sensitive. Preparation must be named and
explicit: retain the stored bytes, retain the prepared bytes, record which
pages were removed, and refuse unsupported layouts. A generic “strip the last
N bytes” helper is unsafe.

`Formats/Latest` is specifically excluded from the current generic trailer
stripping rule because stripping can reduce recovered schema content.
The runtime does not claim to implement a universal error-correction or
whole-file checksum algorithm. CFB transaction signatures do not establish
semantic completeness.

The implementation entry points are the existing compression and stream
readers. Their diagnostics should preserve stream name, member offsets,
prepared length and opaque suffix length.

## Per-file schema

`Formats/Latest` behaves as a per-file serialization schema. It contains class
names, inheritance or parent information, serialization tags, field names, and
compact field-type descriptors. Class tags are not stable across releases;
they must be resolved against the schema in the current file rather than a
global numeric table.

Supported field categories include signed and unsigned scalar values, strings,
GUID-like values, element identifiers, pointers/references, vectors, arrays,
maps, and nested records. The schema vocabulary is substantially understood
for supported profiles, but a classified field type does not prove that every
instance value or enclosing record boundary is known.

Schema-directed traversal is required. Numeric class tags can occur inside
references and payloads as well as at object starts, so frequency scans or
“find the next known tag” heuristics cannot safely enumerate objects.

See the native capability boundaries in
[`native-world-model-research.md`](../native-world-model-research.md) and the implementation modules
[`schema_registry.rs`](../../src/schema_registry.rs),
[`native_parameters.rs`](../../src/native_parameters.rs), and
[`native_document.rs`](../../src/native_document.rs).

### Concrete scalar and descriptor rules

The schema field descriptor is read as a little-endian `u32`: the low byte is
the base discriminator, the next byte is the modifier, and the upper two bytes
are retained as uninterpreted flags. The current registry accepts only
supported base/modifier combinations, bounds recursive depth and field count,
and refuses trailing or unsupported descriptor bytes.

The qualified scalar widths include:

| Base | Stored value in the qualified reader |
| ---: | --- |
| `1` | one-byte boolean, restricted to `0` or `1` |
| `2` | one-byte unsigned scalar |
| `3` | signed little-endian `i16` |
| `4` | signed little-endian `i32` |
| `5` | little-endian `u32` |
| `6` | finite little-endian `f32` |
| `7` | finite little-endian `f64` |
| `9` | 16 raw GUID bytes |
| `11` | signed little-endian `i64` |
| `13` | nested schema field |
| `14` | schema class/reference field |

The `0x10–0x13` and `0x50–0x53` modifiers represent bounded fixed or dynamic
arrays of a scalar/nested field. Reference-container `0x0E` carries a schema
reference rather than an ordinary scalar body. Array counts are bounded by the
reader and are rejected when absent, zero where disallowed, or above the
implementation budget. These are reader guards, not claimed limits of the
RVT format.

Two supported special cases are deliberately narrow. A `ClassDefinitionRef`
`m_ref` with base `10` stores a 16-bit class tag resolved through the file-local
registry. A `VarSketchObj.m_params` owning-pointer vector uses a qualified
vector interpretation because each entry carries its concrete class tag.
These cases must not be generalized to every base-10 or owning-pointer field.

The value decoder is implemented in
[`native_parameters.rs`](../../src/native_parameters.rs), while descriptor
validation and nested schema references are implemented in
[`schema_registry.rs`](../../src/schema_registry.rs).

### Deferred objects and bounded EOF

The graph reader walks an inherited class header and then a deferred-object
queue in encounter order. A fresh positive token introduces a queued object;
repeated positive tokens refer to an object already seen. Null and external
pointers remain serialized field values without local graph edges. A pointer
modifier that is not qualified for the owning field is refused rather than
followed heuristically.

Successful graph extraction requires the bounded cursor to consume the complete
record body. The result retains `consumed_bytes`, object indexes, pointer
offsets, target indexes, and graph resource usage. Object and value budgets,
per-container counts, string lengths, and recursion depth are independent
implementation safeguards. A complete bounded EOF is structural evidence; it
does not mean that every referenced semantic object or geometry representation
was recovered.

### Current ElemTable profiles

`Global/ElemTable` has multiple supported record families. The supported forms
include approximately 12-byte family records, 28-byte Revit 2023 project
records, and 40-byte Revit 2024 project records. These are profiles, not one
portable record grammar.

The supported 2027 `ElemRec` array begins at the schema-derived array
origin. In that profile, a 40-byte row has:

| Offset | Width | Observed field |
| ---: | ---: | --- |
| `+0` | `u64` | original ID suffix |
| `+8` | `u32` | creation episode |
| `+12` | `u32` | last modification episode |
| `+16` | `u32` | last user |
| `+20` | `u64` | current ElementId |
| `+28` | `i64` | owner |
| `+36` | `u32` | partition |

The 2024 40-byte profile also has a `u32` at `+0x1c` used as a monotone version
discriminator. Its use is gated to that profile. Counts and candidate frames
are validation constraints; they are not permission to apply these offsets to
another document kind or release.

The native index implementation is in
[`native_index.rs`](../../src/native_index.rs). It keeps current selection,
creation episodes, storage increments, and duplicate candidates distinct.

## Object graphs and identity

Native graph records contain an owner identity, class information, fields, and
pointer edges. A pointer tuple may encode more than a simple ElementId; its
partition, address, generation, or indirection meaning is not universal.
Graph object indexes and serialized pointer offsets remain available when a
field is projected.

`ElemTable`, `Global/ContentDocuments`, and current graph records represent
different identity and namespace layers. A physical candidate is selected only
when explicit current table membership and ownership evidence agree. Embedded
IDs must not be assigned a main-document UniqueId, and duplicate candidates
must remain unresolved until a qualified discriminator exists.

The narrow partition element-record profiles are version and profile gated.
They are not a general replacement for schema-directed graph decoding.

## Parameters and units

Revit parameters may be built-in, family-defined, project-defined, or shared.
Supported API storage categories include strings, integers, doubles, and
ElementId-valued references. Modern files also retain spec, unit, and parameter
group identifiers in supported profiles.

Raw numeric values must not receive guessed units. Internal length values are
commonly feet at the API boundary, but a native value is unit-qualified only
after its field and spec context are resolved. Unknown specs remain unknown.

Shared-parameter GUIDs are semantic API keys. The qualified profile preserves
their saved GUID byte layout and does not confuse a GUID-looking string with a
definition. Complete binding semantics remain version/profile gated.

Parameter definitions, bindings, values, and source ownership remain separate
observations. A positive definition graph is accepted only when its
`m_paramElemId` agrees with the projected parameter ID. Concrete definition
classes select the family-union storage slot; captions and nonzero slots do not.
Project bindings follow `ParamElemExternal.m_bindingIds` to `ParamBinding` and
verify the reverse parameter reference, category, and instance/type flag. A
missing value is classified as unset only when an applicable binding is
validated; empty strings and zeroes remain values.

The qualified unit path reads project formats from
`UnitsElem.m_units.m_formatOptionsMap`, while raw numeric values remain
unchanged. A static built-in definition catalog is accepted only when its file
schema hash matches the supported profile; it supplies definitions, not owner
values, `HasValue`, or read-only state. These rules are implemented in
[`native_metadata.rs`](../../src/native_metadata.rs) and
[`native_parameter_definitions.rs`](../../src/native_parameter_definitions.rs).

## Extensible Storage

Extensible Storage exposes GUID-identified schemas and typed entities through
the Revit API. The bounded native reader handles two catalog wrappers:
2023 `m_storedSchemas` and 2027 `m_schemaUsageMap`, retaining host or content
usage metadata where present. A catalog entry retains its canonical GUID, name,
field index, field name, type name, container type, subschema GUID, spec type
ID, and raw metadata.

Catalog validation rejects duplicate schema GUIDs, duplicate field names,
duplicate field indices, and noncontiguous indices. Field order follows the
saved entry index; it is not inferred from declaration or alphabetic order.

An `ESEntity` blob uses a schema GUID and discriminator. Discriminator `0`
represents a serialized-absent entity. The supported present form uses
discriminator `-1`, requires the payload schema GUID to match the owner map,
and resolves one owning edge to an `ExtensibleStorageEntity` dynamic object.
Dynamic ES objects use their schema identity rather than a fabricated
`Formats/Latest` class identity.

The typed payload projection covers qualified scalar values, signed 64-bit
values, arrays, typed map keys, nested entities, and bounded recursive field
order. Present default-valued entities remain distinct from absent entities.
Each entity retains owner identity, schema GUID, cell/payload object indexes,
payload edges, and source provenance. The measured ES length spec stores
millimeters independently of document display units; no conversion is applied
to an unqualified spec.

ES references remain distinct from BIM topology and ElemTable ownership.
They must not be emitted as default IFC relationships or canonical host edges
without an ownership or schema gate. Unsupported schema wrappers, payload
discriminators, field types, and resource limits are reported explicitly rather
than omitted from a completeness calculation. The implementation is in
[`native_es_catalog.rs`](../../src/native_es_catalog.rs),
[`native_extensible_storage.rs`](../../src/native_extensible_storage.rs), and
the catalog-aware graph path in [`native_parameters.rs`](../../src/native_parameters.rs).

## Geometry and coordinate frames

The native schema contains geometry-related classes, transforms, curves, faces,
meshes, and material references, but generic B-rep reconstruction remains
partial and version-gated. Saved graphics and operation-derived geometry are
separate projections.

Coordinate results must carry frame and unit claims. Document-internal feet and
Z-up are not interchangeable with family-local coordinates, shared coordinates,
or exported glTF conventions. Matrix orientation, translation, reflection, and
normal transformation require explicit tests.

The saved planar graphics route is documented in
[`native-world-model-projections.md`](../native-world-model-projections.md). The
projection preserves source ownership, loop diagnostics, supplied normals,
materials, and transforms; unsupported or malformed geometry is not replaced
with a bounding box or fabricated mesh.

### Observed non-planar surface records

The current typed evaluator in
[`native_parametric_surface.rs`](../../src/native_parametric_surface.rs) reads
`SurfRev` and `RuledSurf` only after resolving their profile pointers through
the graph edge table. A pointer token is an identifier, not an object index;
repeated references must retain their token, field offset, target class, and
resolved object index. The shared resolver in
[`native_saved_mesh.rs`](../../src/native_saved_mesh.rs) rejects mismatched,
ambiguous, or out-of-range edge metadata before using its uniquely identified
token fallback.

For the observed `SurfRev` records, let the referenced profile evaluation be
`q(v) = [q_x(v), q_y(v), q_z(v)]`. The evaluator uses the serialized center and
orthonormal basis as:

```text
P(u,v) = center + x * (q_x(v) cos(u) - q_y(v) sin(u))
              + y * (q_x(v) sin(u) + q_y(v) cos(u)) + z * q_z(v)
```

The observed meridian profiles have `q_y(v)=0`, which reduces this to the
usual radius/axial form; the full local two-component rotation remains the
typed contract.

`u` and `v` are accepted only inside the ordered finite envelope
`[[u0,v0],[u1,v1]]`. The profile is currently a serialized `GLine` or `GArc`;
other profile classes are refused. Basis vectors must be finite, unit length,
and mutually orthogonal within `1e-6`. The orientation flag changes the
normal direction; it does not change the point equation. At a rotational pole,
the normal path has a bounded limiting-tangent branch and refuses a singular
profile instead of inventing a normal.

For the observed `RuledSurf` records, each profile has its own parameter range.
With `t_i(u) = t_i0 + u (t_i1 - t_i0)`, the normalized-`u` evaluator is:

```text
P(u,v) = (1 - v) * profile1(t_1(u)) + v * profile2(t_2(u))
```

The envelope and orientation checks are the same. `m_Point1` and `m_Point2`
are retained raw but are not substituted for the profile pointers. Both
profile references must resolve through the graph contract.

Face trimming remains a separate concern. The bounded parametric path in
[`native_saved_mesh.rs`](../../src/native_saved_mesh.rs) currently accepts one
closed rectangular UV loop inside the analytic envelope, including explicit
seam handling, collapsed rotational poles, and shared-edge knot refinement.
The implementation is split between [`native_parametric_surface.rs`](../../src/native_parametric_surface.rs)
and [`native_parametric_mesh.rs`](../../src/native_parametric_mesh.rs).
The current grid mesher supports the observed line/arc `SurfRev` profiles and
line-only `RuledSurf` components, with component consistency checks across
shared face edges. It refuses holes, multiple loops, non-rectangular regions,
degenerate/repeated trim vertices, unsupported profile curves, and
out-of-envelope points. The trim loop guard accepts the observed
`EdgeLoop` and `EdgeLoopWithChainEnvelopes` classes only when their owner,
closure, and edge-chain references validate. A valid analytic surface record still does not imply
that every saved face can be meshed.

The observed empty-trim diagnostic is narrower still. A saved channel-103
`Face` is recognized only when its `m_GInfo.m_flags` is `0x8204`,
`m_faceFlags_v9` is `6`, regions are empty, render style is the serialized
`-1` identifier, trim/fill pointers are null, the surface is a finite ordered
Plane envelope, and any resolving `Edge.m_pFace` records are retired: both
adjacency sides are null and no `Edge`/`EdgeLoop` chain reaches them. Active or
malformed edge references fail closed. The diagnostic counter preserves the
raw face and provenance; it is not a general free-plane or removable-face
interpretation. See
[`native_empty_faces.rs`](../../src/native_empty_faces.rs).

## Materials and appearance

Material names, layer references, appearance assets, paint, texture placement,
and shader parameters occur in distinct native structures. A material name or
layer record does not establish rendered appearance parity. Host material
quantity calculations may use exporter-specific approximations and should not
be conflated with closed-shell volume.

Appearance, texture, and material reports must preserve source units and
dependency edges. Unknown assets, unsupported placers, and incomplete face
bindings remain diagnostics. See [`native-world-model-projections.md`](../native-world-model-projections.md)
for the saved-graphics boundary and the appearance capability boundaries in
[`native-world-model-research.md`](../native-world-model-research.md).

## Version strategy and safe pipeline

The safe reader pipeline is:

1. validate CFB structure;
2. identify and record streams without assuming semantic completeness;
3. prepare only stream layouts with an explicit framing rule;
4. scan concatenated members with bounded resource limits;
5. parse the current file's schema registry;
6. walk objects through schema-directed edges;
7. project typed values with source locations and refusal diagnostics.

Unsupported Revit years, unknown schema transitions, resource exhaustion, and
ambiguous identity candidates fail closed. A successful low-level parse may be
returned as a partial structural inventory, but it must not be labelled complete
semantic or geometry parity.

The outer container changes more slowly than the inner schema. Around the
observed 2021 transition, schema and stream organization changed materially;
later files also introduce new parameter-group, element-record, and graphics
profiles. Version gates should be attached to the smallest qualified reader
path, not used to imply that all projections for that year are supported.

## Scope boundary

This reference does not promise a universal native reader, converter-grade IFC,
complete evaluated geometry, complete domain semantics, cross-document identity,
or lossless native writing. It provides durable rules for preserving evidence,
refusing unsupported interpretations, and extending a projection only after a
version-qualified structural and semantic gate.
