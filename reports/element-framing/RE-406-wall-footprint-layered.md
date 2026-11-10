# RE-406: Revit's export writes no `GrossFootprintArea` on a wall whose type has several layers

**Date:** 2026-11-10
**Issues:** #1636 (the second-project measurement), #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-252 and RE-292 (a wall's `GrossFootprintArea` from its box) and RE-402 (the first reading of that rule on the BIM4LCA housing and timber office)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), and rvt-rs's export of it by `main` at `237d6ef6`, which Measure [42906770416](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906770416) kept (`keep_ifc`); the other BIM4LCA files, the Revit 2025 models and the samples through Measure's census
**Runs:** Measure 42906770416 for section 2; the unfiltered run of this change for section 4
**Status:** In progress: section 2 is read from the one pair that was kept; the unfiltered Measure of this change has not run, and sections 4 and 5 are written when it reads.

## 1. What RE-402 left open

RE-292 writes `GrossFootprintArea`, a wall's length times its thickness, on every wall whose box is its solid (RE-270, RE-293) except the shortest, and measured it equal to Revit's on the 186 of the 188 such walls that Revit's export gives one, none different and none that Revit's export leaves out. RE-252 found Revit writes it only where its exporter draws the wall as an extrusion. On the four large housing files RE-402 found it written on 36, 2, 20 and 10 walls whose export Revit writes none for (RE-402 section 4).

## 2. What the walls that differ are

Revit's export of the concrete as built holds 369 walls that rvt-rs's export holds too. 153 of them have a quantity set with one complex quantity per layer (a wall of a type with several layers), and 216 do not. Revit writes `GrossFootprintArea` on 204 of the 216 and on none of the 153; the other 12 of the 216 hold none of `GrossFootprintArea`, `Height` and `GrossSideArea`. rvt-rs writes it on 36 of the 153.

The layers of the wall's type in rvt-rs's export (the layer set or the constituent set it is associated with) separate the two sets of walls:

| Revit's quantity set | layers in rvt-rs's export | walls | rvt-rs writes `GrossFootprintArea` |
|---|---|---:|---:|
| one complex quantity per layer (153) | none (no layer set or constituent set) | 33 | 0 |
| | 2 | 24 | 15 |
| | 3 or more | 96 | 21 |
| none (216) | none | 1 | 0 |
| | 1 | 215 | 94 |

Every one of the 120 layered walls with their layers associated has two or more, and none of the 215 walls of one layer does.

## 3. The change

`export_content::add_wall_areas_and_volumes` writes `GrossFootprintArea` only on a wall whose type's layers (the bands of `m_wall_layers`, `partition_schema_mvp::wall_layer_count_from_fields`) number fewer than two. The table counts the layers of the association; the rule counts the bands the wall's record gives, which the association is made from (RE-88), and section 4 reads the two against Revit's.

## 4. Measured

The unfiltered run of this change against `main` (every Measure model) is dispatched from `main`, whose workflow holds the housing and the timber office; its readings go here when it finishes.

## 5. Not shown

- **The 33 layered walls with no association.** They carry none in rvt-rs's export, and none of them has a footprint written today; the rule reads the bands' count, not the association.
- **Membranes.** A layer with no width is left out of the bands (`layer_bands_field`), so it is not counted; whether Revit's export counts one as a layer is not read.
