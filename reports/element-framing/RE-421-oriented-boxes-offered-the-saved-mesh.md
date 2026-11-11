# RE-421: a curtain mullion or panel drawn as its oriented box is offered the mesh Revit saved for it, and says why where it keeps the box

**Date:** 2026-11-11
**Issues:** #1689 (the gap), #1645 (every box says why), #1636 (second-project measurement); follows RE-106 (curtain mullions and panels are boxes along their own axes), RE-403 (a mullion's box is not its solid) and RE-404 (a mullion's mitred ends)
**Artefacts:** the BIM4LCA office in concrete and timber, as built and at permit (Revit 2024, Revit's IFC4 exports), the housing's twelve other files, the held-out pairs of Revit 2022 to 2027 (counts only), and Autodesk's samples, as Measure fetches them. Never stored.
**Runs:** Measure [42961457553](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42961457553) (unfiltered, head `f2e6cb11c01649b259ca89d3abd189c2e3df2f72`, kept under `refs/archive/2026-11-11/cited/f2e6cb11c016`, against `979ec4287c2fcc455c2fc63dfbcdba46736b5ce7`, `main` with #1691) for section 3; [42964821937](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42964821937) (`tools/re/mullion_profiles_vs_ifc.py` on the office at permit and the housing's four large files) and [42964819358](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42964819358) (both sides' exports of the 58 Autodesk samples kept) for sections 3 and 4
**Status:** Positive. 323 and 326 more of the BIM4LCA office's mullions read `Qto_MemberBaseQuantities.NetVolume` equal to Revit's (10,001 of 10,003 and 7,875 of 7,876 as built), none different; the oriented boxes go from 325 and 327 to 2 and 1, each left saying why; 24 of 126 exports move, 14 of them Autodesk samples whose only change is the oriented-box mullions and panels.

## 1. What was left out

RE-106 draws a curtain mullion or panel that is turned or tilted off the model's axes as the box along its instance's own axes (`BodySource` `partition_family_instance_oriented_box`). The saved-mesh pass (#1191) replaces a body that is a stand-in for the mesh, and `saved_meshes::is_replaceable_box` listed the record box, a beam's box (RE-328) and a family instance's box turned to its angle (RE-87), but not this one. So the pass never looked at these bodies, and #1645's reasons (`RvtElementRecordGeometry.SavedMeshRefusal`, `exported.saved_mesh_refusals`) never reached them either. The BIM4LCA office at permit has 325 of them in concrete and 327 in timber, and each office as built the same; their Revit solids have mitred or cut ends the box does not (RE-404), and the mullions' `NetVolume` Revit writes was missing on every one.

## 2. The change

`is_replaceable_box` also takes `INSTANCE_ORIENTED_BODY_SOURCE`. Everything else follows from what the pass already does for the other three boxes: the mesh replaces the box where it decodes whole, `follow_mesh_over_box` (RE-403) and the mullion quantities (RE-418, RE-404) read it, and where it does not the element keeps its box and carries a reason (`partial: ...`, `no_saved_mesh`, ...), and on a release outside Revit 2023 to 2027 `not_offered: release_outside_2023_to_2027`.

## 3. Measured: every model

Unfiltered Measure 42961457553, every flag, 126 models on each side, head `f2e6cb11` against `979ec428` (`main` with #1691): 102 exports are byte-identical and 24 move, the office's four files, the six `racadv` models of Revit 2015 to 2020, and 14 Autodesk samples (`rac_advanced` of 2015 to 2027 and Snowdon Towers Architectural of 2024; section 4). The housing's six files, the office's two terrain files, Core Interior, Einhoven, RE1 and the 18 held-out models are unchanged: none of them has an oriented box (the pre-registered row expected the housing, Core Interior, Einhoven and RE1 to move; their bodies hold none). `same-project gates` passed on the head.

| the office | concrete as built | concrete at permit | timber as built | timber at permit |
|---|---:|---:|---:|---:|
| bodies still the oriented box | 325 to 2 | 325 to 2 | 327 to 1 | 327 to 1 |
| bodies `saved_graphics_mesh` | 11,100 to 11,423 | 11,217 to 11,540 | 9,134 to 9,460 | 9,132 to 9,458 |
| `Qto_MemberBaseQuantities.NetVolume` equal | 9,678 to 10,001 of 10,003 | 9,706 to 10,029 of 10,031 | 7,549 to 7,875 of 7,876 | 7,545 to 7,871 of 7,872 |
| `NetVolume` missing, different | 325 to 2, 0 | 325 to 2, 0 | 327 to 1, 0 | 327 to 1, 0 |
| `SavedMeshRefusal` reasons | 168 to 170 | 169 to 171 | 167 to 168 | 176 to 177 |

The boxes left are the ones whose saved graphics do not decode whole: each carries `partial: disconnected UV boundary` as its reason, and the reasons rise by exactly the boxes left (2, 2, 1, 1). No `different` rises on any line of `qtos_vs_ifc` or `psets_vs_ifc` of any model, and the only top lines that change are the offices' `qtos_vs_ifc`: `equal` rises and `missing` falls by 323 on the concrete files and 326 on the timber ones.

**Against Revit's body** (`tools/re/mullion_profiles_vs_ifc.py` over both sides, Measure 42964821937: the office at permit in concrete and timber and the housing's four large files). Before, 166 of the concrete office's mullions and 173 of the timber one's had a body that is not Revit's (the oriented box), and Revit's `NetSurfaceArea` of them was no sum the scorer reads. After, 132 and 142 of them read "our body is Revit's", with the mitred ends Revit's solids have (147 and 151 `mitre`/`mitre` bodies from none, 46 `flat`/`mitre` from 29). The 34 and 31 that still differ include 32 and 29 whose mesh is Revit's body to within 1.2e-5 and no better (an end face of 0.005 m² reads 0.00499988, a lateral area of 0.76 reads 0.759991, so the scorer's tolerance of 1e-5 calls the body different, while their `Length` and `NetVolume` read equal; the cause of the noise is not established); the other 2 and 2 are not told apart by tag, since the scorer lists only the first 30 of a category. The 7,560 and 5,292 mullions whose every value read equal read equal after, and the written values of every other category are identical.

On Revit 2015 to 2022 files the pass is not offered, and the boxes keep their bodies: `rac_advanced` of those years has 148 oriented boxes before and after, and each now has `not_offered: release_outside_2023_to_2027` as its `SavedMeshRefusal` (5,160 to 5,308 reasons on the 2015 to 2020 saves, 5,213 to 5,361 on 2021 and 2022), which is why those exports move.

## 4. The Autodesk samples

Autodesk's `rac_advanced` sample holds curtain mullions and panels drawn as oriented boxes (148 in each save, 106 mullions and 42 panels), and so does Snowdon Towers Architectural (1,019, 786 mullions and 233 panels); Measure holds no Revit export of either to score a change against. Run 42964819358 kept both sides' exports of the 58 samples: 44 are byte-identical and 14 move, `rac_advanced` of every release from 2015 to 2027 and Snowdon Towers Architectural of 2024. `tools/re/ifc_element_diff.py` pairs every element by GlobalId and finds, on all 14, that no element is added or removed and that only `IfcMember` and `IfcPlate` change:

| samples | elements changed | what changes |
|---|---:|---|
| `rac_advanced` of 2015 to 2022 (eight saves, outside the pass's releases) | 148 each | `SavedMeshRefusal` is added to the 106 mullions and 42 panels (`not_offered: release_outside_2023_to_2027`); their bodies stay boxes |
| `rac_advanced` of 2023 | 148 | all 148 drawn from their saved meshes (`BodySource` `saved_graphics_mesh`, an `IfcExtrudedAreaSolid` becoming an `IfcTriangulatedFaceSet`); `NetVolume` added on all 106 mullions and 42 panels |
| `rac_advanced` of 2024 to 2027 | 148 each | the same; `NetVolume` added on 96 mullions and 42 panels, and replaced on 8 mullions that already held the box's (RE-403) |
| Snowdon Towers Architectural of 2024 | 1,019 | all drawn from their saved meshes; `NetVolume` added on 233 panels and 6 mullions, replaced on 5 mullions |

Nothing else differs on any of them: no other class, no other property or quantity, and no element is added or removed. The 14 are scored by nothing, so these are read, not measured: the same rule gives Revit's `NetVolume` on 323 and 326 more of the office's mullions with none different (section 3).

## 5. Reproduce

```bash
gh workflow run measure.yml --ref fix/1645-oriented-boxes-offered -f ref=f2e6cb11c01649b259ca89d3abd189c2e3df2f72 -f base=979ec4287c2fcc455c2fc63dfbcdba46736b5ce7 \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
gh workflow run measure.yml --ref fix/1645-oriented-boxes-offered -f ref=f2e6cb11c01649b259ca89d3abd189c2e3df2f72 -f base=979ec4287c2fcc455c2fc63dfbcdba46736b5ce7 \
  -f samples=true -f keep_ifc=true -f scorers=none
```

The scorer of section 3: `python3 tools/re/mullion_profiles_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>` (Measure 42964821937, dispatched from `fix/1046-mullion-axial-extent`, where the scorer is).

## 6. Sources

RE-106, RE-243, RE-268, RE-403, RE-404, RE-418. No Autodesk source was read.
