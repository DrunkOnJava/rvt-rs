# RE-463: Revit writes a multistory stair once per storey, and what the file says about it (#1977)

**Date:** 2026-11-14
**Issues:** #1977; follows RE-65 and RE-68, which measure names and storeys against any copy of a multistory stair and say rvt-rs writes each stair once
**Artefacts:** Autodesk's Snowdon Towers 2024 architectural sample (local only) with Revit's own IFC4 export of it (`bldrs-ai/test-models`, no licence, never committed); Autodesk's `rac_advanced` samples of 2023 and 2024.
**Probe:** `probes/examples/probe_1977_multistory_stairs.rs`; **scorer:** `tools/re/multistory_copies_vs_ifc.py`
**Runs:** Measure probe runs 43551707746 (`b9df5839`), 43552665332 (`c97b02cb`) and 43553309916 (`5b7eaee9`); for the change (#1995), Measure runs 43568153235 (Snowdon Towers and the other 2024 samples, exports kept), 43568156861 (timed) and 43568180058 (unfiltered, scored) at `48cbcd04`, and 43565624655 (main's export of Snowdon Towers architectural, kept); the comparison with Revit's export is local, since the Snowdon export is.
**Status:** positive for what Revit writes, measured on the one file that has a Revit export of a multistory stair; rvt-rs writes the copies from #1995 on Revit 2024, sections 5 and 6.

## 1. The question

RE-65 and RE-68 found that Revit's export writes a multistory stair once per storey under one `Tag`, and names the later copies with a `:2` suffix, and they scored names and storeys against any copy. Nothing wrote the copies, and no issue tracked them. #1977 asks what a copy is, how many there are, and where the file says so.

## 2. What Revit writes

On Snowdon Towers architectural three assembled stairs (621690, 701090 and 882250) are multistory, and Revit's export holds each twice: the stair on L3 and `Assembled Stair:Stair:<id>:2` on L4, with its own parts. Counted by `tools/re/multistory_copies_vs_ifc.py` against the export of run 43546540315 at `c802574c`:

| Class | In Revit's export | In rvt-rs's | Revit's beyond rvt-rs's |
|---|---:|---:|---:|
| `IfcStair` | 6 | 3 | 3 |
| `IfcStairFlight` | 12 | 6 | 6 |
| `IfcMember` (stringers) | 60 | 30 | 30 |
| `IfcSlab` (landings) | 6 | 3 | 3 |
| `IfcRailing` | 22 | 11 | 11 |
| Total | 106 | 53 | 53 |

- **The copy is a vertical translation of one storey.** On the 30 stringer pairs the copy has the original's extents and volume and its centre exactly 0.000, 0.000 and +4.064 m from it, 30 of 30. The storeys read L3 at 10.897 m and L4 at 14.961 m above the lowest storey, 4.064 m apart (L5 is 4.064 m above L4, and L2 is 3.277 m below L3).
- **Railings are written twice under one name.** Each of the 11 railings of the three stairs appears once under the stair and once under its `:2` copy, with the same name and `Tag`.
- **The copy's `GlobalId` is its own.** All 16 bytes differ from the original's on the 30 stringer pairs. A search of 10,068 MD5 keys per pair, built from the shapes of revit-ifc's `GUIDUtil` (CLEANROOM.md source 5; only the keys' wording is used: the element's IFC GUID with `Sub-element:` and parts for index, entity and predefined type, level, copy and assembly, and the `type:name:guid` and project-and-key forms), over the 66 pairs of stairs, flights, stringers, railings and slabs with two entities found none that reproduces a copy's `GlobalId`. So the rule for it is unknown, and a copy rvt-rs writes would carry a `GlobalId` of rvt-rs's own.

## 3. What the file says

The probe (`probe_1977_multistory_stairs`, Measure runs 43551707746 and 43553309916) reads the `MultistoryStairs` records of each file. Snowdon Towers architectural holds three (621689, 701089 and 882249: one below each exported stair's Tag, 621690, 701090 and 882250); the `rac_advanced` saves of 2023 and 2024 hold five; no other sample and no BIM4LCA model holds one.

- **A record names a standard stair and a reference Level, and lists its sets.** `m_standardStairsId` is a `StairsElement` (620883, 700016 and 882136), `m_refLevelId` its base Level (Parking, L2 and L2), `m_typeId` one stair type. The record's `ElementIdIndexPairSet` lists `(index, ElementId)` pairs, and the members fall in sets of 14 indexes: one stair, one landing, 2 runs and 10 supports (stringers), the parts Revit writes. `MultistoryStairsGroupHelper.m_nextIndex` is the next index (84 and 42).
- **Each set is a stair of its own, between two Levels.** The `StairsElement` of each set names its base and top Level, and each is an ordinary element rvt-rs already writes once:

| Group | Sets, with the Levels each stair spans | Span no set covers |
|---|---|---|
| 621689 | Parking to L1 - Block 35 (the standard stair), L3 to L4 (621690), L1 - Block 35 to L2 (622037), L2 to L3 (622384), L5 to R2 (1616455); a sixth set holds 13 parts and no stair | L4 to L5 |
| 701089 | L2 to L3 (the standard stair), L3 to L4 (701090), L5 to R2 (1644692) | L4 to L5 |
| 882249 | L2 to L3 (the standard stair), L3 to L4 (882250), L5 to R2 (1620391; its set lists the stair alone) | L4 to L5 |

- **The span no set covers is where Revit writes the `:2` copy.** In each group the Levels the stairs span are consecutive except L4 to L5, 4.064 m, the height of the L3 to L4 stair (24 risers, 13.333 ft). Revit writes that stair a second time on L4, 4.064 m above, with its parts: one copy for one uncovered span, 3 of 3. No stair of the other spans is written twice in either export (the stairs of the other sets are 1 entity in Revit's export and 1 in rvt-rs's).
- **Nothing else says it.** `m_multistoryUpToLevelId` is -1 on every stair, the `MultistoryStairsDataCell` the schema declares (`m_instancesUpToLevelId`, `m_levelIdIndexPairSet`) is not among the objects of the three records, and `m_excludedBaseLevelIds` is empty. The span is inferred from the sets' Levels, not read.
- **`rac_advanced` holds five groups of one set each** (the standard stair, `01 - Entry Level` to `02 - Floor`, 12.467 ft), so by the rule above none has an uncovered span; no Revit export of those saves is read here, so whether Revit repeats them up the building is not known.

## 4. What is not known

- The rule for a copy's `GlobalId`: no revit-ifc key shape tried reproduces it.
- Whether the uncovered-span rule is Revit's rule or a coincidence of three groups built alike: all three have the one gap, L4 to L5, and the same adjacent stair. A group with a longer gap, a gap of another height or no stair of the same height below it is not in any file measured.
- Whether `rac_advanced`'s five single-set groups are repeated up the building: its Revit exports (`-f racadv=true`) can say, and the scorer reads any pair of exports.
- Where the file keeps the range a stair is repeated over, if it keeps one: the objects of the records hold no up-to Level.
- Whether the same holds on other releases: Snowdon Towers 2024 is the only file measured against a Revit export.

## 5. Rows for the change

Registered on #1977 (comment of 2026-11-14) and in PR #1995 before any run:

1. On Snowdon Towers architectural `tools/re/multistory_copies_vs_ifc.py` reads Revit's beyond rvt-rs's by class: 3 `IfcStair`, 6 `IfcStairFlight`, 30 `IfcMember`, 3 `IfcSlab`, 11 `IfcRailing` (53) before, and 0 after.
2. Of the 3 copies each name, storey, flights' `PredefinedType`, absence of a type object and shift (within 1 mm) equal Revit's: 3 of 3.
3. The originals are as `main` writes them: the 3 stairs' entities keep their names, `Tag`s, `GlobalId`s and type objects.
4. Every other export of the 146 models is byte-identical to `main`'s, and so is every diagnostics file: no other model holds a group with an uncovered span (`rac_advanced`'s groups hold one set each), and the other four Snowdon files hold no multistory stair.
5. Time and peak memory of Snowdon architectural within 3% of `main`'s.

The change: per group, each span between consecutive Levels of the sets' stairs that no set covers, when it is as high as the stair that ends where it begins (0.01 ft), gets a copy of that stair with its parts (landing, flights, stringers, railings), shifted up by the Level difference, in the storey of the span. The stair and each part but a railing are named `<name>:2`, the flights are `NOTDEFINED`, and the copy's `GlobalId`s are rvt-rs's own, derived from the original's by the rule for further pieces of one element (#1840); a copy's record set carries `MultistoryCopyOf` with the original's ElementId. A span the rule does not reach, a second span above one stair, a Level that is no storey and a part that cannot be moved are counted in the export notes and written as before.

## 6. Result

Runs of the final code (`48cbcd04`): 43568153235, 43568156861 and 43568180058; the scoring against Revit's export is local.

1. **Holds.** Revit's beyond rvt-rs's: 53 before (3 `IfcStair`, 6 `IfcStairFlight`, 30 `IfcMember`, 3 `IfcSlab`, 11 `IfcRailing`), 0 after in every class; rvt-rs's beyond Revit's: 0 in every class.
2. **Holds, with the row's wording corrected.** Of the 53 entities of the 3 copies, 53 of 53 equal Revit's in name, storey and type object; 42 of 53 in `PredefinedType` (the stairs, 6 flights, 30 stringers and 3 landings; the 11 railings carry none where Revit writes `NOTDEFINED`, as their 62 originals do, #1713); the shift of the 50 that have a body of their own (6 flights, 30 stringers, 3 landings, 11 railings) is within 1 mm of Revit's, 50 of 50. The row said a copy has no type object; the first run showed that Revit's stringer copies keep theirs (30 of 30), and the stair, its flights, its landing and its railings have none (the 62 railings of the originals have none either). The rule was narrowed to that before the final run.
3. **Holds.** Main's export of Snowdon Towers architectural (run 43565624655, 6,953 products) is contained in the change's: every product has the same class, name, `Tag`, `ObjectType`, `PredefinedType`, containing storey, type object, aggregate whole and property sets. The change adds 53 products and no other (30 `IfcMember`, 11 `IfcRailing`, 3 `IfcSlab`, 3 `IfcStair`, 6 `IfcStairFlight`), all named `:2` but the railings; the 97,401 `GlobalId`s of the export are distinct.
4. **Holds.** Run 43568180058 (unfiltered, with nordic, r2025, samples and holdout, against main's merge-base `92326d1a`): 119 of 120 exports and 101 of 102 `diagnostics.json` files are byte-identical, and the one that differs in each is Snowdon Towers architectural. The scorers' lines move only in that file's `schema.txt`, by the copies: 53 more objects (6,262 to 6,315), 79 more property sets, 30 more `IfcMember` and 3 more `IfcSlab` with a `PredefinedType`, and 9 more swept solids with a position of their own. The other four Snowdon files and `rac_advanced` (five groups of one set each) are byte-identical. The row said 146 models; the run compares 120 exports.
5. **Holds at the final code; the first version failed it.** Run 43568156861: Snowdon Towers architectural 29,318 to 29,838 ms (1.02), peak 797 to 797 MiB (1.00); `rac_advanced` 2024 6,125 to 6,253 ms (1.02), 310 to 306 MiB (0.99); no file is more than 3% slower. The one peak flag, Snowdon HVAC (1.06, 237 to 251 or 258 MiB), is a file with no stair, so the pass reads nothing in it and its export is byte-identical; its peak is two-valued from run to run (the first version's timed run read 0.94 on it).

**What the first version cost.** It read the groups after the saved-graphics pass, which lets the handle's inflated partitions go, in two reads that kept a record index. Measure run 43566139741 timed it: Snowdon Towers architectural 22,504 to 23,542 ms (1.05) and 797 to 1,205 MiB (1.51), `rac_advanced` 2024 4,691 to 4,911 ms (1.05) and 307 to 417 MiB (1.35), every other file within 2%. The read inflated the whole file again (the 408 MiB is Snowdon's 444 MB of partitions) and kept 48 bytes for each record. The final code reads the groups once, before the saved-graphics pass, with no record index kept (`Options::index_records`).

## 7. What is not claimed

- That the uncovered-span rule is Revit's: it reproduces 3 of 3 groups of one building, and the file does not state it. A group with a longer gap, a gap of another height or no stair of the same height below it is in no file measured; the pass counts such a span (`span_not_the_height_of_the_stair_below`) and writes nothing for it.
- Anything about other releases or other files than Snowdon Towers architectural: the pass runs on Revit 2024 only, and no other file has a Revit export of a multistory stair.
- That a copy's `GlobalId` follows any revit-ifc key shape: none tried reproduces it (section 2). A copy's is rvt-rs's own, and its record set says so.
- That the railings' `PredefinedType` is Revit's (`NOTDEFINED`): it is not on the originals either (#1713).
