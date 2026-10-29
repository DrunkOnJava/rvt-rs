- **Openings of windows whose family cuts a solid are cut as Revit cuts
  them (RE-225, #227).** A window set in a wall whose symbol cuts a solid
  (`m_complexCutGeoms`) and no outline had its opening cut to the wall's
  thickness and its box's height; Revit's export cuts it to the solid,
  deeper across the wall and lower at the bottom (on the BIM4LCA office's
  windows 0.328 ft deeper each side and 0.059 ft lower). The opening is now
  as deep as the solid's extent across the wall and as tall as its height,
  down from the body's top: all 269 of the office's windows as built and 265
  at permit within 0.001 ft of Revit's openings. A door's opening keeps its
  box: on the office the solid of a door's symbol brings 4 doors' openings
  closer to Revit's and none within 0.001 ft. No door or window of Core
  Interior, RE1 or puzzbobb's Revit 2025 models cuts a solid.
