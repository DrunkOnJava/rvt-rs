# Redistributable project fixture with saved display graphics

A Revit project small enough to commit, so the saved-graphics batching check
runs on every push instead of only where the external project corpus is
present. Each binary has a sibling `<name>.license.json` recording the source
artifact, its SHA-256, the SPDX license and the provenance.

| Fixture | Source | License | Role |
|---|---|---|---|
| `einhoven-2023.rvt` | `magnetar-io/revit-test-datasets` `Revit/Revit_IFC5_Einhoven.rvt` | MIT | Revit 2023 project, 913 KiB, 2,615 owners with saved graphics (35 of them boxed elements with a Tag): the always-on batch/one-shot equality check (#1075, #1429, #1808) |

`tests/saved_batches.rs` reads every channel-103 owner in it, both ways, and
compares 2,615 elements and their progress events in about a third of a second.
The file's graphs are small, so the check says nothing about where
`OWNER_OBJECT_BUDGET` (20,000 graph objects) splits a streamed buffer; the
Measure runs read the corpus models that reach it.

The fixture is committed unmodified. A replacement has to update
`einhoven-2023.rvt.license.json` and the owner count `tests/saved_batches.rs`
asserts, so the check cannot quietly start comparing nothing.
