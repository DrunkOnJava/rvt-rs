# RE-414 — A `GInstance` whose `m_bHasScale` is set and whose matrix is a rotation is walked

**Date:** 2026-11-10
**Issues:** #1645; follows RE-329 section 1 and RE-342, whose pre-2023 route reads the flag with a rotation matrix; RE-408 left the housing's showers behind this refusal
**Runs:** Measure [42922113349](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42922113349) (the census on the four BIM4LCA as-built files), [42922116931](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42922116931) (the ten 2024 samples), [42925837099](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42925837099) and [42926515359](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42926515359) (the raw fields of the flagged instances and the split by matrix)
**Probe:** `probes/examples/probe_1645_scaled_instances.rs`
**Status:** pending. The rows of section 4 were registered on #1645 before the change was run; the measurement is running and its result goes in section 4.

## 1. The question

The traversal refuses a `GInstance` whose `m_bHasScale` is set (`scaled instance mode is not qualified`) on every route but the pre-2023 research one, where RE-342 walks one whose matrix is a rotation (no export reads those releases). RE-329 section 1 named the 10 sanitary terminals of the BIM4LCA office that it keeps as boxes for it, and after RE-408 the 22 showers of the BIM4LCA housing (`Suihku 1:SU-1`, 11 in each of the two as-built files) are the same: the refusal property of each reads `partial: scaled instance mode is not qualified` as its only reason, and 10 `Shower_columns_15486:SU-1` of the office as built read the same. What the flag means is not read.

## 2. What the instances hold

`probe_1645_scaled_instances` lists the `GInstance`s of every exported element's saved graphics and of the graphics of the symbols they name, with the 3x3 of each flagged one.

- **BIM4LCA, the same in every file read:** 112 flagged instances in 3 symbol records (1059720, 1059722, 1059732), all under sanitary terminals. 111 have the identity matrix and 1 an exact rotation (every entry 0 or 1, no deviation at all), with a translation each.
- **What marks them:** each flagged instance carries a category id of its own in `m_GInfo` (1057871) and bit 0x200 of its flags. No unflagged instance of the file has the flag bit, and one beam's instance is the only other that carries a category. The 112 are exactly the instances with a category in the sanitary terminals.
- **What an instance holds:** a `GInstance` has `m_GInfo`, `m_bHasScale`, `m_forbiddenTarget`, `m_instanceInfo`, `m_oEmbeddedSymbolGRep`, `m_resolveSymInView` and `m_tagId`; its `InstanceInfo` has `m_GRepId`, `m_Trf` (`m_3x3`, `m_or`), `m_cda` and `m_symbolId`. No field but the 3x3 can hold a scale.
- **Autodesk's ten 2024 samples hold none.** RE-342 counted 11,839 flagged instances on 14 files of 2023 to 2027 that are not among them.

## 3. The change

`rigid_instance` (`native_graphics_traversal.rs`): a `GInstance` whose `m_bHasScale` is set is walked when its `InstanceInfo` 3x3 is a rotation (orthonormal, determinant +1 to 1e-9, the identity included), on every route. A mirror, a scale and a shear stay refused. RE-342's older-schema clause folds into it. Every check of an unflagged instance (the representation, the symbol, the targets) follows.

## 4. Rows and result

Registered on #1645 before the change was run on anything.

| Row | Result |
|---|---|
| Of the housing's 22 showers (concrete and timber as built) and the office's 10 (as built), refused for this reason alone, at least 20 of 22 and 8 of 10 are drawn from their saved meshes | pending |
| Revit's export holds them under the same Tag (an `IfcMappedItem` of about 49,500 triangles each on the housing): at least 90% of those drawn have every vertex of Revit's body within 0.01 ft of rvt-rs's surface and rvt-rs's within 0.01 ft of Revit's | pending |
| An element's body differs from `main`'s only if its saved graphics reach a flagged instance with a rotation matrix: no other body, quantity or property scorer line moves, in any of the 107 models | pending |
| What the 112 do not test is stated: no rotation that is not a quarter turn, and no mirror or scale, is in any file with a Revit export | stated in section 5 |

## 5. Not shown

- **A flagged instance with a mirror or a scale.** None is in a file with a Revit export; the rule refuses them.
- **A flagged rotation that is not a quarter turn.** None either; the rule draws it by the same matrix.
- **What the flag means.** It sits on the nested instances that carry a category of their own in this project; no other reading is shown.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=c2ea02c1b0bab26ddaf1c3718a5c7ab20e5262cc -f base=main \
  -f nordic=true -f r2025=true -f samples=true
gh workflow run measure.yml --ref main -f ref=c2ea02c1b0bab26ddaf1c3718a5c7ab20e5262cc -f base=main -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-housing-timber-as-built nordic-office-as-built nordic-office-timber-as-built" \
  -f scorers="mesh_bodies_vs_ifc saved_meshes_vs_ifc box_bodies_vs_ifc" -f keep_ifc=true
```

## 7. Sources

RE-329, RE-342, RE-408. No Autodesk source was read.
