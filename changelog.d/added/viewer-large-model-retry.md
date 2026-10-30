- **The viewer retries a model its limits stopped, with the large-model limits
  (#1075).** A stream over the reader's resource limits was named in the File
  status panel ("Part of the file not read") but the browser could only point
  at `rvt-ifc --large-model`. The panel now offers "Retry with large-model
  limits", which reads the same file again with the stream and inflate limits
  `rvt-ifc --large-model` uses (streams up to 2 GiB), and says a browser tab
  can use at most 4 GiB of memory, so a very large model can still fail to
  open. It comes from one new wasm binding,
  `openRvtBytesWithDiagnosticsModeAndOpenLimits(bytes, mode, limits)`, where
  `limits` is `{ largeModel?, maxStreamBytes?, maxInflateBytes? }` or `null`
  for the defaults; `openRvtBytesWithDiagnosticsMode` is unchanged. The viewer
  also reads `?maxStreamBytes=` and `?maxInflateBytes=` from its address for
  the first open, which is how the browser test reaches the path on a 20 KiB
  fixture with `?maxStreamBytes=200`.
