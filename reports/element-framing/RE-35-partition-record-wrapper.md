# RE-35 — Element frames sit in ElementId-keyed partition records

**Date:** 2026-09-23
**Result:**
- **Positive:** every `Partitions/*` stream of a Revit 2024 or 2025 project opens with a chain of records, one per element, and each record's header carries the element's ElementId. A frame with its ElementId at `+0x00` (the first prologue) *is* the start of its record. A second-prologue frame (RE-30), with no id at `+0x00`, sits inside its element's record body. Its ElementId is therefore **read**, not inferred.
- **Measured negative for RE-34:** the reference-order rule gave a wrong id on a file outside its hold-outs. On `Projeto1` (Revit 2025) it named a door by its type's ElementId. Partitions are not in one ascending ElementId order. RE-35 replaces the inference, and the RE-34 code is removed.
- **Recall:** every entity rvt-rs recovers now reaches 100% of Revit's own export on the RE1 Architecture, Mechanical and Plumbing models. On Snowdon Towers it reaches 100% for doors, windows, curtain walls and panels, railings, ceilings, furniture and fixtures, and 98.5% for walls.

**Oracles:**

| file | Revit | reference | licence | sha256 (`.rvt`) |
|---|---|---|---|---|
| `2024_Core_Interior` (magnetar) | 2024 | `2024_Core_Interior_slim.ifc` | MIT | `c805df44…` |
| Snowdon Towers Sample Architectural (`bldrs-ai/test-models`) | 2024 | Revit 24.0.20.20 IFC4 export | none (local only) | `33271010…` |
| Snowdon Towers Sample Structural | 2024 | `vimaec/vim-hackathon` `Snowdon.r2027.vim` (commit `50f4373`), a later edition | none (local only) | `6dc833f9…` |
| `RE1-Architecture` / `-Mechanical` / `-Plumbing` / `-Electrical` (`Drshelden/IFC-ECS`) | 2025 | Revit 26.2 IFC2x3 exports | MIT | `4a78bdd6…` / `d5597201…` / `03689df2…` / `9875e765…` |
| `Projeto1`, `teste_export_2025` (`255ribeiro/intro_ifc`) | 2025 | Revit 25.4 IFC exports | none (local only) | `51cf7850…` / `73ee5cba…` |

**Probe:** `examples/probe_re35_record_wrapper.rs` walks each partition's record chain and reports where every framed element falls. `tests/partition_record_chain.rs` holds the first-prologue identity on every 2024 and 2025 file in the project corpus.

-----

## 1. How it was found

The Snowdon Structural sample was being checked against its VIM export. `Projeto1` was checked alongside it and exported a door with `Tag` 49480. Revit's export of the same file has that door as 325405, and it lists 49480 as the `IfcDoorType`. RE-34 had given the door its own type's id.

In that partition the wall and the door, both created in the file's last save, sit at the front, ahead of template elements with ids in the hundreds. The frame order is not ascending there. The one reference in the door's list below the next anchor was 49480, so both tie-break directions agreed on it.

The partition's first bytes are `48 c1 00 00 …`, which is 49480, and 0x2c bytes later the first frame starts. The same `u64` + size + constant shape precedes the wall (325345) and the door (325405). It is the record header that frames every element.

## 2. The record

```text
+0x00  u64  ElementId the record belongs to
+0x08  u32  record size, header included (the "record flags" 0xe1 / 0x111 / … of the first prologue)
+0x0c  u16  prologue constant: release constant + 28 (0x059f on 2024, 0x05c7 on 2025)
+0x0e  u16  count (recorded, not interpreted)
...         body
+size  u64  unread
       u32  flag word: 0 or 0x0100_0000
       u32  record size again
```

The next record starts right after the 16-byte trailer. The prologue constant is 12 below the constant the release's bbox marker ends with (RE-32). `partition_record_chain` walks from offset 0 and stops at the first position that is not a header whose trailer echoes its size.

What follows the chain are further runs of records whose ids are not declared in `Global/ElemTable`. They are loaded families' own documents, with their own ElementId space. Frames there are never project elements. On Core Interior, partition 46's chain ends at `0x558502` and the family runs start 72 MB later.

## 3. The chain holds every element

| file | release | declared ids | with a record in a chain | first-prologue frames | starting a record of their own id |
|---|---|---:|---:|---:|---:|
| Core Interior | 2024 | 26,425 | **26,425** | 23,470 | **23,470** |
| Snowdon Architectural | 2024 | 47,233 | 47,148 | 2,075 | **2,075** |
| Snowdon Structural | 2024 | 16,571 | 16,551 | 527 | **527** |
| RE1 Architecture | 2025 | 4,668 | 4,667 | 152 | **152** |
| RE1 Mechanical | 2025 | 4,743 | **4,743** | 50 | **50** |
| RE1 Plumbing | 2025 | 4,827 | **4,827** | 136 | **136** |
| RE1 Electrical | 2025 | 10,266 | **10,266** | 11 | **11** |
| Projeto1 | 2025 | 5,512 | 5,511 | 3 | **3** |
| teste_export_2025 | 2025 | 5,590 | 5,589 | 8 | **8** |

Every one of the 30,432 first-prologue frames starts a record with the same id. No frame of a project element lies outside the chain, and no family-document record yields a declared id.

## 4. Second-prologue ElementIds

`assign_second_prologue_ids` gives each placed-instance second-prologue frame the id of the chain record containing it, when that id is declared. Frames in a record whose id is not declared, and frames after the chain, get none (fail closed).

Exports before (main at `08e2740`, RE-34) and after, scored against Revit's own:

