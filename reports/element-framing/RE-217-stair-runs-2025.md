# RE-217: A Revit 2025 stair run's treads and risers, from its native records

**Date:** 2026-10-24
**Issues:** #357 (stair flights not yet drawn). Follows RE-52 (a run's side profile from its sketch and run type), RE-92 (a run's ends) and puzzbobb's reading of `StairsRun.m_endWithRiser` by name (#357, 2026-10-05).
**Artefacts:** puzzbobb's `R2025_Models.zip` (Apache-2.0, #356, `research/r2025-models.tsv`, sha256 `8673d803fb0b9b6aa20d08448eeed16eb34222566bfaf7a2dc761c064dc30d6a`), with Revit's own IFC4 export of each model:

| file | sha256 |
|---|---|
| `13_Rooms_Stairs.rvt` | `dec47104b7794ab3f78094842df8a88d121a598f7c7e94948828d062b7ac684a` |
| `13_Rooms_Stairs.ifc` | `5480d5a37439a0f7080cdf92799e2efe5f88a35a69322734d46c5912bac6c69b` |
| `14_Rooms_Stairs_endwithriserOFF.rvt` | `f799ab0e8804ab5cd3e004188ca51822f66c1530cba6b057af1e28b68318c8a5` |
| `14_Rooms_Stairs_endwithriserOFF.ifc` | `f2b767316c16e2775fd070714937276656c8c4a4cad25942a58be17b9f35e8b2` |
| `15_Rooms_Stairs_NoRisers.rvt` | `650ed28b7894ccea2c0747c309860aae2cf927c47624474f5fd88e6ad6ac348f` |
| `15_Rooms_Stairs_NoRisers.ifc` | `6a074508e288d4819e3c4ce46dbec0fe03aa2f6f05dd0238c5c31ddcea60ad75` |
| `16_Rooms_Stairs_Monolithic.rvt` | `b8095380e20593317e7905023b1d616bf419e9bfdb987c8434e789c84efb1b6f` |
| `16_Rooms_Stairs_Monolithic.ifc` | `a56e38df2dc6f25d00c670613b2a8e58ce7c8e05ed9e13cfd9e26e7f7854f93f` |
| `17_Rooms_Stairs_Spiral.rvt` | `18040175103afcc6a57f9bf3e9ab4d7eadf112f9e39151318173aa39d9fe8099` |
| `17_Rooms_Stairs_Spiral.ifc` | `b4f7f3a0c6ea75eb7d989730aaf22ff6bf6108bc43d2e6ca7b8ea4cfe2b0e5b2` |

Measure fetches the archive and stores none of it.
**Probe:** `examples/probe_357_native_runs.rs`. **Scorers:** `tools/re/stair_bodies_vs_ifc.py`, `tools/re/run_types_vs_ifc.py`.

**Status:** positive on the two straight runs with treads and risers (models 13 and 14); the other three keep their box.

## 1. What was missing

RE-52 and RE-92 draw a straight run from its sketch (its bounded lines), its run type and its ends, all found at byte anchors measured on Revit 2024. None of those anchors is found on Revit 2025, so every R2025 flight was its record box: on model 13, 317.83 ft³ where Revit's flight is 4.49 ft³ (`stair_bodies_vs_ifc`, Measure run [40140290382](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40140290382)).

## 2. The run's native records

`native_document::extract_graphs` decodes a run's record by its file's own schema. A `StairsRun` record holds, by member name:

- on the run: `m_actualRunWidth`, `m_bottomElevation`, `m_beginWithRiser` and `m_endWithRiser`, and `m_stairsId`, its stairs;
- one `StairsTreadInfo` per tread, in order of `m_startRiser.m_specifiedIdx`: `m_origin`, where the tread's own riser meets the run's centre line, at the riser's top; `m_actDepth`; `m_runDirProj`, the plan direction the run climbs; `m_riserDir`, along the riser; and `m_runType`, 0 for a straight run and 2 on the spiral.

On model 13 the run (12642) has 8 treads, each 0.91864 ft deep, climbing along +X, with origins 0.57058 ft apart in height. The run's sketch follows: its origin is the first tread's origin less half the width across the run, at the bottom elevation; its risers are at each tread's distance along the run and one tread depth past the last; its riser height is the first origin's height over the bottom elevation. A run is read only where every tread agrees with that to 1e-6 ft (one depth, one direction, on the centre line, one riser height per step) and the run begins with a riser.

The run type's construction is named on its `StairsRunType` record: `m_treadThickness`, `m_riserThickness`, `m_nosingLength`, `m_structuralDepth`, `m_hasMonolithicSupport`, `m_hasTread`, `m_hasRiser`, `m_riserIsSlanted` and `m_riserTreadConnection`, with its name on its `SymbolInfo`.

## 3. Which run type

