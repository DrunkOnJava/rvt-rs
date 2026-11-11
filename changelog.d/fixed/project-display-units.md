- **The export reads the project's display units from its `UnitsElem`
  record, one per spec, instead of counting unit strings, and no longer warns
  about units of specs the IFC has no use for (RE-439).** The diagnostics'
  `recovered_unit_identifiers` are now the units the project's Project Units
  set for length, area, volume, angle, mass and time: Autodesk's 2024 Core
  Interior and Snowdon Towers give feet and fractional inches, square and
  cubic feet, degrees, pounds and seconds, where the count gave metres,
  square inches and cubic inches; it gave the metric `rac_basic` and RE1
  Architecture square inches and pounds too, and they now give millimetres,
  square and cubic metres and kilograms. The warning "Unmapped Revit unit
  identifiers were preserved in diagnostics", which listed the electrical,
  HVAC and structural units every multi-discipline template sets (130 of
  them on Core Interior), is gone; a warning names a display unit of the six
  only where no IFC unit is known for it. A new `project_display_units`
  lists the display unit of every spec (133 to 150 on the corpus). A family
  whose records the native walk does not read takes its units from the
  partition's string records where their maps all agree. Revit 2015 to 2020
  files keep their units without spec names and are not read, as before. The
  IFC is unchanged: its values and unit assignment stay SI. `fractionalInches`
  is now read as inches (it was feet) and the diagnostics label a kilogram
  `KILO GRAM` and a gram `GRAM` (they said `GRAM` and `MILLI GRAM`).
