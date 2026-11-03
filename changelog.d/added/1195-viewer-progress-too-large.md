- **The browser viewer shows the decoder's own progress, and says when a
  model is too large for a tab.** The loading card names the stage the
  decoder is in (the stages `rvt-gltf --progress` prints) and fills its bar
  with that stage's count, where it used to sweep and guess "about N s" from
  the file's size. `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes an
  optional fourth argument, a function called with the stage, the count done
  and the count it is out of. A file whose size says it may need more than a
  tab's 4 GiB (about ten times its size, the figure of the largest reference
  model) is warned about before the decode and still tried. When the wasm
  instance runs out of memory, the viewer says the model is too large for the
  browser, how far the decode got, and that `rvt-ifc` and `rvt-gltf` read
  models of any size, instead of `RuntimeError: unreachable` (#1195).
