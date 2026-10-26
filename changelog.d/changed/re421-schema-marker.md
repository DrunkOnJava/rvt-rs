- **The element-record marker of Revit 2024 to 2027 files comes from each
  file's own schema (#421).** Every scan that needs it (element names, stair
  names, type records, Level records) takes the `[Outline][0xFF x4]
  [ElementParents]` marker from the file's `Formats/Latest` (RE-80), and the
  release's measured constant (`bbox_marker`) is the cross-check: a file
  whose schema gives another marker is not scanned, one whose schema does
  not parse keeps the constant. On all 84 2024 to 2027 files measured (5,
  68, 4 and 7) the two agree, and the marker's `ElementParents` tag less 12
  is the schema's `ElementHeader` tag. On 2016 to 2018 that gap is 9, not
  12, so `record_prologue_constant` holds only from 2019
  (`tests/schema_bbox_marker.rs`).
