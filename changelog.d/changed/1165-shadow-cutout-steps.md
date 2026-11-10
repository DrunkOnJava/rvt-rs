- **A wall's shadow cutout step is read as the step it mirrors (#1165, RE-398).**
  A `WallShadowCutoutGStep` names the wall whose record holds the cut
  (`m_otherWallId`) and that wall's step (`m_ggId` is its `m_id`), of the same
  cutter. All 124 of the BIM4LCA office's, `rac_advanced`'s (2024 to 2027) and
  Snowdon Towers Architectural's do: 12 mirror a curtain wall's embed and 112 a
  door's, window's, other family's or opening's cutout. The export reads each
  as that step, so a cut it left out as `shadow_cutout_step_not_read` is
  counted under the reason of the step it mirrors (the office as built:
  `curtain_wall_within_half_foot_of_wall_end` 41 to 42 and
  `door_opening_box_not_read` 1 to 2). No export changes: the opening in a
  shadow's wall is not written until the mirrored step's own rule writes it.
