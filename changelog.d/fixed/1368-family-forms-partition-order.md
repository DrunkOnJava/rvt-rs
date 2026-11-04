- **A family's embedded document is read in partition-number order (#1368).**
  The reader of the records of a family's document (the family extrusions,
  the nested instances and the materials of a family) took the partitions in
  name order, so a file with partitions of unequal digit count
  (`Partitions/9`, `Partitions/10`) would read a record held in both from the
  older copy as the newest. It takes them as `RevitFile::partition_stream_names`
  does.
