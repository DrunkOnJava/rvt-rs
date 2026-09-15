# Native world-model projection boundaries

The native world-model command combines source-qualified projections. Each
projection has its own coverage and refusal diagnostics; one successful
projection does not establish completeness of another.

## Connector networks

Connector inventories preserve owner and port identities, directed edge kinds,
raw target mode, resolution state, reciprocity witnesses, modifiers, and
manager deletion data. Physical cross-owner edges, same-owner internal edges,
logical system references, and unresolved saved references remain distinct. A
logical reference or saved connector edge is not automatically a canonical
`feeds`, `flows_to`, or `connected_to` relationship. World-frame alignment is
unregistered unless separately qualified.

The producer implementation is in
[`native_network.rs`](../src/native_network.rs) and
[`native_network_document.rs`](../src/native_network_document.rs). The native
parser does not load external link documents or apply manufacturer overrides.

## Saved graphics

Channel-103 saved graphics are separate from operation-derived geometry. The
bounded path preserves current-owner and shared-symbol provenance, planar
trimmed faces, faceted meshes, supplied normals, transforms, materials, and
diagnostics for open loops, malformed chains, unsupported surfaces, and missing
owners. A decoded physical record does not establish current liveness, complete
graphics selection, or universal Revit geometry parity.

The implementation is in [`native_saved_mesh.rs`](../src/native_saved_mesh.rs)
and [`native_graphics_traversal.rs`](../src/native_graphics_traversal.rs).
Unsupported geometry remains explicit rather than being replaced with a box,
nearest surface, or fabricated topology.

The saved graphics path also has a bounded analytic `CylSurf` profile. A
qualified cylindrical surface preserves its analytic center, radius, basis,
orientation, source trim, and provenance while emitting a chord-error-bounded
mesh. The current accepted trim is a single rectangular UV region with the
required loop and order checks. Nonfinite values, invalid bases, unsupported
holes or seams, and unrepresentable tessellation budgets are refused. This does
not claim arbitrary trimmed-surface support or exact saved-facet parity. The
GLB encoder requires nonempty primitive arrays, valid indices, and finite unit
normals; the cylinder regression additionally checks nondegenerate cells.
Face-tag absence remains an unresolved identity issue in the current saved-mesh
path and is not treated as a refusal here.
Multiple saved loops are refused until their outer/inner roles and support are
qualified. Graphics inventories keep omission counters and status for
intentional visibility/non-surface filtering separate from malformed or
unsupported geometry, so filtered records are not counted as parser failures.

The combined world-model report retains source hashes, resource budgets,
successful cursor maxima, refused owner IDs/classes, and per-projection status.
Partial or unsupported projections do not disappear from the report and do not
permit a complete-world-model claim.
