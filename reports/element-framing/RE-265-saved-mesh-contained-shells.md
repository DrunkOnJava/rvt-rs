# RE-265 — Why four saved door meshes enclose a quarter more than Revit's bodies

**Date:** 2026-10-29 (corrected the same day: the first version took the second shell for one lying inside the first)
**Issues:** #1103; follows RE-78 (saved display meshes), RE-224 (partial meshes) and #1089 (doors and windows drawn from their saved mesh by default)
**Artefacts:** the BIM4LCA office as built and at building permit (CC BY-SA 4.0, `research/bim4lca-measure.tsv`), Core Interior and RE1 Architecture, as Measure fetches them
**Runs:** Measure [40951777272](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40951777272) (volume outliers, `tools/re/mesh_bodies_vs_ifc.py`), [40952882706](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40952882706) (`examples/probe_re265_saved_mesh_duplicates.rs`), [40953845870](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40953845870) (each outlier's shells), [40971574172](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40971574172), [40973525033](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40973525033) and [40975701952](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40975701952) (`examples/probe_re265_contained_shells.rs` with `tools/re/door_items_vs_ifc.py`)
**Result:** negative for an rvt-rs defect. The four doors' bodies are Revit's: the same four parts as Revit's export, each of the same volume once the leaf is cut where the glass sits. The quarter is the scorer's: Revit's export voids each door with its own opening, and IfcOpenShell subtracts that opening from every part of the door, the glass with it.

## 1. The doors

With saved meshes on, four office doors have every vertex within 2.2e-06 ft of Revit's body in both directions and 1.2455 or 1.2501 times its enclosed volume: 1382481 (`O-7:O-7.12x21`), 1623922 (`UO-3:UO-3.12x21`), 3448563 and 3444654 (`O-7:O-7.12x23`), the first two on both versions of the office.

## 2. Not a repeated or reversed shell

No saved mesh repeats a primitive or a triangle (vertices rounded to 1e-6 ft): 0 of 11,138 on the office as built, 0 of 56 on RE1 Architecture, 0 of 138 on Core Interior. 137 office meshes and 3 on RE1 hold a triangle and its reverse (two-sided sheets); none of the four doors does.

## 3. The four parts on both sides

Each door's saved mesh is four saved-graphics objects, each drawn in one material; Revit's export of the door is four `IfcExtrudedAreaSolid` in one mapped `Body` representation. Their signed volumes (1382481; 3448563 and 3444654 alike at 1.1092, 0.6302, 0.0531 and 5.1388 against 1.3578):

| Part | rvt-rs object | rvt-rs (cubic ft) | Revit item | Revit (cubic ft) |
|---|---|---:|---|---:|
| Frame, `Metalli - teräs, valkoinen` | 52, 28 triangles | 1.0702 | #77534, 28 triangles | 1.0702 |
| Glass, `Lasi - 1 lasilevy, kuvio`, 2.92 × 0.033 × 6.25 ft | 56, 12 triangles | 0.5987 | #77540, 12 triangles | 0.5987 |
| Trim | 59, 12 triangles | 0.0531 | #77552, 12 triangles | 0.0531 |
| Leaf | 57, 32 triangles | 1.3154 | #77546, 12 triangles | 4.9079 |

Three parts are equal to the fourth decimal. The leaf differs by 3.5925 cubic ft, the volume of a through-hole the glass sits in: Revit's saved display mesh draws the leaf with the hole cut, and Revit's export writes the leaf as an uncut block.

## 4. Where Revit's export cuts the hole

Revit's export voids the door with its own `IfcOpeningElement` (`O-7:O-7.12x21:1382481:1`, by `IfcRelVoidsElement`), the family's cut through its leaf. By IFC's rules an opening is subtracted from the whole of the element it voids, and IfcOpenShell does so: the body `mesh_bodies_vs_ifc.py` scores is the four items less the opening, the glass removed with the hole, 1.0702 + 4.9079 + 0.0531 − 3.5925 = 2.4387 cubic ft. rvt-rs's body, Revit's displayed door, is 1.0702 + 1.3154 + 0.5987 + 0.0531 = 3.0374, which is the 1.2455. So rvt-rs's door has every part Revit's export has, cut as Revit draws it; read by IFC's rules, Revit's own export has no glass in it.

Nothing is changed in rvt-rs. A scorer comparing volumes would have to count apart an element voided by its own opening; nothing else measured differs this way.

## 5. Also seen

The ramp flight 1440596 (`Ramp:AP-ulkoluiska`, drawn from its saved mesh only with `--saved-meshes`) has one shell on each side with the same surfaces, rvt-rs's enclosing 6.4909 cubic ft and Revit's 12.9817, exactly twice: Revit's mesh most likely writes every face twice. Kitchen sinks (`Sink Kitchen-Single:PA-5`) differ by 1.1% to 1.3% with vertices 0.011 ft apart, a different case, left as measured.
