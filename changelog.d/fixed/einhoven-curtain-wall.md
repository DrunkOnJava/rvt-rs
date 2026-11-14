- **Revit 2023's ArcWall pattern no longer writes a curtain wall's panels and
  grid lines as walls (RE-440).** On `Revit_IFC5_Einhoven` its 24 matches were
  15 curtain panels, 7 curtain grid lines and 2 records with no ElementId:
  they were written as 24 walls with a placeholder thickness, hid 15 of the
  curtain wall's panels and made three storeys from the grid's rows. A match
  is now kept only where its native record is a wall. Einhoven exports its
  one wall, its curtain wall as an `IfcCurtainWall` of 24 panels and its two
  windows on its one Level, `Level 1`, and four of its warnings are gone. A
  Revit 2023 wall whose type is a curtain wall type is an `IfcCurtainWall`
  named by that type, aggregating its panels; 2023 `rac_basic`'s 9 curtain
  walls take the name its 2024 and 2025 saves give them. A curtain wall whose
  panels carry its body is no longer reported as missing one.
