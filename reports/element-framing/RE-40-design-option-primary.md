# RE-40 — The primary option of a design option set

**Date:** 2026-09-23
**Result:**
- **Positive.** A design option set's name entry repeats the set's option list and closes with the ElementId of its primary option.
- With it, rvt-rs leaves out elements in non-primary options, as Revit's own IFC export does. On Snowdon Towers Architectural this removes 18 exported elements that Revit's export does not hold (9 slabs, 8 walls, one generic model), and none that it does hold.
- The same test adds slab edges (`OST_EdgeSlab`) as a category: all 59 exported are in Revit's export as `IfcBuildingElementProxy`.
- Closes #319.

**Oracle:** Snowdon Towers Sample Architectural (`bldrs-ai/test-models`, local only, `.rvt` sha256 `33271010…`) and Revit's IFC4 export of it (`ecfcb04e…`). The VIM export (`vimaec/vim-hackathon`, local only) names the options. None of the licensed files in CI has a design option, so the positive evidence is local. `tests/design_options.rs` pins that the licensed files read none.

**Probe:** `examples/probe_re40_design_options.rs` prints each set with its name, options and primary, then the placed instances of recovered categories per option.

-----

## 1. Where the primary is not

RE-35 §5 found that an element's frame holds its design option at `+0x2a`, and that Revit's export omits the elements of non-primary options. It left the primary flag unfound:
- The option records (`OST_DesignOptions`, −2006114) are byte-identical apart from their ids.
- The option-set records (`OST_DesignOptionSets`, −2006112) list their options in ascending id order. The primary is the smallest in one set (1500724) and the largest in the other (2059970).
- The three tables in `Global/Latest` that list the options map each option and its id + 1 to a phase (32440, 118390 and 2287613 are `OST_Phases` records). They carry no primary marker either.

## 2. The set's name entry

The option names are UTF-16 strings in `Partitions/68`. Each option has an entry that ends with its name and its set's ElementId. Each set has one that ends with its name and **the primary option's ElementId**:

```text
option-set partition record (in the RE-35 chain)
  +0x00  u64  the set's ElementId
  +0x12  i64  -2006112 (OST_DesignOptionSets)
  +0x56  u32  k
  +0x5a  k*8  the set's own ElementId, then its k - 1 options

option-set name entry
  u32  k - 1
  ...  the same options, u64 each, in the same order
  u32  1
  u32  n, the name's length in UTF-16 code units
  n*2  the set's name, UTF-16LE
  u64  the ElementId of the primary option
```

| set | name | options | last `u64` of the entry |
|---|---|---|---|
| 1500723 | Bandstand Options | 1500724, 1500727, 2220520, 2237100 | **1500724** |
| 2059966 | Bleacher Seating | 2059967, 2059970 | **2059970** |

The VIM export marks 1500724 ("Twisty Trellis") and 2059970 ("Middle Stair") as the primary options.

`partition_design_options::compute_design_options` reads the list from the set's chain record, searches every partition for the entry with that exact list, and takes the `u64` after the name. Each set's entry occurs once in the file. It fails closed: a set with no entry, or with entries that disagree about the primary, stays unresolved, and elements in its options are kept as before.

## 3. Against Revit's export

Placed instances of recovered categories, by design option (`probe_re40_design_options`):

| option | set | primary | instances | in Revit's IFC4 export |
|---|---|---|---:|---:|
| 2059970 | Bleacher Seating | yes | 29 (9 floors, 9 slab edges, 8 walls, a stair, its run, a railing) | **29** |
| 2059967 | Bleacher Seating | no | 26 (9 floors, 9 slab edges, 8 walls) | **0** |
| 1500727 | Bandstand Options | no | 1 generic model | **0** |
| 1500724 | Bandstand Options | yes | 33 (17 generic models, 16 walls) | 33, but not yet attributed by rvt-rs (§5) |

The exporter now drops records in a non-primary option before instance selection. On the Snowdon export:

| | before | after |
|---|---:|---:|
| exported elements not in Revit's export | 118 | **100** |
| slab edges exported (all in Revit's export) | 0 | **59** |
| `element_record_in_non_primary_design_option` | – | 27 (Floor 9, SlabEdge 9, Wall 8, GenericModel 1) |

The 18 removed are exactly the elements RE-35 §5 attributed to non-primary options. No element that Revit's export holds was removed. The remaining 100 are the #309 omissions (9 doors, 38 windows, 47 curtain panels) and the 6 slabs with no design option that RE-35 §5 left unexplained.

Slab edges were measured and left out in RE-37 §3 because 9 of them sat in option 2059967. Revit writes the other 59 as `IfcBuildingElementProxy`, and so does rvt-rs now.

The diagnostics report the left-out instances as the skipped item `element_record_in_non_primary_design_option`. They do not count toward `confidence.unexported_element_records`, since Revit's export leaves them out too.

## 4. Other files

No other file in the corpus has a design option set: Snowdon Structural, Core Interior, the four RE1 models, Einhoven, `teste_export_2025`, Projeto1, `Exemplo_data` and `modelo_bim`. On each, no set resolves, no placed instance of a recovered category carries a design option, and the export is unchanged.

## 5. The primary bandstand

The 33 elements of the primary option 1500724 are in Revit's export but not in rvt-rs's. They sit in chain records 2220523 to 2220557, which `Global/ElemTable` does not declare under the id it reads (`id_primary`, `+16` of the 40-byte record). That is a separate finding, taken up in RE-41. It is not a design-option effect: rvt-rs keeps every element of a primary option.

## 6. What this does not claim

- Only the name-entry shape in §2 is read. The `u32 1` before the name is recorded, not interpreted.
- Revit's IFC exporter can be told to export a specific option. rvt-rs follows the default: the main model plus each set's primary option.
- One file has design options. The shape is read the same way on Revit 2025, but no 2025 file with options has been measured.
