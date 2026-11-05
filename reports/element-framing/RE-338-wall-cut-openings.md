# RE-338 — The openings a cabinet, a door and a curtain wall cut in a wall

**Date:** 2026-11-04
**Issues:** #1165 (unfilled openings), W4, W6 and W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows [RE-321](RE-321-unfilled-openings.md), which wrote the rectangular wall openings and measured these three kinds without writing them
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0), as `research/bim4lca-measure.tsv` pins them:

| file | sha256 |
|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` |

**Scorer:** `tools/re/unfilled_openings_vs_ifc.py` (RE-321)
**Status:** positive for the fire hose cabinets, for a door in a second wall that runs across the door's wall, and for the curtain walls the rule below takes; the rest are counted and left out. The sibling-covered openings are a call for the maintainer (section 6).

## 0. Result

The three kinds RE-321 measured and left out are written where their box is proven, and every step that is not written is counted by reason in `exported.wall_cut_openings_left_out`. On the office (Measure 41950674065, c2033d2a against `main`), every opening written has Revit's box within 0.01 ft, GlobalId, name, `PredefinedType` and both `Reference`s, and none is wrong:

| kind | Revit's, as built / at permit | written | counted, not written |
|---|---:|---:|---|
| a fire hose cabinet | 39 / 38 | 39 / 38 | none |
| a door in a second wall | 39 / 43 | 25 / 33 | the wall runs along the door's wall 9 / 9; the door's own opening box not read 1 / 1; a step with `m_bCutSomething` 1 in a wall the door is not hosted in 4 / 0; a shadow-cutout step 1 / 0 |
| a curtain wall | 70 / 72 | 27 / 16 | the wall's end within half a foot 41 / 50; another wall beside the box 2 / 5; a shadow-cutout step 1 / 1 |

The columns close. The steps in exported walls that name these cutters are, as built, 40 cabinets (39 written, 1 counted apart), 40 doors (25 written, 15 counted) and 73 curtain walls (27 written, 2 counted apart, 44 counted); at permit 40 cabinets (38 and 2), 44 doors (33 written, 1 counted apart, 10 counted) and 72 curtain walls (16 written, 56 counted). Each count is Revit's openings of the kind plus the steps whose opening its export of that file omits (section 6).

The openings written that Revit's export of the file lacks, and its export of the sibling file holds, are 3 as built (two curtain walls and a cabinet) and 4 at permit (two cabinets, a door and RE-321's rectangular wall opening), counted apart by the scorer. Core Interior (63 of 63), RE1 Architecture and the held-out models are as `main` writes them. The census (RE-235, W7) gains 178 equal property values and 198 equal quantities as built, and the openings' values that rvt-rs does not write fall from 531 to 230 property values and from 437 to 170 quantities (section 6).

## 1. What `main` writes and Revit's export holds

On `main` at 240c7d7c the office as built has 220 unfilled openings in Revit's export and rvt-rs writes 75, 58 of them Revit's; at building permit 216, 69 and 52. Of those Revit's export holds that rvt-rs does not write, by what cuts them (`IfcWall` hosts unless said):

| kind (cutter in host) | as built | at permit |
|---|---:|---:|
| a curtain wall | 70 | 72 |
| a door (in a second wall its opening reaches) | 39 | 43 |
| a fire hose cabinet (`IfcFireSuppressionTerminal`) | 39 | 38 |
| a door in a door | 4 | 2 |
| a wall in a wall | 1 | 1 |
| a floor's sketch void in a floor (RE-151, RE-339) | 9 | 8 |

Core Interior (63 of 63 written) and RE1 Architecture hold none of the first three kinds, so the office is the only reference for them here; the held-out models are in section 6.

## 2. The fire hose cabinets

A cabinet's `WallCutoutGStep` is in the record of the wall it is cut in, with `m_bCutSomething` 1 on all 40 steps of the office (RE-321 section 4). The opening is the box of the solid its symbol cuts (`FamilySymbol.m_complexCutGeoms`, the vertices of the faces it lists), placed by the instance's own transform (`InstanceInfo.m_Trf`: the rotation `m_3x3` applied to the symbol's coordinates, then `m_or` added). Its `PredefinedType` is `.RECESS.` where the box is shallower than the wall is thick and `.OPENING.` where it is as deep, and `Pset_OpeningElementCommon.Reference` and `Pset_EnvironmentalImpactIndicators.Reference` are the cabinet's type name. The GlobalId is `Opening:<number>Solid:1` with the wall's and the cabinet's GlobalIds, `<number>` the step's place in the wall's record counting steps of every kind.

RE-321 measured the cabinet openings exact (39 of 39 and 38 of 38) and left them out because the as-built export omits one cabinet that the permit export holds, and the permit export two that the as-built holds. Nothing read tells those from the others: the step, the cabinet's root and symbol, the wall's body (a notch in all 40) and the cabinet's box show no difference. They are written now, and the scorer counts the three on their own line, as it does the rectangular wall opening of RE-321 section 4 (the maintainer, 2026-11-03): an opening rvt-rs writes that Revit's export of the file lacks but its export of the sibling file holds. The cabinet omitted as built is 3388647 in 1494012, and those omitted at permit 3390319 in 1728013 and 3387194 in 3363820.

## 3. A door in a second wall

A door's cut in a wall other than the one it stands in is a `WallCutoutGStep` in that second wall's record with `m_bCutSomething` 0 (the step in the door's own wall has 1). Revit's export holds an opening for 77 of the 79 such steps in exported walls over the two office files; the two without are 3064459 in 1619525 as built and 1728679 in 1728013 at permit, each in the other file's export. Nothing in the step, in the wall's step list or joins, or in whether the second wall is joined to the door's wall tells those two from the 77: the steps' `m_flags` (761727) and `m_bIsTrueBoolean`, their history counts, the wall's representation and a join between the two walls were all compared.

The opening's box is the door's own opening box, in world feet, when the second wall runs across the door's wall: the door's wall thickness by the door's width by the door's opening height, equal to the box of the door's own opening in its own wall within 0.01 ft on 28 of 28 as built and 34 of 34 at permit. That box only touches the second wall's face or end on most of them (no overlap on one axis) and lies 0.105 and 0.164 ft off it on two, so the second wall is the one a box within a quarter foot reaches. Where the second wall runs along the door's wall (11 as built, 9 at permit) the opening is 10 ft across, 5 ft either side of the plane the symbol's cut loop lies in, one face of the door's own box, which rvt-rs does not read; those are left out and counted. A door whose own opening box is not read (a mesh door, a door with no cut loop) is left out and counted too.

The opening's `Reference` is the door's type name. Revit's export also gives it `FireExit`, the glazing set `Pset_DoorWindowGlazingType` and `Qto_OpeningElementBaseQuantities` whose `Width`, `Height` and `Depth` follow the door's frame (RE-244); rvt-rs writes neither the set nor the quantities, as it does not on the door's own opening either, so the census counts them missing.

## 4. A curtain wall

A curtain wall embedded in a wall is a `WallEmbedWallGStep`. Revit's export holds an opening for 69 of the 72 steps in exported walls as built and 71 of 71 at permit; the three omitted are the curtain walls 2295010, 1892067 and 1815871, each in the permit export. Across the wall the opening is the wall's thickness and half a foot beyond each face (70 of 70 as built, 739.8 mm in a 435 mm wall). Along the wall and up it is the curtain wall's record box where the box lies inside the wall's body and nothing trims it, and shorter where something does:

- Where the curtain wall reaches the wall's end the opening stops short of the end by 0.19 to 0.71 ft, as the end is mitered or another wall is joined there (RE-321).
- Where another wall's body touches the opening's box, the opening is cut back by the length of the overlap: 0.328 ft on 3030507 in 2991697, whose curtain wall lies 22 ft or more inside its wall at both ends, and 0.03 and 0.059 ft on four of the permit file's.

The rule written takes only a curtain wall whose box lies at least half a foot inside the wall's body at both ends and whose opening box no other wall's box touches (the walls rvt-rs draws, else the wall's record box), and counts the rest by reason. RE-321's variant that wrote every curtain wall had 16 of its 68 boxes wrong (Measure 41738481767); the half-foot rule alone is exact on 28 of 29 as built, the 29th being 3030507.

## 5. What is left

- **A door in a second wall that runs along the door's wall** (9 steps in each file): a box 10 ft across about a plane rvt-rs does not read. As built, 11 of Revit's 39 door openings are 10 ft across; the other 2 are among the steps counted as a shadow cutout or a cut in a wall the door is not hosted in.
- **A shadow-cutout step** (2 as built, 1 at permit: a curtain wall and a door as built, a curtain wall at permit) and **a door's own-cut step in a wall it is not hosted in** (4 as built): their boxes are not read.
- **A door in a door** (4, 2) and **a wall in a wall** (1, 1): no step names them.
- **The curtain walls that reach a wall's end or have another wall beside them**, counted by reason in `exported.wall_cut_openings_left_out`.
- **The openings written that the file's own export lacks** (section 6), which the scorer counts apart.

## 6. Measured as it lands

Measure [41950674065](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41950674065) (`-f base=main -f nordic=true -f keep_ifc=true -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture"`, scorers `unfilled_openings_vs_ifc psets_vs_ifc qtos_vs_ifc`, c2033d2a against `main` at 77d71cbb):

