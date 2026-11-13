- **An element on a Level that is not a building story is placed on the
  building story below that Level, as Revit's export places it (#1842).** A
  Level whose Building Story setting is off is not a storey of Revit's export
  (RE-118), and rvt-rs bound nothing to it, so an element whose record names
  such a Level was left on no storey unless its height placed it. Revit's
  exporter gives that Level the nearest building story below it, and one below
  every building story none. Autodesk's Snowdon Towers structural sample has
  7 such Levels among its 19, and four of the BIM4LCA housing files have 2 to
  10.
