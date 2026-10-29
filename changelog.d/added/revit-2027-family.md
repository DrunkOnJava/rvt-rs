- **Revit 2027 in the family corpus (#1092, RE-264).** Autodesk's 2027
  sample family joins the twelve-release corpus, so PartAtom, previews,
  `BasicFileInfo` and the schema are measured on 2027 as on 2016 to 2026,
  and `streams::partition_for_year(2027)` is `Partitions/70`. Its schema
  lists 325 fields no earlier release lists, each with byte 2 of its type
  header set to 0x02, 0x12 or 0x22; `FieldType::Flagged` decodes them to
  their base type (77 bool, 79 reference containers, 59 ElementId
  references and so on) and keeps the flag. What the flag means for their
  instance bytes is not measured, so readers treat them as unknown, as
  before.
