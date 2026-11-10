# RE-390: a railing that names a stair is aggregated under it, unless it sits on one of the stair's components

**Date:** 2026-11-09
**Issues:** #1624 (stair bodies short by their railings); follows RE-39 (stair aggregates), RE-254 (a stair railing's `BaseRailing.m_stairsId` and path) and RE-65 (stair names)
**Probe:** `probes/examples/probe_re390_railing_stairs.rs`
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them); puzzbobb's R2025 stair models 13 to 17 (Revit 2025, Revit 25.0.2's IFC4 exports); Autodesk's Snowdon Towers Architectural (Revit 2024, local only) with Revit's IFC4 export
**Runs:** Measure [42679471967](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42679471967) (unfiltered, aggregation without the component rule, head `4ced3176`), [42686643077](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42686643077) (the probe on both office files), [42685410895](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42685410895) (the office's export and Revit's kept, the storey and aggregate scorers) and [42688171578](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42688171578) (unfiltered, head `a43f517d` against `main`, section 4)
**Status:** Positive. Revit's export aggregates each stair railing under its stair on every stair railing measured but one, and that one is told apart by a field of its own record; with that rule rvt-rs's aggregate is Revit's on all 26 stair railings of each office file, on the 10 of the five R2025 stair models and on 65 of 65 that rvt-rs writes on Snowdon Towers. Negative: the stair assembly's volume is now larger than Revit's, because the railings it holds are record boxes (section 4).

## 1. What Revit's export does

Revit's IFC4 export puts a railing hosted on a stair among the parts of its `IfcStair` (`IfcRelAggregates`), beside the stair's runs, landings and stringers, and gives it no storey of its own. rvt-rs wrote them standalone (RE-39 section 3 measured 65 of Snowdon Towers' 70 railings that name a stair as aggregated and could not tell the other five apart), so a stair, which has no body of its own and is the sum of its parts, was short by its railings: 2.74 ft of height and 0.72 ft3 of volume on R2025 model 13.

On the office 26 railings name a stair (`BaseRailing.m_stairsId`, RE-254): 25 are aggregated under it in Revit's export, and one, Tag 2598405, is not. It is a handrail (`KJO-1`) on Level_01 and a plain element of that storey, while its stair, 2598347, is a half-turn stair of the Basement that aggregates its two other railings, 2598350 and 2598353, with its two runs and its landing. Revit's export writes nothing on it that the two others lack: the same `Pset_RailingCommon` and `Qto_RailingBaseQuantities` keys.

## 2. What tells it apart

The probe prints every field of every railing's `BaseRailing` object and its stair's `StairsElement`. Of the 41 railings of each office file only the odd one differs from the aggregated ones in what it names:

| Field of `BaseRailing` | Railing 2598405 | The 25 aggregated |
|---|---|---|
| `m_stairsId` | 2598347 | their stair |
| `m_stairsComponentId` | 2598348, the stair's Run 1 (the `IfcStairFlight` Tag 2598348 in Revit's export) | -1 (none) |
| `m_assocLevelId` | Level_01 | -1, or Level_02a_Parking on two that are aggregated |

`m_stairsComponentId` is set on this one railing and on none of the other 40, both files, and names a run: the railing is hosted on a component of the stair, not on the stair. `m_assocLevelId` is not the discriminator, since two aggregated railings carry one. The other fields (`m_placementOffset`, `m_stairsRailingAttrId`, `m_continuousRailIds`, `m_registeredLocationBackup`) differ between any two railings, aggregated or not.

## 3. What rvt-rs writes

A railing whose record names a stair (`m_stairsId`) and no component of it (`m_stairsComponentId` absent) is aggregated under that stair on Revit 2024 to 2027, where a run, landing or stringer already makes the stair a whole; a railing that names a component is left standalone, as Revit's export leaves it. The rule rests on one railing, which is one in each of two saves of one project: a rule that is Revit's on every railing measured, not shown on a second model with such a railing. A stair no part names keeps its own body.

## 4. Measured

