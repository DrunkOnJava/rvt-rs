# RE-404: a curtain mullion drawn from its saved mesh gets the `NetSurfaceArea` of its lateral faces and of the end faces its solid keeps

**Date:** 2026-11-11
**Issues:** #1046 (curtain mullion quantities), #1636 (second-project measurement); follows RE-268 (which ends Revit's solid keeps), RE-403 (the box is not the solid), RE-387 (the mesh's volume) and RE-418 (the mesh's length)
**Artefacts:** the BIM4LCA office at building permit, in concrete and in timber, and the housing's four large files (Revit 2024, Revit's IFC4 exports), as Measure fetches them. Never stored.
**Runs:** Measure [42946015504](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42946015504) (the scorer `tools/re/mullion_profiles_vs_ifc.py` at `b98bc2e2575d6de675e34a25e9b6f50de3518fff`, on `main` at `0649e224b96285c2bced628c5379bdd4be924727`) for section 1; [42956686568](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42956686568) (unfiltered, head `75fd899c627895a1bd772f791303a8f0baf395f3`, kept under `refs/archive/2026-11-11/cited/75fd899c6278`, against RE-418's `2136399d614606bd760d11ea34322b00cc740045`, kept under `refs/archive/2026-11-11/cited/2136399d6146`) for section 3; [42953777974](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42953777974) (both sides' exports of the Autodesk samples kept) for section 4; [42947332778](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42947332778) (the first version, section 2)
**Status:** Positive. 1,634 to 1,658 more of each office's mullions read `NetSurfaceArea` equal to Revit's and 184 more of the housing's, none different; 21 of the 126 exports move and the other 105 are byte-identical.

## 1. What Revit's `NetSurfaceArea` is, against rvt-rs's body

`tools/re/mullion_profiles_vs_ifc.py` pairs every curtain mullion of rvt-rs's export with Revit's by `Tag`, reads each side's `NetSurfaceArea` in its own unit, measures the body's mesh along its own axis (RE-418) and writes Revit's value as the mesh's lateral area (the faces along the axis) plus a number of its end faces. Where rvt-rs draws the body from the mesh Revit saved, the body is Revit's: the same extent, area and volume.

| model | mullions | `NetSurfaceArea` written and equal | written none, the body Revit's, a whole number of ends | written none, mitred | written none, the body not Revit's | written, different |
|---|---:|---:|---:|---:|---:|---:|
| office at permit | 10,031 | 7,560 | 2,258 | 29 | 166 | 0 |
| timber office at permit | 7,872 | 5,292 | 2,355 | 29 | 173 | 5 |
| housing, concrete, as built | 354 | 306 | 0 | 48 | 0 | 0 |
| housing, concrete, at permit | 354 | 314 | 0 | 40 | 0 | 0 |
| housing, timber, as built and at permit | 444 each | 396 | 0 | 48 | 0 | 0 |

(The offices' remaining 18 mullions have no axis the scorer reads in rvt-rs's body.)

- **Flat ends.** Of the 2,258 and 2,355, Revit's value is the lateral area plus 0, 1 or 2 flat end faces (on the office at permit 1,242, 168 and 848). RE-268 reads that number from the boxes of the mullion's curtain wall, and writes it only for a mullion on a plain host.
- **Mitred ends.** Of the 29 and 29 of the offices and the 184 of the housing's large files (48, 40, 48 and 48), Revit's value is the lateral area plus the mitre face, and plus the flat end where that is kept (housing as built: 36 without it and 12 with it; the office: 29 with it). Never the flat end without the mitre.
- **A body that is not Revit's.** The 166 and 173 have a body that differs from Revit's in extent, area and volume: the 30 listed on the office at permit are drawn from their record's oriented box (`partition_family_instance_oriented_box`), where Revit's solid has mitred ends. They are not drawn from a mesh and are not touched here.

## 2. The rule

