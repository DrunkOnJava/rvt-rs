# RE-151: Revit's unfilled openings are holes in their host's sketch

**Date:** 2026-10-03
**Issues:** #227; backlog item B20
**Artefacts:** `2024_Core_Interior.rvt` with Revit's own export `2024_Core_Interior_slim.ifc`. Measured on GitHub's hosted runners with the Measure workflow, run 37126299322.
**Probe:** `examples/probe_re151_unfilled_openings.rs`.
**Status:** positive. Shipped in #584.

## 1. The question

Revit's IFC4 export of Core Interior writes 201 `IfcOpeningElement`s. A door or window fills 138 of them, and rvt-rs has written those since RE-23. Nothing fills the other 63:
- 42 void floor slabs;
- 20 void shading devices;
- 1 voids a wall.

What is the element each of those openings is tagged with, and where does the opening's shape come from?

## 2. Method

The probe reads the reference export next to the model. It takes every opening that no `IfcRelFillsElement` names, the opening's `Tag`, and the `Tag` of the element its `IfcRelVoidsElement` voids. It then looks the opening's `Tag` up in the partitions' record chains (RE-35): the record, the class it names, and, where the frame decodes, its category, box, owner and reference list. For each host it also lists every sketch line (`OST_SketchLines`, −2000045) that names the host as owner.

## 3. Result

| host | openings | record found | class | category |
|---|---:|---:|---|---|
| `IfcSlab` | 42 | 42 | `CurveElem` | −2000045 |
| `IfcShadingDevice` | 20 | 20 | `CurveElem` | −2000045 |
| `IfcWall` | 1 | 1 | `CurveElem` | −2000045 |

Every one of the 63 openings is tagged with a sketch line, and that line's owner is the opening's host. None of the openings is an element of its own.

Example: slab 20311 owns 31 sketch lines. 27 of them (20312 to 20338) close its outer loop, and 4 (20339 to 20342) close a rectangle inside it, from (20, 25) to (167, 114). Revit's opening for that hole is tagged 20339. Shading device 20953's hole starts at sketch line 20954, and its opening is tagged 20954. On all ten listed openings the tag is the **lowest ElementId of the sketch lines on the hole**.

Revit's export writes the hole twice:
- as a void of the slab's own profile (`IfcArbitraryProfileDefWithVoids`), which rvt-rs already wrote;
- as an `IfcOpeningElement` voiding the slab. It has the hole's outline (an `IfcArbitraryClosedProfileDef`) extruded through the slab's thickness, a placement relative to the slab's with an identity axis, the name `Family:Type:<type id>` (`Floor:Arch Topping Slab - 2":4166`), and no `ObjectType`.

The wall's opening (Tag 55859, wall 55840) is a sketch line too. Its lines lie in the wall's vertical plane: it is a hole in the wall's edited elevation profile, not in a plan sketch.

## 4. What this changes

#584 tags each void loop of a floor's sketch with the lowest id of the sketch lines lying on it, and writes an opening for it as Revit does. Measured against Revit's export (run 37128180603), all 42 slab and 20 shading-device openings are written, with Revit's `Tag` and host, each within 0.001 ft of Revit's box. The test is `tests/unfilled_openings.rs`.

The wall's opening is not read. That needs the wall's edited elevation profile, which rvt-rs does not decode.
