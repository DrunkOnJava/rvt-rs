# RE-414 — A `GInstance` with `m_bHasScale` set and a rotation matrix walks, and the showers behind it stay boxes

**Date:** 2026-11-10
**Issues:** #1645; follows RE-329 section 1 and RE-342, whose pre-2023 route reads the flag with a rotation matrix; RE-408 left the housing's showers behind this refusal
**Runs:** Measure [42922113349](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42922113349) (the census on the four BIM4LCA as-built files), [42922116931](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42922116931) (the ten 2024 samples), [42925837099](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42925837099) and [42926515359](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42926515359) (the raw fields of the flagged instances and the split by matrix), [42926624595](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42926624595) (58 more models), and [42929524267](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42929524267) (the rule at `c2ea02c1` against `main`, `keep_ifc=true`, the four BIM4LCA as-built files)
**Probe:** `probes/examples/probe_1645_scaled_instances.rs`
**Status:** Negative for shipping. Walking a flagged instance whose matrix is a rotation draws none of the 32 showers it was meant for: behind the flag the nested symbols hold faces of three kinds the mesher refuses, so every shower stays a box and no body of any file moves. The flag is not what keeps the showers boxed. The rule is not shipped; its tip is `refs/archive/2026-11-10/feat/flagged-rigid-instances` (`4a1b7baa`).

## 1. The question

The traversal refuses a `GInstance` whose `m_bHasScale` is set (`scaled instance mode is not qualified`) on every route but the pre-2023 research one, where RE-342 walks one whose matrix is a rotation (no export reads those releases). RE-329 section 1 named the 10 sanitary terminals of the BIM4LCA office that it keeps as boxes for it, and after RE-408 the 22 showers of the BIM4LCA housing (`Suihku 1:SU-1`, 11 in each of the two as-built files) are the same: the refusal property of each read `partial: scaled instance mode is not qualified` as its only reason, and 10 `Shower_columns_15486:SU-1` of the office as built read the same. What the flag means is not read.

## 2. What the instances hold

`probe_1645_scaled_instances` lists the `GInstance`s of every exported element's saved graphics and of the graphics of the symbols they name, with the 3x3 of each flagged one.

- **BIM4LCA, the same in every file read:** 112 flagged instances in 3 symbol records (1059720, 1059722, 1059732), all under sanitary terminals. 111 have the identity matrix and 1 an exact rotation (every entry 0 or 1, no deviation at all), with a translation each.
- **What marks them:** each flagged instance carries a category id of its own in `m_GInfo` (1057871) and bit 0x200 of its flags. No unflagged instance of the file has the flag bit, and one beam's instance is the only other that carries a category. The 112 are exactly the instances with a category in the sanitary terminals.
- **What an instance holds:** a `GInstance` has `m_GInfo`, `m_bHasScale`, `m_forbiddenTarget`, `m_instanceInfo`, `m_oEmbeddedSymbolGRep`, `m_resolveSymInView` and `m_tagId`; its `InstanceInfo` has `m_GRepId`, `m_Trf` (`m_3x3`, `m_or`), `m_cda` and `m_symbolId`. No field but the 3x3 can hold a scale.
- **Nowhere else in Measure.** Autodesk's ten 2024 samples hold none, and neither do the 58 other models read (the 2023, 2025, 2026 and 2027 samples, puzzbobb's 24 Revit 2025 models, the four RE1 models, Core Interior and Einhoven; run 42926624595): no export but the BIM4LCA files could move for this reason. RE-342 counted 11,839 flagged instances on 14 files of 2023 to 2027 that are not among them.

## 3. The change tried

A `GInstance` whose `m_bHasScale` is set is walked when its `InstanceInfo` 3x3 is a rotation (orthonormal, determinant +1 to 1e-9, the identity included), on every route; a mirror, a scale and a shear stay refused, and RE-342's older-schema clause folds into it. Every other check of an unflagged instance (the representation, the symbol, the targets) follows. It was run as `feat/flagged-rigid-instances` and is not shipped (section 4).

## 4. Rows and result

Registered on #1645 before the change was run on anything. Measure 42929524267 ran it at `c2ea02c1` against `main`, with the exports kept, on the housing (concrete and timber as built) and the office (as built and timber as built).

| Row | Result |
|---|---|
| Of the housing's 22 showers (concrete and timber as built) and the office's 10 (as built), refused for this reason alone, at least 20 of 22 and 8 of 10 are drawn from their saved meshes | **missed: 0 of 22 and 0 of 10.** The flagged instances are walked, and every shower then reads `partial: multiple cylinder trim loops unsupported; nonrectangular parametric trim; unsupported surface HermiteSurf` (11 in each housing file, 10 in each office file) |
| Revit's export holds them under the same Tag (an `IfcMappedItem` of about 49,500 triangles each on the housing): at least 90% of those drawn have every vertex within 0.01 ft of Revit's both ways | **not exercised:** none is drawn |
| An element's body differs from `main`'s only if its saved graphics reach a flagged instance with a rotation matrix | **holds, with nothing to show:** in the four files the box and saved-mesh body counts are identical to `main`'s (housing concrete 74 and 882, housing timber 73 and 1,025, office 168 and 11,100, office timber 167 and 9,134) and the only differences are the text of the 42 showers' `SavedMeshRefusal` and its counters in the diagnostics; the other models hold no flagged instance (section 2) |
| What the 112 do not test (a rotation that is not a quarter turn, a mirror, a scale) is stated | in section 6 |

## 5. What is behind the flag

The nested symbols that the flagged instances name hold the shower's detail parts, and their saved graphics hold faces of three kinds the mesher refuses, which the instance's refusal had hidden because the walk stopped at the instance:

- `multiple cylinder trim loops unsupported`: a cylinder face with several trim loops (RE-305, RE-407: drawn, its volume is not Revit's by the bar, not shipped),
- `nonrectangular parametric trim` (RE-302: the same, by the 0.01 ft bar against a coarser export),
- `unsupported surface HermiteSurf`: a surface kind with no decoder.

So the 32 showers are blocked by the same three kinds as the housing's and the office's toilets and sinks (RE-329), and the flag was never the blocker. Revit's export holds the housing's showers as an `IfcMappedItem` of about 49,500 triangles, which is the oracle for the day those faces are read; the rule of this report is a prerequisite for them and not a gain by itself.

## 6. Not shown

- **A flagged instance with a mirror or a scale.** None is in a file with a Revit export.
- **A flagged rotation that is not a quarter turn.** None either.
- **That the walked instances are right.** None draws a body, so no row of Revit's export has scored them; the rule would be tested with the showers once their three face kinds are read.
- **What the flag means.** It sits on the nested instances that carry a category of their own in this project; no other reading is shown.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=c2ea02c1b0bab26ddaf1c3718a5c7ab20e5262cc -f base=main -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-housing-timber-as-built nordic-office-as-built nordic-office-timber-as-built" \
  -f scorers="mesh_bodies_vs_ifc saved_meshes_vs_ifc box_bodies_vs_ifc" -f keep_ifc=true
```

`c2ea02c1` and the rule are at `refs/archive/2026-11-10/feat/flagged-rigid-instances` (`4a1b7baa`); the census is `probe_1645_scaled_instances` on the same files and on the other models.

## 8. Sources

RE-302, RE-305, RE-329, RE-342, RE-407, RE-408. No Autodesk source was read.
