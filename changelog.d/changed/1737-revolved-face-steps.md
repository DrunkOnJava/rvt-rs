- **A revolved face's steps around its axis take the reach of the profile over the face's own `v` range, not the centre plus twice the radius, and the triangles of 40 of 126 exports fall (#1737, RE-435).**
  A revolved face with an arc or an ellipse for a profile was cut into steps
  from `angular(c + 2 R)`, `c` the distance of the arc's centre from the axis
  and `R` its radius, though no point of the profile is farther than `c + R`
  and the face holds only the part of the profile its own `v` range covers.
  `profile_reach` takes the farthest of 64 equal steps of that range, and half
  a step's travel added so the true maximum is never short: the chord of a step
  stays within the 0.001 ft the grid keeps at every radius of the face. 40 of
  the 126 exports of the unfiltered run move, by the meshes of elements with
  such a profile (7,231 of 126,381 objects drawn from a saved mesh on both
  sides: pipe and duct fittings, light fixtures, sanitary terminals, railings,
  furniture), and no element drawn before stops being drawn. The triangles
  from saved graphics fall by 9.7% on the BIM4LCA office (a toilet from 17,494
  to 14,025), 6.4% on the large housing exports, 8.5% on Snowdon Towers
  Architectural, 14.1% on its Electrical model, 9.9% on `rac_basic` and 5.7%
  on `rme_advanced`, and the IFCs by 0.9% to 12.7% (Snowdon Towers
  Architectural 269.5 to 251.3 MB). Against Revit's export no class's median
  volume moves by more than 0.0006 and the largest change of a body's is
  0.0015; one body of RE1 Architecture, a shower head, crosses the 0.001 ft
  line of Revit's vertices from the mesh (0.00073 to 0.00103 ft), because the
  chord is now used up at the farthest radius of the face where the old bound
  left a margin, while two sinks of that file move from 0.00395 to 0.00366 ft.
