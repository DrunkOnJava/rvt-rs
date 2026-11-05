- **A family's embedded document is read in partition-number order (#1368).**
  The family extrusions, the nested instances and the materials of a family
  read the records of the family's document partition by partition. They took
  the partitions in name order, which is not number order where the numbers
  have unequal digit counts (`Partitions/100` before `Partitions/99`), so the
  copy of an extrusion record held in several partitions that won, and the
  order the records were read in, followed the names. They take the partitions
  as `RevitFile::partition_stream_names` does, the newest copy last.
