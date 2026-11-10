- **No false RE-354 warning for a family's type table (RE-395).** A family
  type table's Fire Rating and Manufacturer entries in a `Family` element's
  block are the rows of the family's own type table, not a type's value,
  and rows that differ are not a disagreement. They are now left out, and the export
  no longer warns that a type text parameter is left out on Core Interior,
  Snowdon Towers Architectural (2024 and 2025) or Golden Nugget Architektur,
  where every such warning was a `Family`. No IFC changes on the 32 projects
  measured.
