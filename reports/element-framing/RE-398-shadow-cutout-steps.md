# RE-398: a wall's shadow cutout step names the step of another wall, and its blocker is that step's

**Date:** 2026-11-10
**Issues:** #1165 (unfilled openings Revit's export writes and rvt-rs does not); follows RE-321 and RE-338 (what a wall's native record says cuts it)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024; Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them); Autodesk's `rac_advanced` sample project of Revit 2024 to 2027 and Snowdon Towers Architectural 2024, which Measure fetches for a run and never stores
**Runs:** Measure [42882469082](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42882469082) and [42886617099](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42886617099) (`probe_re398_shadow_cutout_links`, before and after it printed a shadow step's own flag; the second is the one section 2 reads); Measure [42897187670](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42897187670) for the change (unfiltered: `nordic`, `r2025`, `samples`, `racadv`, `bimcollab`, `holdout`; head `05ab584d` against `main` at `9066e124`, 118 models on each side)
**Status:** Positive, as attribution only. All 124 `WallShadowCutoutGStep`s of the office, `rac_advanced` and Snowdon Towers Architectural name a wall that holds the step they mirror, an embed or a cutout of the same cutter. They are read as that step's kind, and the export counts the cut it leaves out under that step's reason rather than as `shadow_cutout_step_not_read`. No export changes on any of the 118 models, so no opening is written that was not, and no scorer output moves: only the left-out counts in seven files' `diagnostics.json`.

## 1. The question

`exported.wall_cut_openings_left_out.shadow_cutout_step_not_read` counted the steps of a wall's native record that are a `WallShadowCutoutGStep` (2 on the office as built, 1 at permit, 1 on each of `rac_advanced` 2024 to 2027 and 54 on Snowdon Towers Architectural), and said nothing about why Revit's opening is not written, because the step's own fields hold no box. #1165 lists such openings among the ones Revit's export writes and rvt-rs does not. Which wall's record holds the cut, and is a shadow's reason the same as that wall's?

## 2. What a shadow step is

`probe_re398_shadow_cutout_links` lists every shadow step of a file with what its names lead to. A shadow step holds the cutter (`m_cutter`), `m_otherWallId` and `m_ggId`; the other wall's record holds an embed or a cutout step of the same cutter whose `m_id` is the shadow's `m_ggId`. On all 124 shadow steps of the seven files that hold any, that step exists, is of the same cutter and is an embed (12) or a cutout (112):

| file | shadow steps | resolve to a step of another wall |
|---|---:|---:|
| office as built | 2 | 2 |
| office at permit | 1 | 1 |
| `rac_advanced` 2024, 2025, 2026, 2027 | 1 each | 1 each |
| Snowdon Towers Architectural 2024 | 117 | 117 |

By the class of the cutter's export: 50 doors, 32 windows, 27 building element proxies (families), 12 curtain walls (the embeds) and 3 openings. A shadow's own `m_bCutSomething` is 0 on 85 and 1 on 39, and the mirrored step's is 1 on 120 of them: the two flags are not the same on 81 of the 124 (a shadow's is about its own wall, where the cutter is not hosted; the other step's is about the other wall). RE-338 reads the flag as "a further wall the cutter reaches" when it is 0 and "the cutter's own host" when it is 1, so the shadow keeps its own.

## 3. The change

`native_wall_cuts::resolve_shadow_steps` replaces the kind of each shadow step with the kind of the step it mirrors, where the other wall holds one of the same cutter (an embed or a cutout, matched by `m_ggId` against `m_id`), and keeps the shadow's own `m_bCutSomething`. A shadow whose other wall holds no such step stays a shadow. `attach_wall_cut_openings` calls it on the steps it reads. An earlier version took the mirrored step's flag as well; the probe's own-flag column found it wrong on the 81, and it was corrected before this measurement.

## 4. Measured

Run 42897187670, `-f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true`, the change at `05ab584d` against `main` at `9066e124`, 118 models on each side. Nothing moves in any export (`ifc.sha256` is equal for all 118), and the census, the body scorers and the placement scorers read as on `main`. Of every output only `diagnostics.json` differs, in seven files, in the left-out counts:

| file | `shadow_cutout_step_not_read` | moves to |
|---|---:|---|
| office as built | 2 to 0 | `curtain_wall_within_half_foot_of_wall_end` 41 to 42, `door_opening_box_not_read` 1 to 2 |
| office at permit | 1 to 0 | `curtain_wall_within_half_foot_of_wall_end` 30 to 31 |
| `rac_advanced` 2024, 2025, 2026 | 1 to 0 | `curtain_wall_within_half_foot_of_wall_end` 2 to 3 |
| `rac_advanced` 2027 | 1 to 0 | `curtain_wall_box_not_read` 2 to 3 |
| Snowdon Towers Architectural | 54 to 0 | `door_second_wall_parallel_to_door_wall` 4 to 38, `door_cuts_a_wall_it_is_not_hosted_in_as_its_own` 3 to 18, `curtain_wall_box_not_read` 13 to 18, `wall_body_not_an_axis_aligned_prism` 4 to 5 |

On Snowdon the 54 land on the reasons that already stood for the same cutters' other steps, and the largest, `door_second_wall_parallel_to_door_wall` (Revit cuts the opening ten feet across, about the plane the door's symbol cuts in), is now the blocker of 38 openings that were counted under no reason of their own.

## 5. Not shown

- **No opening is written by this change.** A shadow's opening is written where the mirrored step's own rule writes it, and none of the 124 passes that rule on these files; the change only says why. Revit's IFC4 exports of the office hold, on all 3 of its shadow steps (2 as built and 1 at permit), an `IfcOpeningElement` whose `Tag` is the cutter's ElementId in the shadow's wall as well as in the other wall; the other 121 have no export to check.
- **The housing.** A pre-registered row on #1636 holds the rule to the BIM4LCA housing and the timber office before any run reads it there: the exports must not move and the counters must move as above.
- **#1165 stays open.** The next decoder is the one that now has most of the openings: the door's second wall, `door_second_wall_parallel_to_door_wall`.

## 6. Reproduce

The probe comes from `-f ref`, the commit of this change; the scorers from `--ref main`.

```bash
gh workflow run measure.yml --ref main -f ref=05ab584d1a7f4531406d929e9ed266e3c354f732 -f base=main \
    -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
gh workflow run measure.yml --ref main -f ref=05ab584d1a7f4531406d929e9ed266e3c354f732 -f base=none \
    -f nordic=true -f models="nordic-office-as-built nordic-office-permit" -f scorers=none \
    -f probe=probe_re398_shadow_cutout_links -f probe_scorer=-
```
