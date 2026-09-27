# RE-77 — Type text parameters: Type Mark, Description, Fire Rating

**Date:** 2026-09-27
**Issues:** #35, #155
**Credit:** jakobhirn-bit (Discussion #112: value blocks open `[owner u64][56 × 0xff][3 × 0x00]`, string sections keyed by BuiltInParameter), STE1200 (same thread: the parameter store keyed by BuiltInParameter).

**Result:** positive for three text parameters of element types; instance parameters and numbers not read.

- An element's parameter values sit in value blocks that open with its ElementId, 56 bytes of `0xff` and 3 zero bytes. Within a type's block, some text parameters are `i64 BuiltInParameter · u32 n · UTF-16 × n`. An entry belongs to the block it follows, up to the next; the block's ElementId must be declared in `Global/ElemTable`.
- The partition record chain (RE-35) cannot attribute them: it covers the leading records of each partition, 4% of Snowdon Towers' inflated bytes, and 113 of the 531 Type Mark entries. The rest are in the loaded families' documents after it.
- Which BuiltInParameter is which was established against the model's own exports, not from any list: matching each entry's value with the parameters of the same ElementId in the VIM export of Snowdon Towers' 2027 edition (`vimaec/vim-hackathon`, local only) named `-1001405` Type Mark, `-1010103` Description and `-1001206` Fire Rating. Other ids in this shape carried values that matched no parameter of their owner, and are not read.
- Each element gets its type's values through the type RE-38 or #322 already resolves, as Revit shows a type's parameters on its instances. A type whose blocks hold two different values for one parameter gets neither.

## Measured

`tools/re/type_parameters_vs_vim.py` (per instance, against the same parameter of the instance's family type in the VIM) and `tools/re/type_parameters_vs_ifc.py` (per Tag, against the `FireRating` Revit's IFC export writes in its `Pset_*Common` sets):

| file | parameter | rvt-rs values | checked | equal | note |
|---|---|---:|---:|---:|---|
| Snowdon Towers (2024) | Type Mark | 1,284 | 1,276 (VIM) | 1,257 | all 19 others: the 2027 VIM holds an empty value |
| | Description | 70 | 70 (VIM) | 70 | |
| | Fire Rating | 696 | 696 (IFC4 export) | 696 | Revit's export rates 166 more elements, not read |
| | | | 696 (VIM) | 695 | the other: VIM empty |
| RE1 Architecture (2025) | Fire Rating | 7 | 7 (IFC export) | 7 | |
| Core Interior (2024) | Type Mark 138, Fire Rating 227 | | none | | its reference exports carry neither |

No value rvt-rs writes disagrees with a value either oracle holds.

## Not claimed

- Instance parameters (Mark, Comments, dimensions) are not stored in this shape and are not read.
- Numbers: a key-first `i64 · f64` scan found matching values for only a handful of ids (Railing Height 17 of 19 on Snowdon), which is not a layout. Not read.
- Recall: types whose parameter sits outside a value block, or in more than one with different values, get nothing. Fire Rating reaches 696 of Snowdon's 862 rated elements.
