# RE-177: Revit 2027 element records, read with the schema's constants

**Date:** 2026-10-06
**Issues:** #421 (element records beyond 2024 to 2026); follows RE-32, RE-80 and RE-124 (every per-release constant is a schema class tag) and RE-35 (the partition record chain).
**Artefacts:** Autodesk's own sample projects of Revit 2026 and 2027, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rac_basic` (2026 `bc547fe9…d99dc`, 2027 `c9ce4d01…91a01`), `rac_advanced` (`f5fb3712…83ace`, `a88eb9f8…024ed`), `rst_basic` (`e7cf23a9…75418`, `01685786…d1453`), `rme_basic` (`37127cf5…ead94`, `886bb0d1…ae1a7`) and `rme_advanced` (`1f34a8f1…5d7af`, `899ea8a6…fca7a`). The 2027 `rst_advanced` and `rac_basic_sample_family` were read too.
**Probe:** `examples/probe_re177_revit_2027.rs`.
**Status:** positive against the 2026 copy of each model. **There is no Revit 2027 IFC export here**, so nothing in this report is measured against Revit's own export of a 2027 file. The oracle is the same model saved by Revit 2026, which RE-124 measured against Revit's export.

## 1. Before

On `main` a 2027 project is opened and its schema read, but every partition reader is gated to 2023 (or 2024) to 2026, so `rvt-elements` gives no building element: on 2027 `rac_basic` it lists 933 `Material` name candidates, and `rvt-ifc` writes the spatial scaffold with 933 `IfcMaterial` and nothing else.

## 2. The constants

The tags the file's schema gives the classes RE-124 lists, read by the probe:

| constant | class | 2026 | 2027 |
|---|---|---|---|
| record marker head, tail | `Outline`, `ElementParents` | 0x0161, 0x05f1 | 0x0164, 0x0610 |
| `ElementHeader` | `ElementHeader` | 0x05e5 | 0x0604 |
| Level elevation marker | `Plane` | 0x0265 | 0x027a |
| element-data header | `CellList` | 0x02fd | 0x0310 |
| layer frame | `VerticalRegionsStructure` | 0x1165 | 0x11ab |
| material object | `Material` | 0x0a9c | 0x0acc |
| material name frame | `PatternHelper` | 0x0164 | 0x0168 |
| material name end | `PhysicalParamSet` | 0x0cac | 0x0ce0 |

