- **A roof's sketch holes stay in its `GrossArea` and `GrossVolume` (RE-413,
  #1636).** A flat roof Revit's export writes as a slab, drawn from a sketch with
  holes, gets its outline's area for `Qto_SlabBaseQuantities.GrossArea` and that
  area times its depth for `GrossVolume`, as Revit's export does: it writes the
  holes as openings and takes them off its `NetVolume` alone. A floor's holes are
  still taken off its `GrossArea`, which is Revit's on the 8 floors with holes of
  the BIM4LCA housing as built. The change moves one roof, in each of the four
  large saves of that project, whose two quantities were 0.9% to 2.9% under
  Revit's and are equal now; of 126 models no other export moves, so no second
  project exercises the rule.
