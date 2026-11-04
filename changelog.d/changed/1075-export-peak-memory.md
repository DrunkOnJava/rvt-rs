- **A large model exports in a fraction of the memory it did.** The peak
  of `rvt-ifc` and `rvt-gltf` on a very large project (#1075: 10.3 GB on a
  438 MB file) was several copies of the same bytes held at once, not the
  model. Four are gone, and every export is byte-identical to before.
  Building the handle's cache of inflated partitions held each partition
  twice (every gzip member's output beside the joined copy), and the stored
  stream beside its stripped copy: members now inflate straight into the one
  buffer and the stored pages strip in place. Every native pass copied each
  partition it read, and joined each continuation group into a buffer of its
  own: the pass strips the stored bytes in place, and a group of consecutive
  members of the cache is borrowed from it. The partition's string records
  were scanned in a copy of the whole partition, joined by a separator: each
  chunk is scanned where it lies. The saved graphics of doors and windows
  decoded every symbol they share before meshing the first door; a batch of
  symbols is decoded now, the doors and windows that name it meshed, and the
  batch freed before the next. On the BIM4LCA office as built (Revit 2024,
  90,842 entities) the heap held at the highest point of a stage fell, by
  the probe's exact count: inflating the 361 MiB partition from 901 to
  633 MiB (resident 907 to 489) and the saved graphics from 1,568 to 996 MiB
  before the batches were sized down. `rvt::compression` gains two public
  functions, `prepare_stream_owned` and
  `strip_revit_page_checksums_in_place`. The probe `probe_1075_heap_stages`
  measures each stage in exact heap bytes, and `probe_1075_heap_sites` names
  the call sites that hold the heap (dhat).
