- **Revit 2015 to 2023 curtain mullions and panels have their type and family, and
  the mullions their material (#1190, RE-367).** On `rac_advanced` all 3,276
  mullions and 1,349 panels take the type and family of the 2024 save, and every
  mullion type the material its 2024 save's `m_renderStyleId` holds. Against
  Revit's own IFC2X3 exports (counted apart) the parts have Revit's `Name`,
  `ObjectType` and `Reference`: all 1,349 plates and 3,276 members of
  `rac_advanced` 2015 to 2020, the 67 plates and 206 members of Vitoria's `ALD26
  Before` and of the held-out `ALD26 Project`. A plate's type object carries the
  ElementId of a plate in Revit's export, not the type's, which is the exporter's
  convention. **A panel of these releases takes no material:** none of the 67
  plates of `ALD26 Before` has the one a panel type's `m_renderStyleId` gives.
