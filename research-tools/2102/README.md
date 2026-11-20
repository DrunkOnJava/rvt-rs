# The reads behind RE-494 (#2102)

Scratch tools for the reads RE-494 reports, kept here as they were run (2026-11-19 and 2026-11-20). They are not maintained and not in CI. Each takes IFC exports (rvt-rs's from a Measure run with exports kept, and Revit's own) and prints counts; none writes model content out. Run each with IfcOpenShell and NumPy, for example `uv run --no-project --with ifcopenshell --with numpy python -I <tool> ...`; the VIM tools load `tools/re/footing_meshes_vs_vim.py` and the site frame comes from `tools/re/ifc_common.py`.

| tool | what it reads |
|---|---|
| `edges.py` | an element's connected shells: volume, area, extents, open and same-way edges (the shell table) |
| `slice.py` | a bent bar's sections square to its leg: points, corners, area, corner radius (`legs.py` holds the shell split) |
| `place.py` | every Revit vertex beyond 0.001 ft of rvt-rs's surface: inside or outside the nearest closed shell, Revit's shell, position |
| `barbox.py` | a bar shell's box in the site-undone frame and its tips |
| `whichshell.py` | which shell a listed point is on |
| `items.py` | the representation items an export wrote for an element |
| `pair.py` | both directions' vertex-to-surface distances and the volume ratio for listed Tags |
| `box_check.py`, `box_axes.py`, `overhang.py` | newly drawn bodies against the record box the base drew: overhang and gap per axis, and which shell reaches past (`ifc_body_diff.py` is `tools/re`'s, copied beside them) |
| `shells_vs_vim.py`, `extents_vs_vim.py` | an element's shells and extents in rvt-rs's export and in the VIM |
