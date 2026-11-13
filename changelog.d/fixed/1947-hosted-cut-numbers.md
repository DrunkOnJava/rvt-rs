- **A face-hosted cut's opening keeps its place among its host's openings
  when an earlier one is not written (#1947).** Revit's export names each
  opening a door or window cuts in a host after the host and its place among
  that host's cutting instances, the highest ElementId first. rvt-rs counted
  only the openings it wrote, so a cut declined before one (no transform read,
  not an upright prism, a host drawn otherwise) shifted the name and GlobalId
  of every opening after it. Each cut now takes its number before any is
  declined. Found by the batch 9 export audit.