**Aggregates scorer** (`aggregates_vs_ifc.py`, a railing's whole in each file; the office and R2025 rows are run 42688171578, head `a43f517d` against `main`):

| Export | Railings that name a stair | Same whole as Revit's | Aggregated by rvt-rs only | Aggregated by Revit only |
|---|---:|---:|---:|---:|
| office as built and at permit (each) | 26 | 25 (before the rule: 25 and 1 that Revit leaves out) | 0 (before: 1) | 0 |
| R2025 stair models 13 to 17 (each) | 2 | 2 | 0 | 0 (on `main`: 2) |
| Snowdon Towers Architectural | 76 under `IfcStair` in Revit's export | 65 | 0 | 0 under `IfcStair`; 4 under an `IfcRamp` |

On `main` none of the office's 25 was aggregated. Snowdon's 142 railings: Revit's export aggregates 76 under an `IfcStair` and 4 under an `IfcRamp`; rvt-rs exports 131 of the 142, so 11 of Revit's 76 are railings rvt-rs does not export at all. The 4 railings Revit aggregates under a ramp are not written: a ramp is not a stair whole here, and the 8 office railings that name a ramp are none of them aggregated by Revit. The other 62 railings of Snowdon and the 15 of the office are aggregated by neither file.

**The same run, every model** (118 models, head against `main`): 20 exports moved (both office files, the five R2025 stair models, Snowdon Towers, the eight racbasic and racadvanced saves of 2024 to 2027 and the four held-out saves), 0 census regressions in the property and quantity censuses, and the scorers that differ are `aggregates_vs_ifc` (7 models: the two office files and the five stair models, each railing that names a stair from "only Revit aggregates" to "same whole"), `storeys_vs_ifc` and `mcp_answers_vs_ifc` (2 models each, the office files) and `stair_bodies_vs_ifc` (the same 7). On the office the 41 railings are 25 aggregated by both files with the same whole and 16 aggregated by neither (Tag 2598405 and 15 others, of which 8 name a ramp).

**Storeys** (`storeys_vs_ifc.py`, each office file): rows on a different storey from Revit's fall from 6 to 4 (the 2 `IfcRailing` rows gone, the 4 `IfcRamp` rows stay), and the same-storey rows rise from 12616 to 12618 in the as-built file and from 12662 to 12664 in the permit file. The scorer lists the differing rows by type only, so the two railings are not named here. The MCP storey answers move the same 2 rows in the as-built file.

**Stair bodies** (`stair_bodies_vs_ifc.py`, the stair as the sum of its parts): R2025 model 13's stair has every vertex of Revit's body within 0.001 ft of rvt-rs's surface where it had none (0 of 1 to 1 of 1; its farthest Revit vertex 2.75 ft to 0.306 ft, its z range 0 to 5.35 ft to 0 to 8.09 ft, Revit's); the other stair models and the office's 35 stairs keep their count (the spiral model 17's stays 0 of 1).

**A negative: the stair assembly's volume is larger than Revit's.** The railings now in the sum are record boxes, a box of the railing's whole extent, where Revit's railing is a thin body (every railing read under stairs 2598347 and 2859203 of the as-built export, 5 of 5, carries `BodySource` `partition_element_record_bbox`; RE-303 is why a railing's saved mesh is often not whole and the box stays):

| Stair | rvt-rs on `main` | rvt-rs with the rule | Revit |
|---|---:|---:|---:|
| office 2598347 (ft3) | 100.9 | 707.6 | 101.6 |
| office 1503002 | 38.9 | 81.4 | 41.1 |
| office 3057027 | 6.0 | 911.9 | 7.2 |
| the office's 10 other stairs that hold railings (each) | 110.9 to 116.7 | 3612.8 to 4094.4 | 110.4 to 116.6 |
| R2025 model 13 | 8.06 | 25.9 | 8.78 |
| R2025 model 14 | 8.00 | 23.8 | 8.71 |
| R2025 model 17 (spiral) | 683.7 | 1365.1 | 58.9 |

These are the same 13 stairs of each office file (the as-built file's figures; the permit file's differ by under 0.5 percent). Nothing exported reads the sum: the property and quantity censuses move on no model, and a railing's own body and quantities are the same standalone or aggregated. What is read is a stair opened in a viewer or scored by `stair_bodies_vs_ifc`, which draws the railing boxes it always drew inside the stair that now owns them. The volume of the assembly is not exact and stays so until the railings' bodies are (RE-303).

## 5. Not shown

- **A stair whose parts none names.** Its railings stay standalone: a stair no run, landing or stringer names keeps its own body, and a railing alone must not take it away.
- **Railings on a ramp.** Revit's export aggregates the 4 of Snowdon Towers under their `IfcRamp`; rvt-rs does not (the 8 of the office are not aggregated by Revit either), so the ramps' bodies are not summed from their railings.
- **The stair body's volume.** A stair that is the sum of its parts now includes each railing's body, which is a record box (RE-303 holds the railings whose saved meshes are not whole): the stair assembly's volume read by `stair_bodies_vs_ifc.py` is larger than Revit's by that (section 4: 2 to 127 times on the office's 13 stairs that hold railings), where before it was short by the railing.
- **Snowdon's component rule.** The probe runs on the Measure models only; Snowdon is local and its railings that name a component, if any, are not listed. Its 65 of 65 aggregated railings are Revit's and none of its aggregated railings is one Revit leaves out.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref feat/stair-railings-aggregate -f ref=ade6d7de4bc4b2a971b91c82c14e85853e792259 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit" -f scorers=none -f probe=probe_re390_railing_stairs -f probe_scorer=-
gh workflow run measure.yml --ref feat/stair-railings-aggregate -f ref=a43f517d8953c9aaaf5aa1b34fb1cc19acf5159b -f base=main -f nordic=true -f r2025=true \
  -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
```