| | office as built | office at permit |
|---|---:|---:|
| unfilled openings written / both written and Revit's, `main` | 75 / 58 | 69 / 52 |
| unfilled openings written / both, this change | 169 / 149 | 159 / 139 |
| property values equal, `main` / this change | 44,223 / 44,401 | 44,387 / 44,559 |
| property values missing, `main` / this change | 1,361 / 1,384 | 1,364 / 1,396 |
| quantities equal, `main` / this change | 29,546 / 29,744 | 29,833 / 29,995 |
| quantities missing, `main` / this change | 9,931 / 10,000 | 9,917 / 10,013 |
| values Revit's export does not write, `main` / this change | 10 / 16 | 16 / 20 |
| Revit's openings' values rvt-rs does not write, not scored, `main` / this change | 531 / 230 | 565 / 265 |
| Revit's openings' quantities rvt-rs does not write, not scored, `main` / this change | 437 / 170 | 456 / 198 |
| different | 1, unchanged | 1, unchanged |

The new missing values are on the door openings: `Pset_OpeningElementCommon.FireExit` and the quantities of Revit's frame-relative `Width`, `Height` and `Depth` (section 3), which rvt-rs does not write for a door's own opening either. The 6 and 4 more values Revit's export does not write are the two `Reference`s of each opening written under the sibling rule. Core Interior's 63 of 63 and RE1 Architecture's 1 of 3 are identical to `main`'s, and `exported.wall_cut_openings_left_out` is empty on both.

