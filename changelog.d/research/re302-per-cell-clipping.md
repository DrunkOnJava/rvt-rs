- **Clipping a trimmed loop to its grid cells takes the BIM4LCA office back
  under the schema step's size limit, and the readers stay off `main`
  (#1247, RE-302).** RE-302's readers (ellipse profiles, curved ruled
  surfaces, trimmed parametric faces) draw the office's 43 `WC-istuin:WC-1`
  toilets, which `main` keeps as boxes. With each earcut sliver cut at every
  grid knot they took the office as built from 568,885 to 2,097,501
  triangles and its IFC from 82.1 to 181.8 MB, over the 120 MB below which
  the schema step validates. With each loop clipped to the grid cells it
  covers and the pieces' corners shared they take it to 1,191,138 triangles
  and 104.6 MB (the permit file to 1,195,576 and 104.6 MB), validated again
  with the 4 rule findings `main` has; a toilet is 14,471 triangles against
  Revit's 1,668. They do not land: the toilets are 1.006 of Revit's volume
  but fail the 0.01 ft bar and "ref on ours" (0 of 43), and the default
  export, which they should not change, moves on one held-out model (14
  more of its windows drawn from saved meshes, all within the #1280 bar).
  Measured against `main` on 19 `--saved-meshes` pairs and 111 default
  export pairs (Measure 41849309128 and 41849313875); the readers stay on
  the draft #1361. Report: `reports/element-framing/RE-302-ellipse-profiles-and-trimmed-parametric-faces.md`
  (§6 and §7).
