# RE-448: a Revit 2015 to 2023 door or window is set in the wall its own record names

**Date:** 2026-11-12
**Probe:** `probes/examples/probe_re448_opening_hosts.rs`
**Follows:** RE-23 and #439 (the host from the reference list), RE-245 (the host from the native record, 2024 to 2027), RE-280 (the 4-byte ElementId of 2015 to 2023)
**Files:** Autodesk's `rac_basic` and `rac_advanced` sample projects on every local save from 2015 to 2024 (`research/autodesk-sample-projects.tsv`), the local `Arch Link Model` (2019), and the other local 2015 to 2023 files for the comparison of section 4.
**Result:** positive: on 2015 to 2023 a door's or window's native record names its host in `m_hostId` as on 2024, and RE-245's rule applied there sets every door and window of both projects in the wall the 2024 save's export sets it in.

## 1. Before

On Revit 2015 to 2023 rvt-rs sets a door or window in the nearest wall its partition record's reference list names: the nearest below its own ElementId, then the nearest above, not a curtain wall (#439, `attach_host_candidates`, `bind_opening_hosts`). RE-245 corrects that from the element's native record where the list names several walls, but on 2024 to 2027 only. It runs inside the cut-loop pass (`attach_cut_loop_sizes`, `CUT_LOOP_REVIT_VERSIONS`), and its reader takes an ElementId only in 2024's `{"m_id":{"m_id64":n}}` shape and reads records on 2023 to 2027.

Exported from their 2023 and 2024 saves, the same projects set doors and windows in different walls (`IfcRelVoidsElement` ∘ `IfcRelFillsElement`):

| Project | Doors and windows | Set in another wall on 2023 than on 2024 |
|---|---:|---:|
| `rac_advanced` | 124 | 28 (22 doors, 6 windows) |
| `rac_basic` | 33 | 1 (door 906937) |

Door 154488 is voided in wall 154229 on 2023 and in 153506 on 2024. The 2023 opening crosses wall 154229 at right angles to the door, 0.66 m from where the 2024 opening is. Window 145607's opening is 2.6 m along the facade from the window.

## 2. What the records hold

Every door's and window's `FamilyInstance` record names its host in `m_hostId` on 2015 to 2023 too, as the 4-byte `{"m_id":{"m_id":n}}`. The probe reads it in either shape (`native_symbol_cuts::instance_hosts`) beside the host the library binds:

| File | Doors and windows | In a wall | Native host read |
|---|---:|---:|---:|
| `rac_advanced` 2015, 2023, 2024 | 124 | 107 | 107 |
| `rac_basic` 2015, 2023, 2024 | 33 | 26 | 26 |

The 17 and 7 others are set in no wall on any of these saves, 2024 included. The native host of each of the 29 in section 1 is the wall the 2024 export sets it in: 154488's `m_hostId` is 153506 on 2015, 2023 and 2024.

## 3. What changes

- `native_symbol_cuts::instance_hosts` reads the first `m_hostId` of each instance's native record, in either ElementId shape, on Revit 2015 to 2027 (`NATIVE_RELEASES`).
- On the 2015 to 2023 path, after `bind_opening_hosts`, `rebind_opening_hosts_to_native` moves each door or window that is set in a wall to the wall its `m_hostId` names, where that is another wall that is not a curtain wall: RE-245's rule. A door or window set in no wall stays so.
- The 2024 to 2027 path is unchanged.

The probe, run on this change, finds every bound door and window in its native host on every save above: `same` 107 of 107 and 26 of 26.

## 4. Results

`rvt-ifc --profile ci` of `main` and of this change, on 58 local files of Revit 2015 to 2023 and on Core Interior (2024) and RE1 Architecture (2025):

- 40 files are identical apart from `FILE_NAME`, among them every 2024 and 2025 file, BIMcollab and Einhoven.
- 20 differ, each only in `IfcRelVoidsElement` and the placement and profile of the openings it relates. No element, property or quantity changes.

| Files | Doors and windows rebound | Against the same project's 2024 save |
|---|---:|---|
| `rac_advanced` 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, and a renamed 2019 copy of it | 28 | each set in the 2024 save's wall; on 2023, each opening's centre and both its faces in that wall within 0.0005 m of the 2024 opening's |
| `rac_basic` 2015 to 2023 (9 saves) | 1 (906937) | the same |
| `Arch Link Model` (2019, local only) | 19 | no later save |

On `main` the same 29 openings lay 0.66 to 4.8 m from the 2024 save's, measured centre to centre in plan, and 0.31 to 0.92 m off the 2024 opening's wall faces. The comparison takes each opening's world box from the placements and extrusions of the two files.

## 5. Limits

- A door's opening on 2015 to 2023 is its record box, frame and trim included, where the 2024 path cuts the outline the door's symbol cuts (RE-215, 2024 to 2027): door 154488's is 1.067 by 2.21 m on 2023 and 0.915 by 2.134 m on 2024, with the same centre and faces. This change does not touch it; RE-450 does.
- The rule rebinds only a door or window the reference list set in a wall. One whose list names no wall, or only a curtain wall, stays unhosted, as on 2024.
- No Revit export of a 2015 to 2023 save is local. The witness is the same project's 2024 save, as for RE-364, RE-366 and RE-447.
