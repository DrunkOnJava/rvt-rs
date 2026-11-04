- **Exports of Revit 2015 to 2023 files take 0.57 to 0.84 of the time they
  took, and 1.1 to 1.7 times as long as with 0.11.0 (#1237).** 0.12.0 took
  2.0 to 3.4 times as long as 0.11.0. Four changes, none of which changes an
  export (2015 to 2023; 2024 to 2027 pending the full Measure):
  - the read of the parameters named for a property decodes a record only if
    its body holds the id of a definition the read uses, and reads the types
    only if one of those definitions is a plain `IsExternal`, whose type
    value an element takes. This assumes that a value is saved against its
    definition's id itself; on Autodesk's 33 sample projects of 2015 to 2023
    every record that gave such a parameter held an id. On the 2015
    `rac_advanced`, 6 of the 5,608 records it decoded held one, and no sample
    defines `IsExternal`;
  - the phase read, which takes the created and demolished phase of every
    record in the file, reads a record's Element base fields straight from
    its bytes where the schema has the layout measured on Revit 2015 to 2027,
    and by the graph decoder otherwise. It agreed with the graph decoder on
    all 1,014,473 records of Autodesk's 58 sample projects, 2015 to 2027;
    none fell back (Measure run 41782852434, on a branch whose reader is
    byte-identical to this change's);
  - the graph decoder works out a class's field keys once for each class a
    record uses, copies no field for the items of a container, and keeps no
    field spans it never reads, and a class is found by name through an
    index. The reference models, Revit 2023 to 2025, take 0.77 to 0.87 of
    the time (run 41782871819);
  - the parameter definitions pass asks once for each class whether it owns a
    definition, not for each record.

  Each comparison was timed on a runner of its own, the median of three
  exports each, so compare ratios, not seconds, between the two: this change
  against `main` before it (Measure run 41782871819) and against 0.11.0 (run
  41782874568), on Autodesk's sample projects. 33 of 33 exports and
  diagnostics of 2015 to 2023 are byte-identical to `main`'s. The time of
  this change over `main`'s is 0.64 for the 6 projects of 2015, 0.68 for the
  9 of 2016 to 2018, 0.68 for the 15 of 2019 to 2022 and 0.76 for the 3 of
  2023; over 0.11.0's, 1.49, 1.51, 1.47 and 1.13, single projects 1.09 to
  1.65. No sample project's peak memory is more than `main`'s (0.91 to 1.00;
  the reference models 0.94 to 1.01). What remains over 0.11.0 is the
  parameter definitions pass, which decodes every `FamilySymbol` record for
  two root fields, the wall records' top joins and the native records'
  preamble.
- **`rvt::research` (unstable): the named-parameter read changes its entry
  point (#1237).** `native_element_parameters::read_defined_values` is
  replaced by `read_property_values`, which also takes the captions whose
  type values the reader uses. `native_document::Options` gains
  `filtered_parameters_only` (off by default), and `native_parameters` gains
  `Spans` and `decode_object_fields_spanning`.
