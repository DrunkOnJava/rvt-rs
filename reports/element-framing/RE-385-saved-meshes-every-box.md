# RE-385: what keeps an element's saved mesh from replacing its box, on Snowdon Towers Electrical

**Date:** 2026-11-09
**Issues:** #1191 (W6: no body is a bounding box), after #1610 (`cbeba4c7`: every boxed element drawn from its saved mesh by default on Revit 2023 to 2027); follows RE-78 (saved display meshes), RE-87 (a family instance's record box turned to its angle), RE-224 (a partial mesh keeps its box) and RE-329 (what keeps a saved mesh from decoding whole)
**Artefacts:** Autodesk's Snowdon Towers sample models: the Revit 2024 Electrical model as `research/autodesk-sample-projects.tsv` pins it (sha256 `e50e1b71…1750`), and the Revit 2025 editions of Electrical (40,505,344 bytes, sha256 `0b6d815e7bc2bbaafabed2d60d349b1fda9e4866fb35d997b3891d7dc9a8af8a`), Architectural (97,775,616 bytes, `8ac90a85bc3e2f3677b9978850bc90c50f79e488ab58b54e6da0efd53150d334`) and Facades (52,715,520 bytes, `8b94ed1b8aed1e70cc4916a587d87e74b52beac1a44a1b29d7e0984388972370`), which Autodesk ships with Revit 2025 and which are not pinned or committed; Autodesk's other sample projects of 2023 to 2027, Core Interior, Einhoven and the Golden Nugget models for the export comparison (section 5)
**Probe:** `probes/examples/probe_re385_saved_meshes_every_box.rs`
**Status:** positive for four of the five causes. With the four changes below, Snowdon Towers Electrical (2024 and 2025) keeps 18 of its 2,428 elements as boxes, against 300: the 16 drops of the art-glass pendants, whose nested instances are forbidden the targets 10 (left refused), one pendant and a floor the saved meshes are never offered. Every one of the 2,409 meshes that replaces a box fits inside it, and on the 13 models probed no mesh the changes add is larger than its box. **Open:** what the bits of `m_forbiddenTarget.m_targets` name (section 3.5); the 1,304 boxes left on Architectural and 157 on Facades (section 6).

## 1. The question

A user opening Snowdon Towers Electrical 2025 in the viewer saw "132 of 2428 drawn from the element's own data; 2296 are the element's bounding box". Before #1610 (`main` at `52282ea1`) the viewer and the CLIs drew only doors and windows from their saved meshes by default; #1610, merged as `cbeba4c7`, makes every boxed element of a Revit 2023 to 2027 file a candidate. In that scope, at `cbeba4c7`, the file still keeps 300 elements as boxes: 170 record boxes (one of them a floor, which `attach_scoped` never offers a saved mesh: it has a resolved profile) and 130 family instances drawn as their record box turned to their angle (RE-87), which `saved_meshes::attach_scoped` never offered to their saved mesh. The probe lists what each element's saved graphics report.

## 2. What the probe measures

`probe_re385_saved_meshes_every_box` exports one file three times (`SavedMeshChoice::Off`, `Default` and `All`) and prints, for each export, the bodies by `BodySource`. For every element that is a box (`partition_element_record_bbox`, `partition_beam_box_along_axis` or `partition_family_instance_turned_box`) with saved meshes off and a `saved_graphics_mesh` in the export for every element, it sets the mesh's extents in the element's local frame against the box's, both sorted, and counts a mesh as fitting when each is at most the box's plus 0.02 ft. For every element still a box in that export that `attach_scoped` would offer a saved mesh (it has a placement and no resolved profile), it reads the element's saved scene at detail level 3 (`native_saved_scene::extract_at_detail`) and counts the distinct diagnostics it reports, digits replaced by `#`, one per element and message. An element can report several, so the counts sum to more than the elements.

## 3. The five causes, on Snowdon Towers Electrical 2025

