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

The integration test `tests/json_schema_contracts.rs` validates these schemas
against real CLI payloads when the redistributable sample corpus is available.
Tier-1 / `gen-fixture` payloads are also covered for the decoded-elements and
element-counts contracts so Cloud / no-Autodesk environments stay green.

`tests/support_matrix.rs` validates the seeded support matrix against
`support-matrix.schema.json` and enforces honesty ceilings (COR-001 /
TEST-001 / DOC-001): converter-grade RVT-to-IFC and generic typed recovery
must not be marked `verified`.
