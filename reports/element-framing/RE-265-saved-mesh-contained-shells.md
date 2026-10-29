# RE-265 — Why four saved door meshes enclose a quarter more than Revit's bodies

**Date:** 2026-10-29
**Issues:** #1103; follows RE-78 (saved display meshes), RE-224 (partial meshes) and #1089 (doors and windows drawn from their saved mesh by default)
**Artefacts:** the BIM4LCA office as built and at building permit (CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior and RE1 Architecture, as Measure fetches them
**Runs:** Measure [40951777272](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40951777272) (volume outliers, `tools/re/mesh_bodies_vs_ifc.py`), [40952882706](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40952882706) (`examples/probe_re265_saved_mesh_duplicates.rs`), [40953845870](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40953845870) (each outlier's shells and their signed volumes)
**Result:** positive. The four doors' saved meshes hold a second closed shell lying wholly inside the first; the first alone is Revit's body. Nothing in any saved mesh measured is a repeated shell or a repeated triangle.

## 1. The doors

With saved meshes on, four office doors have every vertex within 2.2e-06 ft of Revit's body in both directions and 1.2455 or 1.2501 times its enclosed volume: 1382481 (`O-7:O-7.12x21`), 1623922 (`UO-3:UO-3.12x21`), 3448563 and 3444654 (`O-7:O-7.12x23`), the first two on both versions of the office.

## 2. Not a repeated or reversed shell

The probe keys each primitive of a saved mesh by its triangles (vertices rounded to 1e-6 ft) and each triangle by its corners in winding order. No mesh repeats a primitive or a triangle: 0 of 11,138 on the office as built, 0 of 56 on RE1 Architecture, 0 of 138 on Core Interior. 137 office meshes and 3 on RE1 hold a triangle and its reverse (two-sided sheets); none of the four doors does.

## 3. A shell inside a shell

Grouping each body's triangles into shells joined through shared vertices, and summing each shell's signed volume:

| ElementId | rvt-rs shells (cubic ft) | Revit's shells (cubic ft) |
|---|---|---|
| 1382481, 1623922 | 2.4387 and 0.5987 | 2.4387 |
| 3448563, 3444654 | 2.5201 and 0.6302 | 2.5201 |

The larger shell equals Revit's single shell to the fourth decimal, and the smaller one's vertices all lie on Revit's surface, so it sits inside the larger one with faces flush with it: a part of the door's family Revit's export writes as one solid with the leaf. #1108 leaves out a shell enclosing positive volume whose interior lies inside a larger shell of the same body.

## 4. Also seen

The ramp flight 1440596 (`Ramp:AP-ulkoluiska`, drawn from its saved mesh only with `--saved-meshes`) has one shell on each side with the same surfaces, rvt-rs's enclosing 6.4909 cubic ft and Revit's 12.9817, exactly twice: Revit's mesh most likely writes every face twice. Kitchen sinks (`Sink Kitchen-Single:PA-5`) differ by 1.1% to 1.3% with vertices 0.011 ft apart, a different case, left as measured.
