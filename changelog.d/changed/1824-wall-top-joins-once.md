- **A Revit 2015 to 2023 export reads its walls' records once, not twice, for
  whether each wall's top is joined (#1824).** The read of the walls' records
  that draws their bodies already finds each top join. It now keeps that
  answer for the export's later readers of it, and reads the walls it draws
  no body for (those with no type in the export) in the same pass, for their
  type, top join and load bearing alone. Before, the later readers read every
  wall's record again for those members. Exports are unchanged.
