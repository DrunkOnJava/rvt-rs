- **A model with a stream over a resource limit no longer exports empty
  without a word (#1075).** A partition larger than 256 MiB, a compressed
  member that inflates past 256 MiB, or a partition that inflates past
  1 GiB in total used to be skipped by every reader, so a large client
  model came out with 0 elements, 0 storeys and exit status 0. Each such
  stream is now recorded (`RevitFile::unread_streams`), listed in the
  export diagnostics (`stream_not_read_resource_limit`, with a warning per
  stream), and `rvt-ifc` and `rvt-gltf` write no model and exit with
  status 3 unless `--allow-partial`. `--large-model` (streams up to 2 GiB,
  8 GiB inflated), `--max-stream-bytes` and `--max-inflate-bytes` raise the
  limits. Partition streams and the walker's `Global/Latest` now inflate
  under the file's `OpenLimits` instead of the defaults, so Python's
  `max_inflate_bytes` reaches them; a stream's inflated total is
  `max_inflate_bytes` or 1 GiB, whichever is larger. Python's `write_ifc()`
  and `rvt_to_ifc()` raise `ValueError` while a stream is unread unless
  `allow_partial=True`, and `RevitFile.unread_streams()` lists them. Not
  measured on the reporter's files, which are private.