A run's native record does not name its run type. Its root names its stairs (`m_stairsId`) and its tread-riser symbol (`m_triserSymId`), and no object anywhere in its graph holds the id of a `StairsRunType` (the probe's `typeref` search, Measure run [40145190288](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40145190288)). Two places do:

- the run's element record. Its reference list names exactly one `OST_StairsRuns` type definition (RE-28's `unique_type_reference`), which is how RE-52 finds a 2024 run's type;
- the stairs' type. The run's `StairsElement` names its `StairsType` in `m_typeId`, and that names its default run type in `m_runTypeId`.

Revit lets one run take a type other than its stairs' default, so the two can differ; only the first is stored per run. `tools/re/run_types_vs_ifc.py` compares both with the Tag of the `IfcStairFlightType` Revit relates each flight to (Measure run [40147064143](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40147064143)):

| model | runs | own record type = Revit's | stairs' default = Revit's | the two differ |
|---|---:|---:|---:|---:|
| R2025 models 13 to 17 (Revit 2025) | 5 | 5 | 5 | 0 |
| BIM4LCA office as built (Revit 2024) | 35 | 35 | 35 | 0 |
| BIM4LCA office at building permit (Revit 2024) | 35 | 35 | 35 | 0 |

No measured run has a type of its own, so these files do not tell the two apart. rvt-rs takes the record's, the one stored per run, and a run whose record names no one run type keeps its box. Models 13 to 15 take type 2333, "Non-Monolithic Run:50 mm Tread 10 mm Riser" in Revit's export, and 16 and 17 take 2338, "Monolithic Run".

## 4. What rvt-rs does

`attach_native_stair_run_bodies` reads each run's own run type from its element record (§3). `native_stairs::native_runs` then reads each run's sketch, riser height and ends, and its type's construction and name, in one `extract_graphs` pass, and each run is drawn as RE-92's `run_side_profile` draws a 2024 run, with the type's id and name. A run whose records do not give a sketch, or whose type the profile does not draw, keeps its box. It runs on Revit 2025 (`NATIVE_RUN_REVIT_VERSIONS`), the release measured.

## 5. Measured

`stair_bodies_vs_ifc`, Measure run [40148021632](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40148021632), this change against `main`:

| model | run | before | after | Revit |
|---|---|---|---|---|
| 13 | 12642 | box, 317.832 ft³ | 4.490337 ft³, every Revit vertex within 1.7e-6 ft of rvt-rs's surface | 4.490337 ft³, 104 vertices |
| 14 (end with riser off) | 12642 | box, 316.419 ft³ | 4.428920 ft³, every Revit vertex within 1.4e-6 ft of rvt-rs's surface | 4.428920 ft³, 98 vertices |
| 15 (no risers) | 12765 | box | box | |
| 16 (monolithic) | 13476 | box | box | |
| 17 (spiral) | 13665 | box | box | |

On 13 and 14 the solids are Revit's: the volumes agree to the sixth decimal and every vertex of Revit's body lies on rvt-rs's surface. Revit's mesh splits each riser face at the riser's thickness (0.0328 ft, 10 mm), which adds a vertex pair per riser (104 against rvt-rs's 88 on model 13, 98 against 84 on 14), so the scorer's exact-vertex test still counts them as different. Each flight also carries its type's name: `type_objects_vs_ifc` TypeName equal to Revit's goes from 21 to 22 of 25 on each model.

The other three keep their box, each for its own reason:

- 15: its type has no risers (`m_hasRiser` false). RE-52's profile draws treads and risers, and a riserless run is a set of separate treads (#357), which it does not draw.
- 16: a monolithic run with upright risers. RE-92 draws a monolithic run only with slanted risers.
- 17: a spiral run (`m_runType` 2), whose sketch the straight-run rule rejects.

Core Interior, Einhoven, the four RE1 models, the other 21 R2025 models and both BIM4LCA office files are byte-identical, as are their diagnostics (Measure run [40149007983](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40149007983), this change rebased onto `main` at cc7051b3; models 13 and 14 give the same export as in run 40148021632). The office's runs are Revit 2024 and are drawn by RE-52 or kept as boxes, as before.

## 6. What it does not cover

