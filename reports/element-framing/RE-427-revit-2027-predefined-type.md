# RE-427: Revit 2027's export writes no PredefinedType on the occurrence of ten classes, and still writes one on proxies, openings, spaces and every type object

**Date:** 2026-11-11
**Issues:** #1692 (the 2027 pairs write none where rvt-rs wrote a value), follows RE-419 (#1681, #1686: furniture's `NOTDEFINED`) and #1472 (`ObjectType` from 2026); the census it opened is #1713
**Artefacts:** the 18 held-out models (counts only: hemanth1986's house and office saved by 2022, 2024, 2025, 2026 and 2027, `modelo_bim`, `teste_export_2025`, the CIADD house, BIM-Speed's four and BIMwhale's), Autodesk's sample projects of 2015 to 2027, the BIM4LCA office and housing, the 24 R2025 models and the other reference models, as Measure fetches them. Never stored.
**Scorer:** `tools/re/predefined_types_vs_ifc.py`, `tools/ci/scrub-holdout-results.sh`
**Runs:** Measure [43010340133](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43010340133) (the held-out pairs, the widened scorer, `main` at `5cf94975`), [43010792248](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43010792248) (every scorer on the held-out pairs, the baseline for sets, quantities and GlobalIds), [43011481868](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43011481868) (the scorer on every model, the census of section 5) and the change, unfiltered with `keep_ifc` against `main` at `5cf94975`: [43012719379](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43012719379) (head `257adfc2`, which failed two checks, row 6) and [43017489469](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43017489469) (the final head `5d3cccc1`, `same-project gates` passing, kept under `refs/archive/2026-11-11/cited/5d3cccc1b477`)
**Status:** Positive on the ten classes, narrow on the rest. Revit 2027's export writes no `PredefinedType` on the occurrence of stairs, stair flights, slabs, members, furniture, doors, windows, walls, railings and roofs (71 elements of the two held-out pairs) and still writes one on proxies, openings, spaces and every type object; rvt-rs now leaves it out on those ten classes from 2027, the 2027 pairs read `differs` 0 on them, and 119 of 126 exports are byte-identical (the 7 that move are 2027 files). The widened scorer found a longer list of `PredefinedType` gaps on 2015 to 2026 saves, filed as #1713.

## 1. The question

#1692 found that on the two held-out pairs saved by Revit 2027 (a house and an office, counts only) Revit's own export writes no `PredefinedType` on any class `tools/re/predefined_types_vs_ifc.py` read, where the same projects saved by 2024, 2025 and 2026 write `STRAIGHT_RUN_STAIR`, `STRAIGHT`, `FLOOR`, `STRINGER`, `NOTDEFINED` and rvt-rs wrote them: 7 elements on four classes. The scorer read eight classes; rvt-rs writes a `PredefinedType` on many more. Does the 2027 export write none on every class, or on some?

## 2. What the scorer reads now

It pairs every `IfcElement` class that declares a `PredefinedType` by Tag (it paired eight), counts the elements one file holds and the other does not by class, tallies the `PredefinedType` of every `IfcElementType` class of both files and of every product that is no element and declares one (spaces, which carry no Tag, and ports), and opens each file once. The held-out scrub keeps the class lines as counts beside IFC class names and enumerators, never a Tag.

## 3. What Revit's 2027 export writes

Measure 43010340133, the two 2027 pairs (house and office), elements both files hold:

| class | house | office | Revit's export of the same projects saved by 2026 | rvt-rs before this change |
|---|---:|---:|---|---|
| `IfcStair` | 1 | | `STRAIGHT_RUN_STAIR` | wrote it (differs) |
| `IfcStairFlight` | 1 | | `STRAIGHT` | wrote it (differs) |
| `IfcSlab` | 2 | 1 | `FLOOR` | wrote it (differs) |
| `IfcMember` | 2 | | `STRINGER` | wrote it (differs) |
| `IfcFurniture` | 4 | 8 | `NOTDEFINED` | none (RE-419) |
| `IfcDoor` | 3 | 4 | `DOOR` | wrote it (differs) |
| `IfcWindow` | 12 | 10 | `WINDOW` | wrote it (differs) |
| `IfcWall` | 10 | 9 | `NOTDEFINED` | wrote `NOTDEFINED` |
| `IfcRailing` | 2 | | `NOTDEFINED` | none |
| `IfcRoof` | 1 | 1 | `NOTDEFINED` | none |

Revit's 2027 export writes **no `PredefinedType` on any of these 71 elements**. It writes one on the others:

| class | house | office | Revit's 2027 export | rvt-rs |
|---|---:|---:|---|---|
| `IfcBuildingElementProxy` | 48 | 40 | `NOTDEFINED` on every one | none (all releases, #1713) |
| `IfcOpeningElement` | 15 | 14 | `OPENING` | `OPENING` |
| `IfcSpace` | 4 | 4 | `NOTDEFINED` | `SPACE` (all releases, #1713) |
| type objects: door 2, window 1, slab 1 (and a roof's), stair 1, flight 1, member 1, wall 2, railing 1, furniture 2, proxy 3 (house) | | | their enumerators (`DOOR`, `WINDOW`, `FLOOR`, `STRAIGHT_RUN_STAIR`, `STRAIGHT`, `STRINGER`, `NOTDEFINED`) | equal on every class but slabs and roofs |

On the slab and roof types: Revit's 2025, 2026 and 2027 exports type the roof by an `IfcSlabType` `ROOF` (a second slab type) and write no `IfcRoofType`, where rvt-rs writes an `IfcRoofType` `NOTDEFINED`; Revit's 2024 export writes the `IfcRoofType`, as rvt-rs does. That is a type-object difference of the roof and not of 2027.

Why Revit's 2027 export writes none on those classes and a value on the others is not known. Every class it omits has a type object that states an enumerator, and the proxies, which it does not omit on, have type objects too (3 in each pair), so "the occurrence is left out where its type states one" is a pattern and not a rule. The two pairs are the only 2027 exports of Revit that the repository holds.

## 4. The rule

The occurrence of `IFCSTAIR`, `IFCSTAIRFLIGHT`, `IFCSLAB`, `IFCMEMBER`, `IFCFURNITURE`, `IFCDOOR`, `IFCWINDOW`, `IFCWALL`, `IFCRAILING` and `IFCROOF` is written with `PredefinedType` `$` when the file's release is 2027 or later (`PREDEFINED_TYPE_LEFT_OUT_FROM`, `PREDEFINED_TYPE_LEFT_OUT_CLASSES` in `src/ifc/step_writer.rs`), as `ObjectType` is left out from 2026 (#1472, Griffin, 2026-11-09: from the release only, since a 2027 save can only be exported by Revit 2027 or later). The type objects, their GlobalIds and the property sets that depend on a predefined type take the element's value as before, so none of them moves; proxies, openings and spaces are as before. A class the two pairs do not hold (columns, beams, coverings, curtain walls, plates, footings, ramps, shading devices and the MEP terminals) is not measured on 2027 and keeps its value. A member is read on two stringers only, so a mullion is left out with them on the strength of its class: no 2027 pair holds a curtain wall.

## 5. What the widened scorer found beyond 2027

On the 42 models with a Revit IFC4 export (Measure 43011481868, `main` at `5cf94975`) rvt-rs writes none, or `NOTDEFINED`, where Revit's export writes a value on 3,842 of 4,359 walls (`PARTITIONING`, `SOLIDWALL`, `PLUMBINGWALL`, `SHEAR`, `PARAPET`), all 378 railings (`GUARDRAIL`, `HANDRAIL`), 25 of 43 roofs, 82 of 116 proxies, all 38 ramps, all 174 light fixtures, all 48 pipe segments, 42 spaces and 10 stairs and flights, and writes `DOOR` where Revit writes `TRAPDOOR` on 52 of 1,508 doors; every other class reads equal on every element (columns, beams, coverings, curtain walls, members, openings, plates, slabs, windows and the terminals). The held-out pairs show the same on 2024 to 2026 and, on every pair of 2022 to 2026, a wall type `STANDARD` in Revit's export and `NOTDEFINED` in rvt-rs's. These are #1713 and are not changed here.

## 6. The registered rows

Registered on #1636 before the first run was read (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6794324964).

| row | the bar | result |
|---|---|---|
| 1 | Revit 2027 writes none on every class, type and space of the two pairs; a class that reads another value is named and the writer leaves out only the classes that read none | **Missed as worded, and its fallback applies.** Ten classes read none (71 elements); proxies (88, `NOTDEFINED`), openings (29, `OPENING`), spaces (8, `NOTDEFINED`) and every type object read a value (section 3). The writer leaves out the ten classes only. |
| 2 | The number of classes the pairs hold is counted and the unmeasured ones named | **Held.** The pairs hold 12 element classes (10 read none, 2 a value), 11 type classes and the spaces; columns, beams, coverings, curtain walls, plates, footings, ramps, shading devices and the MEP terminals are not in them (section 4). The five 2027 samples hold 6,320 curtain wall mullions, which the member rule reaches on the strength of the class: the pairs hold two stringers and no mullion. |
| 3 | On the 2022, 2024, 2025 and 2026 held-out pairs `differs` 0 and `NOTDEFINED where Revit writes none` 0 on every class both files hold, except the 2022 pair's furniture | **Held** on all 18 held-out models (run 43010340133). The elements rvt-rs leaves unset where Revit writes a value (`unset in rvt-rs`: proxies 88 per year, railings, roofs, the 2022 pair's stair, flight, furniture and railing) are #1713 and are not 2027. |
| 4 | After the change the 2027 pairs agree: `differs` 0, `unset` 0, `NOTDEFINED where Revit writes none` 0; rvt-rs's type and space tallies read `{'-': n}` as Revit's do | **Held for the ten classes, and the second half rested on row 1.** Every one of the 71 elements reads `same` (stairs 1, flights 1, slabs 3, members 2, furniture 12, doors 7, windows 22, walls 19, railings 2, roofs 2); the proxies' 88 `unset` and the spaces' `SPACE` against `NOTDEFINED` are as on `main` and as in 2024 to 2026 (#1713); the type tallies are as on `main`, equal but for slabs and roofs. |
| 5 | Every export of a save of 2022 to 2026 is byte-identical to `main`'s; for each 2027 model that moves, only `PredefinedType` attributes change | **Held.** 119 of 126 exports are byte-identical; the 7 that move are the two 2027 held-out pairs and Autodesk's `rac_advanced`, `rac_basic`, `rme_advanced`, `rme_basic` and `rst_basic` of 2027. On the five samples no entity is added or removed and the only attributes that change are `PredefinedType` (an enumerator to `$`) on doors, members, slabs, stairs, flights, walls and windows, and the export time: 239, 18, 3,619, 1,755 and 1,755 values. |
| 6 | Sets, quantities, GlobalIds and schema findings stay | **Held for the exports, missed by two checks that read the value.** `psets_vs_ifc`, `qtos_vs_ifc`, `global_ids_vs_ifc`, `names_vs_ifc` and the schema findings are unchanged on all 126 models. The MCP consistency check (the server answered the element's predefined type, the export wrote none: 31 and 24 differ on the two pairs) and the schema arity check (a null `PredefinedType` on a door, member, slab, wall or window) failed on the first run, 43012719379; the viewer's panel, the schedule CSV and the MCP server now report what the export writes (`category_map::predefined_type_left_out`) and the arity check reads the release from the project's description. |
| 7 | No new read; time and peak memory within the runs' noise | **Not timed:** the change reads nothing (it is an attribute written as `$`). |

## 7. Reproduce

```bash
# the scorer on the held-out pairs (counts), then on every model
gh workflow run measure.yml --ref research/1692-predefined-types -f ref=<code SHA> -f base=none -f holdout=true -f models="holdout-*" -f scorers=predefined_types_vs_ifc
# the change against main
gh workflow run measure.yml --ref research/1692-predefined-types -f ref=257adfc2d1ed0cc43ea6064e58aba72a12ec086c -f base=5cf9497503de71582766b422d150038e4cda712f -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
# one pair of exports
python3 tools/re/predefined_types_vs_ifc.py <rvt-rs.ifc> <revit.ifc>
```
