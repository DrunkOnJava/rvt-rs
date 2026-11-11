# RE-413: a roof's holes stay in its `GrossArea` and `GrossVolume`

**Date:** 2026-11-10
**Issues:** #1636 (the second-project measurement), #1016 (quantities); follows RE-249 (a floor's `GrossArea`), RE-295 (a floor's `NetVolume`), RE-297 (a ceiling's `GrossArea`), RE-298 (a flat roof written as a slab) and RE-402 section 4 (`Qto_SlabBaseQuantities.GrossArea` and `GrossVolume`: 1 different on each of the four large housing files)
**Artefacts:** the BIM4LCA housing, concrete as built (Revit 2024; Revit 24.0.5's IFC4 export, CC BY-SA 4.0, as Measure fetches it), and rvt-rs's export of it by `main` at `237d6ef6`, which Measure [42906770416](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906770416) kept (`keep_ifc`); the other BIM4LCA files through Measure's census
**Runs:** Measure 42906770416 for section 2, [42925098078](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42925098078) (the 12 BIM4LCA models) and [42928202421](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42928202421) (unfiltered, 126 models) for section 4
**Status:** Positive, on one element: the one different `GrossArea` and the one different `GrossVolume` of each large housing file are equal, a single roof in four saves of one project, and no other export of the 126 in the unfiltered run moves, so no second model exercises the rule.

## 1. What RE-402 left open

On each of the four large housing files one `IfcSlab` has a `Qto_SlabBaseQuantities.GrossArea` and `GrossVolume` that differ from Revit's, both by the same 0.9% (RE-402 section 4). RE-249 and RE-298 measured the rule that writes them on the floors and flat roofs of the BIM4LCA office, where none differed.

## 2. What the pair shows

`BIM4LCA housing, concrete as built`: the slab with Tag 1567765 (`Basic Roof:YP-1.KL`, an `IfcSlab` Revit writes for a roof) has the same body in both exports: a 0.82 m extrusion of 168 triangles whose volume is 101.6165 m³ in each, with the same upward, downward and side areas (123.9226, 123.9226 and 56.2520 m²). The two files differ in what they write beside it:

| | Revit's export | rvt-rs's |
|---|---|---|
| profile | `IfcArbitraryClosedProfileDef`, the outline | `IfcArbitraryProfileDefWithVoids`, the outline and 3 inner curves of 1.1224 m² in all |
| openings | 3 `IfcOpeningElement` (`…:1` to `…:3`), each with the roof's Tag | none |
| `GrossArea` | 125.0450 m² | 123.9226 m² |
| `GrossVolume` | 102.5369 m³ | 101.6165 m³ |
| `NetVolume` | 101.6165 m³ | none |

125.0450 is the outline's area (the extrusion's area with its holes left in); 125.0450 less the holes' 1.1224 is rvt-rs's 123.9226. `GrossVolume` is `GrossArea` times the 0.82 m depth in Revit's export, 125.0450 x 0.82 = 102.5369, and the body's volume, with the holes off, is its `NetVolume`.

The other slabs of the file whose profile has holes in rvt-rs's export (13 in all) read two ways:

| element | holes | Revit's `GrossArea` is | rvt-rs writes |
|---|---|---|---|
| 5 floors (1406275, 1407283, 1457056, 1551639, 1552573) | 1 each, 0.32, 4.264, 0.36, 0.36 and 0.36 m² | the outline's area less the holes | the same, equal |
| 3 floors (2173994, 2174950, 2174994) | 2 each, 0.3124 m² in all | the outline's area less the holes (52.8999 of 53.2123) | none |
| 3 ceilings written as coverings (1698561, 2002081, 2002264) | 1 each, 0.5255 m² | the outline's area less the holes (28.3125 of 28.8380) | none |
| roof 1567765 | 3, 1.1224 m² in all | the outline's area (125.0450) | the outline less the holes (123.9226) |
| roof 2175398 (`Basic Roof:YP-1.OL`) | 1, 0.36 m² | the outline's area (92.7792) | none |

Each floor and ceiling has its holes taken off Revit's `GrossArea`, and each of the 2 roofs with holes has them left in. Revit's export gives each of the 10 slabs with holes an `IfcOpeningElement` for each hole, floors included, so the openings do not tell them apart; the category does.

## 3. The change

`add_slab_gross_area`: a roof with holes in its sketch gets the area of its outline for `GrossArea` and that area times its depth for `GrossVolume`, where it got the area less the holes. Its `NetVolume`, where it is written, is still the area less the holes times the depth. A floor's `GrossArea` and a ceiling's are as before.

## 4. Measured

Measure [42925098078](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42925098078) (head `1182462a` against `main` at `0ec5deea`, the 12 BIM4LCA models, `qtos_vs_ifc` and `psets_vs_ifc`): the exports of the four large housing files move and the other 8 are byte-identical (the four offices as built and at permit, concrete and timber, and the four terrain files), their censuses with them. On each of the four housing files the two census lines go from one different value to none, and one more is equal; the values rvt-rs writes that Revit's export omits stay 42, 2, 28 and 15:

| file | `GrossArea` equal / different / missing | `GrossVolume` equal / different / missing | quantities different |
|---|---|---|---:|
| concrete as built | 28 / 1 / 13 to 29 / 0 / 13 | 13 / 1 / 28 to 14 / 0 / 28 | 35 to 33 |
| concrete at permit | 31 / 1 / 6 to 32 / 0 / 6 | 22 / 1 / 15 to 23 / 0 / 15 | 89 to 87 |
| timber as built | 33 / 1 / 3 to 34 / 0 / 3 | 12 / 1 / 24 to 13 / 0 / 24 | 74 to 72 |
| timber at permit | 33 / 1 / 3 to 34 / 0 / 3 | 12 / 1 / 24 to 13 / 0 / 24 | 13 to 11 |

The unfiltered Measure (42928202421, head `50301810` against `main`, 126 models: the BIM4LCA files, the 24 Revit 2025 models, the held-out, spent and BIMcollab models, RE1, Core Interior, Einhoven and Autodesk's samples of 2015 to 2027; every scorer) moves 4 exports, the four large housing files, and 122 are byte-identical. The census differences of the four are the roof's `GrossArea` and `GrossVolume` lines and nothing else (the same-project gates pass on the head). So the change moves one roof, the Tag 1567765 of each of those four saves, and no model of the 126 other than those four has a roof with holes that rvt-rs writes quantities for: no second project exercises the rule, and nothing here checks it against another Revit export.

## 5. Not shown

- **More than one element.** The change moves one element: the roof 1567765 and its counterpart in each of the four large housing saves, so the four files are one roof four times, not four exercises. The other roof with holes (2175398) gets no quantities from rvt-rs, so the change cannot move it, and the 8 floors are on the branch that did not change. Section 4 reads the 126 models and finds no other.
- **Why Revit takes a floor's holes off and not a roof's.** The rule rests on 2 roofs and 8 floors of one project's concrete as built, and only the first of the 2 roofs is one rvt-rs writes quantities for.
- **The openings.** Revit's export writes an `IfcOpeningElement` for each of these holes; rvt-rs writes none (#1165).
- **The roof 2175398's quantities.** rvt-rs writes none for it, so this change does not move it; Revit's `GrossArea` for it is the outline's, as the rule says.
