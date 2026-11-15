# RE-484: A family instance Revit writes as a covering takes the area of its saved mesh's face where the mesh is one part with one-piece faces

**Date:** 2026-11-15
**Issue:** #1209 (W7: coverings' `GrossArea`, the roofing coverings); follows RE-480 (the one-piece saved face of a ceiling), RE-297.
**Artefacts:** the BIM4LCA office (concrete and timber, as built and at building permit) and housing (concrete and timber, as built and at permit), Revit 2024, with Revit's own IFC4 exports; the Revit 2025 models, Core Interior, RE1, the Autodesk samples of Revit 2015 to 2027 and the held-out projects (counts only) for what else moves.
**Scorer:** `tools/re/covering_areas_vs_ifc.py` (branch `research/1209-covering-areas`), at `611b0d6c017a` for the census of every model with a Revit export (Measure [43708293783](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43708293783)) and at `4c81a269a9df`, which adds the parts and pieces, for the BIM4LCA files (Measure [43708847856](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43708847856)), both on `main` and archived under `refs/archive/2026-11-15/cited/`. It does not land.
**Status:** positive, on two projects of one author. The 72 family-instance coverings of the BIM4LCA office and housing that Revit gives a `GrossArea` and whose saved mesh is one part with one-piece faces now carry Revit's value, 72 of 72 equal and none different; the 10 vents of several parts and the 36 downpipes Revit gives none keep none, and nothing else moves: no sample project, no held-out project, no other export.

## 1. Where Revit's value comes from

revit-ifc 24.2's `GrossAreaCalculator` (CLEANROOM.md source 5, `Exporter/PropertySet/Calculators/GrossAreaCalculator.cs` at `e5caa9b81d`; cited, not copied; RE-480 section 1) gives an element an `IfcQtyGrossArea` override, else the area of the extrusion the body exporter made of its body (`IFCExportBodyParams.ScaledArea`), else nothing. The fallback the 26.1 add-in adds, `HOST_AREA_COMPUTED`, is a host element's parameter, which a family instance is not expected to hold (not read here). So a family instance Revit writes as a covering has a `GrossArea` where its body was exported as an extrusion, and the value is that extrusion's profile area.

## 2. Which coverings are family instances

Revit's export writes some family instances of specialty equipment and roof accessories as `IfcCovering` `ROOFING`. rvt-rs writes them as coverings whose `RevitClass` is `FamilyInstance`, with their saved meshes as bodies (`BodySource` `saved_graphics_mesh`). `covering_areas_vs_ifc` pairs every `IfcCovering` of both exports by Tag. Of the models with a Revit export, only the BIM4LCA files hold such coverings; RE1 Architecture, Core Interior and the 24 Revit 2025 models hold none.

| file | family-instance coverings | with Revit's `GrossArea` | without |
|---|---:|---:|---:|
| concrete office, as built | 12 | 12 | 0 |
| timber office, as built | 12 | 12 | 0 |
| concrete housing, as built | 24 | 15 | 9 |
| concrete housing, permit | 23 | 14 | 9 |
| timber housing, as built | 24 | 15 | 9 |
| timber housing, permit | 23 | 14 | 9 |

The permit files of the office hold none. The office's 24 are ventilation units (`Ilmanvaihtokoneet`); the housing's are ventilation units, vents (`IV-venttiili`), snow guards (`Lumieste`), downpipe casings (`Syöksytorven kotelo`) and downpipes (`Syöksytorvi`).

## 3. The face of a one-part mesh is Revit's value

For each, the scorer meshes rvt-rs's body with IfcOpenShell 0.9.0.post1 and takes its largest downward and largest upward horizontal faces (triangles whose normal is vertical to 0.999, grouped by height to 0.001 ft), each face's connected pieces, and the body's connected parts (triangles sharing a corner 1e-5 ft apart):

| file | with Revit's `GrossArea` | one part, faces one piece each and agreeing to 0.1% | of those, the lower face equal to Revit's |
|---|---:|---:|---:|
| concrete office, as built | 12 | 12 | 12 |
| timber office, as built | 12 | 12 | 12 |
| concrete housing, as built | 15 | 13 | 13 |
| concrete housing, permit | 14 | 11 | 11 |
| timber housing, as built | 15 | 13 | 13 |
| timber housing, permit | 14 | 11 | 11 |

- **72 of 72** coverings that pass the test have Revit's `GrossArea` as their face's area, to 1e-3 relative.
- **The vents fail it** (2 or 3 per housing file, 10 in all): their meshes are 2 to 4 parts, and Revit's value is not their face but one solid's, for example 0.3381 ft² (a 200 mm circle) against a face of 1.0117 ft², or 1.3526 against 2.7012.
- **The downpipes Revit gives no value fail it too** (9 per housing file, 36 in all): the 70 mm ones (`HLV-3.ST70`, 2 per file) have no upward horizontal face, and the 120 mm ones (`HLV-2.ST120`, 7 per file) have faces of 0.1212 and 0.0096 ft² that do not agree. No covering Revit gives no `GrossArea` passes the test.
- **Which face.** The office's units are near-cubes in rvt-rs's body (their three extents equal to 0.001 ft on 12 and 10 of 12), so there any face would give the same area. The housing's snow guards are long bars (27.5 by 0.13 by 0.53 ft for `Lumieste1`): Revit's value, 3.6035 ft², is their horizontal face, not a side (14.6 or 0.07 ft²). So the face is the horizontal one, as for the ceilings of RE-480, which an extrusion along Z gives.

