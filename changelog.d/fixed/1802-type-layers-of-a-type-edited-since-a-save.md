- **A wall, floor, ceiling or roof type edited since an earlier save keeps its
  layers (#1802, RE-446).**
  A type edited after an earlier save is rewritten into a newer partition and
  its older copy stays, and the layers of a type whose copies gave different
  lists were dropped, so every type edited since had none: its walls no
  `IfcWallType`, `Width` or `Reference`, its floors and ceilings no layer set.
  On Revit 2024 and later such a type now takes the copy of the partition the
  element index routes it to (the rule the transform reader uses, RE-216), and
  none where no partition is named or that partition's copies disagree. On the
  BIM4LCA files 180 elements gain their type and its layers: the 14 walls of
  `VS-2.CLTpinta` and `SPR-1` of the timber office at building permit (the
  first type's width is Revit's 0.158 m, its older copy's 0.166 m is not), the
  concrete housing's 34 walls as built and 18 at permit, 108 coverings, 4
  slabs and 2 roofs. Every layer set written is within 0.001 ft of Revit's, and
  over the 42 models with a Revit export the quantities equal to Revit's rise by
  358 and the property values by 488, none different; the three values of the
  timber office that RE-444 stopped writing for want of these layers (a wall's
  `NetVolume`, an opening's `Height` and `Width`) are written again. Six
  exports move (the timber office at permit, the concrete housing and the
  timber housing as built and at permit, the terrain timber housing) and 120 are
  byte for byte unchanged. What it costs: 6 walls of the timber office at permit
  and 6 walls and 16 coverings of the concrete housing at permit take another
  body, and two wall ends of the housing that were exact are 0.27 and 0.29 m off
  (#1806); the 19 type objects it creates carry no `IsExternal`, as no type
  does (#1185). Not done: Revit 2008 to 2023 keep dropping such a type, because
  the current copy gave four floors of a held-out Revit 2020 file materials
  Revit's export does not give (#1805), and no model but the BIM4LCA files holds
  a type this reaches, so the rule rests on one project.
