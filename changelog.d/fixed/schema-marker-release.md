- **A file that kept an older release's schema reads as that release
  (#421).** A project saved by Revit 2024 to 2027 can carry the schema, and
  so the element-record marker, of the release it passed through: in a
  field report, 25 of 49 such project files carried the marker of 2022,
  2023, 2024 or 2025, and all 25 read as zero elements and zero Levels,
  because the marker did not match the saving release's. The readers now
  take the release the file's schema belongs to (`schema_release`: a 2024
  to 2027 marker, or a 2015 to 2023 release's class tags) when it is older
  than the saving release (`RevitFile::layout_release`). A schema that
  matches no measured release still reads nothing. Metadata, reports and
  the export's GlobalIds keep the saving release; the diagnostics say when
  a file is read as another release than the one that saved it, and give
  the support status of the release it is read as. On Autodesk's 58 sample
  projects of 2015 to 2027 each schema is its own release's and every
  export is byte-identical (Measure run 40928760469). Not yet measured on
  an affected file. A file whose `Global/ElemTable` fits neither record
  size still reads no element records, and the diagnostics now say so;
  `probe_421_schema_marker` prints the table's fit as counts only.
