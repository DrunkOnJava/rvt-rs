- **`rvt-gltf --progress`.** The conversion prints its progress to stderr
  in the `progress: <stage> <done>/<total>` lines of `rvt-elements
  --progress`, and `WalkerControl` gains two stages for saved graphics:
  `saved-graphics` (reading them, 0 then 1/1) and `saved-meshes` (one line
  per element meshed). A cancelled token stops the export with
  `Error::Cancelled`. The .glb is byte-identical with and without the flag.
