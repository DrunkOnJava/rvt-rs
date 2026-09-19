# Native projection status

Native projections retain the source ownership and serialized location needed
to explain what was read. A successful graph decode does not establish
semantic or API parity.

Projected records may retain the owner identity, stream and continuation group,
record offset, body size, graph object index, field path, projection status, and
refusal diagnostics. Missing, unsupported, failed, and resource-limited records
remain explicit; they are not converted to empty values or silently dropped.

A repeated native identity is an observation within its saved namespace. It is
not proof of a cross-document world entity or shared lineage. Partial reports
may preserve successful records while reporting the unsupported remainder.

The projection builders are implemented in
[`native_document.rs`](../src/native_document.rs),
[`native_spatial_boundaries.rs`](../src/native_spatial_boundaries.rs), and
[`native_spatial_context.rs`](../src/native_spatial_context.rs).
