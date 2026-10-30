# Reports

The measured findings behind rvt-rs's readers and exports. Code, docs and
issues cite them by id.

- `element-framing/RE-NN-<topic>.md`: format findings, filed by id. Each
  states its status (positive or negative), the files it was measured on
  with their sha256, the reference export, and counts against Revit's own
  export. A new report's reproduce commands name a commit, not a branch.
  `CONTRIBUTING.md` ("Reverse-engineering findings") says how a new one is
  written.
- `security/`: the investigation of discussion #112 (issue #151).
- `validation/`: held-out measurements, made on models no reader was fixed
  against.

## Ids

`RE-NN` ids are assigned in order and never reused. Some have no report.
The first ones, to RE-23, began as a planned task list
(`RE-01-synthesis.md`), and a few of those tasks were folded into other
reports or dropped, though early reports still mention them; a later id
with no report was never used. Rustdoc and the user docs cite only ids that
have a report: `tools/ci/api-doc-ids.py` fails otherwise.

Two ids name two reports each:

- **RE-15** is the first ArcWall and geometry probe thread, written up as
  two syntheses: `RE-15-arcwall-trailer-synthesis.md` (the Revit 2023
  ArcWall trailer) and `RE-15-synthesis.md` (doors, slabs, layers and
  names, #86 to #89).
- **RE-29** is two findings of 2026-09-19:
  `RE-29-join-reference-list.md` (the reference list names a wall's joins
  and the walls that cut a column, #238, #239) and
  `RE-29-room-instance-and-envelope.md` (rooms as element records, #90).
  A citation tells them apart by its issue number.

Two element-framing reports carry no id and are cited by file name:
`32bit-container-and-building-story.md` and
`storage-increment-table-budget.md`.
