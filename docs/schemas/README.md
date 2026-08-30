# JSON Schemas

These schemas document the stable JSON surfaces used by the CLIs, Python
bindings, viewer diagnostics, and release-support workflows. They are
forward-compatible by default: new additive fields may appear without a schema
version bump, but incompatible removals or type changes require a version bump
in the producing payload.

| Schema | Producer |
|---|---|
| [`summary.schema.json`](summary.schema.json) | `rvt-info -f json`, `RevitFile.summarize()` |
| [`schema-diagnostics.schema.json`](schema-diagnostics.schema.json) | `rvt-schema --diagnostics -f json`, `SchemaTable::diagnostics()` |
| [`element-records.schema.json`](element-records.schema.json) | `rvt-doc --json`, Python `read_adocument()` field dictionaries |
| [`decoded-elements.schema.json`](decoded-elements.schema.json) | `rvt-elements`, Python `RevitFile.decoded_elements()` |
| [`element-counts.schema.json`](element-counts.schema.json) | `rvt-elements --counts`, Python `RevitFile.element_counts()` |
| [`export-diagnostics.schema.json`](export-diagnostics.schema.json) | `rvt-ifc --diagnostics`, Python `export_diagnostics_json()` |
| [`corpus-report.schema.json`](corpus-report.schema.json) | `rvt-corpus -f json` |
| [`support-matrix.schema.json`](support-matrix.schema.json) | Checked-in [`docs/support-matrix.json`](../support-matrix.json) (audit A3) |
| [`capability-manifest.schema.json`](capability-manifest.schema.json) | `rvt-capabilities` honest snapshot (`capability::CapabilityManifest`) |
| [`es-observation.schema.json`](es-observation.schema.json) | ES remap research observations (H-ES5; not a production decode claim) |
| [`es-capability.schema.json`](es-capability.schema.json) | Research capability promotion stub (report §15.16) |
| [`witness-registry.schema.json`](witness-registry.schema.json) | Checked-in [`research/witness-registry.json`](../../research/witness-registry.json) (`tests/witness_registry.rs`) |
| [`witness-observation.schema.json`](witness-observation.schema.json) | `rvt-ifc --observation`, `tools/ci/witness-ifcopenshell.py --observation` (OctetProof §6.2) |
| [`witness-verdict.schema.json`](witness-verdict.schema.json) | `tools/ci/witness-verdict.py` (OctetProof §6.3) |

Research mirrors also live under [`research/es-remap/`](../../research/es-remap/).
See [`docs/research/unified-research-report.md`](../research/unified-research-report.md).

The integration test `tests/json_schema_contracts.rs` validates these schemas
against real CLI payloads when the redistributable sample corpus is available.
Tier-1 / `gen-fixture` payloads are also covered for the decoded-elements and
element-counts contracts so Cloud / no-Autodesk environments stay green.

`tests/support_matrix.rs` validates the seeded support matrix against
`support-matrix.schema.json` and enforces honesty ceilings (COR-001 /
TEST-001 / DOC-001): converter-grade RVT-to-IFC and generic typed recovery
must not be marked `verified`.

`es-observation` / `es-capability` / `capability-manifest` schemas are research
or doctor contracts. They are validated structurally in unit tests via serde
round-trips of Phase 1 types; they are **not** wired to production CLI success
claims. `rvt-capabilities` emits an honest snapshot (ArcWall 2023 verified,
compound / ES remap unsupported).
