# RE-105 — Steel members carry their section as a material profile

**Date:** 2026-09-28
**Issues:** #94 (material association via profile); follows RE-82, RE-103 and RE-104
**Result:**
- Every structural member rvt-rs draws as its type's I section (RE-103, RE-104) is now written with an `IfcMaterialProfileSetUsage`, as IFC4 models a profiled member's material, instead of an `IfcMaterialConstituentSet`:
  - one `IfcMaterialProfileSet` per type, named after it;
  - its `IfcMaterialProfile` pairs the type's one material (RE-82) with an `IfcIShapeProfileDef` equal to the member's body.
- `IfcMaterialProfile` used to be written with a 1 × 1 m rectangle stand-in. It now takes the model's real profile where there is one, and the stand-in only where there is none.
- `IfcMaterialProfileSetUsage` keeps CardinalPoint 5. In IFC4's `IfcCardinalPointReference` that is the mid-depth centre, where the exporter centres every profile. The writer's comment called it "bottom-left" and is corrected.

| Snowdon Towers structural | main | RE-105 |
|---|---:|---:|
| I-section beams and columns with a material profile set usage | 0 | 417 of 417 |
| their profile equal to their body's I, material Steel | – | 417 |
| material profile sets | 0 | 12 |

-----

## 1. Measured

Measured with IfcOpenShell 0.8.5 on the written IFC:
- all 377 beams and 40 columns drawn as an I have `ifcopenshell.util.element.get_material` return an `IfcMaterialProfileSetUsage`;
- its set is named after the member's `TypeName`;
- its single profile's OverallWidth, OverallDepth, WebThickness and FlangeThickness equal the body's `IfcIShapeProfileDef`;
- its material is Steel, the one the members' constituent sets held.

`tools/ci/ifc_schema_arity.py` passes on the file (36,932 instances across 52 entity types).

The GLB is byte-identical: a member with a one-material profile set takes that material's colour, as one with a one-material constituent set did. `rvt-schedule`'s material column names the profile set's material (Steel) rather than the set's name, and now fills for these 417 members.

## 2. Other files

Only Snowdon Towers structural changes. Every other local file is byte-identical to RE-104's output apart from the timestamp, so no witness observation changes.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Structural.rvt (local only) | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` |

```bash
./target/ci/rvt-ifc "Snowdon Towers Sample Structural.rvt" -o structural.ifc
python3 tools/re/beam_sections_in_ifc.py structural.ifc
python3 tools/ci/ifc_schema_arity.py structural.ifc
```