**The sibling rule.** The scorer counts an opening rvt-rs writes that Revit's export of the file lacks but its export of a sibling file holds on its own line, not as an extra (RE-321 section 4, the maintainer, 2026-11-03, for one rectangular opening at permit). Applied here, it covers every opening the as-built export omits (two curtain walls, a cabinet; the omitted door 3064459 in 1619525 is not written) and the permit export (two cabinets, a door). Nothing read tells them from the openings Revit's export of the same file holds (RE-321 section 4 for the cabinets and curtain walls, section 3 above for the doors). They are written, as the rectangular opening is, and this is a call to confirm: the standing rule that where Revit's exporters disagree the value is left out would drop them, 3 of the 169 openings written as built and 4 of the 159 at permit.

**The held-out models.** Measure [41948301483](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41948301483) (`-f holdout=true -f models="holdout-* spent-*"`, 51db798e against `main`; the later commits only count more of the steps left out): the exports of all 19 held-out and spent models are byte for byte `main`'s, so they hold no cabinet, no door cutting a second wall and no curtain wall embedded in an exported wall to write, and they say nothing of the rules.

**Not shown.** The curtain-wall rule is read from the office's two files and nothing else; its neighbour test was chosen on the as-built file's one wrong box and passes the permit file's four others (in an analysis of the office's export, not committed); the first Measure of the writer (41946182305, 17966cbd) wrote doors only where the box overlapped the second wall, 11 and 17, and its figures for doors are superseded.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha> -f base=main -f nordic=true -f keep_ifc=true \
  -f models="nordic-office-as-built nordic-office-permit core-interior re1-architecture" \
  -f scorers="unfilled_openings_vs_ifc psets_vs_ifc qtos_vs_ifc"
```

The counted reasons are in each model's `diagnostics.json`, `exported.wall_cut_openings_left_out`.