A curtain mullion drawn from its saved mesh (a closed mesh of flat faces) that holds no `NetSurfaceArea` gets `mesh_volume::mullion_surface_area` of the prism its mesh is along the axis its record runs it along (RE-418's `prism_along`) and of `kept`, the end faces RE-268 reads for it: the lateral area plus `kept` of its two flat ends of one area each; or, of a flat and a mitred end, the lateral area plus the mitre face and, where `kept` is 2, the flat end, and none where `kept` is 0. `kept` is read for a mullion on any host (it is kept in the file's memo as the partition's mullions are read, `RevitFile::mullion_kept_ends`); until now it was written to a mullion's record only where its host is plain, and the box's own `NetSurfaceArea` only there too.

**The first version was wrong on four mullions of the timber office at permit.** Measure 42947332778 (the first version, `fe2246850fc720052cf5de67cdeaa994dcf9d12e`, kept under `refs/archive/2026-11-11/cited/fe2246850fc7`) read `equal` on 1,622 more mullions on each of the concrete office's two files and on 1,646 and 1,622 more on the timber office's two, and on all 184 mitred mullions of the housing; the concrete office stayed at 0 different. The timber office at permit went from 5 to 9 different: tags 2495554, 3022734, 3481354 and 3481512, vertical 2.1 m mullions, written 0.7704 m2 where Revit's is 0.7632, one end face (0.0072 m2) too many. Each stands under a 0.4 m piece of the same section whose end face is exactly its own (end to end at 65.82 ft, and the other three likewise): a partner, which hides the end in Revit's solid. RE-268 reads the ends from the boxes of one curtain wall (`m_hostId`), and a continuation on another wall is not in its group, so it counted both ends (`kept` 2).

`mesh_volume::partner_ends` finds a partner in the meshes themselves, whatever the wall: another mullion running the same way (cosine over 0.999) with an end within 1 cm of the end and its body beyond it. A mullion gets no `NetSurfaceArea` where it has a partner at an end that `kept` counts, that is where it has more partners than `2 - kept`. The check can only take a value away from what `kept` alone would write.

## 3. Measured: every model

Unfiltered Measure 42956686568, every flag, 126 models on each side, the head against RE-418's: 105 exports are byte-identical and 21 move, the eight BIM4LCA files that hold curtain mullions drawn from their meshes and 13 Autodesk samples (section 4); the held-out files' counts, the four terrain files and every other model do not change. `same-project gates` passed on the head.

| file | `NetSurfaceArea` equal | missing | quantities equal (of those Revit writes) | different |
|---|---:|---:|---:|---:|
| office, concrete, as built | 7,532 to 9,166 of 10,003 | 2,471 to 837 | 34,281 to 35,915 of 39,754 | 1, unchanged |
| office, concrete, at permit | 7,560 to 9,194 of 10,031 | 2,471 to 837 | 34,616 to 36,250 of 40,006 | 1, unchanged |
| office, timber, as built | 5,431 to 7,065 of 7,876 | 2,443 to 809 | 27,253 to 28,887 of 33,800 | 3, unchanged |
| office, timber, at permit | 5,292 to 6,950 of 7,872 | 2,575 to 917 | 26,831 to 28,489 of 33,663 | 22, unchanged |
| housing, concrete, as built | 306 to 354 of 356 | 50 to 2 | 3,636 to 3,684 of 5,358 | 0, unchanged |
| housing, concrete, at permit | 314 to 354 of 356 | 42 to 2 | 3,598 to 3,638 of 5,062 | 4, unchanged |
| housing, timber, as built | 396 to 444 of 452 | 56 to 8 | 3,909 to 3,957 of 5,872 | 0, unchanged |
| housing, timber, at permit | 396 to 444 of 452 | 56 to 8 | 3,863 to 3,911 of 5,664 | 0, unchanged |

On the timber office at permit `NetSurfaceArea` keeps its 5 different (the first version made 9, section 2), and the quantities rvt-rs writes that Revit does not (0, 0, 3, 3 on the offices; 7, 0, 2, 2 on the housing) are unchanged on every file. The housing's gain of 48, 40, 48 and 48 is the 184 mitred mullions of section 1, all of them. The row registered on #1636 before the first version's run holds on its four points for this version: the offices gain far more than the 1,500 it asks, the housing's 184 mitred mullions read equal (it asks 90), nothing is different, and every export without a curtain mullion is byte-identical.

## 4. The Autodesk samples

Measure [42953777974](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42953777974) kept both sides' exports of the 58 Autodesk samples (`samples=true keep_ifc=true scorers=none`; the head `75fd899c627895a1bd772f791303a8f0baf395f3`, kept under `refs/archive/2026-11-11/cited/75fd899c6278`, against `2136399d614606bd760d11ea34322b00cc740045`, RE-418's, kept under `refs/archive/2026-11-11/cited/2136399d6146`): 13 exports move and 45 are byte-identical, and the 13 are `rac_advanced` (2024 to 2027), `rme_advanced` and `rme_basic` (2024 to 2027) and Snowdon Towers Architectural (2024). No Revit export of them exists to score against, so each moved file is read element by element.

`tools/re/ifc_element_diff.py` pairs every `IfcElement` of the two exports of a file by GlobalId: the only change on all 13 is `NetSurfaceArea` added to the `Qto_MemberBaseQuantities` of curtain mullions, one `IfcQuantityArea` each (79 mullions in each `rac_advanced` save, 569 in each `rme_advanced` and `rme_basic` save and 9 in Snowdon's, 4,877 in all); nothing is removed or changed, and no element is added or removed.

`tools/re/mullion_added_areas.py <before> <after>` reads each added value against the mullion's own mesh, measured along the axis it is a prism along: on all 13 files every one is the body's lateral area plus 0, 1 or 2 of its flat ends or, of a mitred one, its mitre face and, where the end is kept, its flat end, and none is anything else:

| file | added | lateral only | + 1 flat end | + 2 flat ends | mitre (+ 1 flat end) |
|---|---:|---:|---:|---:|---:|
| `rac_advanced`, each of 4 saves | 79 | 62 | 16 | 1 | 0 |
| `rme_advanced` and `rme_basic`, each of 8 saves | 569 | 71 | 66 | 401 | 31 (all with one flat end) |
| Snowdon Towers Architectural 2024 | 9 | 0 | 8 | 0 | 1 (no flat end) |

That is the rule's own shape and no more: what those files' Revit exports would write is not known here.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref feat/1046-mullion-surface-area -f ref=75fd899c627895a1bd772f791303a8f0baf395f3 -f base=2136399d614606bd760d11ea34322b00cc740045 \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
```

The scorer of section 1: `python3 tools/re/mullion_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>`; the samples of section 4: `python3 tools/re/mullion_added_areas.py <before.ifc> <after.ifc>` over the kept exports of a run with `-f samples=true -f keep_ifc=true -f scorers=none`.

## 6. Sources

RE-243, RE-248, RE-268, RE-387, RE-403, RE-418. revit-ifc's `NetSurfaceAreaCalculator` and `PropertyUtil.CreateBeamColumnBaseQuantities` as RE-268 read them (CLEANROOM.md source 5). No Autodesk source was read.
