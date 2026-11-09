# RE-375: what Revit's export does with the part of a wall's plan that a floor leaves uncovered

**Date:** 2026-11-09
**Issues:** #1067 (the stub of wall 3004978, RE-320 sections 4 and 7); follows RE-267 and RE-320.
**Artefacts:** the BIM4LCA office as built (`ARK_NordicLCA_Office_Concrete_As-Built.rvt`, sha256 `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906`; Revit's export `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382`) and at building permit (`1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1`; `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d`), Revit 2024, CC BY-SA 4.0, as pinned in `research/bim4lca-measure.tsv`; puzzbobb's 24 Revit 2025 models, Core Interior, the four RE1 models and the two terrain offices as Measure fetches them; Autodesk's Snowdon Towers Architectural 2024 (sha256 `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1`) with Revit's own IFC4 export of it, which is local only, so its results are counts here.
**Oracles:** Revit's own IFC4 exports of those models.
**Scorer:** `tools/re/wall_outside_floor_vs_ifc.py`.
**Status:** negative for a rule that rests on more than one wall. Over 225 pieces of five buildings, Revit keeps the wall's own height on 191 and cuts it on 5 (the 2 pieces of the stub of 3004978, 2 on a wall under a sloped roof, and 1 on a wall of Snowdon Towers whose cutter is not attributed); the other 29 are walls whose finish layers end at another height. Three readings fit the stub and disagree with no other piece: a piece under 0.05 ft² and inside other walls is cut; a piece wholly past an end of Revit's Axis takes the cut of that end; an end piece from 0.05 to 0.2 ft wide is cut. Each holds one wall, or two with one under a sloped roof, and no feature the scorer reads (width, area, place along or across the wall, whether the piece lies inside another wall, where the Axis ends, how Revit draws the wall) separates the 5 from the 191 on more than that. The first reading ships in #1596, labelled and counted apart (section 8).

## 1. The question

RE-320 draws a wall's top at the underside of the floor, roof or footing its top join names over the part of the wall's plan that element covers, and at the wall's own height over the rest. On the BIM4LCA office as built that is Revit's top on 606 of 607 walls. The one it is not is 3004978, which RE-320 section 4 leaves at its full height: its roof covers all of its plan but a stub 0.0853 ft long at one end, wholly inside the wall it abuts, and Revit cuts the stub with the rest of the wall. RE-320 found nothing that separates the stub from the pieces Revit keeps, and the only rule it could state rests on that one wall (`wall_tops::embedded`). This report asks the question of Revit's own bodies, over every wall of the models that have a Revit export, to see whether something does.

## 2. What the scorer reads

For each `IfcWall` both files hold under one `Tag` (the pairing and the fit of Revit's frame onto rvt-rs's are those of `wall_top_cover_vs_ifc.py`), from Revit's bodies alone:

- the elements that cut the wall: the floors, roofs and footings of Revit's export whose lowest point lies inside the wall's height, whose plan meets the wall's, and under whose plan Revit's wall has horizontal faces at that lowest point (over 0.001 ft²);
- the pieces of Revit's plan of the wall outside the plans of all of them, over 1e-4 ft²;
- for each piece: how much of it Revit's body has at the wall's own height (the higher of the two files' highest points), which is `kept` at 0.99 or more, `cut` at 0.01 or less and `mixed` between; where it lies along the wall's plan (the whole length, one end, neither) and across it (both faces, one, neither); how much of it is inside another wall that shares more than 0.5 ft of height; and, where Revit writes a straight `Axis` (a polyline, an indexed poly curve or a trimmed line), whether it lies wholly past an end of the Axis, across one, or between them.

Only floors, roofs and footings are read as cutters. Revit also cuts walls at other elements (a ceiling, a stair), so a piece Revit cuts may be cut by an element the census does not read.

## 3. Results

Measure runs [42588689964](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42588689964) (every model with a Revit export), [42589969107](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42589969107) (the two offices, with the Axis read) and [42591992609](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42591992609) (four Revit 2025 models, with the Axis read), the scorer from the branch that holds it and `main` at `b042c61c4fb15f6628f6be17bc0d32557aa96d48` for the code. Snowdon Towers is run locally: `rvt-ifc` from run 42589969107's `measure-bin-head` artifact, in a Linux container, against Revit's IFC4 export.

| model | walls in both files | walls an element cuts in part | pieces | kept | mixed | cut |
|---|---:|---:|---:|---:|---:|---:|
| the office as built | 607 | 49 | 56 | 51 | 3 | 2 |
| the office at permit | 589 | 46 | 53 | 52 | 1 | 0 |
| Snowdon Towers Architectural (local only) | 1,078 | 109 | 114 | 88 | 25 | 1 |
| Revit 2025 models, under a sloped roof | 5 to 16 | 4 | 2 | 0 | 0 | 2 |
| Core Interior, RE1, the terrain offices | 2 to 360 | 0 | 0 | 0 | 0 | 0 |
| **all** | | | **225** | **191** | **29** | **5** |

The 29 `mixed` pieces are on walls whose finish layers Revit ends lower than the core (RE-320 section 7): Revit's body is at the wall's own height over part of the piece and lower over the rest. The 2 Revit 2025 pieces are one wall in two models, under a roof by rafters that slopes; the wall's top follows the slope, which is not a cut at a level, and rvt-rs draws no wall under such a roof (RE-267), so they are not compared below.

## 4. The stub

Wall 3004978 of the office as built, and the roof 1875411 its top join names:

- Revit's roof leaves the same stub uncovered as rvt-rs's does: `pair`: rvt-rs covers 0.9849 of the wall's plan, gap 0.0243 ft², and Revit covers 0.9849, gap 0.0243. The roof's outline is not at fault.
- Revit's wall top is 246.3025 ft along all 5.01 ft of the centre line, the stub's end included. rvt-rs's highest point is 255.5446, 9.2421 ft over Revit's (`wall_heights_vs_ifc`: the wall is the one of 607 whose top is not Revit's).
- At building permit the same wall ends flush with the roof's edge (x 300.2910 in rvt-rs's frame), with no stub, and its top is 245.8005 in rvt-rs and in Revit: equal. As built its plan is 0.0853 ft longer, into wall 1630903, whose plan holds the whole stub.
- Revit's `Axis` of the wall starts 0.0853 ft inside its plan, at the roof's edge: the stub lies wholly past the Axis start. Revit draws the wall as three extrusions, one for each layer (`SweptSolid`); rvt-rs draws one.
- RE-340's saved mesh of the wall (a probe edit that is not landed; Measure run 42583199886) is `partial_saved_graphics`, 16 triangles, and ends at x 300.2910, the roof's edge, in both files: the native solid does not hold the stub. The IFC body is the longer one and is cut at the roof's underside over its whole length.
- What the wall costs today: rvt-rs's body is 31.66 ft³ against Revit's 16.72, 14.94 ft³ over. Stepped as RE-320's first draft drew it, the body would differ from Revit's by the stub's 0.0243 ft² standing 9.2421 ft over Revit's top, 0.22 ft³ (computed from the areas and heights above, not measured by a run). Cut whole, it is Revit's.

## 5. What does not separate the cut pieces from the kept ones

The two cut pieces of the stub, the one cut piece of Snowdon Towers, and the 191 kept. The Snowdon piece is on a wall whose top Revit steps at more places than the floors read give, so its cutter is not attributed.

| feature | the stub (office as built) | the Snowdon piece | kept pieces |
|---|---|---|---|
| width along the wall | 0.0853 ft (and 0.0427 for its second polygon) | over 1 ft | at an end, 3 under 0.05 ft (0.0313, 0.0321 and 0.0328, the office as built), none from 0.05 to 0.2 ft, 3 of 0.2 to 1 ft and 65 of over 1 ft; the other 120 are not at an end |
| area | 0.0243 ft² in all | over 1 ft² | from 0.0018 ft² (the office as built) up |
| inside other walls | all | 0.84 | 5 of 191 at 0.99 or more (1 on the office, 4 on Snowdon Towers), each of 0.05 ft² or more; the rest less, most none |
| Revit's Axis | wholly past the start | between the ends | 2 wholly past an end (Snowdon Towers, whose Axis lies outside every floor), 80 across an end, 109 between the ends |
| how Revit draws the wall | `SweptSolid`, 3 extrusions | `SweptSolid`, 6 | both forms, in both |

Three readings that would have made a rule hold one wall each:

- A piece inside other walls and under 0.05 ft² is cut, and none is kept: true, but the class holds one wall. No piece of any other model is in it, so the data cannot say it separates; it only says nothing disagrees.
- A piece past an Axis end takes the cut of the Axis end it lies past (Revit's export extending the native solid's profile past its end, which the saved mesh of section 4 fits): true of the stub, whose Axis end is under the roof, and of the 2 kept pieces on Snowdon Towers, whose Axis ends are under no floor. It also needs the stub's Axis end to lie on the roof's edge, where a point is under the roof or not by 1e-4 ft, and the census holds no other piece past an Axis end under a floor.
- An end piece from 0.05 to 0.2 ft wide is cut: the only two in the census, the stub and a piece of a wall under a sloped roof, both are, and the kept end pieces are narrower (3, under 0.05 ft) or wider (68, over 0.2 ft). That is two points for two thresholds, with no mechanism, and one of the two is under a roof the census does not read.

## 6. Not measured

- Cutters other than floors, roofs and footings (ceilings, stairs), and walls under a sloped roof: the census does not read them, and the Snowdon piece and the 2 Revit 2025 pieces may be theirs.
- The held-out models: the scorer prints the pieces' `Tag`s, which a held-out run does not keep.
- Revit 2015 to 2023 and 2025 to 2027 beyond the 24 test models: nothing there has a wall an element cuts in part and a Revit export that holds it, bar those.
- Why Revit cuts the stub: the census finds the one piece, and the saved mesh and the Axis say where the native solid ends, not why the export extends it at the cut level.

## 7. Reproduce

```bash
# Every model with a Revit export, and the two offices with the Axis read:
gh workflow run measure.yml --ref main -f ref=b042c61c4fb15f6628f6be17bc0d32557aa96d48 -f base=none \
    -f nordic=true -f r2025=true -f scorers=wall_outside_floor_vs_ifc
gh workflow run measure.yml --ref main -f ref=b042c61c4fb15f6628f6be17bc0d32557aa96d48 -f base=none \
    -f nordic=true -f models="nordic-office-*" -f scorers=wall_outside_floor_vs_ifc
# Snowdon Towers, local only: rvt-ifc built from main on the sample, then
python3 tools/re/wall_outside_floor_vs_ifc.py <rvt-ifc export of the Architectural sample> <Revit's IFC4 export of it>
```

The saved mesh of section 4 needs RE-340's probe with the wall named (`probe_1385_wall_saved_mesh`), which is not landed. Section 8's run is in that section.

## 8. The rule that ships

#1596 applies the first reading of section 5 and nothing else. `wall_tops::steps` drops what the lowest underside leaves of a wall's plan where `wall_tops::embedded` says it is a stub (under 0.05 ft², 0.99 or more of it inside other walls' plans), so that the wall is cut whole at that underside, as Revit cuts it; the wall carries `TopCoverStub` (`inside_another_wall`) and `exported.wall_top_stubs_cut` counts it. Among the 225 pieces of section 3 the stub is the only one that meets both conditions, so the rule rests on one wall, as section 5 says of each reading; what a Measure adds is whether it disagrees with any other wall.

Measure run [42598250689](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42598250689): the head `abc505ad4dbd7fed3c1bd46cf5ab8f4f5b8c5a54` against `main` at `4ad2ce90d1d79dacd680357db7b7331348c1ffe0`, `-f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true`, every scorer, 118 models on each side.

- The office as built: wall 3004978's top goes from 255.5446 to 246.3025 ft, which is Revit's, and `wall_heights_vs_ifc` gives base and top Revit's on 607 of 607 walls (606 before); the line for the wall whose top was not Revit's is gone. `wall rules: top stubs cut` reads `inside_another_wall=1`.
- The other 117 models read `none`, the 18 held-out ones included (a held-out run keeps counts only). The 18 held-out exports are byte-identical to `main`'s.
- Seven exports differ from `main`'s: the office, and six that the head's `earcut` re-splits. The head predates #1592, so its `Cargo.lock` holds `earcut` 0.4.11 against `main`'s 0.4.12, and the six are the ones that PR names: rac_advanced on 2023, 2024, 2025, 2026 and 2027, and Snowdon Towers Architectural 2024. Their `diagnostics.json` differ from `main`'s only by the new `exported.wall_top_stubs_cut`, empty on each (as on every other model that reads `none`), `top stubs cut` reads `none` on each, and no scorer output of theirs changes. The other 111 exports are byte-identical.
- No other scorer output moves: the scorer diff holds the office's `wall_heights_vs_ifc.txt` and `wall_rules.txt`, and the two Areas gate results, which only the head side runs.
- `same-project gates` passed on the head: 15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save.

What this does not show: that Revit cuts the stub for a reason the rule names. Section 6's last item stands, and a wall whose floor leaves a stub that is not under 0.05 ft² or not inside other walls is drawn as before. A second wall the rule cuts would show in `exported.wall_top_stubs_cut`, and Revit's export of that file would be the first test of the reading beyond this wall.

To reproduce, from a commit that holds the rule: `gh workflow run measure.yml --ref main -f ref=abc505ad4dbd7fed3c1bd46cf5ab8f4f5b8c5a54 -f base=main -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true`. `base=main` is `main` on the day it runs, so a later run also shows what `main` has changed since `4ad2ce90`.
