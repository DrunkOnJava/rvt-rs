- **Reading an ElementId in a 4-byte `Identifier` in every native reader
  moves three Revit 2023 exports no export checks (RE-282, #1158).** Revit
  2015 to 2023 save `Identifier` as a 4-byte `m_id`, 2024 and later as an
  `m_id64`, and every native reader but the Element base and the phase order
  reads only the second. Reading both shapes everywhere leaves 91 of 94
  exports byte-identical on Measure's unfiltered run (all 30 samples of 2015
  to 2022, BIMcollab, the held-out models, all 25 samples of 2024 to 2027,
  the R2025 models, the offices, Core Interior and RE1) and moves Einhoven
  and two of Autodesk's 2023 samples: 36 and 30 connector ports, a wall
  layer set and material associations appear that nothing measures on
  Revit 2023. So each reader is
  admitted to 2015 to 2022 on its own measurement, reading the ids it needs
  itself, and the other readers stay silent on 2023.
