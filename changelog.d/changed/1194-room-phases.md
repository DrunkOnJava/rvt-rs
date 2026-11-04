- **A room whose phase is not the export phase is left out, and `rvt-ifc
  --phase` names the export phase (#1194, RE-326).** A room's phase is the
  `m_phaseId` field of its `RoomElem` record on Revit 2015 to 2027, not its
  created phase, which is unset on every room. Revit's exporter leaves out a
  room whose own phase is not the export phase, the project's last unless the
  export setup or a view names another, and rvt-rs now does the same: the
  diagnostics count the rooms in `skipped` as `room_not_in_export_phase`, one
  item for each phase they sit in (`phase`, new in the diagnostics schema), and
  `rvt-ifc --phase <name|id>` chooses the phase, which the RE-184 rule for the
  other elements reads too. A room whose phase is unset, unread or not one of
  the project's is written, and so is an Area or an MEP space. The 621 rooms of
  the 12 models with a Revit export are all in the last phase and in Revit's
  export, so none changes; the rule is unmeasured against a Revit export made
  with the default setup, because no project in the corpus has a room outside
  its last phase and such an export. Autodesk's `rac_basic_sample_project` has
  14 rooms in Working Drawings, the second of three phases, which a public
  Revit 2018 export of it holds: its saves from Revit 2016 on no longer write
  them by default, and write them with `--phase "Working Drawings"`, all 14
  of the 17 `IfcSpace` of that export and no other entity changing (Measure
  runs 41784415396 and 41784419457). Its Revit 2015 save
  keeps them, because both phase rules start at Revit 2016 (#1332). The Python
  bindings and the viewer export the last phase. Report:
  `reports/element-framing/RE-326-room-phases.md`.
