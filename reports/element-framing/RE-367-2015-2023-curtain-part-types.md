# RE-367: Revit 2015 to 2023 curtain parts' types, families and materials, and a type's own material wins on every release

**Date:** 2026-11-06
**Issues:** #1190 (geometry and properties on 2015 to 2023, 2026 and 2027); follows RE-366 (curtain walls on these releases), RE-109 (32-bit name entries), RE-166 and RE-201 (a curtain part's material is its type's `m_renderStyleId`), RE-280 (native records on 2015 to 2022).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored.
**Scorer:** comparison by `Tag` with the same project's 2024 save; `probes/examples/probe_re367_curtain_materials.rs`.
**Status:** positive against the same samples saved by a later release. **No curtain part of a 2015 to 2023 file is scored here against a Revit export**, and a later save cannot catch an error it shares. It builds on RE-366: this reads the parts that recognises.

## 1. Before

With RE-366, `rac_advanced` 2023 draws 3,312 members and 1,349 panels with no type object, `ObjectType`, `Other.Family`, `Other.Type`, `Reference` or material. A type's name entry on these releases holds no category (RE-109), so `resolve_type` cannot run, and `type_and_family_names_2023` looks for a type record these types do not have.

## 2. The type and family

- **The type** is the one id in a part's reference list that has a name entry and is no element's record. On `rac_advanced` 2023 that is exactly one id, the 2024 save's type, on all 3,276 mullions and 1,349 panels. The 36 stringers have no such id and take no type here.
- **The family** is the `u32` at +265 past a type's ElementId and four bytes, where that has a name entry: `Rectangular Mullion`, `L Corner Mullion` and `System Panel` on 2015, 2019 and 2023.

## 3. The material

A part's material is its type's `m_renderStyleId` (RE-166, RE-201), which reads on these releases with `NATIVE_RELEASES`. `type_material_names_32` is not used for these types: with a type id it gives `Site - Tarmacadam` for every mullion and panel of `rac_advanced` 2023, glass included, and `Vapour / Moisture Barriers - Damp-proofing` on 2015.

Each type's material against its 2024 save's `m_renderStyleId`, by `probe_re367_curtain_materials`, on the types both hold:

| | types equal, of those both hold |
|---|---:|
| `rac_advanced` 2015 to 2020 (each) | 511 of 511 |
| `rac_advanced` 2021 to 2023 (each) | 517 of 517 |
| `rac_basic` 2015 | 82 of 82 |
| `rac_basic` 2016 to 2023 (each) | 91 of 91 |
| `rme_basic`, `rme_advanced` 2015, 2019 (each) | 465 of 465 |
| `rst_basic` 2017 to 2023 (each) | 3 of 3 |
| `rst_basic` 2016 | 2 of 2 |

## 4. Against a later save's export

Each part by `Tag`, against the 2024 save's export:

| | type id, type, family, `ObjectType`, `Reference` | material |
|---|---:|---:|
| `rac_advanced` 2015 to 2023, mullions (each) | 3,276 of 3,276 | see below |
| `rac_advanced` 2015 to 2023, panels (each) | 1,349 of 1,349 | see below |
| `rac_basic` 2015 to 2023, mullions (each) | 144 of 144 | 144 of 144 |
| `rac_basic` 2015 to 2023, panels (each) | 44 of 44 | 44 of 44 |

With section 4b's change, a `rac_advanced` part's material also equals the 2024 export's on every save: 3,276 and 1,349 of each.

## 4b. A type's own material wins on every release

`attach_curtain_materials` left alone a part that already had a material, so what a type's map gave beat `m_renderStyleId`, which RE-201 measured as Revit's on Snowdon Towers. On three of Autodesk's 2024 samples the map gives one material for every mullion and panel, glazed panels included:

| 2024 save | type | parts | before | now |
|---|---|---:|---|---|
| `rac_advanced` | mullions | 3,276 | `Wood - Laminated` | `Metal - Aluminium, Black-Anodized` |
| `rac_advanced` | `Glazed` | 1,190 | `Wood - Laminated` | `Glass` |
| `rac_advanced` | `Solid` | 159 | `Wood - Laminated` | `Stone - Granite` |
| `rme_advanced`, `rme_basic` (each) | mullions | 1,450 | `Wood - Laminated` | `Metall - Lack - Grau` |
| `rme_advanced`, `rme_basic` (each) | `Verglasung` | 438 | `Wood - Laminated` | `Fassade - Glas` |
| `rme_advanced`, `rme_basic` (each) | `Metallpanel` | 191 | `Wood - Laminated` | `Fassade - Metallpanel` |

Now a type's `m_renderStyleId` replaces what the map gave. Why the map gives one material on these files is not decoded.

**No Revit export scores a part this changes.** Against Revit's own exports, 20,989 mullions and panels equal Revit's before and after: the BIM4LCA office at both stages (10,263 and 10,300), its terrain housing (414) and RE1 Architecture (12). None of them changes, and neither do any of the 206 local project files, which hold no 2024 save of these three samples. So this shows no regression and the changed values rest on RE-201's rule, the same project's type names and, on `rac_advanced`, on the 2015 to 2023 saves. `racadv-*` hold `rac_advanced` with Revit's IFC2X3 export of older saves, not of 2024.

## 5. What else changes

Against RE-366's head, across the 33 sample saves of 2015 to 2023:

- **Added, none changed.** 36,584 members and 15,055 panels gain `Other.Family`, `Family Name`, `Family and Type`, `Type`, `Type Id`, `Identity Data.Type Name`, `Reference` and `Pset_EnvironmentalImpactIndicators.Reference`, and their materials.
- **Nothing else.** No element appears or goes, and no storey, GlobalId, host or void changes. Only those members and panels change material: no element of any other class does.
- **Byte identity.** Of 206 local project files of 2008 to 2027, 193 export IFC and diagnostics byte for byte as RE-366's head does (`SOURCE_DATE_EPOCH=1700000000`). The 13 that change are 2017 to 2023 files holding curtain walls. No file of 2024 or later, or before 2015, changes. Of Autodesk's samples, section 4b also changes `rac_advanced`, `rme_advanced` and `rme_basic` of 2024, their materials only.

## 6. Not measured, not read

- Against a Revit export: see the status above. `racadv-*` hold these parts with Revit's export, for Measure.
- A curtain wall's own type (`Exterior Curtain Wall`, `Storefront`): these have no name entry, and their data holds no name the reader finds.
- A stringer's type on `rac_advanced`.
- A part's `NetVolume` and `NetSurfaceArea`, and a curtain wall's `Length`.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
cargo run --profile ci -p rvt-probes --example probe_re367_curtain_materials -- samples/2023-racadvancedsampleproject.rvt 2023 > 2023.txt
cargo run --profile ci -p rvt-probes --example probe_re367_curtain_materials -- samples/2024-racadvancedsampleproject.rvt 2024 > 2024.txt
diff 2023.txt 2024.txt
```
