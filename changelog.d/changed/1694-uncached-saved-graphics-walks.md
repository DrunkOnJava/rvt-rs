- **The saved-graphics pass of a default export keeps no inflated copy of the
  partitions it walks (#1694).** The handle held every inflated partition at
  the pass's peak: 36% to 43% of the peak resident memory of the largest
  files. The pass now indexes its records from those copies before letting
  them go. Each of its reads then inflates only the groups that hold a record
  it selects. On the largest files one partition holds nearly every group, so
  inflating whole partitions cost a walk of the file for each read.
