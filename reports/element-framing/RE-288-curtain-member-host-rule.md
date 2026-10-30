# RE-288: curtain mullions' and panels' IsExternal by the host rule

**Date:** 2026-10-30
**Issues:** [#1187](https://github.com/DrunkOnJava/rvt-rs/issues/1187), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-193 §6, RE-226 §2, RE-241 and RE-269
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, exporter 24.0.5.432), RE1 Architecture (Revit 2025, IFC2X3 export by exporter 26.2.0.20) and every other model Measure fetches, as RE-269 lists them
**Runs:** Measure [41135969591](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41135969591) (`-f base=main -f nordic=true -f r2025=true -f ifc_flags=--revit-set-placement`, every model and scorer, commit fdf8bfca) and [41136306387](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41136306387) (the final commit cf7b7600, the office and RE1 Architecture: byte-identical exports to the first run's)
**Status:** positive for the rule's code and for RE1 Architecture. The value is checked against a newer exporter's export only on 12 members that are false by either rule, so the exterior members' values rest on revit-ifc's source.

## 1. The decision

RE-269 showed that the value Revit's export of the office writes on a curtain mullion or panel is the exporter's cache, not the model's, and left rvt-rs writing none. The maintainer decided on 2026-10-30 to write the value revit-ifc's `IsElementExternal` gives from IFC_v24.2.20 on, the host rule: a family instance with no `IsExternal` parameter and no type Function takes its host's value. Nothing is written on a member type, and the replay of the cache is never written.

## 2. Revit's rule, read in revit-ifc

Read in revit-ifc at `IFC_v24.2.20` and `IFC_v24.3.40` (CLEANROOM.md source 5; read and cited, no code copied). `CategoryUtil.IsElementExternal` returns null for an element that is an `ElementType` (the first line of the function) and caches a value only under the id of the element it is asked about, plus, from a type Function, under its type's id. A mullion or panel is a family instance whose type has no Function (RE-269 §2), so it reaches the host step: `IsElementExternal(familyInstance.Host)`, the curtain wall's value, which is the wall's `IsExternal` parameter of that name, else its type's Function (external unless interior), else a roof, else false. The wall's own `Pset_CurtainWallCommon.IsExternal` parameter is read by the property set entry and not by `IsElementExternal`, so it does not decide the members'. The two tags differ from the office's exporter in exactly the cache of types RE-269 §2 describes (`IFC_v24.1.0` and `IFC_v24.2.0` also accept a type and cache it as not external).

## 3. What rvt-rs does

`attach_curtain_member_is_external` (`partition_schema_mvp/curtain.rs`) gives each curtain mullion and panel that has no value of its own (no parameter, no type Function) the `IsExternal` of the curtain wall it is aggregated under (`m_aggregate_whole`, RE-46, RE-62, RE-72), which `attach_is_external` calculated for that wall. A part with no curtain wall gets none. The `Pset_MemberCommon` or `Pset_PlateCommon` of the entity it is written as holds it, as for every element (RE-193).

## 4. Measured

Every model's export hash, head against `main` (run 41135969591): the office as built, the office at permit and RE1 Architecture change; Core Interior, Einhoven, RE1 Electrical, Mechanical and Plumbing and the 24 R2025 models are byte-identical.

| | office as built | office at permit | RE1 Architecture |
|---|---:|---:|---:|
| `diagnostics.decoded.parameter_value_count`, `main` | 45,375 | 45,561 | 268 |
| the same, this change | 55,637 | 55,860 | 280 |
| values written by it | 10,262 | 10,299 | 12 |
| of them, mullions | 10,001 of 10,003 | 10,029 of 10,031 | 10 of 10 |
| of them, panels | 260 of 260 | 269 of 269 | 2 of 2 |
| of them, a curtain panel written as a door | 1 | 1 | 0 |
| Revit's values on objects rvt-rs writes: equal, `main` | 45,088 | 45,270 | 196 |
| the same, this change | 45,089 | 45,271 | 208 |
| different (the mullion whose `Span` differs), both | 1 | 1 | 0 |
| missing, `main` / this change | 436 / 435 | 433 / 432 | 64 / 52 |
| counted apart as the exporter's cache (RE-269), both | 10,003 mullions, 260 panels, 4 mullion types | 10,031, 269, 4 | none |

- **The office.** The 10,263 members and panels' values Revit's export writes are counted apart (RE-269) and not compared, so their values do not appear as different. rvt-rs's differ from them where the cache turned a member false: 8,579 of 10,003 mullions and 33 of 260 panels as built, 8,577 and 32 at permit (RE-269 §3 measured the host rule against this export). The one value that moved from missing to equal is the office's curtain panel written as a door (`IfcDoor` 1746275 as built, one at permit): its class is a curtain panel, so it takes its curtain wall's value, false, which equals Revit's.
- **The two mullions rvt-rs does not write** (Quad Corner Mullions 1417045 and 1430620, true on the second): rvt-rs finds no curtain wall for them (they lack `m_aggregate_whole`), and Revit's export does not put them under one either, so they are not in the cache bucket and stay missing.
- **RE1 Architecture** (exporter 26.2.0.20): the 10 mullions and 2 panels, which stand on one interior curtain wall, are 12 of 12 equal; they were 0 of 12.
- **No other value moves**: the equal, different and missing counts of every property but `Pset_DoorCommon.IsExternal` and the two member sets are the same as `main`'s on each of the three changed models.

## 5. What is not shown

- **Exterior curtain members on a newer exporter.** Revit's export of RE1 Architecture is the only one of an exporter after IFC_v24.2.20 (26.2.0.20) that has curtain members, and its curtain wall is interior, so the rule gives false and so does the export. The R2025 models (exporter 25.0.2.419) hold no curtain member, and Core Interior's export writes no property values. That an exterior curtain wall's members are true in a newer exporter's export is read from revit-ifc's source, not measured: about 9,140 of the office's 10,003 mullions would be true by the rule (RE-269 §4).
- **The office's value.** rvt-rs's value on the office is not Revit's export's, which is the cache of 24.0.5.432, as the first bullet of §4 says; the census cannot show it, since it counts those values apart.
- **Member types.** Revit's office export writes `IsExternal` false on the four mullion types and no newer exporter writes one (§2), so none is written.
- **Doors and windows set in a curtain wall's grid** (RE-46) are doors and windows, and take their own rules: only a curtain panel written as a door took the host's value above, by its class.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=fdf8bfca60605085893098265ecb8b47dd8a2d75 -f base=main \
  -f nordic=true -f r2025=true -f ifc_flags=--revit-set-placement -f psets_missing="Pset_MemberCommon.IsExternal,Pset_PlateCommon.IsExternal"
```
