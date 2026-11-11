- **A curtain mullion or panel drawn as its oriented box is offered the mesh Revit
  saved for it, and says why where it keeps the box (#1689, RE-421).** RE-106 draws
  a mullion or panel turned or tilted off the model's axes as the box along its own
  axes; the saved-mesh pass took the record box, a beam's box and a turned box but
  not this one, so these bodies were never replaced and carried no reason. On Revit
  2023 to 2027 the pass now draws them from their saved meshes where all of the mesh
  decodes: the BIM4LCA office's 325 (concrete) and 327 (timber) go to 2 and 1,
  `Qto_MemberBaseQuantities.NetVolume` reads equal to Revit's on 323 and 326 more
  mullions of each office (10,001 of 10,003 and 7,875 of 7,876 as built), none
  different, and the boxes left say `partial: disconnected UV boundary`. Autodesk's
  `rac_advanced` of 2023 to 2027 draws all 148 of its oriented boxes from their
  meshes, Snowdon Towers Architectural all 1,019. On Revit 2015 to 2022 the boxes stay
  boxes and now say `not_offered: release_outside_2023_to_2027`. Measured on one
  project with a Revit export (the BIM4LCA office, Revit 2024); the Autodesk samples
  have none to score against.
