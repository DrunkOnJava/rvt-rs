# RE-463: Revit writes a multistory stair once per storey, and what the file says about it (#1977)

**Date:** 2026-11-14
**Issues:** #1977; follows RE-65 and RE-68, which measure names and storeys against any copy of a multistory stair and say rvt-rs writes each stair once
**Artefacts:** Autodesk's Snowdon Towers 2024 architectural sample (local only) with Revit's own IFC4 export of it (`bldrs-ai/test-models`, no licence, never committed); Autodesk's `rac_advanced` samples of 2023 and 2024.
**Probe:** `probes/examples/probe_1977_multistory_stairs.rs`; **scorer:** `tools/re/multistory_copies_vs_ifc.py`
**Runs:** Measure probe runs 43551707746 (`b9df5839`), 43552665332 (`c97b02cb`) and 43553309916 (`5b7eaee9`); the comparison with Revit's export is local, since the Snowdon export is.
**Status:** positive for what Revit writes, measured on the one file that has a Revit export of a multistory stair; open for how to write it, see section 5.

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

## 5. Rows for a change

Registered on #1977 before any change, and not run: no change is made here.

1. For each multistory group, each span between consecutive Levels that no set's stair covers, from the lowest base to the highest top of its sets, with the height of the stair of the span below it, gets a copy of that stair's set: the stair, its landing, runs, supports and railings, shifted up by the Level elevation difference. On Snowdon Towers architectural that is 3 copies of 14 elements each (a stair, a landing, 2 flights and 10 stringers) and 11 railings, the 53 entities of section 2, with `Name` `<stair name>:2` on the stair, flights, supports and landing and the same name on the railings, each copy's stair contained in the storey of its span's base Level (L4) and its parts aggregated under it.
2. `tools/re/multistory_copies_vs_ifc.py` reads, against Revit's export, 53 entities more in Revit before the change and 0 after, with the shift of every copy within 1 mm of Revit's.
3. Every other export of the 146 models is byte-identical to `main`'s: no other model holds a group with an uncovered span (`rac_advanced`'s groups hold one set each).
4. A copy's `GlobalId` is rvt-rs's own, derived the way `rvt-rs` derives the further pieces' (#1840), and the diagnostics say that these are not Revit's; or the rule is found first.

## 6. What is not claimed

- That rvt-rs writes any copy: it writes none, and the supported profile says so.
- That the uncovered-span rule is Revit's: it reproduces 3 of 3 groups of one building, and the file does not state it.
- Anything about other releases or other files than Snowdon Towers architectural.
- That a copy's `GlobalId` follows any revit-ifc key shape: none tried reproduces it (section 2).
