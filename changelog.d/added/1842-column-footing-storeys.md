- **A column that names three or more Levels, and a footing that names none, are
  put in a storey on Revit 2027 (#1842).** A column takes the highest Level its
  record names at or below its box's base, else the lowest; a footing takes the
  highest Level of the model at or below its box's base. On the one Revit 2027
  structural model that holds them (a reporter's, counts only) these put 48 of
  48 columns and 88 of 88 footings in the storey Revit's export has them in,
  where 48 columns and 3 footings were in none and 6 footings in another. Both
  rest on that model alone, so they apply to 2027 only, are listed under
  "Rules that rest on one project", and mark each element they place on its
  `LevelBindSource`.
