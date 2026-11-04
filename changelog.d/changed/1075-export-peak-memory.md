- **A large export no longer holds several copies of the same bytes at its
  peak (#1075).** The peak of `rvt-ifc` and `rvt-gltf` on the reporter's
  438 MB project (10.3 GB at the export stage) was not measured here, since
  the file is not available; the figures below are from the BIM4LCA office
  as built (Revit 2024, 90,842 entities), whose stages a counting allocator
  sizes in exact heap bytes. Four copies are gone. Building the handle's
  cache of inflated partitions held each partition twice (every gzip
  member's output beside the joined copy) and the stored stream beside its
  stripped copy: members now inflate straight into the one buffer and the
  stored pages strip in place. Every native pass copied each partition it
  read and joined each continuation group into a buffer of its own: a pass
  strips in place, and a group of consecutive members of the cache is
  borrowed from it. The partition's string records were scanned in a copy of
  the whole partition, joined by a separator: each chunk is scanned where it
  lies, after the records that begin in the separator before it. The saved
  graphics of doors and windows decoded every symbol they share before
  meshing the first door: a batch of symbols (about 20,000 graph objects) is
  decoded now, the doors and windows that name it meshed, and the batch
  freed before the next. The heap at the highest point of each stage, in MiB
  (main in run 41778285915, this change in run 41781699897 at d1fc5d3c):
  inflating the 361 MiB partition 901 to 633 (resident 907 to 489), the
  partition strings 838 to 459, the export 848 to 770, the saved graphics
  1,568 to 868, so the highest stage of the whole export from 1,568 to 868.
  The IFC of the office as built, at building permit and both terrain
  variants, and of Core Interior, is byte-identical to main's in that run;
  the unfiltered Measure run is the check on the rest. The probes
  `probe_1075_heap_stages` (exact heap bytes per stage),
  `probe_1075_heap_sites` (the call sites that hold the heap, by dhat) and
  `probe_1075_saved_batches` (the batched saved graphics against the
  one-shot read, element by element) reproduce them.
