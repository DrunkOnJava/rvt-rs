- **The body scorers judge a curve in the fine tessellation (#1365, RE-331
  section 8).** On #1355 the maintainer chose "Fine where both draw curves":
  `mesh_bodies_vs_ifc`, `family_extrusion_bodies_vs_ifc` and
  `saved_meshes_vs_ifc` now take the verdict ("ref on ours", both directions
  within 0.001 ft, the boxes) of a body from a mesh at one equal, fine
  deflection when both files write it as true curves (an extrusion of a circle,
  of an outline with an arc and the like, and no mesh item, read from each
  file's own representation items by `tools/re/true_curves.py`), and from
  IfcOpenShell's default tessellation otherwise. A saved mesh is triangles on
  rvt-rs's side, so it is never in the set: no saved-mesh, door or window
  verdict moves. Only the bodies in the set are meshed finely. The rows say
  which tessellation decided: a line counts the bodies, the class tables gain a
  last column, the census rows end `decided by the fine tessellation`, and the
  `fine` rows of #1355 show those bodies in both tessellations. Without such a
  body the output is what it was. Report:
  `reports/element-framing/RE-331-scorer-tessellation.md` section 8.