The 2026 column equals RE-124's table, and every 2023 to 2025 constant rvt-rs holds equals its file's tag as well. The 2027 values are the same on all seven 2027 files read (the six sample projects and the sample family). `ElementHeader` is 1540, the value RE-140 and puzzbobb (#421) give.

Three layouts carry over unchanged, each checked on the bytes:
- **Layer records** are 41 bytes as on 2026: the layer lists of 2026 and 2027 `rac_basic` are byte for byte the same.
- **Wall join lists** keep 2025's count offset of 16: with it, 2027 gives every join of the 2026 copy.
- **Mullion type materials** stay at +135 of the `SysMullionFamSym` data object: all 353 types of 2027 `rac_advanced` hold there the material their 2026 copies do. RE-166 (B66) found none on 2027; that survey ran while 2027's material names were not read, so no value could be recognised as a material.

## 3. Revit 2027 writes most frames in the second prologue

Frames by prologue (an ElementId at `+0x00` or not, RE-30) and kind:

| model | 2026 first prologue | 2027 first prologue | 2027 second prologue |
|---|---:|---:|---:|
| `rac_basic` | 1,265 | 327 | 938 (333 symbols) |
| `rac_advanced` | 9,043 | 977 | 8,066 (1,274 symbols) |
| `rst_basic` | 5,987 | 4,546 | 1,441 (249 symbols) |
| `rme_basic` | 12,723 | 4,107 | 8,617 (1,317 symbols) |
| `rme_advanced` | 12,683 | 12,683 | 0 |

A 2027 second-prologue frame opens with `ff`×12 and two `u16` (`04 00` or `19 06`, then a per-file value) where a first-prologue frame has the ElementId, the record size and `ElementHeader`'s tag. It sits inside a record of the partition's record chain (RE-35), whose header carries the ElementId. RE-35 already gives placed instances that id. Type symbols had no id, so a column lost the section its type symbol gives (RE-26). On `rac_basic`, `rac_advanced` and `rme_basic`, 17,621 second-prologue frames take their enclosing record's id; 17,619 of them take the id that the 2026 copy's first-prologue frame of the same category, kind and box carries. Of the other two, one symbol's box is shared with another symbol of the 2026 file and one box has no 2026 frame. Symbols now take their enclosing record's id from 2027 on (`SECOND_PROLOGUE_SYMBOLS_FROM`). Before 2027, symbols are left out: on Snowdon Towers Facades (2025) they would give seven columns a type section that no export here checks.

`rme_advanced` 2027 keeps every frame in the first prologue, so the second prologue is how these files were saved, not a rule of the release.

## 4. One false pipe type name

With 2027 admitted, 33 pipes of 2027 `rme_basic` took the type name `였縿괜쭃`. RE-130's scan accepts an id repeated 56 bytes on, followed at +306 by a counted UTF-16 name. Here it matched an element's reference lists, which hold id 712996 twice 56 bytes apart. On all six pipe and duct type names of RE1 Plumbing and Mechanical, the four bytes before the name's length are zero; at this match they are not. The scan now requires that `u32 0` (`MEP_CURVE_TYPE_NAME_GUARD`), and the 33 pipes have no type name, as on 2026.

## 5. The 2027 projects against their 2026 copies

`probe_re177_revit_2027` compares the production elements of the two files by ElementId. It compares every field except the three that locate a record (stream, offset, class tag), with floats to nine significant digits.

| model | elements, 2026 / 2027 | same ElementId and class | fields that agree | fields that differ |
|---|---:|---:|---:|---:|
| `rac_basic` | 638 / 638 | 638 | 9,624 | 45 |
| `rac_advanced` | 5,841 / 5,841 | 5,841 | 104,254 | 129 |
| `rst_basic` | 541 / 541 | 541 | 8,064 | 0 |
| `rme_basic` | 6,921 / 6,921 | 6,921 | 120,706 | 10 |
| `rme_advanced` | 6,877 / 6,877 | 6,877 | 119,808 | 0 |

Every difference is in the bytes of the 2027 file or in a reader no 2027 constant reaches:
- **79 corner mullions** (`L Mullion 1`, `rac_advanced`) take no curtain wall. Their reference lists name both walls of the corner on 2027 and one on 2026, so RE-46's rule (exactly one) leaves them standalone.
- **Type materials of 12 elements** (`rac_basic`: 6 walls, 3 doors, 2 plants, 1 entourage) **and 22 railings** (`rac_advanced`). These come from RE-82's map scan, which has no per-release constant. It reads a type's value block up to the next declared block, and 2027 stores the partition's objects in another order. Some windows now end inside another object's one-entry map, and some no longer do. For example, `rac_basic` type 600634 gains `Water` from a map 29,648 bytes after its block; the 2026 file has no such map within 200 kB. 2026's values include matches of the same kind. The 2026 file gives 34 types `Vapour Retarder` (material 421) where 2027 does not, and the first of them, 416, is itself a `Material` object. The railings' 2026 material is `Misc. Air Layers - Air Film - Outside Surface`.
- **21 mullions' `m_levelId`** (`rac_basic`) name another Level. They are aggregated in their curtain walls, so it places nothing: all 450 exported elements of `rac_basic` are on the same storey in both exports.
- **10 `m_original_symbol`** (`rme_basic`) name 888201 where 2026 names 343624. They are read from each instance's own `GElement` object (RE-167), which 2027 writes with the other symbol; the two symbols' boxes are identical.
- **3 entourage boxes** differ by 1.7e-6 ft: 2027 holds `5.833333492…`, the `f32` of 2026's `5.8333…`.

The IFC export of 2027 `rac_basic` has the same entities as 2026's, class for class: 47 `IfcWall`, 16 `IfcDoor`, 17 `IfcWindow`, 9 `IfcCurtainWall`, 144 `IfcMember`, 44 `IfcPlate`, 14 `IfcSpace`, 6 storeys, and the rest. The only exception is 87 `IfcMaterialConstituent` where 2026 has 85, from the type materials above. Storeys, with their GlobalIds, are the same. 424 of 450 element GlobalIds are the same; the other 26 are `IfcOpeningElement`, whose ids rvt-rs derives from the document, and the two files' documents differ. All six 2027 project exports pass IfcOpenShell 0.9.0's schema validation with no issue (16,877 to 216,072 entities). The 2027 family lists its 24 materials, as the 2026 family does.

## 6. Nothing else moves

With these changes, the IFC export of every 2023 to 2026 project here is byte-identical to `main`'s, owner-history timestamp aside. That is 55 distinct files: Autodesk's samples of 2023 to 2026 (`rac` and `rst`, and `rme` of 2026), Core Interior, Einhoven, the seven Snowdon Towers models of 2025, the two Golden Nugget and three Japanese sample projects, the four RE1 models, and small Revit 2025 test models.

## 7. What is not shown

- **Revit's own export of a 2027 file.** The 2026 copies are the oracle. Where 2027 reads the same as 2026, it inherits RE-124's measurement against Revit; where it differs (section 5), nothing here says which side is Revit's.
- What the second prologue's two `u16` are.
- Readers still gated to 2023 to 2025, as on 2026: roof slopes, stairs' dimensions, beam axes, IFC export overrides, room solids, the opening index.

## 8. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2026 2027
cargo run --release --example probe_re177_revit_2027 -- samples/2026-racbasicsampleproject.rvt samples/2027-racbasicsampleproject.rvt
```

The Measure workflow runs it on every sample with `-f samples=true -f probe=probe_re177_revit_2027`.
