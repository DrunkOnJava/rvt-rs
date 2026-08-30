# Redistributable `Global/ElemTable` excerpts

Decompressed-stream excerpts used by always-on `elem_table` regression tests,
so the record-framing invariants stay gated even when no external corpus is
present. Each binary has a sibling `<name>.license.json` recording the source
artifact, its SHA-256, the SPDX license, and the exact derivation.

| Fixture | Source | License | Role |
|---|---|---|---|
| `2024-core-interior-elemtable-head-tail.bin` | `magnetar-io/revit-test-datasets` `Revit/2024_Core_Interior.rvt` | MIT | Record-origin regression for #206 — the 40 B project variant whose records begin one `u32` ahead of the `FF` marker |

These are excerpts of a decompressed CFB stream, not Revit files: they are
fed straight to `elem_table::detect_layout` / `parse_records_from_bytes`.
The Autodesk-owned `rac_basic_sample_family` corpus is still never
redistributed here (see `SECURITY.md`).