## 4. The rule

In the saved-mesh pass, an `IfcCovering` whose `RevitClass` is `FamilyInstance` and that holds no `Qto_CoveringBaseQuantities.GrossArea` (`coverings_without_area`). Where its saved mesh decodes with no diagnostic (`partial_refusal`), is one connected part (`one_part`: every triangle joined to every other through corners 1e-6 ft apart), and its largest upward and largest downward horizontal faces are each one piece of the same area to 0.1% (`one_piece_face_area`, RE-480), it gets that area as `GrossArea` (`add_covering_areas`). Any other keeps none, and no value already written is replaced. Ceilings are unchanged from RE-480.

## 5. Measured

Measure [43709928629](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43709928629), the change (`049e4593`) against its merge-base with `main` (`340c8727`, which holds RE-480): every reference model, the BIM4LCA office and housing, the 24 Revit 2025 models, the Autodesk samples of every release and the 19 held-out projects (counts only).

`qtos_vs_ifc`, `Qto_CoveringBaseQuantities.GrossArea`, before and after:

| file | coverings with Revit's `GrossArea` | equal | different | missing |
|---|---:|---:|---:|---:|
| concrete office, as built | 90 | 74 → 86 | 0 → 0 | 16 → 4 |
| timber office, as built | 88 | 72 → 84 | 0 → 0 | 16 → 4 |
| concrete housing, as built | 51 | 36 → 49 | 0 → 0 | 15 → 2 |
| concrete housing, permit | 46 | 32 → 43 | 0 → 0 | 14 → 3 |
| timber housing, as built | 60 | 44 → 57 | 0 → 0 | 16 → 3 |
| timber housing, permit | 46 | 32 → 43 | 0 → 0 | 14 → 3 |

- The 72 values are the 72 coverings of section 3: 12, 12, 13, 11, 13 and 11.
- What is still missing on the housing is its vents (2, 3, 2 and 3), which the guard refuses, and on the timber housing as built one more value, missing before and after and on none of the ceilings or family instances the census pairs by Tag (it pairs 128 of Revit's 129 coverings there, and accounts for each ceiling and family instance among them); what it is is not read here. On the office it is the 4 ceilings in pieces of RE-480. The 36 downpipes are not in these counts, since Revit gives them no `GrossArea`, and none of them gained one.
- The whole census moves by those values alone: `different` stays 0 on the housing and 1 on each office file, and "rvt-rs quantities Revit does not write" stays 1, 0, 2 and 2 on the housing and 0 and 3 on the office.

What moves (`ifc.sha256`): 6 of 120 exports, these six files. Every other export is identical by `ifc.sha256`: the office permit files, the housing terrain files, Core Interior, RE1 Architecture, Einhoven, the 24 Revit 2025 models and every Autodesk sample of every release, so the rule writes nothing on a model with no Revit export. Of the 6 `diagnostics.json` files that differ, the only field that moves is `exported.total_entities`, by one for each covering given a new quantity set (13 on the concrete housing as built, 12 on the office as built). The 65 output files of the 19 held-out projects are identical on both sides. `same-project gates` passes on `049e4593` (15 samples held to their earlier saves).

## 6. The registered rows

Registered on #1209 before the run (2026-11-15, comment 6893635627), read from Measure [43709928629](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43709928629).

1. **The family-instance coverings take Revit's `GrossArea`: 72 values equal and none different.** Met: 72 of 72 (12, 12, 13, 11, 13, 11), none different.
2. **No covering Revit gives no `GrossArea` gains one; "values Revit does not write" unchanged everywhere, held-out included.** Met: 1, 0, 2, 2 and 0, 3 as before, and the held-out outputs are identical.
3. **`different` unchanged everywhere; the held-out model with 3 missing covering values may gain `equal`, never `different`.** Met: `different` unchanged; that model's counts are unchanged (the rule does not fire there).
4. **The exports that move are those holding such a covering, and only by that quantity.** Met: the six files above, by `exported.total_entities` alone in their diagnostics, with every other scorer line unchanged.
5. **Where it fires on a model with no Revit export, the count is reported per model.** Reported: 0 on every model, since no sample's export moves.

## 7. Where it rests

The coverings it reaches are those of the BIM4LCA office and housing: two buildings, by one author, each saved by Revit 2024, and no other model with a Revit export holds a family instance written as a covering. Every one with Revit's value either passes the test and is equal or fails it, and every one without fails it. Two buildings put the rule outside `docs/one-project-rules.md`, but its families are one author's (ventilation units, snow guards, vents, downpipes), so another author's roof equipment would test it further.

## 8. Reproduce

```bash
# the census of every model with a Revit export, then the parts and pieces on the BIM4LCA files (scorers from the research branch's archived tips)
gh workflow run measure.yml --ref research/1209-covering-areas -f ref=67c05d89a80679fff83e8b92ab7747fb2d656cd1 -f base=none -f nordic=true -f r2025=true -f scorers=covering_areas_vs_ifc
# the change
gh workflow run measure.yml --ref main -f ref=049e4593e791a0d9003e0652115b3551137c78df \
  -f base=340c872701033a7ecf6e8ab56d365a78458c8c75 -f nordic=true -f r2025=true -f samples=true -f holdout=true
```

The scorer runs from a branch (`--ref`); once the branch is gone, push its archived tip (`refs/archive/2026-11-15/cited/4c81a269a9df`) to a branch to rerun it.
