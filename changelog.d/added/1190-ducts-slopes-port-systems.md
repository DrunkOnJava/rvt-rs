- **Ducts have their type, section, line and ports from their own native record on Revit 2015 to 2027, ports their `SystemType`, and Exhaust Air ports their flow (#1190, #96, #834, #869, RE-371).**
  Every duct of the releases the native record path reads is typed (`Family`,
  `Type`, type `Reference`) and drawn as its section along its one line, at
  any slope, with its `Length`, its taps' ports and, where it is centred, its
  `Constraints` elevations, `Slope` and justification; a port has the
  `SystemType` its connector's system's classification names (`VENTILATION`,
  `EXHAUST`, `HEATING`), and an Exhaust Air port is `SOURCEANDSINK`. A port
  joined to another is placed with the axis Revit's export gives it however
  the two flow. On Revit 2015 to 2022 the family instances' ports are read
  and joined too. On Snowdon Towers HVAC (2025, local only) the record gives
  Revit's own type name, `Length`, section, body and `Constraints` values on
  1,053 of 1,053 ducts, and `SystemType` on every port whose system is
  named; on RE1 Mechanical the 25 ducts' type, length and body. **Flex ducts
  are not read, and no duct of 2015 to 2023, 2026 or 2027 is scored against
  a Revit export.**
