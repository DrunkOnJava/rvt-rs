- **The saved-graphics pass of a default export keeps no inflated copy of the
  partitions it walks (#1694).** It inflates each partition as it walks it, once
  for each batch of owners, where the handle held every inflated partition at
  the pass's peak: 36% to 43% of the peak resident memory of the largest files.
