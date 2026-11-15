- **The partition scanner stops at 16,777,216 candidates by default and says
  so (#1833).** `ScanOptions::default()` kept every schema tag followed by two
  zero bytes, up to one for every four bytes of a stream, and each candidate
  holds about 250 bytes, so a crafted stream could take about 67 GB of
  candidates from the 1 GiB the reader lets a stream inflate to. Real files keep
  millions (at most 9,675,757 in a file and 6,531,651 in a stream, on the 28 files
  of Revit 2023 and 2024 of Measure run 43663157788), so the cap leaves 1.7 times
  the most any of them keeps and cuts none. A scan that reaches it returns the
  first candidates in stream order with `PartitionScan::truncated` set; a caller
  that needs more sets `max_candidates` to `None`. Nothing in the export or the
  CLIs calls the scanner, so no export changes.
