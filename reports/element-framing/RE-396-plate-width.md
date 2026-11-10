# RE-396: a curtain panel drawn as a vertical box has its longer horizontal side for Width

**Date:** 2026-11-09
**Issues:** #1046 (base quantities, W7), #1016 (quantities); follows RE-387 (the volume of a body drawn from a mesh of flat faces) and #1191 (every element that would be a box is drawn from the mesh Revit saved for it)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them)
**Runs:** Measure [42676702980](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42676702980) and [42678202079](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42678202079) (`tools/re/mesh_features_vs_ifc.py`, commit `734cb1dba615`, on the code of the RE-387 head: which measures of a saved mesh equal which quantity Revit writes, and which objects Revit omits a quantity on); and [42857525344](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42857525344) (unfiltered: this change's code `6241f253` against `main` at `ae08c1da`, section 3)
**Status:** Positive for the office: a curtain panel drawn from a mesh that is a vertical box gets `Width`, the longer horizontal side of the box, and it is Revit's on 257 of 257 panels as built and 266 of 266 at permit, the 3 panels Revit gives none are not boxes and get none, and nothing is written that Revit's export omits. The rule rests on one project in two states (Revit 2024), which is all there is to check it on: no other model with Revit's export holds a curtain panel.

## 1. What Revit writes

The office's export writes `Qto_PlateBaseQuantities.Width` on 257 of its 260 curtain panels as built (266 of 269 at permit), alongside `NetVolume`, which RE-387 writes. rvt-rs wrote the `Width` of none.

## 2. The mesh against Revit's Width

`mesh_features_vs_ifc.py` takes the `IfcTriangulatedFaceSet` bodies rvt-rs writes, works measures of each out in the element's own frame (its volume, its extents along its axes, the sides of the smallest rectangle around its plan, the areas of its faces) and counts, per entity and quantity, how many of Revit's values each measure equals (a relative 1e-5, or 1e-6 where that is larger, as `qtos_vs_ifc.py` scores). On the office as built, of the 257 panels Revit gives a `Width`:

| Measure of the panel's mesh | Equal to Revit's `Width` |
|---|---:|
| the longer side of the plan's smallest rectangle (`obb_long`) | 257 |
| the extent along the element's x axis | 180 |
| the middle extent | 163 |
| the extent along its y axis | 77 |
| the largest extent | 94 |

Revit's `Width` is the longer side of the panel's plan, which is the x extent on 180 panels and the y extent on 77: it is a horizontal side, and where the panel is a vertical sheet of glazing (0.025 m, the case the three panels below show) it is the panel's horizontal length. The measures do not separate a vertical sheet from a horizontal one, whose plan is its face, so the rule below keeps to panels whose smallest extent is horizontal. The three panels Revit gives no `Width` (`System Panel:Lasitettu`, 0.025 by 1.5 by 2.4 m) are the three whose mesh is not a box: their volume is 0.5% under the product of their extents (0.09058 m3 against 0.09102 for the first), where the 257 others agree with it to the scorers' tolerance (257 of the 260 panels' volumes equal the box's). At permit the same three of 269 omit it.

## 3. What rvt-rs writes

`saved_meshes::attach_scoped_with_control` takes, of each curtain panel (`IfcPlate`) it draws from a mesh whose faces are all flat and which closes (RE-387), the extents of the corners the triangles use in the element's own frame (`mesh_volume::box_plan_width`): where the mesh's volume is the product of the three extents to 1e-4 and the smallest extent is not the vertical one, `Width` is the longer of the two horizontal extents. It is added to the panel's `Qto_PlateBaseQuantities` where it holds none.

### The change against `main`

Measure [42857525344](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42857525344), unfiltered (`nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`), this change's code `6241f253` against `main` at `ae08c1da`, 118 exports. 22 move (the office as built and at permit, RE1 Architecture and 20 Autodesk samples, whose curtain panels have no Revit export to score); the other 96 are byte-identical, held-out models included. Only the quantity census of the two office files (`qtos_vs_ifc`) and the MCP answers that quote it change; `decoded.parameter_value_count` and `exported.total_entities` are the same on both sides, and no body, property or other scorer moves.

| Export | Revit's quantities on objects rvt-rs writes | Equal, before to after | Different | Written that Revit's export does not |
|---|---:|---:|---:|---:|
| office as built | 39,754 | 31,904 to 32,161 | 1 to 1 | 0 to 0 |
| office at permit | 40,006 | 32,196 to 32,462 | 1 to 1 | 0 to 0 |

`Qto_PlateBaseQuantities.Width` goes from 0 to 257 of 257 equal as built and from 0 to 266 of 266 at permit, none missing. RE1 Architecture's export writes no base quantities (RE-242), so the census counts what rvt-rs writes beside them apart (its panels' `Width`, which `rvt-ifc --no-base-quantities` leaves out); the 20 samples gain the values and nothing else. Only the office exercises the rule, so the 3 panels that are not boxes are the only place it is shown to leave a value out.

**This lifts a leave-out.** The census page said, from 2026-10-26, that a curtain panel's `Width` is left out because Revit's export gives none on 3 panels that nothing tells apart (RE-247). The box test of section 3 tells them apart (their volume is 0.5% under the product of their extents), as RE-291 did for a joined column's `Length` on 2026-11-03, so the leave-out becomes a rule.

## 4. Not shown

- **Panels that are not vertical boxes.** A horizontal or sloped panel, a panel with an angled end and one with a cut-out get none: Revit's value for them is not measured (the office's 260 panels are 257 vertical boxes and 3 other shapes, none of which Revit gives a `Width`).
- **Other projects.** No other model has curtain panels with a Revit export measured, so the rule rests on one project's two states, Revit 2024, and on the 257 panels one glazing type's family covers.
- **`Perimeter`, `GrossArea`, `NetArea`** of the panels: Revit's export writes none of them on the office's panels.

## 5. Reproduce

```bash
# The scorer, from its scratch commit (the scorers come from --ref), on the code of the RE-387 head:
gh workflow run measure.yml --ref measure/mesh-features -f ref=a82047aba5850c4ced3c6240c8aac4af307cdbed \
    -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-permit" \
    -f scorers=mesh_features_vs_ifc
# The change against main, unfiltered:
gh workflow run measure.yml --ref main -f ref=6241f253b93eedb6ad6822ee93ea4cc93db33d5a -f base=ae08c1dad8e1e7c10ed06ccadb1a1112e186384f \
    -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
```
