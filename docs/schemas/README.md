# JSON Schemas

These schemas document the stable JSON surfaces used by the CLIs, Python
bindings, viewer diagnostics, and release-support workflows. They are
forward-compatible by default: new additive fields may appear without a schema
version bump, so the output schemas leave `additionalProperties` open, but
incompatible removals or type changes require a version bump in the producing
payload. A list of values, such as a status or a reason code, is open the same
way: a minor release may add a value, so code that reads one handles a value
it does not know (`RELEASE.md`, "What 1.0 keeps stable";
`tools/ci/schema-compat.py` passes an added value and fails a removed one).
The schemas of checked-in data (the support matrix, the witness registry) stay
closed.

| Schema | Producer |
|---|---|
| [`summary.schema.json`](summary.schema.json) | `rvt-info -f json` on one file, `RevitFile.summarize()` |
| [`file-metadata.schema.json`](file-metadata.schema.json) | `document` in `rvt-info -f json`, `rvt::metadata::FileMetadata`, Python `RevitFile.metadata()` / `rvt.read_metadata()` |
| [`inventory-row.schema.json`](inventory-row.schema.json) | `rvt-info <folder or several files> -f json` (array) / `-f jsonl` (one row per line) |
| [`schema-diagnostics.schema.json`](schema-diagnostics.schema.json) | `rvt-schema --diagnostics -f json`, `SchemaTable::diagnostics()` |
| [`element-records.schema.json`](element-records.schema.json) | `rvt-doc --json`, Python `read_adocument()` field dictionaries |
| [`decoded-elements.schema.json`](decoded-elements.schema.json) | `rvt-elements`, Python `RevitFile.decoded_elements()` |
| [`element-counts.schema.json`](element-counts.schema.json) | `rvt-elements --counts`, Python `RevitFile.element_counts()` |
| [`export-diagnostics.schema.json`](export-diagnostics.schema.json) | `rvt-ifc --diagnostics`, Python `export_diagnostics_json()` |
| [`corpus-report.schema.json`](corpus-report.schema.json) | `rvt-corpus -f json` |
| [`corpus-doctor.schema.json`](corpus-doctor.schema.json) | `rvt-corpus doctor --json` |
| [`support-matrix.schema.json`](support-matrix.schema.json) | Checked-in [`docs/support-matrix.json`](../support-matrix.json) (audit A3) |
| [`capability-manifest.schema.json`](capability-manifest.schema.json) | `rvt-capabilities` honest snapshot (`capability::CapabilityManifest`) |
| [`elem-table.schema.json`](elem-table.schema.json) | `rvt-elem-table --json` (`Global/ElemTable` header and records) |
| [`history.schema.json`](history.schema.json) | `rvt-history --json` (document history), and with `--all-strings` / `--partitions` the string records |
| [`analyze.schema.json`](analyze.schema.json) | `rvt-analyze --json` |
| [`inspect.schema.json`](inspect.schema.json) | `rvt-inspect --json`, including its error report |
| [`ifc-compare.schema.json`](ifc-compare.schema.json) | `rvt-ifc-compare --json PATH` (`ifc::compare::IfcCompareReport`) |
| [`schema-table.schema.json`](schema-table.schema.json) | `rvt-schema --json` (`formats::SchemaTable`; `--diagnostics` prints `schema-diagnostics`) |
| [`es-observation.schema.json`](es-observation.schema.json) | ES remap research observations (H-ES5; not a production decode claim) |
| [`es-capability.schema.json`](es-capability.schema.json) | Research capability promotion stub (report §15.16) |
| [`witness-registry.schema.json`](witness-registry.schema.json) | Checked-in [`research/witness-registry.json`](../../research/witness-registry.json) (`tests/witness_registry.rs`) |
| [`witness-observation.schema.json`](witness-observation.schema.json) | `rvt-ifc --observation`, `tools/ci/witness-ifcopenshell.py --observation` (OctetProof §6.2) |
| [`witness-verdict.schema.json`](witness-verdict.schema.json) | `tools/ci/witness-verdict.py` (OctetProof §6.3) |

Research mirrors also live under [`research/es-remap/`](../../research/es-remap/).
See [`docs/research/unified-research-report.md`](../research/unified-research-report.md).

The integration test `tests/json_schema_contracts.rs` runs every CLI that
prints JSON and validates its output against its schema: on the tier-1
synthetic fixtures for `rvt-info`, `rvt-elements`, `rvt-analyze`, `rvt-inspect`
(its report and its error report), `rvt-history --all-strings`, `rvt-schema`,
`rvt-corpus doctor`, `rvt-capabilities` and `rvt-ifc-compare`, so those run
with no Autodesk corpus present, and on the family corpus for `rvt-info`,
`rvt-history`, `rvt-elem-table`, `rvt-schema`, `rvt-doc`, `rvt-ifc
--diagnostics` and `rvt-corpus`. CI's Linux test job has the corpus and fails
if it is missing.

`tests/support_matrix.rs` validates the seeded support matrix against
`support-matrix.schema.json` and enforces honesty ceilings (COR-001 /
TEST-001 / DOC-001): converter-grade RVT-to-IFC and generic typed recovery
must not be marked `verified`.

`es-observation` and `es-capability` are research contracts, not tied to any
CLI's output. `capability-manifest` is what `rvt-capabilities` prints, an
honest snapshot of what is supported, and the contract test validates it.
