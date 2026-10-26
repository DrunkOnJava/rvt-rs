# RE-243 — A curtain mullion's NetVolume

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-226 (`Span`), RE-241 (the mullion records)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40310235259](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40310235259) and [40314338847](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40314338847) (`tools/re/member_volumes_list.py` on `main`), [40312076536](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40312076536) and [40313160489](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40313160489) (`examples/probe_re241_mullion_record_scalars.rs` with `tools/re/record_scalars_vs_ifc.py`, all mullions and those on straight hosts), and [40315769414](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40315769414) (this change against `main`, with `timing`)
**Result:** positive: where Revit's export draws a mullion as an extrusion of its section, its `NetVolume` is the mullion's box across it times its `Span`; rvt-rs writes it on the mullions it can tell are drawn that way, none different.

## 1. Revit's NetVolume

revit-ifc's `NetVolumeCalculator` (CLEANROOM.md source 5) takes the volume of the solid it writes. Of the office's 10,003 curtain mullions as built, Revit's export writes 9,796 as one `IfcExtrudedAreaSolid` and 207 as a mesh (`IfcPolygonalFaceSet`); on 9,796 of the 10,003 its `NetVolume` is its own body's volume. The volume of rvt-rs's box of each mullion (its two sides across the mullion times its `Span`, RE-226) equals Revit's `NetVolume` on 9,791: every extrusion but 6 (whose box rvt-rs draws twice as long, and which have no `Span`), and 1 of the meshes.

## 2. Which mullions Revit draws as a mesh

`record_scalars_vs_ifc.py`, ranking every scalar of each mullion's native record, its type's and its host's against whether Revit's body is anything but extruded solids:

| host curtain wall | mullions | Revit's body an extrusion | NetVolume = box volume |
|---|---:|---:|---:|
| curved (its record an `ArcWall`) | 264 | 93 | 88 |
| straight, its record holding an `ExtrusionGStep` (an edited profile) | 1,944 | 1,908 | 1,908 |
| straight, no `ExtrusionGStep` | 7,795 | 7,795 | 7,795 |

Every mesh is on a curved curtain wall (its horizontal mullions follow the arc) or on one with an edited profile (the 36 meshes there are the mullions its sloped edges cut). No field of the mullion's own record or its type's tells those 36 apart from the 1,908 others on the same three curtain walls.

## 3. What rvt-rs does

`native_mullion_hosts::mullions_on_plain_hosts` reads, in one pass, each mullion's native record (`m_hostId`, `m_masterSymbolId`), every `SWall` and `ArcWall` record, and every mullion type (`SysMullionFamSym`), and keeps the mullions whose host is an `SWall` whose record holds neither an `ArcWall` nor an `ExtrusionGStep` and whose type sets no profile family (`m_profileId` unset, as on all four of the office's types); a copy that disagrees leaves the element out. On those, a mullion of the Rectangular Mullion or Quad Corner Mullion family, whose section is a full rectangle, carries `Qto_MemberBaseQuantities.NetVolume`: its box's two sides across it times its `Span`. A circular or corner mullion's section is not its box's, and is left out, as is every mullion on a curved or edited curtain wall: 1,996 of Revit's values that the box would give stay missing, so that none is written different.

## 4. Measured

This change against `main` (run 40315769414, `qtos_vs_ifc`, `psets_vs_ifc`, the schema step and `timing`):

| model | `NetVolume` equal | different | missing | quantities Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 7,795 of 10,003 | 0 | 2,208 | 0 |
| office at permit | 7,823 of 10,031 | 0 | 2,208 | 0 |

The office's quantities equal go from 11,993 to 19,788 of 40,577 as built (11,920 to 19,743 of 40,792 at permit); the property census, IfcOpenShell's validation (no finding) and every other count are as on `main`. Only the two office exports and RE1 Architecture's move: its 10 mullions carry a `NetVolume` too, which its export (no base quantities, RE-242) does not write; Revit's body of each of them is exactly its box (the W6 box census, `tools/re/box_bodies_vs_ifc.py`), so the box's volume is its solid's. The pass costs 2.2 s on each office file, 18.2 s to 20.3 s on one runner (12%), and nothing measurable on the other models.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers="qtos_vs_ifc member_volumes_list"
```
