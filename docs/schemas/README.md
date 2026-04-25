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
| [`export-diagnostics.schema.json`](export-diagnostics.schema.json) | `rvt-ifc --diagnostics`, Python `export_diagnostics_json()` |
| [`corpus-report.schema.json`](corpus-report.schema.json) | `rvt-corpus -f json` |

The integration test `tests/json_schema_contracts.rs` validates these schemas
against real CLI payloads when the redistributable sample corpus is available.
