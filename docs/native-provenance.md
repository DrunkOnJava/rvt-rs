# Native projection provenance

Native projections are source observations over the RVT bytes. A successful
graph decode does not by itself establish semantic or API parity.

Every projected record should retain, where available:

- the owner ElementId and native UniqueId;
- source stream and continuation-group identity;
- record offset and body byte count;
- record body hash;
- graph object index and serialized field path;
- projection status and refusal diagnostics;
- the source RVT hash before and after extraction.

Missing, unsupported, failed, and resource-limited records remain explicit.
They are not converted to empty values or silently dropped. A source hash
change invalidates a reproducibility claim. A repeated native UniqueId is a
saved-record identity observation within its source namespace; it is not proof
of a cross-document world entity or shared lineage.

This contract is implemented by the native record reader and the projection
builders, including [`native_document.rs`](../src/native_document.rs),
[`native_spatial_boundaries.rs`](../src/native_spatial_boundaries.rs), and
[`native_spatial_context.rs`](../src/native_spatial_context.rs). Projection
reports may be partial while preserving the successful records and explicit
diagnostics.
