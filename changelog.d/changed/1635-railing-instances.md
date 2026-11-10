- **A railing is drawn from its saved mesh where its saved graphics are whole
  (#1635, RE-397).** A railing's saved graphics name its parts as instances of
  other elements. The walk skipped the instances flagged hidden in the normal
  draw and forbidden the targets 53246, which hold up to 59% of a railing's
  body, and refused a flip control, which is a grip with no surface. Both are
  read now. On the BIM4LCA office 15 of the 41 railings are drawn from their
  saved meshes (their bounding boxes within 0.01 ft of Revit's export's), as are
  2 on each of the four Revit 2025 stair models and 1 on RE1 Architecture; the
  four stairs, which aggregate their railings, go from 2.6 to 3.0 times Revit's
  volume to Revit's (the stair volume RE-390 disclosed). Two of the office's railings, the `KAI-3`, are 19.5% over
  Revit's volume, all of it in their posts: the saved graphics hold a round
  post, and Revit's export draws it as a quadrilateral inscribed in that circle,
  the difference of the hexagon it draws on the 30 mm rails of the BIM4LCA
  housing, so the volume reads over where every vertex is within 0.01 ft of
  Revit's surface. 224 bodies on 19 of
  Autodesk's samples change from a record box in the same way, with no Revit
  export to score them. 26 railings of the office keep their boxes: the 22 stair
  railings, and 4 whose faces the parametric reader refuses.
