- **A curtain mullion's end beside an oblique mullion, or a partner end beside a mullion across inside, no longer hides it or leaves its mullion out, and a joint centred under a mullion across is hidden (#1767, RE-438).**
  Revit's `Qto_MemberBaseQuantities.NetSurfaceArea` counts every end of the
  BIM4LCA concrete office as built that has only an oblique mullion at it (50
  of 50), a partner and an oblique one (76 of 76) or a partner and a mullion
  across inside (52 of 52), and hides the 4 ends of the timber office at permit
  where a partner meets a mullion across at its centre. The rule that read a
  mullion's ends hid the partner ends, kept those joints and left a mullion out
  for an oblique mullion or one across inside at an end (150 of the concrete
  office as built's 10,003, 153 of the timber office's 7,876). It now reads 131
  and 130 of them, with Revit's count on every one, and the offices gain 108
  (concrete, as built and at permit) and 107 (timber, as built and at permit)
  `NetSurfaceArea` values equal to Revit's, none different; the 4 values
  different at permit fall to 1, the 3 pieces of a chain of 50 x 50 mm
  mullions being Revit's. 9 of 126 exports move, by that value alone (the
  offices, 32 mullions of each `rac_advanced` save, 24 of Snowdon Towers
  Architectural); the 70 and 69 mullions whose partner end an oblique mullion
  keeps wait for the mesh path (#1790).
