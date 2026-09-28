# RE-118 — A Level's Building Story setting, and Snowdon's structural storeys

**Date:** 2026-09-28
**Issues:** #219 (storeys), #408 (held-out models); follows RE-24, RE-51 and RE-107
**Artefacts:**
- Autodesk's Snowdon Towers Sample Structural and Architectural (Revit 2024, local only), with the VIM export of the sample's 2027 edition, whose "Building Story" parameter is the oracle;
- `2024_Core_Interior.rvt` (2024) and `RE1-Architecture.rvt` (2025), with Revit's IFC exports.

**Result:** a Level keeps Revit's Building Story setting in its data, and only a building story becomes a storey. Snowdon Towers' structural model, whose Levels rvt-rs refused as a storey set, now has Revit's 12 building stories as its storeys. Every other model's IFC is byte-identical:
- Core Interior, Snowdon Towers Architectural, Plumbing, HVAC and Electrical;
- RE1 Architecture, Projeto1 and teste_export_2025;
- the two 2023 projects.

| Snowdon Towers Structural | before | RE-118 | VIM export |
|---|---:|---:|---:|
| storeys | 0 (Levels refused) | 12 | 12 building stories |
| storey names and elevations as the VIM's | – | 12 of 12 (elevations within 1e-4 ft) | |
| elements in the storey of their VIM Level, where that Level is a building story | 0 of 210 | 208 of 210 | |

-----

## 1. The setting

Past a Level's name (RE-24's name block), the first `ff × 8 · 00 01` is followed by:

```text
f64   0 on most Levels (3.0 on two of Snowdon's architectural Levels)
u64   an ElementId
u8    1 on a building story, 0 on any other Level
```

That is `partition_level_records::building_story_after`. Against the VIM's "Building Story" parameter the byte is exact on every Level:

| model | Levels | building stories | byte |
|---|---:|---:|---|
| Snowdon Structural | 19 | 12 | 1 on the 12, 0 on the 7 others: Top of Footing and the six "TOS" Levels |
| Snowdon Architectural | 18 | 18 | 1 on all |
| Core Interior (2024) | 15 | 15 storeys in Revit's IFC | 1 on all |
| RE1 Architecture (2025) | 2 | 2 storeys in Revit's IFC | 1 on both |

The ElementId before it is 323 on Snowdon's structural Levels, 305 on RE1's and 2118242 on the architectural ones. It is not read.

**The pad byte.** The three bytes between a Level's name block's `ff` run and its name length were required to be zero. On 8 of the 19 structural Levels the first is 1: L1_43_High, Parking, L2, L3, L4, L5, R2 and Parapet 2, all building stories, while the other four stories and all seven non-stories carry 0. So it is not the Building Story setting, and it no longer refuses the Level.

## 2. What is written

**Storeys.** They are the Levels not flagged as non-stories (`ifc::apply_partition_level_storeys`). A Level whose setting is not found stays a storey, as before.

**Elements on a non-story Level.** Such a Level binds no element. RE-59's and RE-68's rules place the element if they can; otherwise it stays in the building. On the structural model that is 121 elements, footings and framing on Top of Footing and the TOS Levels, all in the building. The VIM gives their Level, not the storey Revit's IFC export would contain them in, and no IFC export of that model is at hand to say.

**Two stragglers.** Two steel columns the VIM puts on L3 stay in the building: 642990 and 643097. No Level rule binds them (`LevelBindResolved` false). Why is not decoded.

**Elements outside the scores.** Of the model's 1,282 exported elements, 951 have no Level in the VIM, so they appear in neither count. Overall, 626 elements are in a storey and 656 in the building.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Structural.rvt (local only) | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` |
| Snowdon.r2027.vim (local only) | the VIM export of Snowdon Towers' 2027 edition |

```bash
cargo run --profile ci --example probe_re118_building_story -- "Snowdon Towers Sample Structural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Structural.rvt" -o structural.ifc
python3 tools/re/storeys_vs_vim.py structural.ifc Snowdon.r2027.vim "Snowdon Towers Sample Structural"
```
