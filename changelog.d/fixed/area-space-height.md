- **An Area's `Qto_SpaceBaseQuantities.Height` is the height of its body
  (RE-317).** Revit's export writes it on all 30 Areas of each BIM4LCA office
  file and rvt-rs wrote none of them: an Area exported from a Revit 2024 to 2027
  file now says the distance from its Level up to the Level above, the depth its
  body is extruded, and no room or family-instance space changes. A Revit 2015
  to 2023 Area keeps none, the height of its export not being measured.
