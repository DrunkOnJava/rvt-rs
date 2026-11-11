# RE-423: a beam's six base quantities are the sums of the prism its saved mesh is

**Date:** 2026-11-11
**Issues:** #1693 (beams' base quantities), #1636 (second-project measurement); follows RE-353 (the beam and column quantities revit-ifc writes), RE-387 (the mesh's volume), RE-404 (a mullion's surfaces from the mesh) and RE-388 (a wall's quantities from its mesh)
**Artefacts:** the BIM4LCA housing's four large files and its two terrain files, and the office's four large files (Revit 2024, Revit's IFC4 exports), the held-out pairs of Revit 2022 to 2027 (counts only), and Autodesk's samples, as Measure fetches them. Never stored.
**Runs:** Measure [42964333038](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42964333038) (unfiltered, head `781ed0cd1a4508d32b6260508d094b5190c36e43`, kept under `refs/archive/2026-11-11/cited/781ed0cd1a45`, against `5d291201904b9e61d8ce62c3201308a152d4c65a`, RE-404's head) for section 3; [42964336667](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42964336667) (both sides' exports of the 58 Autodesk samples kept) and [42967574116](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42967574116) (the same for the first version, head `4a1b78d692d447778fb403b3f0447679a971af69`, kept under `refs/archive/2026-11-11/cited/4a1b78d692d4`, whose beam bodies are the saved meshes) for section 4; [42967412141](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42967412141) (`tools/re/beam_quantities_vs_ifc.py` on the new head) for sections 1 and 3; [42960616099](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42960616099) (the first version, unfiltered) for section 2
**Status:** Positive on the BIM4LCA housing and the timber office, with the concrete office's hollow tubes and one concrete housing beam left without. 266 of the 284 beams of the eight large files read all six quantities equal to Revit's, none different, from none; 19 of 126 exports move, the other 107 are byte-identical, and the 11 Autodesk samples that move gain only the six values on 938 beams, which no Revit export scores and which the first version's exports show to be the rule's own sums of each beam's mesh.

## 1. What Revit writes

`tools/re/beam_quantities_vs_ifc.py` pairs every `IfcBeam` of rvt-rs's export with Revit's by `Tag`, reads each side's `Qto_BeamBaseQuantities` in their own units (Revit's export declares millimetres, square metres and cubic metres), the kind of body Revit writes, and the body's mesh along the axes it is a prism along (every plane of its triangles but at most two, its ends, parallel to the axis). On the four files the scorer read, the concrete housing as built, the timber housing at permit and both offices at permit, 142 beams:

| | beams | body Revit writes | extrusion runs |
|---|---:|---|---|
| concrete housing | 22 | `IfcExtrudedAreaSolid(ArbitraryClosedProfile)` | across the `Length` on 21, along it on 1 |
| timber housing | 41 | `IfcExtrudedAreaSolid(ArbitraryClosedProfile)` | across on 38, along on 3 |
| timber office | 71 | `IfcExtrudedAreaSolid(ArbitraryClosedProfile)` | along on all 71 |
| concrete office | 8 | `IfcExtrudedAreaSolid(ArbitraryProfileDefWithVoids)`, hollow rectangular tubes | along on all 8 |

Revit writes six quantities on every one of them in `Qto_BeamBaseQuantities` besides `Length` (`CrossSectionArea`, `OuterSurfaceArea`, `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume`, `NetVolume`), and rvt-rs wrote none of the six on any.

- **Each of the six is a sum of Revit's own body's faces** on the 134 beams that are not tubes. `CrossSectionArea` is the area of one end face (the profile's area), `OuterSurfaceArea` the lateral area (the outer perimeter times the depth), `GrossSurfaceArea` and `NetSurfaceArea` the lateral area plus both flat ends (the perimeters times the depth plus two areas), `GrossVolume` and `NetVolume` the volume (the profile's area times the depth). This is what revit-ifc's `CreateBeamColumnBaseQuantities` computes from the extrusion it writes (RE-353, CLEANROOM.md source 5).
- **Revit's extrusion axis is the longest axis the body is a prism along with two flat ends,** on all 142 beams (rank 1 of the axes the scorer finds: one such axis on 57 of the 142, and three on the other 85, a box being a prism along each of its three axes). On the timber office's 71, the tubes' 8 and 4 of the housing's 63 it is the beam's line; on the other 59 of the housing's it is across it, the beam modelled as its side profile extruded by its width. A rule that took the extrusion along the line would be wrong on those.
- **The 8 tubes are the exception.** Their quantities are no sum of their body's faces to the scorer's tolerance: `CrossSectionArea` is 0.00389474 m² where half the area of the flat faces is 0.00389354, `NetSurfaceArea` 11.0787 m² where all the faces sum to 11.0778, `GrossVolume` 0.0221416 m³ where the body's volume is 0.0221347 (0.01% to 0.03% apart). `GrossSurfaceArea` is `OuterSurfaceArea` plus the two ends' net area and `NetSurfaceArea` is every face, inner ones included, so a hollow section needs the outer perimeter told from the inner, which the mesh does not give. Why the values are 0.01% to 0.03% off the faces is not established.

