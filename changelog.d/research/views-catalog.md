- **Views, sheets and placed viewports** (research, behind `unstable-research`):
  `rvt::research::views::read` and the `rvt-views` CLI, which builds only with
  that feature (`cargo install rvt --features unstable-research`), list every
  view's name, class, scale, plane and 3D camera,
  every sheet's number and title-block fields, and every viewport's sheet,
  view and box centre. The centre rule was measured on one viewport against
  the Revit API (#1117); the catalog is experimental, and a turned viewport,
  an uncropped view and a view title's printed position are not read.
