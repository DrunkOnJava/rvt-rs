# RE-63 — System-family elements named `Family:Type:ElementId`

**Date:** 2026-09-24
**Result:**
- Walls, curtain walls, floors, building pads, ceilings, roofs and railings are now named as Revit's own IFC export names them.
  - `Name` is `Family:Type:ElementId`, for example "Basic Wall:8" Interior Partition 3 Hour:20796" rather than "Wall-20796".
  - `ObjectType` is `Family:Type`.
- Family instances already were (RE-38). A system family's type has a name in the file (#322), but its family has none: Revit derives "Basic Wall" from the type's kind.
- RE-61's reading gives the family:
  - a curtain wall is a Curtain Wall, and a railing a Railing;
  - a floor is a Floor, and a building pad a Pad;
  - a wall, ceiling or roof whose type has compound layers is a Basic Wall, a Compound Ceiling or a Basic Roof.
- Measured against the names in Revit's own IFC exports, over every system-family element matched by `Tag`:

| file | walls | curtain walls | floors | ceilings | roofs | railings |
|---|---:|---:|---:|---:|---:|---:|
| Snowdon Towers (local only) | 1,077 of 1,077 | 42 of 42 | 167 of 182 | 68 of 68 | 20 of 20 | 101 of 131 |
| 2024_Core_Interior | 360 of 360 | — | 100 of 100 | — | — | — |
| RE1 Architecture | 7 of 7 | 1 of 1 | 2 of 2 | 6 of 6 | — | 0 of 1 |

- `ObjectType` equals Revit's wherever `Name` does, and on the 15 Snowdon floors below too.
- Family-instance names are unchanged: 3,831 of Snowdon's 4,413 and 394 of Core Interior's 394 equal Revit's, as before.

-----

## 1. The rule

`partition_schema_mvp::system_family(class, has_layers)` maps the class to its family. `attach_system_family_names` sets it on each element that has a type name and no family. For walls, ceilings and roofs, whether the type has compound layers is decided per type:
- an element of the type drawn in its layers shows it (RE-53, RE-56, RE-57);
- otherwise the type's own data is read (`partition_compound_structure::scan_type_layers`).

Without the per-type check, 23 Snowdon walls and 17 roofs that are not drawn in layers (tapered roofs among them), and 45 of Core Interior's walls, would keep their `Class-ElementId` name.

The element's `RvtElementRecordGeometry` property set carries `FamilyName` with `FamilyNameSource` `system_family_by_category`, so the derived name is never taken for one read from the file.

## 2. The residuals

- **Floors (15 on Snowdon).** Revit exports each of these floors as two slabs, the second named with a `:2` suffix, and rvt-rs writes one. The `Tag` match meets either of Revit's two. rvt-rs's name is the first part's, and `ObjectType` is equal.
- **Railings (30 on Snowdon, 1 on RE1).** Their type name is not read (#322). They keep `Railing-ElementId`.
- **Curtain panels that are walls (18 on Snowdon).** Revit names them `Basic Wall:…`, and rvt-rs exports them as panels named `CurtainWallPanel-ElementId`. Named since RE-64.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| IFC Exports/2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |

```bash
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/names_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

The Core witness observations change by the new `FamilyName` and `FamilyNameSource` property values. Both verdicts PASS.