## 2. The rule

`saved_meshes::beam_mesh_quantities` reads the saved mesh of a beam drawn along its location line (`BodySource` `partition_beam_axis`, RE-328), and the pass reads that of a beam it replaces from its record box (#1191) the same way, with `mesh_volume::beam_quantities`, as the prism Revit's export extrudes the beam's profile into. All of these hold, or the beam gets none of the six:

1. **A whole, closed, single solid with no hole through it.** No part of its saved graphics failed to decode, all its faces are planes (`all_faces_flat`), the triangles with their vertices welded form one closed surface wound one way (`closed_mesh`: every directed edge has its opposite once), they are one piece (a union-find over the welded vertices), and the Euler characteristic is a sphere's (vertices less edges plus faces is 2, which for a closed triangle mesh is `2V - F = 4`). A hollow section has inner and outer faces and the mesh does not tell the outer perimeter from the inner; a solid with a hole beside another solid has a single solid's characteristic, which the piece count refuses (a review by a DeepSeek worker, Tendrils, found it; the test is a box beside a square ring, which `beam_quantities` accepts without the count and refuses with it).
2. **A prism along an axis with two flat ends of one area.** The candidate axes are the distinct directions of the mesh's edges (at most 48, so a curved or free-form solid is not read). For each, `prism_along` (RE-418, RE-404) says whether every plane but two is parallel to it, those two being its ends (at most 32 planes); both ends must be perpendicular to the axis and of one area, to 1e-4.
3. **The longest such axis.** Revit's extrusion axis is the longest (section 1). Where two axes that are not parallel tie in extent (a cube), none is taken.
4. **The volume is the end area times the extent**, to 1e-4.

Then `CrossSectionArea` is the end area, `OuterSurfaceArea` the lateral area, `GrossSurfaceArea` and `NetSurfaceArea` the lateral area plus both ends, `GrossVolume` and `NetVolume` the volume. They are written to `Qto_BeamBaseQuantities` where the beam holds none of them, creating the set where it has none, in the order of the entities.

**The beam's body is not touched.** A beam drawn along its line stays the extrusion of its resolved section, which is Revit's own form, as a wall stays what it is while its mesh gives its quantities (RE-388). The first version of this change offered these beams to the saved-mesh pass as stand-ins and replaced their bodies with the mesh (`BodySource` `saved_graphics_mesh`); its quantities were right (section 3), but the `same-project gates` check failed on six Autodesk saves, which hold `rac_advanced` to 29 and `rst_basic` to 345 beams drawn along their axes in every save (`tools/re/bodies_across_saves.py`), and the pass had taken 6 of the 29 and 331 of the 345. The replacement was the question the first version left open for the beams whose body was already Revit's (64 of the timber office's 71 at permit); it is answered by keeping them.

## 3. Measured: every model

