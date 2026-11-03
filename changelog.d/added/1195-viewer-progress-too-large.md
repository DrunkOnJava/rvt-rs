- **The browser viewer shows the decoder's own progress, and says when a
  model is too large for a tab.** The loading card names the stage the
  decoder is in (the stages `rvt-gltf --progress` prints) and fills its bar
  with that stage's count, where it used to sweep and guess "about N s" from
  the file's size. `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes an
  optional fourth argument, a function called with the stage, the count done
  and the count it is out of. `rvt-gltf --progress` now also prints a
  `partition-scan` line for each `Partitions/*` stream as it is first
  inflated, which is most of a project's open, where it printed the stage's
  start and end only. A file whose size says it may need more than a tab's
  4 GiB (at least about ten times its size, the figure of the largest
  reference model; the warning can fire on a file that fits) is warned about
  before the decode and still tried. When the wasm instance runs out of
  memory, the viewer says the model is too large for the browser, how far the
  decode got and where, and that `rvt-ifc` and `rvt-gltf` with `--large-model`
  (streams up to 2 GiB and, on a 64-bit machine, inflating up to 8 GiB, for a
  file of up to 2 GiB), or Python with a larger `max_stream_bytes` and
  `max_inflate_bytes`, can read a larger file, instead of
  `RuntimeError: unreachable`. A stop a long way short of the ceiling says
  "Ran out of memory" and that the file may be damaged, and a panic that is not
  a memory failure keeps its own message (#1195).
