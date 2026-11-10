- **A roof's sketch holes stay in its `GrossArea` and `GrossVolume` (RE-413,
  #1636).** A flat roof Revit's export writes as a slab, drawn from a sketch with
  holes, gets its outline's area for `Qto_SlabBaseQuantities.GrossArea` and that
  area times its depth for `GrossVolume`, as Revit's export does: it writes the
  holes as openings and takes them off its `NetVolume` alone. A floor's holes are
  still taken off its `GrossArea`, which is Revit's on the 8 floors with holes of
  the BIM4LCA housing as built. On each of that project's four large files the
  roof's two quantities, 0.9% under Revit's, are equal now, and no other BIM4LCA
  export moves.
