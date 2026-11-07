- **Revit 2015 to 2023 curtain mullions and panels have their type, family and
  material (#1190, RE-367).** On `rac_advanced` all 3,276 mullions and 1,349 panels
  take the type and family of the 2024 save, and every type the material its 2024
  save's `m_renderStyleId` holds. **No curtain part of these releases is scored
  against a Revit export here**; each is held to the same Autodesk project's later
  save, which cannot catch an error that save shares.
