# Integration readiness and portability audit

This document records portability and provenance facts for the integrated
native world-model and research additions. Historical attribution is separate
from executable dependency analysis.

Run the portable inventory from the repository root:

```sh
python3 tools/audit-portability.py . --json-out /tmp/rvt-rs-portability-audit.json
```

The tool inventories tracked and nonignored untracked candidate source,
documentation, test, and configuration files. It excludes `.git`, `target`,
`node_modules`, `.venv`, and `__pycache__`; it does not load files larger than
8 MiB. Findings are references for review, not proof that a particular person
accessed a source or that a historical document is a runtime dependency.

## 2026-09-14 bounded inventory

The run against the current integrated working tree found 607 candidate files.
The counts are files containing at least one matching reference:

| finding | files | interpretation |
|---|---:|---|
| Absolute developer/home/Windows/temp path | 74 | Review for examples, tests, receipts, or genuinely portable defaults. |
| Sibling/workspace reference | 0 | None detected by the portable inventory. |
| Private fixture/receipt/corpus reference | 107 | Usually research evidence or optional corpus input; must not become an implicit runtime input. |
| Process/external tool invocation | 112 | Classify as build, optional oracle, research runner, or production runtime dependency. |
| Revit/API/ODA/Docker reference | 340 | Historical attribution and validation references are allowed; executable requirements need explicit scope. |
| Oversized candidate skipped | 0 | Files at or above 8 MiB were not read. |
| Unreadable candidate | 0 | — |

The complete machine-readable result is generated outside the repository by the
command above. Re-run it after changes rather than treating these counts as a
permanent baseline.

## Concrete readiness issues

- Candidate files refer to private Revit corpora, API snapshots, ODA/Docker
  outputs, and Revit-hosted runners. Keep those as optional, source-hashed
  validation inputs; do not make default Rust execution depend on them.
- The `tools/oracle/runner/pyrevit` path is an external Revit-hosted oracle
  scaffold. Its README states that the first run still requires a Revit host;
  this is an optional research runner, not a default CI/runtime requirement.
- The native-world-model research handoff records integrated implementation and
  validation provenance. Its claims should continue to identify whether each
  result is byte-derived, API-witness-derived, commercial-parser-derived,
  compiled-routine-derived, or a Rust projection.

## Phased integration checklist

1. **Portable source inventory:** run `tools/audit-portability.py`; review every
   absolute path, sibling path, private fixture path, and process invocation.
2. **Executable dependency check:** run the default Rust gates from a clean
   checkout with no Revit/ODA/Docker/private corpus. Optional corpus and oracle
   gates must skip or fail with an explicit prerequisite.
3. **Fixture provenance:** for committed byte excerpts and generated catalogs,
   retain source hashes, version/build, extraction command, and evidence tier.
   Do not commit private RVTs, API binaries, ODA binaries, or oversized receipts.
4. **Runtime boundary check:** verify native CLIs consume only caller-provided
   RVT/JSON/texture inputs and static checked-in data; validation oracles run in
   separate commands and are never silently loaded by production extraction.
5. **Release gates:** run format, clippy, rustdoc, unit/integration tests, the
   portability inventory, and the optional corpus/oracle gates whose inputs are
   available. Record skipped external gates and their reasons in the receipt.
