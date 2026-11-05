- **A click in the viewer picks what is drawn (#1422).** A raycast does not look
  at `visible`, so with a category hidden a click on what is drawn behind it
  selected the hidden element instead, and a category hidden before the meshes
  arrived stayed drawn. A new browser test,
  `viewer/tests/folded-pieces.spec.ts`, opens the BIM4LCA office as built, hides
  every category but one, and clicks until a click lands on a piece folded into
  another element's body, read from the raycast's own hit: the panel, GlobalId
  and tree row are the owner's, it states no size, and the category and schedule
  counts are the elements the export writes, and a stair flight shows the storey
  of its stair. It runs in the viewer's browser
  tests on a hash-checked copy of the model, and says NOT MEASURED if the host
  of that copy cannot be reached. Before this, nothing in a browser exercised a
  folded piece.
