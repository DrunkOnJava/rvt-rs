- **The progress an open reports covers the recovery over the element
  records (#1195).** After the last `Partitions/*` stream was inflated, the
  decoder reported nothing until the saved graphics, and on Core Interior
  that stretch was most of an open: the viewer's bar swept with "Building the
  model". A new stage, `partition-recovery`, counts the recovery's steps
  (`rvt::control::Stage::PartitionRecovery`, printed by `rvt-gltf --progress`
  and passed to the viewer's `progress` function), and the viewer's card shows
  it as "Reading element records" with its count. The last stream hands
  straight over to it, so the card does not name what comes after the streams
  for an instant. No export changes.