- Riserless and spiral runs (§5). Monolithic runs with upright risers are drawn since §7.
- A run whose own type differs from its stairs' default: none was measured (§3). rvt-rs takes the run's own, which is the one Revit stores per run.
- Releases other than 2025. The members are the same on 2024: on the BIM4LCA office, `native_runs` reads the sketch of 34 of its 35 runs (the probe's `native` lines). All 34 are monolithic with upright risers, as model 16 is, so the profile draws none of them, and 2024 runs are still drawn by RE-52 from its byte anchors, measured on Snowdon Towers. Those 35 office flights, with model 16, are the oracle for drawing a monolithic run with upright risers.

## 7. Addendum (2026-10-24): monolithic runs with upright risers

RE-92's monolithic profile draws each riser from the step's inner corner, the nosing length behind the riser line, up to its nosing on the line. With no nosing the riser runs straight up the riser line whether the type calls it slanted or upright, so the profile is the same. `monolithic_side_profile` now also takes a monolithic type with upright risers where its nosing is 0. Every monolithic run measured has no nosing: model 16's "Monolithic Run" (type 2338) and the BIM4LCA office's 35 "Monolithic Run" flights, all with a structural depth of 0.4921 ft.

`stair_bodies_vs_ifc`, Measure run [40150690633](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40150690633), against `main` (beb01811):

| model | flights | before | now |
|---|---:|---|---|
| R2025 model 16 (Revit 2025, native path) | 1 | box, 237.31 ft³ | 14.748815 ft³ against Revit's 14.748816, every Revit vertex within 2.2e-6 ft of rvt-rs's surface |
| BIM4LCA office as built (Revit 2024, RE-52 path) | 35 | every flight its box | 5 with Revit's volume and every Revit vertex on rvt-rs's surface; 29 at 94.1% to 96.4% of Revit's volume; the spiral run 3057703 its box |
| BIM4LCA office at building permit | 35 | every flight its box | the same |

The five that match are 1440878, 1503045, 2598348, 2861134 and 2867922. On each of the other 29 a vertex of Revit's body lies 0.544 to 0.612 ft from rvt-rs's surface, which is the run's structural depth over the cosine of its pitch on each storey (0.5436, 0.5475, 0.5543, 0.5577 and 0.6124 ft): the vertical depth of its soffit below the inner corners. Which part of those 29 bodies differs is not measured yet. It is the same depth by which Revit's landing underside follows a flight's soffit below the landing's slab (RE-214 §3), so the likely remainder is the soffit where a flight meets a floor or a landing, which RE-92's profile cuts at the run's base and its last riser line.

Model 16's `IfcStair` is 14.75 ft³ against Revit's 15.29: Revit's stair holds 0.54 ft³ more than its flight, in a part rvt-rs does not draw. Every other model's IFC is byte-identical: Core Interior, Einhoven, the four RE1 models and the other 22 R2025 models.

## 8. Addendum (2026-10-24): where a monolithic run's underside ends

§7 left 29 office flights at 94.1% to 96.4% of Revit's volume. `tools/re/flight_junctions_vs_ifc.py` (research branch `re/357-flight-junctions`, Measure run [40153963831](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40153963831)) lists every vertex of Revit's body off rvt-rs's surface. On each short flight they are at its start. Revit's start face reaches below the run's base: 2859204's to 181.5323 ft, its base 182.0866 less 0.5543 ft, which is the run's structural depth over the cosine of its pitch, where its underside meets its first riser line. RE-92's profile cuts the underside at the base instead. The flights that matched are cut at their base, as RE-92 draws them.

Two members of each run's `StairsRun` record, read by name (`examples/probe_357_native_runs.rs`, Measure run [40154688351](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40154688351)), tell them apart:

| office flights | `m_baseRiserIndex` | `m_extendBelowBase` | Revit's underside |
|---:|---|---|---|
| 5 | 0 (start at the stair's base) | 0 | cut at the base |
| 8 | 0 | −0.98425 ft (300 mm) | runs on to the first riser line, 0.548 to 0.558 ft below the base, above the cut |
| 21 | 8, 10 or 19 (start on a landing) | 0 | runs on to the first riser line |

The 35th, the spiral run 3057703, starts at its base with no extension and is not drawn (§5).

So the floor a run starts on cuts its underside at `m_extendBelowBase` below its base, and a run that starts on a landing is not cut. `RunSketch::base_cut_feet` carries that cut. `native_stairs` reads it for a 2025 run from its record; for a 2024 run drawn from RE-52's byte anchors, `native_run_base_cuts` reads the same two members, and a run that does not give them is cut at its base, as before.

Measure run [40155621830](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40155621830), this change against `main` (25de971b), `stair_bodies_vs_ifc`:

| model | before | after |
|---|---|---|
| BIM4LCA office as built | 5 of 35 flights with Revit's volume | 34 of 35, every vertex of Revit's body on rvt-rs's surface; the spiral run 3057703 its box |
| BIM4LCA office at building permit | 5 of 35 | 34 of 35 |

Every other model's IFC is byte-identical: Core Interior, Einhoven, the four RE1 models and the 23 R2025 models (model 16's run starts at its base with no extension, so it is cut at its base as before).

**Not re-measured: Snowdon Towers** (local only, never measured on CI). RE-92's 4 monolithic Snowdon flights that equal Revit's were drawn with the underside cut at the base. By this rule a flight drawn exactly that way starts at its stair's base with no extension, so it is expected to stay the same, and a Snowdon flight that starts on a landing or extends below its base changes. That is an inference from the office, not a measurement of Snowdon.
