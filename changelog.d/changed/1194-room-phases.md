- **A room's phase is read, and a room in an earlier phase than the last is
  written and counted, not left out (#1194, RE-326).** A room's phase is the
  `m_phaseId` field of its `RoomElem` record on Revit 2015 to 2027, not its
  created phase, which is unset on every room. The 621 rooms of the 12 models
  with a Revit export (Core Interior, RE1 Architecture, the BIM4LCA office as
  built and at permit and its two terrain variants, six R2025 room models) are
  all in the last phase and all in Revit's export, so none of them tells a rule
  that leaves out a room in another phase from one that does not. The only
  rooms in another phase are Autodesk's `rac_basic_sample_project`'s 14, in the
  second of three phases on all 13 releases, and a public Revit 2018 export of
  it holds all 14. Revit's exporter takes the export phase from the export
  setup or a view and only by default the last phase, and rvt-rs reads neither,
  so no room is left out. The export diagnostics' `warnings` give the number of
  rooms written that are in another phase (expected from the phase probe: 14
  on every save of that sample and none on any other model of the corpus); the
  exported IFC does not change. Unmeasured: a Revit export made with the
  default setup that leaves such a room out.
  Report: `reports/element-framing/RE-326-room-phases.md`.
