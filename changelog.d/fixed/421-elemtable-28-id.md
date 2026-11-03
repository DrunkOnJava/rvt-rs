- **A 28-byte `Global/ElemTable` is recognised by its first id, the one
  `declared_ids` reads as the ElementId.** The layout check tested the
  second id, which on 8 client files saved by Revit 2024 to 2026 with a 2022
  or 2023 schema holds an older, lower number on 1% to 37% of records, so
  their tables were read as 12-byte records with a "fits neither its 28- nor
  its 40-byte layout" warning. On the reporter's build of this change the
  warning is gone on all 8; whether their records' ElementIds are Revit's is
  not yet measured (#421, found by puzzbobb, measured by hemanth1986).
