# Native world-model research checkpoint

The native path is separate from the legacy walker/exporter. Runtime
extraction uses Rust and the source RVT alone. API witnesses and commercial
parser binaries are research/validation inputs only. The research records
implementation-derived binary/debugger findings explicitly; it does not claim
these additions were exclusively clean-room work.

Portable projection contracts are documented separately:

- [native projection provenance](native-provenance.md) defines source hashes,
  record identity, field provenance, and explicit partial/refusal states;
- [saved spatial boundaries](native-spatial-boundaries.md) defines the
  carrier-versus-evaluated boundary distinction;
- [spatial context](native-spatial-context.md) defines transforms, references,
  and linked placement without inferred containment;
- [world-model projection boundaries](native-world-model-projections.md)
  defines connector edge kinds, saved graphics scope, and completeness limits.

These documents describe runtime-facing contracts. Private source files,
workspace inventories, binary probes, and staging receipts are not runtime
dependencies and are not part of the public reproducibility surface.

## Public-fork status (2026-09-14)

The native path is opt-in and parallel to the legacy walker/exporter. Use the
[`rvt-native-document`](../src/bin/rvt_native_document.rs),
[`rvt-native-scene`](../src/bin/rvt_native_scene.rs),
[`rvt-native-world-model`](../src/bin/rvt_native_world_model.rs), and
[`rvt-native-saved-scene`](../src/bin/rvt_native_saved_scene.rs) entrypoints
with the source-qualified contracts above. The portable byte rules are in the
[native format reference](research/native-format-reference.md), and the
reproducible probe index includes
[`research_native_coverage`](../examples/research_native_coverage.rs),
[`research_current_content`](../examples/research_current_content.rs),
[`research_physical_graph`](../examples/research_physical_graph.rs), and
[`research_saved_scene`](../examples/research_saved_scene.rs).

A recorded checkpoint contains 48 lab cases, 144 structural rows, and 72
strict rows, with 1,272 Rust tests at that checkpoint. These counts are
historical evidence and should not be read as the current release gate. The
native scope remains bounded: multi-loop/non-rectangular curved trims, full
regeneration, visibility parity, and render parity are unproven.

The native readers preserve source ownership, bounded budgets, and explicit
refusal states. They provide measured projections for saved records, spatial
context, equipment-like records, connectors, materials, lifecycle data, and
saved graphics where the qualified profile supports them. They do not claim
universal typed recovery, complete semantic coverage, cross-document identity,
or converter-grade IFC output.

We have generated and analyzed many Revit files to discern the file structure and variation.

## Extension guidance

New native claims should include a version-qualified structural rule, a
source-qualified output field, an explicit refusal condition, and a small
reproducible probe under `examples/`. Aggregate authored test matrices may be
reported when their inputs and scope are public; private model names,
filenames, hashes, identifiers, and per-file counts do not belong in this
document.
