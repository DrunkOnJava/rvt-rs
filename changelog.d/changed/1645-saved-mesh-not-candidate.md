- **Every box the saved-mesh pass leaves says why, also one it did not look
  at (#1645).** An element drawn as a record box, a beam's box or a turned box
  that the pass did not take as a candidate (a stair or curtain wall whose
  parts draw it, an element with no location or no ElementId, a class outside
  the scope a caller asked for) now has `not_a_candidate: ` and the reason as its
  `SavedMeshRefusal`, so that `exported.saved_mesh_refusals` counts every such
  box and not only the candidates whose saved graphics were read. No body is
  drawn differently.