Unfiltered Measure [42964333038](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42964333038), every flag, 126 models on each side, head `781ed0cd` against `5d291201` (RE-404's head, which this change stacks on): 107 exports are byte-identical and 19 move, the housing's six files (the four large and the two terrain files), the timber office's two, and 11 Autodesk samples (`rac_advanced` and `rst_basic` of 2023 to 2027 and Snowdon Towers Structural of 2024; section 4). The concrete office's four files are byte-identical: its 8 beams are the hollow tubes, which get none. The 18 held-out models' counts are identical to the parent's. `same-project gates` passed on the head.

| `Qto_BeamBaseQuantities`, each of the six quantities | beams | equal, before | equal, after | `different` |
|---|---:|---:|---:|---:|
| concrete housing, as built and at permit | 22 | 0 | 21 | 0 |
| timber housing, as built and at permit | 41 | 0 | 41 | 0 |
| timber office, as built and at permit | 71 | 0 | 71 | 0 |
| concrete office, as built and at permit | 8 | 0 | 0 | 0 |
| each of the housing's two terrain files | 2 | 0 | 2 | 0 |

On the eight large files that is 266 of the 284 beams on each of the six quantities, from none: +126 values on each concrete housing file (21 beams times six), +246 on each timber housing file, +426 on each timber office file, and +12 on each terrain file. No line of `qtos_vs_ifc` or `psets_vs_ifc` of any model gains a `different` or a value Revit does not write, and the only top lines that change are those eight files' `qtos_vs_ifc`, `equal` rising and `missing` falling by the same count. The 18 beams still without the six are the concrete office's 16 tubes (8 in each of its two states) and the concrete housing's one beam in each state, which no prism axis with two flat equal ends reads (section 4).

**No body changes.** The `body_sources` of the 101 models that report them are identical on both sides, and the two samples with minimum counts in `tools/re/bodies_across_saves.py` still draw 29 (`rac_advanced`) and 345 (`rst_basic`) beams as `partition_beam_axis` in every save. The first version of this change (head `4a1b78d6`, Measure [42960616099](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42960616099)) wrote the same values on the same beams and failed that check, as section 2 says.

## 4. The Autodesk samples and the one beam

**The samples.** Run 42964336667 kept both sides' exports of the 58 Autodesk samples: 47 are byte-identical and 11 move, `rac_advanced` and `rst_basic` of every release from 2023 to 2027 and Snowdon Towers Structural of 2024. No Revit export of them exists, so each is read. `tools/re/ifc_element_diff.py` pairs every element by GlobalId and finds that no element is added or removed and that only `IfcBeam` changes, on each of them only by `Qto_BeamBaseQuantities` gaining the six quantities and nothing removed or changed: 12 beams in each `rac_advanced` save, 162 in `rst_basic` of 2023 and 171 in each of 2024 to 2027, 32 in Snowdon Towers Structural, 938 in all.

**Why the second version's own exports cannot show the arithmetic.** The beam's body is kept, and on many of these beams it is not the mesh the six came from: `rst_basic`'s `Timber_50 x200` beam 1175807 has a 50 mm by 200 mm section and a volume of 0.01367 m³, an extent of 1.367 m, while its kept extrusion is the record box's 1.5 m; Snowdon's `CB24x24` beam 606873 reads a section of 0.216774 m² where its extrusion's is 0.3716. `tools/re/beam_added_quantities.py` run on those exports reads the six values of 8 of `rac_advanced`'s 12 beams as the sums of the kept extrusion's faces and those of 4 not; on `rst_basic` only `CrossSectionArea` matches the extrusion's (98 of 162 beams in 2023, 106 of 171 in each later save) and the other five match on none; on Snowdon's none do. That is the extrusion being something other than the mesh, not the rule being wrong, as the next paragraph shows.

**The arithmetic, on the first version's exports.** The first version replaced the beam's body with its mesh, so its exports show the mesh itself. Run 42967574116 kept them, and `tools/re/beam_added_quantities.py` over the same 11 samples reads every one of the six values on every one of the 938 beams as the sum the rule gives of that beam's own faces, the end's area, the lateral area, the lateral area and both flat ends, or the volume, and none anything else. The second version writes the same values on the same beams: its counts per sample are the first version's, so the single-piece test refused none of them.

**The beam still without.** The concrete housing's 22 beams in each state read 21 equal; the beam without the six has a body Revit draws as an extrusion along its `Length` (the other 21 are extruded across it) and an `IfcBeam` whose mesh `beam_quantities` does not read as a prism with two flat, square ends of one area along a longest axis. Why is not established, and it gets none.

**What is left.** The 8 tubes' six quantities need the profile with its voids and rounded corners (Revit's are 0.01% to 0.03% off the faces of its tessellated body), which the mesh does not give. Beams drawn from a saved mesh carry no `Length`, `Roll`, `Slope` or `Span` (28 of the timber housing's 41, 7 of the timber office's 71): #1697. A beam's body that is not Revit's solid (the tubes, 20 of the concrete housing's 22 beams, 10 of the timber housing's 41) is untouched by this change: #1698.

## 5. What it costs

The quantities come from the beams' saved graphics, which the saved-mesh pass reads, and that costs time and memory on some files. Measure run [42972934548](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42972934548) (`timing`: base `47aa70be` and head `e10641ce` built once and timed in turn on one runner per file, three rounds, 68 files; peak memory is GNU time's `%M`) reads 11 files above the noise and 57 within it:

| file | time, base to head | peak memory, base to head |
|---|---|---|
| `rst_basic` 2023 | 1,131 to 1,454 ms (+29%) | 147 to 314 MiB (2.1 times) |
| `rst_basic` 2024 | 1,258 to 1,528 ms (+21%) | 143 to 326 MiB (2.3 times) |
| `rst_basic` 2025 | 1,197 to 1,480 ms (+24%) | 143 to 326 MiB (2.3 times) |
| `rst_basic` 2026 | 1,186 to 1,480 ms (+25%) | 143 to 326 MiB (2.3 times) |
| `rst_basic` 2027 | 1,527 to 1,851 ms (+21%) | 144 to 327 MiB (2.3 times) |
| `rac_advanced` 2023 | 5,410 to 6,269 ms (+16%) | 557 to 1,010 MiB (1.8 times) |
| `rac_advanced` 2024 | 6,217 to 7,216 ms (+16%) | 579 to 1,090 MiB (1.9 times) |
| `rac_advanced` 2025 | 5,873 to 6,743 ms (+15%) | 567 to 1,081 MiB (1.9 times) |
| `rac_advanced` 2026 | 5,744 to 6,621 ms (+15%) | 562 to 1,075 MiB (1.9 times) |
| `rac_advanced` 2027 | 6,808 to 7,729 ms (+14%) | 566 to 1,077 MiB (1.9 times) |
| Snowdon Towers Structural 2024 | 3,767 to 6,383 ms (+69%) | 494 to 1,224 MiB (2.5 times) |