At `main` (`cbeba4c7`, #1610 merged), whose default on Revit 2023 to 2027 is the every-element scope, on the 299 elements of the 2025 edition offered a saved mesh (on the 2024 edition the rows are the same, but for no `nested detail-family filter` row and 116 in the `decoded` row):

| what the saved graphics report | elements |
|---|---:|
| `unsupported parametric profile curve GEllipse` | 136 |
| `nested detail-family filter requires view semantics` | 131 |
| `(status decoded_saved_graphics)`: the whole mesh decodes, but the element was never offered it | 113 |
| `boundary sample was discarded by triangulation` | 28 |
| `instance target restrictions are unsupported` | 16 |
| `nonlinear ruled profiles need a curvature policy` | 2 |
| `multiple cylinder trim loops unsupported` | 1 |

### 3.1 Turned boxes

RE-87 draws a family instance whose record box is not aligned with the project axes as that box turned to its angle (`partition_family_instance_turned_box`). It is as much a stand-in for the element's shape as the record box, but `attach_scoped` offered only record boxes and beam boxes to the saved meshes. On Electrical 130 receptacles, outlets, conduit elbows and pendants are drawn so; 116 of them have a saved mesh that decodes whole. They are now offered as a record box is.

### 3.2 `GEllipse`

The domes of the pendant lights (Pendant-Dome and others) are `SurfRev` faces whose meridian is a `GEllipse`, which the profile reader refused. Its fields, as the file's own `Formats/Latest` names them, are `m_center`, `m_xVec`, `m_yVec` (unit, orthogonal), `m_xLen`, `m_yLen` (the semi-axes) and `m_endParams`; the point at `t` is `m_center + m_xLen cos t m_xVec + m_yLen sin t m_yVec`. The reader takes it with those checks, and a revolved ellipse is stepped as a circle of its larger semi-axis would be, which bounds its chord error at least as tightly, since the second derivative of the ellipse is never longer than that semi-axis.

### 3.3 Ruled surfaces between curves

A `RuledSurf` was read only between two lines. The two hand dryers have one between arcs. The grid of a ruled component now takes, for each profile, the steps its own curvature needs (one per foot along a line, the angular step of its radius across an arc, of its larger semi-axis across an ellipse), together with the twist bound already used for lines.

### 3.4 A ring that repeats its first point

The end caps of 28 conduits are trim rings whose last sample repeats their first, 1e-12 ft apart. The triangulation keeps one of the two and the boundary check refused the face because the other was in no triangle. A sample that coincides with one already used is now taken as used.

### 3.5 Instances forbidden the targets 10 (left refused)

Each of the 16 drops of the cascading art-glass pendants nests a `GInstance` whose `m_forbiddenTarget.m_targets` is 10, placed at the ceiling, where the drop's mesh starts. The traversal refuses any value other than 0, and this report leaves it so. Drawn as 0 is drawn, each such pendant's mesh fits inside its record box, but fitting the box cannot exclude geometry a view would leave out inside it, and what the bits name is not read. A reader keeps the box for what it does not understand (RE-224). What would decide it: a file whose Revit export shows a nested instance with these targets drawn or left out, or two saves of one file, by two releases, in which the value changes.

### 3.6 Detail-family filters

On the 2025 edition the 131 elements reporting `nested detail-family filter requires view semantics` also report one of the causes above, or decode whole (3 of the 116 turned instances). That message comes from a filter the traversal leaves out at detail level 3, and it does not keep a mesh from being whole (RE-224). With the four changes none of them is left.

## 4. Result

The same export, with the four changes (`feat-bbox-bodies`, on `main` at `cbeba4c7`, where #1610 landed):

| file | boxes at `cbeba4c7` | boxes with RE-385 | of those, not offered | meshes that replace a box | of which fit it |
|---|---:|---:|---:|---:|---:|
| Snowdon Towers Electrical 2024 and 2025 | 300 | 18 | 1 | 2,127 → 2,409 | 2,409 |
| Snowdon Towers Architectural 2025 | 1,435 | 1,304 | 278 | 2,372 → 2,502 | 2,502 |
| Snowdon Towers Facades 2025 | 166 | 157 | 1 | 168 → 177 | 177 |
| Core Interior 2024 | 392 | 392 | 392 | 138 → 138 | 138 |

The 18 left on Electrical are the 16 art-glass pendant drops (`instance target restrictions are unsupported`, section 3.5), Pendant Light - Linear 1632276, a cylinder with two trim loops (`multiple cylinder trim loops unsupported`), and the floor 1484949 (`Floor:Concrete 6"`), which is not offered a saved mesh.

Fitting the box is a bound, not a match: a light's record box includes its light source's shape (an exterior spot of 0.5 ft has a 13 ft box), so many meshes are far smaller than their box. Revit's own export of these files is not held, and no mesh here is scored against it.

## 5. The export elsewhere

`rvt-ifc` at `cbeba4c7` against the same with the four changes, on 29 Revit 2023 to 2027 projects held locally: Autodesk's sample projects of 2023 to 2027 (`rac_basic`, `rst_basic`, `rac_advanced`, `rme_basic`, `rme_advanced`, `rst_advanced`, as `research/autodesk-sample-projects.tsv` pins them), its five pinned Snowdon Towers 2024 models, the seven Snowdon Towers 2025 models, Core Interior, Einhoven and the two Golden Nugget models (local only). Each was exported by default and with `--saved-meshes`, which are the same scope on these releases. No export fails, and every file has the same number of building elements.

The IFC (less `FILE_NAME` and `IfcOwnerHistory`) is the same on 8: Einhoven, Core Interior, `rst_basic` 2023, 2024 and 2027, `rac_advanced` 2026 and 2027 and `rst_advanced` 2027. On the 21 others more bodies are saved meshes:

| file | saved meshes | record boxes (`exported.bounding_box_bodies`) | seconds, default |
|---|---|---|---|
| Snowdon Towers Electrical 2025 | 2,127 → 2,409 | 169 → 3 | 10.3 → 11.6 |
| Snowdon Towers Electrical 2024 | 2,127 → 2,409 | 169 → 3 | 11.7 → 13.7 |
| Snowdon Towers Architectural 2025 | 2,377 → 2,508 | 897 → 799 | 45.0 → 46.2 |
| Snowdon Towers Architectural 2024 | 2,376 → 2,507 | 897 → 799 | 49.7 → 51.2 |
| Snowdon Towers Plumbing 2025 / 2024 | 2,712 → 2,890 / 2,716 → 2,895 | 142 → 121 | 15.2 → 15.2 / 16.2 → 17.3 |
| Snowdon Towers HVAC 2025 and 2024 | 1,023 → 1,071 | 441 → 439 | 6.6 → 6.3 / 7.0 → 7.0 |
| Snowdon Towers Structural 2025 and 2024 | 62 → 83 | 172 → 156 | 6.6 → 6.2 / 7.1 → 7.2 |
| Snowdon Towers Facades 2025 | 168 → 177 | 116 → 116 | 8.8 → 9.2 |
| Snowdon Towers Site 2025 | 45 → 47 | 3 → 3 | 6.6 → 6.0 |
| Golden Nugget Gebäudetechnik 2025 | 844 → 1,826 | 473 → 467 | 33.1 → 37.5 |
| Golden Nugget Architektur 2025 | 527 → 734 | 276 → 234 | 51.2 → 55.9 |
| `rac_basic` 2023, 2024, 2025, 2026, 2027 | 343 → 354, 313 → 336, 312 → 335, 314 → 337, 314 → 337 | 54 → 43, then 38 → 27 | −0.2 to +0.6 |
| `rme_advanced` 2027 / `rme_basic` 2027 | 5,014 → 5,023 / 5,100 → 5,108 | 230 → 229 / 190 → 190 | +0.1 / −1.0 |

`exported.bounding_box_bodies` does not count turned boxes, which is why a file can gain far more meshes than it loses record boxes (Golden Nugget Gebäudetechnik). The times are one run each on one machine, not a benchmark, and runs of one file differ by up to about 9% (Site is faster with the changes): the cost is the meshing of the elements newly drawn, up to 17% on Snowdon Towers Electrical 2024 and 13% on Golden Nugget Gebäudetechnik.

The probe on 13 of these (both Golden Nugget models, `rac_basic` 2024, the five Snowdon Towers 2024 models, `rme_advanced` 2027, Snowdon Towers Electrical, Architectural and Facades 2025, and Core Interior) finds every mesh that replaces a box inside it, except 50 on Golden Nugget Architektur, 0.03 to 1.23 ft taller or wider than their box, which are the same 50 elements at `cbeba4c7`: none is drawn by these changes.

## 6. Not shown

- What `m_forbiddenTarget.m_targets` means (section 3.5). The elements that report `instance target restrictions are unsupported` keep their box: 16 on Electrical, 478 on Architectural, 87 on Facades, 33 on Golden Nugget Architektur and 21 on Golden Nugget Gebäudetechnik.
- Elements that stopped at one of the four causes read here can now stop at a later one, and keep their box: `nonrectangular parametric trim` (Architectural 2025 6 → 7, Facades 6 → 12, Golden Nugget Architektur 73 → 80), `malformed ruled-surface edge is connected to face #` (Structural 2024 0 → 43, Facades 3 → 4, Golden Nugget Architektur 0 → 2), `missing surface pointer target` (Golden Nugget Architektur 7 → 9, Gebäudetechnik 0 → 2), `shared curved edge changes its fixed parameter` (`rac_basic` 2024 3 → 4).
- `surface-bearing component requires qualified owner/view visibility` goes from 25 to 2 elements on Architectural and from 10 to 1 on Facades. Which of the four changes removes it was not traced.

## 7. Reproduce

```
cargo run --profile ci -p rvt-probes --example probe_re385_saved_meshes_every_box -- "Snowdon Towers Sample Electrical.rvt"
```
