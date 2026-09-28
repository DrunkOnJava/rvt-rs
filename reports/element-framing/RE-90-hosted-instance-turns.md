# RE-90 — Turned instances hosted on ceilings and walls

**Date:** 2026-09-28
**Issues:** #227
**Result:**
- RE-87 turned a family instance only where its transform's Z axis is the model's. An instance hosted on a ceiling (Z down) or a wall (Z flat) also has one vertical axis, and its other two lie flat and give its plan direction. On Snowdon Towers that direction agrees with Revit's IFC4 placement, modulo a right angle, on every such instance whose Revit placement has a flat axis. The one exception is a Model Text element, which Revit places without a turn and which is not a family instance.
- Such an instance is now drawn as RE-87's turned box along its first flat axis. On Snowdon Towers that adds 58: 44 proxies and 14 light fixtures.

| Snowdon Towers' turned family instances, `turned_instances_vs_ifc.py`, against RE-89 | RE-89 | RE-90 |
|---|---:|---:|
| turned instances drawn turned | 181 | 239 |
| extents along the instance's own axes within 0.01 ft of Revit's body | 55 | 75 |
| within 1 ft | 178 | 218 |
| better / worse than RE-89, per element | | 44 / 0 |

-----

## 1. The direction

`InstanceTransform::plan_axis` returns the first flat axis of a transform that has a vertical one (up or down), and `None` for a tilted instance. Against Revit's placements (`instance_transforms_vs_ifc.py` data, Snowdon):

- Instances with a vertical axis but not upright, and turned off the model's axes: 44 proxies, 14 light fixtures and 133 members agree with Revit's placement modulo a right angle. The members are curtain-wall and framing parts, which keep their own bodies.
- The only disagreement among them is 2034755, a Model Text element (`Text3dElem`), which Revit places with no turn. It is not a `FamilyInstance`, so the attach leaves it alone.
- 806 instances have no vertical axis: 613 members, 177 plates and 16 proxies. They keep their box.

## 2. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

- **Snowdon Towers:** only the 58 instances change. 44 are closer to Revit's body along their own axes, and the 14 light fixtures score as their box did.
- **Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven and the MIT house (2024 and 2025):** IFC byte-identical apart from the timestamp.
