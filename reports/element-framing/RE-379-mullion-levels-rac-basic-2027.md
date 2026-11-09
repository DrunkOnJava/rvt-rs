# RE-379: Revit 2027's `rac_basic` names another Level for 21 of its 144 mullions; their host walls, boxes and storeys are the 2025 save's

**Date:** 2026-11-09
**Issues:** #1493 (from #1454, RE-346); follows RE-27 (a record's counted reference list names the Level that hosts it)
**Artefacts:** Autodesk's `rac_basic_sample_project` and `rac_advanced_sample_project` saves of Revit 2025, 2026 and 2027 (`research/autodesk-sample-projects.tsv`, fetched by Measure); no Revit export of any of them
**Probe:** `probes/examples/probe_1493_mullion_level_refs.rs`
**Status:** both differences explained, no export changes. The first is a difference between the files; the second does not reproduce.

## 1. The question

#1493 listed two differences between the 2027 and the 2025 save of the same Autodesk project that the same-project check does not score:

- 21 of `rac_basic`'s 144 curtain mullions have another `LevelElementId` on the 2027 save (123 are equal);
- one curtain wall of `rac_advanced` has a `Qto_CurtainWallQuantities.Length` on the 2025 save and none on the 2027 save.

Its done-when asks that each be explained from the bytes.

## 2. The 21 mullions

`LevelElementId` is the Level that the first counted reference list of a mullion's partition record names, when it names exactly one (RE-27); rvt-rs writes it, with `LevelBindSource`, in the element's `RvtElementRecordGeometry` set. The probe prints the file's Levels and, for each of the 144 mullion records and each wall record, the Levels its list names and its box.

On the 2025 and 2026 saves of `rac_basic` the 21 records name Level 1 (311, 0 ft). On the 2027 save they name `Level 1 Living Rm.` (515270, −1.8045 ft) on 11 and `Level 2` (245423, 9.8425 ft) on 10. Everything else the saves hold of them is equal:

- the 6 Levels are the same in the three saves, with the same ElementIds and elevations, and the 2025 and 2026 saves' 144 mullion lists are identical;
- the 21 records' boxes are equal in the 2025 and 2027 saves to 1e-6 ft, and so are the 56 wall records'; the rest of each of the 21 lists, and its length, is equal (the lists are in ascending order, so the Level's slot moves to where its new id sorts);
- the 21 mullions belong to three curtain walls (704275 with 11, 708088 with 4, 709245 with 6), and none of the 56 wall records of the project names another Level on the 2025 save than on the 2026 or the 2027 one. Those walls name `Level 1 Living Rm.` and `Ceiling` (704275, whose box starts at −1.8045 ft), and `Level 2` and `Roof Line` (the other two, whose boxes start at 9.8425 ft), on all three saves.

So on the 2025 and 2026 saves these mullions name Level 1, at 0 ft, where their hosts' base Levels are at −1.8045 ft and 9.8425 ft (the 21 boxes start at five heights: −1.8045 for 6, 5.0853 for 1, 6.9226 for 4, 9.8425 for 7 and 18.5696 ft for 3, the vertical mullions at their host's base and the horizontal ones between); the 2027 save names their host's. That is a difference between the files, not between the readers: the same code reads the three. It does not look like a change in what the 2027 format stores: on `rac_advanced`, whose 3,276 mullion records are on both the 2025 and the 2027 save, no list names another Level (79 lists differ, each one reference longer on the 2027 save, 74 from 7 to 8 references and 5 from 6 to 7, and in a slot other than a Level).

**What rvt-rs writes.** A mullion is contained in the storey of its curtain wall, which it is a part of, so the exports of the 2025 and the 2027 save contain all 144 members in the same storeys: the 21 are in `Level 1 Living Rm.` (11) and `Level 2` (10) on both. The one Level or storey value of a member that differs between the exports is the marker, and it says what each file's record names: `LevelElementId` 311 on the 2025 and 2026 exports and 515270 or 245423 on the 2027 one, with `LevelBindSource` `partition_element_record_reference_list` on all. On the 2025 export it names a Level other than the one the mullion is contained in, which is what the record says. Two other properties of the members differ between the 2025 and the 2027 export, on all 144 and for reasons that have nothing to do with a Level: `Pset_MemberCommon.IsExternal` is written on the 2025 export (true) and on neither the 2026 nor the 2027 one, which 0.16.0 writes without the `IsExternal` of members, and `RvtElementRecordGeometry.SourceStream` names the `Partitions` stream each record is in (14, 15 and 16).

## 3. The curtain wall's `Length`

It does not reproduce. On the exports of `main` at `b969296e` with the probe (Measure run 42614703262), all 34 curtain walls of `rac_advanced` have `Qto_CurtainWallQuantities.Length` on the 2025, the 2026 and the 2027 save, and every one of the 35 wall and curtain wall lengths of the 2025 save is on the 2027 save, equal. The run that saw the difference (42119142486, 2026-11-05) measured the branch `lane/1454-gates` at `95fe042f`, not `main`; which change closed it is not identified, and none was bisected.

## 4. Not shown

- **Which save Revit's own export would follow.** No Revit export of any of these saves exists, so whether Revit's exporter names the Level the record holds or the host's for a mullion's own properties is not measured. rvt-rs writes the record's, as the marker's name says, and the mullion's storey is the wall's.
- **Why the 2027 sample differs.** The three walls' mullions may have been regenerated when Autodesk saved the 2027 sample; nothing in the files says so.
- **Any other project's mullions.** `rac_advanced` has none that differs; the other samples hold no curtain mullions.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=7e1a94d96874fd55c27ccb620e6422e3d0d30985 -f base=none \
    -f samples=true -f sample_years="2025 2027" -f keep_ifc=true -f models="re1-architecture" -f scorers=none \
    -f probe=probe_1493_mullion_level_refs
```

Each sample's `probe.txt` holds the Levels, the mullion lines and the wall lines; with `keep_ifc` its `model.ifc` holds each member's container and `RvtElementRecordGeometry` set.
