- **The native record framing holds on Revit 2015 to 2022 (RE-280, #1158).**
  On Autodesk's 30 sample projects of those releases and BIMcollab's
  Revit 2015 example, every one of 27,762 record groups frames under the
  layout Revit 2023 uses (a `u32` id, a word, the length, the body and the
  length again), the five global streams parse (the extensible-storage
  catalogue on 22 of 31), and 98.8 to 100 per cent of the declared
  elements' object graphs decode, with the owner id agreeing on every one.
  Before Revit 2018 a group's declared body size counts each record's
  trailing length word, on all 12,005 groups of 2015 to 2017. Against the
  same project's 2023 copy, every ElementId both files hold has the same
  class on 23 pairs, and every saved parameter of 2021 and 2022 has the same
  value. Nothing is admitted yet: `native_element::decode_base` refuses
  `Identifier` as 2015 to 2023 write it (a 4-byte `m_id`), which also keeps
  the phase filter from reading Revit 2023, and the extensible-storage
  catalogue needs `m_specTypeId`, which 2021 introduced.
  `probes/examples/probe_re280_native_framing_pre2023.rs` is the measurement.
