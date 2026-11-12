- **Each read of the saved-graphics pass visits only the partitions that hold
  a record it selects (#1694).** The handle's record index, which answers
  that without a walk, held only the elements' records (channel 102). So every
  read of the saved graphics (channel 103) walked every partition twice, once
  to find its records and once to decode them, whatever it selected. Each
  channel a read selects alone now has an index of its own, built by the first
  such read. The exports are unchanged.
