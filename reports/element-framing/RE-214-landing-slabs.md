# RE-214: A monolithic stair landing's slab, from its own lines, its top and its type's thickness

**Date:** 2026-10-24
**Issues:** #824 (the BIM4LCA office's slab tops; its 21 landings were drawn as their record box). Follows RE-25 and RE-50 (plan profiles from sketch lines and their exact ends), RE-28 (an element's type from its reference list), RE-178 (a body between its own bottom and top) and RE-217 (stair runs read by member name).
**Artefacts:**

| file | sha256 | licence |
|---|---|---|
| `ARK_NordicLCA_Office_Concrete_As-Built.rvt` (Revit 2024) | `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906` | CC BY-SA 4.0, VTT (`research/bim4lca-measure.tsv`) |
| `ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc` (Revit's IFC4 export) | `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit.rvt` (Revit 2024) | `1c12c2058128315edbd65a8bf97f60da971aee8625f21233b8477d9c6165c1d1` | CC BY-SA 4.0 |
| `ARK_NordicLCA_Office_Concrete_BuildingPermit_Revit.ifc` (Revit's IFC4 export) | `bc3bd3366a3c413ce74840300a590f59102b740118ef37898e906253b6b7c08d` | CC BY-SA 4.0 |

Measure fetches all of them and stores none.
**Probe:** `examples/probe_re214_landing_outlines.rs` (first measured as `probe_re210_landing_outlines` on the research branch `re/824-landings`). **Scorers:** `tools/re/landing_outlines_vs_ifc.py`, `tools/re/plan_profiles_vs_ifc.py`, `tools/re/stair_bodies_vs_ifc.py`.
**Runs:**
- [40141204360](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40141204360): each landing's lines as a ring against Revit's outline;
- [40144894546](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40144894546): Revit's body heights against the lines' elevation;
- [40146084523](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40146084523): the landings' stored solids against Revit's bodies (branch `re/357-stairs-native`);
- [40146872950](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40146872950): each landing's type through its stairs, and its members;
- [40147777110](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40147777110): four of Revit's landing bodies, vertex by vertex;
- [40149131581](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40149131581): each landing's own record type;
- [40150910097](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40150910097): each gate `attach_landing_slabs` reads, landing by landing;
- [40156315852](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40156315852): this change against `main` (25de971b).

**Status:** positive on the top, the outline and the main underside of the office's 21 monolithic landings; negative on the underside where a flight meets a landing, which is not drawn.

## 1. What was missing

The office has 22 `StairsLanding` elements. Each was its record box, which spans its stair's whole storey (12.47 to 14.17 ft tall), where Revit's export draws a slab about 1 ft thick with a notch where its flights meet it. `plan_profiles_vs_ifc` read 20 of them as slightly larger than Revit's top (18.96 against 18.90 ft², 19.11 against 19.04) and 3058084 as 0.55% off (#824).

## 2. The outline

A landing's own element data holds bounded lines (RE-50's `04 00 08 01` records), in two groups: one at the landing's top, and on 11 landings a second, closed, at elevation 0. The group at the top has gaps where the landing's flights meet it. Joined end to end, with each gap bridged straight to the nearest end of an unused line, it is Revit's outline:

| landings | lines at the top | gaps bridged | outline against Revit's |
|---:|---:|---:|---|
| 11 | 6 | 0 | within 0.001 ft |
| 10 | 4 | 2 | within 0.001 ft |
| 1 (3058084) | 4 | 1 | 0.117 ft off; Revit's has an arc there |

## 3. The top and the thickness

- **Top:** the landing's `StairsLanding.m_baseElevation`, read by member name, is the elevation of its lines and the top of Revit's body on all 22.
- **Type:** the one `OST_StairsLandings` type definition the landing's element record names (RE-28), the same as its stairs' landing type (`StairsLanding.m_stairsId` → `StairsElement.m_typeId` → `StairsType.m_landingTypeId`) on all 22. 21 landings take type 461553, a monolithic landing (`m_hasMonolithicSupport` true) with `m_thickness` 0.98425 ft (300 mm); 3058084 takes 141731, which is not monolithic.
- **Underside:** on the three monolithic landings printed in full (2859208, 2861138, 3202702), Revit's main underside is the top less `m_thickness` (185.1852, 172.3292 and 175.1969 ft). Where a flight meets the landing, Revit's underside follows the flight's soffit instead: along the edge where a flight arrives it dips below the slab, and where one leaves it rises into it, each at the flight's pitch (riser over tread, 0.5185 on 2859208). The landing's lowest point is the top less the riser height plus the run's structural depth over the cosine of the pitch, on 21 of 22 to 0.0001 ft. 3058084's body is 0.0984 ft thick where its type's `m_thickness` is 0.995 ft.
- The landings' stored solids (two `Geometry` objects each) are not Revit's body: one holds six of Revit's top vertices only, the other lies up to 4 ft from it.

## 4. What rvt-rs does

On Revit 2024 (`NATIVE_LANDING_REVIT_VERSIONS`), `attach_landing_slabs` reads each landing's own type (§3), and `native_stairs::native_landings` its top and, where the type is monolithic, the type's thickness. The landing's lines at its top, in the first 16 KiB of its data (`partition_stairs::scan_landing_lines`; the whole 64 KiB window RE-52 reads a run in holds more lines further on, 18 per landing against the 4 at the top), make its outline (`landing_outline`, §2): the lines that meet end to end are joined into chains first, then each chain to the next across its gap, so two gaps of one length cannot close the outline early. It must have an area, not cross itself, and lie in its record box to `SKETCH_BOX_TOLERANCE_FEET`. The landing is then drawn as that outline from its top down its type's thickness (`LANDING_BODY_BOTTOM_FIELD`, `LANDING_BODY_TOP_FIELD`, placed by RE-178's body z range), with `BodySource` `partition_landing_slab` and `ProfileSource` `partition_landing_lines`. Its own placement and `BoundingBoxHeight` stay its record box's. A landing whose type is not monolithic, or whose outline does not hold, keeps its box.

## 5. Measured

Measure run [40156315852](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40156315852), this change against `main`, on both BIM4LCA office files:

| | before | after |
|---|---|---|
| landings drawn as their slab (`BodySource` `partition_landing_slab`) | 0 | 21 of 21 monolithic |
| slabs with Revit's outline within 0.01 ft (`plan_profiles_vs_ifc`), as built | 40 of 63 | 60 of 63 |
| the same, at building permit | 38 of 63 | 58 of 63 |
| a landing's body height (`stair_bodies_vs_ifc`) | its record box, 12.47 to 14.17 ft | 0.98425 ft, top and main underside Revit's |
| a landing's volume against Revit's | 12.7 to 15.0 times | 100.4% to 104.7% |

The three slabs whose outline still differs on the as-built file are 3058084 (the landing that is not monolithic, which keeps its box), and the roofs 1461758 and 2991742 (#824). The volumes are over Revit's because Revit's underside rises into the slab where a flight leaves the landing and dips below it where one arrives, which is drawn flat; that is the measured approximation, labelled by its `BodySource`. Every other model's IFC is byte-identical: Core Interior, Einhoven, the four RE1 models and the 23 R2025 models.

A landing that was its box took the storey its box's top met, the storey above. A slab binds by its top face, and a landing's top is not a storey elevation, so the 21 now have no storey of their own, as the office's 35 flights already have none: the diagnostics' `unsupported_geometry_missing_level` count goes from 39 to 60. Each landing stays a part of its `IfcStair` through `IfcRelAggregates`, and `storeys_vs_ifc` is unchanged.

## 6. What it does not cover

- The underside where a flight meets a landing (§3). It follows the flight's soffit, which needs the flight drawn first: the office's 35 flights are monolithic with upright risers, still boxes (#357).
- 3058084, the one landing that is not monolithic, whose outline has an arc: drawn since, as one tread (§7).
- Releases other than 2024: no other model with landings has a Revit export to measure against.

## 7. Addendum (2026-10-25): a landing that is one tread (#824)

The office's landing 3058084 sits beside the spiral run 3057703 (RE-217 §11), and Revit's export draws it 0.0984 ft thick with an arc in its outline. Its native records give both (`examples/probe_824_landing_loops.rs`, Measure runs [40179086934](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40179086934) and [40179920379](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40179920379)):

- **Outline.** Its `StairsLanding` record holds `CurveLoop`s at its top (`m_baseElevation` 249.0814 ft): four `GLine`s (`m_origin`, `m_dirVec`, `m_endParams`) and a `GArc` (`m_center`, `m_xVec`, `m_yVec`, `m_radius`, `m_endParams`) of radius 0.259402 ft about (305.5719, 59.6613), the spiral run's inner circle. Each curve starts where the one before ends. That is Revit's outline, the arc where the landing meets the spiral.
- **Thickness.** Its type, 141731 ("Non-Monolithic Landing"), is not monolithic (`m_hasMonolithicSupport` false), has a tread (`m_hasTread`) and takes it from its runs (`m_useSameTriserAsRun`), so its own `m_treadThickness` (0.164 ft) does not apply. Its stair's one run type, 3051881, has `m_treadThickness` 0.0984 ft, Revit's thickness.

`native_stairs::native_tread_landings` reads that for a landing whose type is not monolithic, has a tread and takes it from its runs, when its stair's runs (`StairsRun.m_stairsId`) all name one tread thickness; `partition_schema_mvp::attach_landing_slabs` records the loop (`LANDING_TREAD_OUTLINE_FIELD`), and the IFC writer draws it as one `IfcExtrudedAreaSolid` over an `IfcIndexedPolyCurve` of its lines and three-point arc, from its top down that thickness, `BodySource` `native_landing_tread`, as Revit writes a tread. A landing whose stair's runs name more than one tread thickness, or whose type takes its own tread, keeps its box: neither is measured.

Measure run [40181002275](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40181002275), this change against `main` (ba38971f), `-f nordic=true -f r2025=true`:

| | before | after |
|---|---|---|
| 3058084's body (`stair_bodies_vs_ifc`), both files | its record box, 196.06 ft³ | 1.462462 ft³, Revit's 1.462462; every vertex of Revit's within 0.0000022 ft of rvt-rs's surface |
| slabs with Revit's outline within 0.01 ft (`plan_profiles_vs_ifc`), as built | 61 of 63 | 62 of 63 |
| the same, at building permit | 59 of 63 | 60 of 63 |

The as-built file's last slab outline off is roof 2991742, whose four hatch openings are not cut (#824). Only the office's IFC changes for this; that run's R2025 differences are #974, which the branch predated.

