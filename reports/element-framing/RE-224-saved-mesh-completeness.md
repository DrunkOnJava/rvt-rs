# RE-224 — What the saved scene records about a partial saved mesh

**Date:** 2026-10-25
**Issues:** #255 (saved graphics), W6 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-78 §Shapes
**Run:** Measure [40227454417](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40227454417), `examples/probe_re224_saved_mesh_status.rs` with `tools/re/saved_mesh_status_vs_ifc.py`, `--saved-meshes`, `keep_ifc`, `-f nordic=true`, at `re/saved-mesh-status` 14325427
**Result:** partial. A decode failure the scene records marks a partial mesh, and never a complete one on six of the seven models; but some partial meshes carry no record at all, so the option stays opt-in. `--saved-meshes` now keeps the box where part of the saved graphics did not decode.

## 1. The question

RE-78 found the saved display meshes exact where they are complete and only part of Revit's body elsewhere, and nothing in the file to tell the two apart, so `--saved-meshes` is opt-in. `native_saved_scene` already keeps, for each element, what its graphics walk could not read:

- decoder diagnostics: a face that did not tessellate (its surface or trim not supported), a graphics class the walk does not know, a branch whose visibility depends on the view;
- unbounded faces, filters rejected at the detail level, branches hidden by their visibility flags, curve-only branches;
- a status, `partial_saved_graphics` where any diagnostic or unbounded face is left.

`saved_meshes::attach` read none of it. The probe prints that record for every element `attach` replaces; the scorer meshes each body and Revit's body for the same Tag and calls it exact when every tested vertex of each lies within 0.001 ft of the other's surface (RE-78 §Shapes), then sets each part of the record against it.

## 2. Measured

Bodies scored (a class over 300 bodies on an even stride of 300 by Tag, as RE-78):

| file | scored | exact | not exact | not exact with a decode failure | not exact with no record |
|---|---:|---:|---:|---:|---:|
| Core Interior (2024) | 138 | 138 | 0 | 0 | 0 |
| BIM4LCA office as built (2024) | 1,709 | 1,294 | 415 | 357 | 58 |
| BIM4LCA office at permit (2024) | 1,797 | 1,376 | 421 | 359 | 62 |
| RE1 Architecture (2025) | 56 | 45 | 11 | 5 | 6 |
| RE1 Electrical (2025) | 49 | 31 | 18 | 2 | 16 |
| RE1 Mechanical (2025) | 68 | 65 | 3 | 0 | 3 |
| RE1 Plumbing (2025) | 41 | 7 | 34 | 0 | 34 |

A decode failure is any diagnostic but two. The faces that did not tessellate on the office are cylinders with a left-handed basis (168), non-rectangular cylinder and parametric trims (87, 33), several cylinder trim loops (23), ellipse profiles (49), ruled profiles needing a curvature policy (43), `HermiteSurf` surfaces (6), disconnected UV boundaries (8), missing surface pointers (8), and the graphics classes `GImposter` (43) and `GBiFlipControl` (18). On Core Interior, both office files and RE1 Architecture, Mechanical and Plumbing, no mesh with one is exact.

The two others mark a branch the walk leaves out for its view semantics, and the mesh without it is mostly the body:

| diagnostic | with it, exact | with it, not exact |
|---|---:|---:|
| nested detail-family filter requires view semantics (office as built) | 271 | 35 |
| surface-bearing component requires qualified owner/view visibility (RE1 Electrical) | 21 | 0 |

Unbounded faces (192 exact, 3 not on the office), rejected filters, hidden branches and curve-only branches split exact and inexact meshes alike, so none of them is a signal.

RE1 Electrical's two exact meshes with a decode failure carry `unsupported normal binding` and `GImposter`; the office's 43 `GImposter` meshes are all inexact, so the class stays a failure.

## 3. What rvt-rs does

`saved_meshes::attach` keeps an element's record box where its saved graphics carry any diagnostic but the two view-semantics ones (`NESTED_DETAIL_FILTER_DIAGNOSTIC`, `VIEW_COMPONENT_DIAGNOSTIC` in `native_graphics_traversal`), and counts it in `SavedMeshReport::partial`; `rvt-ifc --saved-meshes` prints that count.

Measure [40231871182](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40231871182), this change against `main` with `--saved-meshes`, `mesh_bodies_vs_ifc`:

| file | bodies drawn from a saved mesh, before → after | exact (0.001 ft) | kept their box |
|---|---|---:|---:|
| Core Interior | 138 → 138 | 138 → 138 | 0 |
| office as built | 1,709 → 1,352 scored (10,781 drawn of 11,259) | 1,294 → 1,294 | 357 |
| office at permit | 1,797 → 1,438 scored (10,896 drawn of 11,374) | 1,376 → 1,376 | 359 |
| RE1 Architecture | 56 → 51 | 45 → 45 | 5 |
| RE1 Electrical | 49 → 45 | 31 → 29 | 4 |
| RE1 Mechanical | 68 → 68 | 65 → 65 | 0 |
| RE1 Plumbing | 41 → 41 | 7 → 7 | 0 |

On the office 1,294 of the 1,352 bodies drawn from a saved mesh are now Revit's, against 1,294 of 1,709. RE1 Electrical gives up two exact meshes whose `GImposter` and normal binding did not decode. The default export, without `--saved-meshes`, is unchanged byte for byte.

## 4. What stays open

- The meshes that are partial with nothing recorded: the office's 58, RE1 Plumbing's 34 pipe fittings (median volume 1.22 times Revit's, RE-78), RE1 Electrical's 16. Until a signal for them is found, the option stays opt-in, and the default export keeps record boxes.
- Whether the saved meshes become the default is a separate decision: they cost size, memory and time (RE-78: Snowdon Towers 12 MB to 111 MB, 5.6 GB peak), and the browser viewer, with its 4 GiB ceiling, keeps them off whatever the CLI does.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref feat/saved-mesh-partial-gate -f ref=feat/saved-mesh-partial-gate -f base=none \
  -f nordic=true -f keep_ifc=true -f ifc_flags=--saved-meshes \
  -f probe=probe_re224_saved_mesh_status -f probe_scorer=saved_mesh_status_vs_ifc
```
