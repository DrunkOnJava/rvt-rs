- **The native readers' shared preamble is read in less time (#1335).** Before
  its first pass over a file's records every native reader reads the schema,
  the history, the element index and the extensible storage catalog. The
  element index no longer searches its layout for the leaves of each of its
  thousands of rows, the schema's checksums no longer format a string for each
  byte, and the catalog reads the document it takes its schemas from without
  building the values of the fields it ignores. No export changes. The figures
  are in the pull request.