The other 57 read 0.99 to 1.01 in time and 0.96 to 1.00 in peak memory: Core Interior, the concrete office's two states (33.1 s to 33.1 s and 27.8 s to 27.7 s; 1,439 to 1,440 MiB and 1,550 to 1,552 MiB) and its two terrain files, RE1's four models, Einhoven, and 47 Autodesk samples (all of 2015 to 2022, which the pass does not read, and the rest of 2023 to 2027).

**The pass ran in all 11 before this change.** Their `body_sources` hold `saved_graphics_mesh` bodies at the base commit (5,123 to 5,225 in `rac_advanced`, 67 to 91 in `rst_basic`, 83 in Snowdon Towers Structural), so what is added is the reading of the beams' graphics, not the pass beginning. The 2024 saves read 29, 345 and 923 beams drawn along their axes (`partition_beam_axis`), of which 12, 171 and 32 gain quantities. Why that costs what it does is not established: the time added is 0.9 to 1.0 s on `rac_advanced`'s 29 beams, 0.3 s on `rst_basic`'s 345 and 2.6 s on Snowdon's 923, so it does not scale with the beams read, and no profile by stage has been taken. [#1694](https://github.com/DrunkOnJava/rvt-rs/issues/1694) tracks the pass's peak memory; these 11 files are a second set of numbers for it.

**The files it is scored on.** Measure's timing table held the reference models, the office's four files and the samples, so the housing and the timber office were timed apart: run [42976698994](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42976698994), from a throwaway workflow branch whose timing job also takes these files, base `47aa70be` and head `302415ba` (this change, whose later commits are documents), one runner per shard, three rounds. The four office files, timed again as controls, read 1.01, 0.99, 1.00 and 1.00.

| file | time, base to head | peak memory, base to head |
|---|---|---|
| Housing Concrete, as built | 23,814 to 23,876 ms (1.00) | 1,170 to 1,155 MiB (0.99) |
| Housing Concrete, permit | 22,799 to 22,648 ms (0.99) | 1,071 to 1,103 MiB (1.03) |
| Housing Timber, as built | 23,664 to 23,494 ms (0.99) | 1,320 to 1,335 MiB (1.01) |
| Housing Timber, permit | 23,506 to 23,064 ms (0.98) | 1,255 to 1,261 MiB (1.00) |
| Office Timber, as built | 35,002 to 35,228 ms (1.01) | 1,618 to 1,622 MiB (1.00) |
| Office Timber, permit | 45,242 to 44,995 ms (0.99) | 2,005 to 2,056 MiB (1.03) |

So the read of their 41 to 71 beams a file costs no measurable time (0.98 to 1.01) and 0 to 3% of peak memory. Not measured: a structural model larger than Snowdon Towers Structural.

**Opting out.** `--no-saved-meshes` runs no pass, so it leaves out the six quantities and this cost with the rest of the pass.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=781ed0cd1a4508d32b6260508d094b5190c36e43 -f base=5d291201904b9e61d8ce62c3201308a152d4c65a \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
gh workflow run measure.yml --ref main -f ref=<head> -f base=<base> -f samples=true -f keep_ifc=true -f scorers=none
```

The scorers: `python3 tools/re/beam_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>` (Measure 42967412141: `-f nordic=true -f scorers=beam_quantities_vs_ifc -f "models=nordic-housing-concrete-as-built ..."`), and `python3 tools/re/beam_added_quantities.py <before.ifc> <after.ifc>` for the exports a run keeps.

The timing of section 5: `gh workflow run measure.yml --ref main -f ref=e10641ce4d037de4900605beb3375dbfe5482670 -f base=47aa70bedff92f93ebb118e0773134be2a5e9c9a -f nordic=true -f samples=true -f timing=true` (the head is kept under `refs/archive/2026-11-11/cited/e10641ce4d03`); the housing's and timber office's rows are run 42976698994, dispatched the same way (`timing`, base `47aa70be`) from a throwaway workflow branch that adds their files to the timing job, which the timing job of `main` takes once its change lands.

## 7. Sources

RE-353 (revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities`, CLEANROOM.md source 5), RE-387, RE-388, RE-404, RE-418. No Autodesk source was read.
