# RE-229 — The lengths a door's or window's symbol stores, against Revit's sizes

**Date:** 2026-10-25
**Issues:** #810 (door and window sizes); follows RE-215 (cut loops), RE-218 (sizes from the record box) and RE-225 (cut solids)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), Core Interior (Revit 2024) and RE1 Architecture (Revit 2025), as Measure fetches them
**Runs:** Measure [40230195608](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40230195608) (`examples/probe_re229_symbol_lengths.rs` with `tools/re/symbol_lengths_vs_ifc.py`)
**Result:** negative for #810's last two doors: no length their symbol stores is Revit's width. Positive as a cross-check of the sources rvt-rs already uses.

## 1. Question

RE-218 sizes every door and window of the office from its cut loop or its record box except doors 1756089 and 1765267, which Revit's export gives as 5.8629 ft wide and 4.3570 ft high. Their record box is 5.9766 ft wide and their symbol's cut solid 3.1168 ft (RE-225). revit-ifc takes the width from the closed `ExporterIFCUtils.GetMinSymbolWidth` (CLEANROOM.md source 5), so the question was whether the symbol stores the width among its parameters, as RE-221 found a hollow section's sizes among its type's.

The probe prints, for every door and window the export writes, each length its symbol's type stores (`partition_length_entries`), named by the family parameter's caption or left as a BuiltInParameter id; the scorer counts, per parameter, how often its value is Revit's `OverallWidth` or `OverallHeight` within 0.001 ft.

## 2. Measured

| file | doors and windows | parameters that are Revit's width | Revit's height | no stored length is the width |
|---|---:|---|---|---:|
| office as built | 538 | `Opening_Width` on 269 of 269 windows and 163 of 163 doors, `Aukkoleveys` on 24 of 24 | `Opening_Height` 163 of 163, `Aukkokorkeus` 24 of 24, -1001300 191 of 203 | 69 |
| office at permit | 534 | `Opening_Width` on 265 of 265 windows and 151 of 151 doors, `Aukkoleveys` on 24 of 24 | `Opening_Height` 160 of 160, `Aukkokorkeus` 24 of 24, -1001300 180 of 192 | 81 |
| Core Interior | 138 | -1001301 on 132 of 132 doors and 6 of 6 windows, -1001305 on 6 of 6 windows | -1001300 on 132 of 132 doors and 6 of 6 windows, -1001304 6 of 6 | 0 |
| RE1 Architecture | 5 | -1001301 on 4 of 4, `Width` on 1 of 1 | -1001300 on 4 of 4 | 0 |

The office's families name their own width parameters (`Opening_Width`, `Aukkoleveys`, `Window_Width`, `Door_Leaf`), so which stored length is the width depends on the family that defines it, where Autodesk's families on Core Interior and RE1 use the built-in -1001301 and -1001300.

Doors 1756089 and 1765267 store 4.2717 (-1001300), 3.1168 (-1001301), 3.9370 (-1001304), 2.2966 (-1001305), `Window_Width` 2.3032, `Window_Height` 3.9436, `Door_Leaf` 2.8937, `Door_Leaf_Height` 4.3570, `Door_Leaf_Bottom` -0.2067 and `Door_Leaf_Top` 4.1503 ft. `Door_Leaf_Height` is Revit's height; nothing is 5.8629 ft, nor is any sum or difference of two of them.

## 3. Conclusion

A symbol's stored lengths add no source: where they give the width, the cut loop or the record box already does (RE-215, RE-218), and a parameter's meaning depends on its family's own name for it, which rvt-rs does not guess. #810's two doors stay open: their width is in none of the record box, the cut solid and the stored lengths.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref docs/re229-symbol-lengths -f ref=docs/re229-symbol-lengths -f base=none \
  -f nordic=true -f r2025=true -f probe=probe_re229_symbol_lengths -f probe_scorer=symbol_lengths_vs_ifc
```
