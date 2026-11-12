- **A Revit 2015 to 2023 export reads its walls' records once, not twice, for
  whether each wall's top is joined (#1824).** The read of the walls' records
  that draws their bodies already finds each top join. It now keeps that
  answer for the export's later readers of it. Before, they read every wall's
  record again for that one member. Exports are unchanged.
