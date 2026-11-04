- **Partition streams are taken in the order Revit numbers them, not by
  name.** `RevitFile::partition_stream_names` sorted the `Partitions/NN`
  streams as text, which puts `Partitions/9` after `Partitions/46` and
  `Partitions/189` before it. So a reader that keeps the first or last copy
  of an element framed in several partitions could keep the wrong one
  when the partition numbers had different digit counts. Two cases had
  their own text comparison on top: the newest frame of an instance whose
  copies tie on vertical extent, and the first copy of a sketch line or a
  roof's sketch line (Revit 2024 and later).
  - `partition_stream_names` now sorts by partition number, and
    `partition_stream_name` returns the lowest-numbered stream.
  - The copy pickers compare partition numbers
    (`PartitionElementRecord::frame_order`).
  - Revit 2023 and earlier already compared numbers when picking a frame.
  - On every Measure model no export moves (see the pull request).