| file | exported before | in Revit's export | exported after | in Revit's export | lost |
|---|---:|---:|---:|---:|---|
| Snowdon Architectural | 4,132 | 4,036 | **4,795** | **4,678** | none |
| RE1 Architecture | 67 | 67 | **74** | **73** | none |
| RE1 Mechanical | 48 | 48 | **66** | **66** | none |
| RE1 Plumbing | 39 | 39 | **124** | **123** | none |
| Projeto1 | 1 | **0** (wrong id) | **2** | **2** | 49480, the wrong id |
| teste_export_2025 | 0 | 0 | **8** | **8** | none |
| Core Interior | byte-identical | | | | |

Recall against Revit's export, per entity Revit wrote:

- **RE1 Architecture:** walls 7/7, the curtain wall 1/1, doors 5/5, slabs 2/2, furniture and casework 23/23, fixtures 7/7, specialty equipment 9/9, ceilings 6/6, mullions 10/10, panels 2/2, railing 1/1.
- **RE1 Mechanical:** flow segments 31/31, flow fittings 35/35. The 7 air terminals are not a recovered category.
- **RE1 Plumbing:** flow segments 63/63, flow fittings 54/54, flow terminals 6/6.
- **Snowdon Architectural:** doors 132/132, windows 68/68, curtain walls 60/60, panels 462/462, railings 131/131, ceilings 68/68, furniture 345/345, fixtures 134/134, walls 1,062/1,078, slabs 176/200, columns 114/118.

Floors, building pads and ceilings, which RE-34 never assigned because their lists name their sketch one id earlier, now get their ids. Snowdon exports 176 slabs, and on the Structural sample all 26 slabs are in the VIM as `OST_Floors`.

Where RE-34 had assigned an id, the record agrees on 13,943 frames and differs on 5. In all 5 the record's id is the right one:
- the Projeto1 door;
- on Snowdon, a multistory stair, its stair and its run, each taken as the next id;
- a roof taken as its sketch's id.

Stairs and roofs are not exported, which is why the four Snowdon errors never surfaced.

`element_record_without_element_id` now counts only frames inside the chain. On Snowdon Architectural it is 20: 16 walls and 4 columns. That is exactly the 1,078 − 1,062 walls and 118 − 114 columns of Revit's export that rvt-rs does not attribute. It is absent on every RE1 model and on Core Interior. RE-41 attributes the 20: their records' ids are the ElemTable's second id.

## 5. What Revit's export leaves out

Every exported id Revit's export does not hold is a real element of the category exported:

- **Snowdon:** 9 doors, 38 windows and 47 curtain panels. The VIM holds each of them under the same category. These are the #309 omissions Revit's exporter makes by type.
- **Snowdon:** 15 slabs and 8 walls that neither Revit's IFC export nor the VIM holds. They are bleacher steps.
  - 9 of the slabs (2062310 to 2062429) and all 8 walls (2062316 to 2062435) carry design option 2059967 in the frame's third sentinel slot at `+0x2a`. The VIM's DesignOption table names 2059967 "Continuous Bleachers no Stairs" (not primary).
  - On the recovered categories, all 18 instances carrying option 2059970, "Middle Stair <primary>", are in Revit's export.
  - All 18 carrying a non-primary option (2059967, or 1500727 "Simple Dome") are not.
  - The option records (category −2006114) and option-set records (−2006112) list their members but carry no primary flag in these bytes, so rvt-rs does not filter on it yet.
  - RE-40 found the primary in each set's name entry, and the exporter now leaves the non-primary options out.
  - The other 6 slabs (2287679 to 2287714) carry no option; why Revit leaves them out is not established.
- **RE1 Architecture:** door 417199, a Single-Flush door of type 381264 in its own wall. Revit exported the file's other five doors.
- **RE1 Plumbing:** fixture 442378, a floor-level fixture of type 442362. Neither frame sets any sentinel slot; the cause is not established.

## 6. Why RE-34 worked where it was measured, and failed where it did not

Anchored frames show that partitions are several ascending runs, not one.

| file | anchors | descents |
|---|---:|---:|
| Core Interior (partition 46) | 15,545 | 8 |
| Snowdon Architectural | 2,033 | 2 |
| RE1 Architecture | 152 | 7 |
| RE1 Electrical | 11 | 2 |
| teste_export_2025 | 8 | 1 |

On RE1 Architecture each descent starts a run of lower last-modified revision (the ElemTable's 40-byte records carry two such counters at `+24` and `+28`). The runs do not follow any one counter exactly.

RE-34's hold-outs passed because the anchors and the agreement rule kept every run's picks consistent. On `Projeto1` the out-of-order run has no anchor, and the one candidate left below the next anchor was the door's type. The rule, the context threshold (`SECOND_PROLOGUE_CONTEXT_REFERENCES`), `SKETCH_OWNING_CATEGORIES` and `examples/probe_re34_holdout.rs` are removed. The RE-34 report stands as the record of what was measured.

## 7. What this does not claim

- The header's `u16` count, the trailer's leading `u64` and the flag word are recorded, not interpreted.
- The walk is proven on 2024 and 2025 only. 2023 (constant 1398 by the rule) and 2026 (1509) are not measured, and element records stay unsupported there.
- Design-option filtering (§5) is observed, not implemented: nothing in the bytes read so far marks an option primary.
- The `ElementIdSource` IFC property and the `element_id_from_reference_order` warning are gone. A second-prologue id is now as direct a read as a first-prologue one, and `PartitionElementRecord::id_from_enclosing_record` says which kind of frame it came from.
