- **A project whose last phase holds no room writes the rooms of the latest
  phase that holds any (#1359).** Autodesk's `rac_basic_sample_project` has its
  14 rooms in Working Drawings, the second of three phases, and none in its
  last, Learning Content, so the room-phase rule of RE-326 (0.14.0) left out
  all 14 and an importer's room count went from 14 to 0. Where no phase is
  named and the last phase holds no room, `rvt-ifc`, the Python bindings, the
  MCP server and the viewer now write the rooms of the latest phase that holds
  any, because rooms all in one phase cannot overlap and the only Revit export
  of this model holds them. It is a choice by Griffin (2026-11-05), not a
  measurement of Revit's default: no Revit export made at the last phase of
  such a project exists, and a project with no room in its last phase gets
  earlier rooms that Revit's default might leave out.
  - The diagnostics name the phase and count its rooms in the new
    `room_phase_fallback` (`docs/schemas/export-diagnostics.schema.json`), absent
    in every other export. `rvt-ifc` and `rvt-schedule` print a line, and the
    Python bindings raise a `RuntimeWarning` naming the phase and saying to pass
    `phase=` (`decoded_elements()` and `element_counts()`, which take no phase,
    say to export with it). `RevitFile::room_phase_fallback` gives the phase.
  - Every other element still follows the last phase. A phase named
    (`rvt-ifc --phase`, `phase=`, the MCP server's `phase`), the last included,
    is exact and writes none of these rooms. A project with a room in its last
    phase, and a project whose rooms are all in it, are unchanged, byte for
    byte. A room whose phase is unset, unread or not one of the project's is
    written as before and does not count towards the last phase holding a room.
  - Measured (Measure run 42011174043 at `992196c6`, 111 exports): the 2016
    to 2027 saves of `rac_basic_sample_project` each write 14 `IfcSpace` (0
    before), and the other 99 exports, held-out ones included, are byte for
    byte as before. The default is byte for byte the `--phase "Working
    Drawings"` export and pairs 14 of the Revit 2018 export's 17 spaces by
    GlobalId; `--phase` with the last phase writes what 0.14.0 wrote on all ten
    saves of 2018 to 2027 (RE-326 §9.4).
