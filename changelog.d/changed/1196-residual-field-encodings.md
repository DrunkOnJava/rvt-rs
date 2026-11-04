- **`Formats/Latest`: 11 of the 14 residual field names are classified, and
  Revit 2027's flagged fields are shown to hold no instance bytes (#1196,
  RE-323).** `FieldType` gains `PointerArray` (`0e 13`: `GEdge`'s three
  face and edge arrays, two external pointers each) and `PointerContainer`
  (`0e 53` and `0e 54`: a counted container of external or owning pointers),
  appended to the non-exhaustive enum, and `Flagged` takes byte 2 = 0x04
  (`VarExpr::m_refCt`, an `i32`). The native graph decoder reads the
  instances of six of these fields in Autodesk's sample projects with no
  layout failure, no record left with a byte unconsumed and every non-null
  pointer resolving to an object of its record: `GEdge`'s three arrays
  (111,985 instances each, 57 of 58 projects, all 13 releases 2015 to 2027),
  `VarExpr::m_refCt` (138,477), `VarSketchConstrObj::m_constrElems` (58,876)
  and `VarSketchObj::m_params` (93,092) on 2018 to 2027. Five more fields
  (`LFormatDefaults`' three, `VarFunction::m_args`,
  `VarSketchDimObj::m_refPts`) have no instance in any file read and are
  classified by their header alone. `ZoningAppInfo::first` was `Unknown`
  only because the schema scan cut a field's type at the end of its class's
  4,096-byte window, two bytes after the field name; the scan now reads it
  from the stream. 2027's 339 flagged fields (`DBView::m_horzDirTemp`,
  edit-mode managers) occupy no bytes: the native decoder skips them, and
  97,000 of the 97,005 records of Autodesk's five 2027 sample projects still
  end exactly at their body's last byte (the other 5 fail on a size
  budget), with 110 of the fields' classes among them. Still unclassified:
  `ClassDefinitionRef::m_ref` (no instance in any file read),
  `VarParam::m_pSubst` (31,705 instances in the 2015 to 2017 sample
  projects, every one a null pointer, which two layouts read alike) and
  `VarSketchDimObj::m_dimValue` (none). The coverage gate also reads each
  schema with the exact grammar, which names these fields by the classes
  that declare them (the scan files `GEdge`'s under `GInfo` and
  `VarSketchDimObj`'s under `VarSketchObj`), and fails on a residual entry
  that no longer decodes `Unknown`. The `schema-table` JSON schema gains two
  `FieldType` branches. Report:
  `reports/element-framing/RE-323-residual-field-encodings.md`.
