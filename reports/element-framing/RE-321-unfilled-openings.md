# RE-321 — The openings Revit's export writes that nothing fills

**Date:** 2026-11-03
**Issues:** #1165 (unfilled openings), W4, W6 and W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-151 (a floor's sketch voids), RE-225 (a roof's hatch openings), RE-237 and RE-244 (what a door's opening carries), RE-266 (a wall's cutout steps)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0), as `research/bim4lca-measure.tsv` pins them:

| file | sha256 |
|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` |

**Probe and scorer:** `probes/examples/probe_re321_unfilled_openings.rs`, `tools/re/unfilled_openings_vs_ifc.py`
**Runs:** Measure [41734524720](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41734524720) (the census probe), [41736585444](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41736585444) (rectangular openings, symbols and cutters), [41738481767](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41738481767) (all three kinds written, a7931166) [41742295094](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41742295094) (rectangular openings alone, ff53b6cd), and, for the writer as it lands, [41748430211](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41748430211) (every model, the Autodesk samples and the held-out models, 35425a96 against `main` at 0e0582e1) and [41754617363](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41754617363) (the 2024 samples with their IFC kept, 35425a96)
**Result:** positive for the rectangular wall openings, which are written: 30 of 30 as built and 24 of 24 at permit, with Revit's box, GlobalId, name, type, `Reference` (twice) and `Depth`, and none that Revit's export lacks as built (at permit one, counted apart, §4). Negative for the other kinds: fire hose cabinets and curtain-wall cuts are exact on most openings but each also writes openings Revit's export does not (or a wrong box), and nothing read tells those apart, so they are measured and left out (§4, §5).

## 1. What Revit writes

Revit's export of the office writes 687 `IfcOpeningElement`s as built (677 at permit). A door or window fills 463 (457). The other 224 (220) fill nothing; each is tagged with the ElementId of the element that cuts it, named for the wall it voids, and holds `Pset_OpeningElementCommon.Reference` and `Pset_EnvironmentalImpactIndicators.Reference` (the cutter's type name; the host's for an opening of its own geometry), a `Qto_OpeningElementBaseQuantities` and, where a door cuts, `FireExit`. By what cuts:

| cutter | as built | at permit | host | written before RE-321 |
|---|---:|---:|---|---|
| a floor's sketch void (RE-151) | 36 | 35 | floor | 27 of 36 and 27 of 35 |
| a roof's hatch (RE-225) | 4 | 4 | roof | 1 key of the 4 (they share a Tag) |
| a curtain wall embedded in a wall | 70 | 72 | wall | no |
| a fire hose cabinet (`IfcFireSuppressionTerminal`) | 39 (31 `.OPENING.`, 8 `.RECESS.`) | 38 | wall | no |
| a rectangular wall opening (`SWallRectOpening`) | 30 | 24 | wall | no |
| a door in a second wall | 39 | 43 | wall | no |
| a door in itself | 4 | 2 | the door | no |
| a wall in itself (`US-12`) | 2 | 2 | wall | no |

Core Interior's export holds 63 (42 floors, 20 shading devices, 1 wall), all of which rvt-rs writes since RE-151 and B55; RE1 Architecture's holds 3 and the two terrain files 1, none of the kinds below. `tools/re/unfilled_openings_vs_ifc.py` keys them by the cutter's Tag and the host's, which no two share.

## 2. Which records name them

A wall's native record lists the elements that cut it, one step each, and each step's `m_instId` is the cutter:

- `WallEmbedWallGStep`: a curtain wall (69 of the 70, the 70th by a `WallShadowCutoutGStep`);
- `WallCutoutGStep`: a family instance (the 39 cabinets and 38 of the 39 doors cutting a second wall, the 39th by a `WallShadowCutoutGStep`);
- `WallRectOpeningGStep`: a rectangular wall opening, whose own `SWallRectOpening` record names the wall in `m_hostId`.

The order of the steps is Revit's numbering of the wall's openings. revit-ifc's `OpeningUtil.AddOpeningsToElement` (CLEANROOM.md source 5) numbers the openings of a host from 1 as `ExporterIFCUtils.GetOpeningData` lists them, and `CreateOpeningGUID` hashes `Opening:<number>Solid:<solid>`, the host's GlobalId and the cutter's. The number is the step's place in the record, counting the steps of every kind (those of the doors and windows that fill openings, too). The GlobalId so built, from the host's and the cutter's GlobalIds (`revit_global_ids`, the cutter's being the `SWallRectOpening` record's own for a rectangular opening), is Revit's on 68 of 68 curtain walls written, 39 of 39 cabinets and 30 of 30 rectangular openings as built, and on 70, 38 and 24 at permit.

## 3. Bodies

Every body Revit writes for these three kinds is a rectangle extruded across the wall:

- **A rectangular opening:** the two corners its `RectOpeningData.m_aPoint` saves, along the wall and up, through the whole wall (depth the wall's thickness). Its element record's bounding box is not this: it is padded to 8 ft high, and 12 of the 24 at permit differ from Revit's by up to 0.95 ft. The corners are exact, within 0.003 ft, on 30 of 30 as built and 24 of 24 at permit.
- **A curtain wall:** its record box along the wall and up, across the wall the host's thickness and half a foot beyond each face (739.8 mm in a 435 mm wall). Where the curtain wall reaches the wall's end Revit cuts the opening back to the wall's body by 0.19 to 0.71 ft, which the box does not give: the box cut back to the wall's record box equals Revit's on 38 of 70 as built, and the opening written against rvt-rs's wall is within 0.01 ft on 52 of 68 as built and 46 of 70 at permit (Measure 41738481767).
- **A cabinet:** the box of the solid its symbol cuts (`m_complexCutGeoms`, RE-225), placed by the instance's own transform, `InstanceInfo.m_Trf` with the rotation `m_3x3` applied to the symbol's coordinates and `m_or` added: 39 of 39 as built and 38 of 38 at permit within 0.002 ft. It is `.RECESS.` when the box is shallower than the wall is thick and `.OPENING.` otherwise, which is revit-ifc's own test (`OpeningUtil.CreateOpening` compares the opening's length with the host's width): 8 of 8 recesses and 31 of 31 openings as built, 9 and 29 at permit.

`Qto_OpeningElementBaseQuantities.Depth` is the extrusion's depth on every opening of the three kinds. On a rectangular opening Revit's `Width` and `Height` are the rectangle's extents along the wall and up, the right way round (30 of 30 as built, 24 of 24 at permit), unlike a door's opening, whose are swapped (RE-244, RE-235 section 5); they are written. On the other two kinds they were not compared.

## 4. What is written and what is not

A rectangular opening is written where its wall is: the 87 steps of `WallRectOpeningGStep` name 87 distinct openings, 30 of which Revit writes as built (24 of 80 at permit); the others are in walls Revit's export does not write and rvt-rs does not. The writer (`ifc::wall_cut_openings`) is `IfcEntity::WallCutOpening`, placed in the wall's frame as the opening of its elevation profile is (B55), tagged with the opening's ElementId, named for the wall, `Pset_OpeningElementCommon.Reference` and `Pset_EnvironmentalImpactIndicators.Reference` the wall's type name, `Width`, `Height` and `Depth` the rectangle's extents and the wall's thickness, and the GlobalId Revit's.

**One opening at permit is counted apart (the maintainer, 2026-11-03, as RE-253 and #1048 for a held-out wall).** 2793737's record (`RectOpeningData`, phases, design option), its wall's `WallRectOpeningGStep` and the wall's extent are byte for byte the same in the as-built and the permit file, and Revit's export writes the opening as built and does not at permit; no other opening overlaps it and it lies inside the wall's body. Nothing read says why: the permit export reads as made from another state than the saved file. The rule is generic and in the scorer: an unfilled opening rvt-rs writes that Revit's export of the file does not hold, but its export of a sibling file (another Revit export in the same directory with the same first four name words, here the office as built) does, is counted on its own line and not as an extra. No ElementId is in the writer or in any list.

The other kinds write an opening Revit's export does not, or a wrong one, and what separates them is in neither file:

| kind | exact | written that Revit's export lacks | wrong |
|---|---|---|---|
| a rectangular opening | 30 of 30 as built, 24 of 24 at permit | 0, and 1 at permit counted apart (above) | 0 |
| a fire hose cabinet | 39 of 39, 38 of 38 | 1 as built (3388647 in wall 1494012), 2 at permit (3390319 in 1728013, 3387194 in 3363820) | 0 |
| a curtain wall | 52 of 68 | 3 as built (1815871 and 1892067, well inside their walls, and 2295010) | 16 |

- **The cabinets.** The step is `m_bCutSomething` 1 on all 40, and the cabinet's root and symbol, the step's fields, the wall's body (which holds the recess as a notch in all 40, the omitted one's too, 8 vertices inside the cabinet's box), the cabinet's box (no two share one) and its reach into the wall (50 mm on all) show no difference between the 39 and the 1 (2 at permit).
- **The curtain walls.** Both the 16 boxes and the 3 extras are walls whose ends or whose neighbours cut the opening. A rule that takes only a curtain wall at least half a foot inside its wall's ends is exact on 28 of 29 as built; the 29th (3030507 in 2991697) is cut back 0.328 ft in the middle of a long wall, where an adjoining wall ends, and the extras 1815871 and 1892067 lie 0.66 ft and more inside theirs. The three extras' `WallEmbedWallGStep` holds more edge history (78 and 81 entries) than the steps of the openings Revit writes that were compared, but that is a count of the step, not a rule that was shown to hold.

**Measured as it lands** (41748430211, `-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true`):

| | office as built | office at permit |
|---|---:|---:|
| rectangular openings written, Revit's | 30 of 30 | 24 of 24 |
| box within 0.01 ft; GlobalId, name, type, both `Reference`s equal | 30, 30 | 24, 24 |
| property values equal, before / after | 44,163 / 44,223 of 45,525 / 45,585 | 44,339 / 44,387 of 45,704 / 45,752 |
| quantities equal, before / after | 29,448 / 29,538 of 39,388 / 39,478 | 29,762 / 29,834 of 39,679 / 39,751 |
| values and quantities missing or different | unchanged (1,361 and 1; 9,939 and 1) | unchanged (1,364 and 1; 9,916 and 1) |
| values Revit's export does not write | 10, unchanged | 14 to 16: the two `Reference`s of the counted-apart opening |
| openings Revit's export lacks (scorer's extras) | 17, unchanged: 9 floor-void and 8 `none in IfcWall` | 16, unchanged, and 1 counted apart |

The 8 `none in IfcWall` extras on both files predate this change: the same 8 are in `main`'s export in the same run, as are the 9 and 8 floor-void extras and the two terrain files' 2 each. Core Interior (63 of 63), RE1 Architecture, the R2025 models, Einhoven and the held-out models' counts are byte for byte `main`'s; so are the scorer outputs of every sample but the exports' attribute counts in the Autodesk samples, where the same rule writes openings: Rac Basic +2 and Rac Advanced +1 in each of 2024 to 2027 and Snowdon Towers Architectural +9. The samples have no Revit export in reach but Snowdon Towers', Revit's own IFC4 export of its architectural file (local only, `_corpus_candidates/snowdon/`): 9 of the 9 openings rvt-rs writes there are in it, by Tag and host (Measure 41754617363, the kept IFC compared with that export by Tag and host, the scorer's keys), and the other samples are NOT MEASURED.

## 5. What is left

- **A door in a second wall** (39 as built, 43 at permit; 37 extrusions, 2 meshes). On the doors printed, the opening is the door's cut loop (RE-215) in the door's frame, across the loop's plane by the thickness of the door's own wall (0.328 ft, 100 mm) on some and by 10 ft, 5 ft either side of the loop's plane, on others; nothing read decides which, and 2 are meshes.
- **A door in itself** (4, 2) and **a wall in itself** (2): the openings of RE-225's symbols cut solid in the door and of the wall's own profile; no step names them.
- **A floor's sketch voids** (9 of 36 as built are not written; they are on main's census) are RE-151's. The 8 `none in IfcWall` openings the scorer lists as written that Revit's export lacks on both office files predate this change: they are in main's export.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref re/1165-unfilled-openings -f ref=<sha> -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=unfilled_openings_vs_ifc \
  -f probe=probe_re321_unfilled_openings -f probe_scorer=-
```

The runs above measured commits of branch `re/1165-unfilled-openings` (4d8c9c56, e12246a8, a7931166, ff53b6cd). The cabinet and curtain-wall writer that a7931166 measured was removed in ff53b6cd, and is in the branch's history.
