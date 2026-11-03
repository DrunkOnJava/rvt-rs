- **A 28-byte `Global/ElemTable` is recognised by its ElementId, the record's
  first id.** The layout check tested the second id, which on 8 client files
  saved by Revit 2024 to 2027 with an older schema holds an older, lower
  number on 1% to 37% of records, so their tables were read as 12-byte
  records with a "fits neither its 28- nor its 40-byte layout" warning. The
  warning is gone on all 8; their element records are not yet measured
  against Revit's ElementIds (#421, found by @puzzbobb, measured by
  @hemanth1986).
