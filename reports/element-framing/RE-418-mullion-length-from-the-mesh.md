# RE-418: a curtain mullion's `Length` and `Span` are the extent of its saved mesh along its own axis where its record box overruns the solid

**Date:** 2026-11-10
**Issues:** #1046 (curtain mullion quantities), #1636 (second-project measurement); follows RE-403 (the box is not the solid), RE-387 (the mesh's volume) and RE-268 (kept end faces)
**Artefacts:** the BIM4LCA office at building permit, in concrete and in timber, and the housing in its four large files (Revit 2024, Revit's IFC4 exports), as Measure fetches them. Never stored.
**Runs:** Measure [42943892070](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42943892070) (the scorer `tools/re/mullion_profiles_vs_ifc.py` at `e24fe0903abe134ec10719e458a6b7b94d7307ff`, kept under `refs/archive/2026-11-11/cited/e24fe0903abe`, on `main` at `9bf2ccf801e1bf9eec6967f4d3b46a35818af4ee`) for section 1; [42950801466](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42950801466) (unfiltered, head `59534e00e1cef6d7113901ea85b8588ac47f78f8`, kept under `refs/archive/2026-11-11/cited/59534e00e1ce`, against `612daaba4e791055b11444d8860bb7ce7d394f10`, its merge base with `main`) for sections 3 and 4; [42946108250](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42946108250) (the first version, head `c4e7e54d1ba2c24d6c4246705da5cffaf93b00fb`, kept under `refs/archive/2026-11-11/cited/c4e7e54d1ba2`, unfiltered against `main` at `134db9761fc48eaf4eceefb3f37e7591e7ec5e29`) and [42948431190](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42948431190) (the Autodesk samples with both sides' exports of that version kept) for the false positive of sections 2 and 4
**Status:** Positive. 49 of the 71 mullions of the BIM4LCA timber office at permit whose `Length` differs from Revit's now equal it, and so does their `Span`; 1 of the 126 exports moves and the other 125 are byte-identical, the 58 Autodesk samples among them.

## 1. What the scorer found

`tools/re/mullion_profiles_vs_ifc.py` pairs every curtain mullion (`IfcMember`, `PredefinedType` MULLION) of rvt-rs's export with Revit's by `Tag`, reads each side's `Length`, `Span`, `NetVolume` and `NetSurfaceArea` in their own units (Revit's export declares millimetres, square metres and cubic metres), and measures the body's mesh along the axis it is a prism along: the planes of its triangles, at most two of them (its ends) not parallel to the axis, and where several axes do that (a box has three), the one whose extent is nearest the side's `Length`.

- **Revit's `Length` is the extent of its own body.** On the 10,031 mullions of the office at permit it is the mesh's extent along its axis on every one but one (a mullion whose body the scorer does not read as a prism), and the same on the timber office's 7,872 and on the housing's 354 to 444 per file.
- **rvt-rs's body is Revit's on most.** Its mesh has the same extent and area as Revit's on 9,847 of the office's 10,031 mullions at permit and 7,681 of the timber office's 7,872; the others are cut differently or are not prisms the scorer reads.
- **The written `Length` is the record box's extent along the mullion, and it is Revit's on all but 71 of the timber office's 7,872 mullions** (19 of the concrete office's 10,031; none on the housing's). Of the 71, **49 are mullions whose box overruns the solid: rvt-rs's mesh is Revit's and its extent along its axis is Revit's `Length`**; 18 have no axis the scorer can read as theirs and 4 differ with the mesh's extent too (3 flat-ended, 1 mitred), and none of those 22 is a change this report makes.

| mullions of the timber office at permit | count |
|---|---:|
| `Length` and `Span` written equal to Revit's, and the mesh's extent equal | 7,773 + 28 |
| written different, the mesh's extent equal to Revit's | 49 |
| written different, the mesh's extent different too | 3 + 1 |
| written different, no axis read | 18 |

## 2. The rule

`saved_meshes::follow_mesh_over_box` (RE-403) already replaces a curtain mullion's box `NetVolume` with its mesh's where the two differ by more than 0.1%, and takes out the box's `NetSurfaceArea`. Where it does, the mesh is not an extrusion of the box, and the box's length is not the solid's either: the mullion's `Length` becomes the extent of its mesh along the axis its record runs it along (`RevitFile::mullion_axes`, set as the partition's mullions are read, from the box's axis the written `Length` is measured on), where the mesh is a prism along that axis (`mesh_volume::extent_along`: every plane of its triangles but two parallel to it, at most 32 planes) and the extent lies within 35% of the written `Length`; otherwise the written `Length` stays. A `Span` follows its `Length`, as it does at the record. A difference under 1e-4 ft (the mesh's corners are single precision) leaves the written value alone.

The axis is the record's, not one found from the mesh. The first version of this change took the one extent of the mesh, along the axes it is a prism along, that lay within 35% of the written `Length`, and an element-by-element diff of the Autodesk samples (section 4) showed it wrong on one mullion of each `rac_advanced` save, a 50 by 150 mm piece with a gabled top: along its own axis it has three planes that are not parallel to it (its bottom and the two of the gable), so only the side of its section looked like a prism axis, and its `Length` went from 0.2018 m to 0.15 m, the section's width. No Revit export of those saves scores it, and the scorer, which picks the axis nearest the written `Length`, would not have shown it either. The record's axis cannot be taken for a side of the section, and `extent_along` has a test for that piece's shape. The search of the first version over every pair of planes was also quadratic in the planes of a mesh (a 24-sided tube bent over 200 segments has about 9,600), which a read of the first version found: the axis is given, the mesh is read once along it, and a mesh with more than 32 planes (the most of 27,059 mullions measured is 13) is not read.

## 3. Measured: every model

Unfiltered Measure 42950801466, every flag, 126 models on each side: 125 exports are byte-identical and 1 moves, the timber office at permit; none of the 58 Autodesk samples moves, the four `rac_advanced` saves of Revit 2024 to 2027 among them (section 4). The row registered on #1636 before the first version's run holds on all three points of this version too. Its second named the timber office as built as the one other model I expected to move; it does not (it is byte-identical).

| timber office at permit | before | after |
|---|---:|---:|
| `Qto_MemberBaseQuantities.Length` equal / different / missing | 7,801 / 53 / 18 | 7,850 / 4 / 18 |
| `Pset_MemberCommon.Span` equal / different / missing | 7,801 / 53 / 18 | 7,850 / 4 / 18 |
| quantities Revit writes that rvt-rs writes: different | 71 | 22 |
| property values: different | 101 | 52 |

Every other line of `qtos_vs_ifc` and `psets_vs_ifc` is identical. The 49 are written 0.01 m to 0.279 m over Revit's (0.49% to 14.3%): ten mullions of one type at 2.23 m against 1.951 m (ElementIds 3076107 to 3076116), two at 2.28 m against 2.001 m (3076105 and 3076106), one at 2.0 m against 1.951 m (3520147) and 36 more. The 4 that still differ and the 18 with no `Length` are the 22 of section 1 that the rule does not touch. The `same-project gates` check passed on the head (15 samples held to their earlier saves, and the Areas on 4 saves and on the 2021 save).

## 4. The Autodesk samples

Autodesk's `rac_advanced` sample holds curtain mullions too, in four saves of Revit 2024 to 2027, and Measure holds no Revit export of them to score a change against. The first version of this change moved all four (run 42946108250), and run 42948431190, which kept both sides' exports of the 58 samples, let `tools/re/ifc_element_diff.py` read them element by element: it showed one mullion of each save wrong, the gabled piece of section 2, whose `Length` went from 0.2018 m to the 0.15 m of its section's width. With the record's axis (section 2), all 58 samples' exports in run 42950801466, the four `rac_advanced` saves among them, are byte-identical to the parent's (`ifc.sha256` of each model), so the change reaches no mullion of any sample and the piece the first version got wrong is as it was.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref fix/1046-mullion-axial-extent -f ref=59534e00e1cef6d7113901ea85b8588ac47f78f8 -f base=612daaba4e791055b11444d8860bb7ce7d394f10 \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
```

The scorer of section 1: `python3 tools/re/mullion_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>` (its Measure runs are 42943892070 and 42946015504).

## 6. Sources

RE-243, RE-268, RE-387, RE-403. revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities` as RE-353 read it (CLEANROOM.md source 5). No Autodesk source was read.
