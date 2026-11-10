- **An instance visible in the normal draw and forbidden the targets 10 is drawn
  (#1645, RE-408).** The saved-graphics walk refused any `GInstance` whose
  `m_forbiddenTarget.m_targets` was not 0, which kept the appliances, chairs, sinks
  and pendant lamps that nest one as record boxes. The pair of visibility 0 and
  targets 10 is drawn now, and every other mask stays refused. On Autodesk's
  Snowdon Towers Architectural 332 bodies come from their saved meshes, all in
  Revit's own export, 301 of them within 0.01 ft of it both ways and within 2% of its
  volume (the 16 art-glass pendants read 1.04 to 1.10 of its volume with every
  vertex within 0.0093 ft, the 15 kitchen sinks 0.0106 ft from its surface), and 16
  more on Snowdon Towers Electrical, which has no export to score them. Six
  of puzzbobb's Revit 2025 models gain nine window trims and muntins each (54
  bodies), all in Revit's export at volume 1.0000 with every vertex within 2.03e-6 ft
  of it both ways. The housing's showers meet the next refusal and stay boxes.
