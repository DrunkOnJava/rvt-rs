# RE-67 — Model text named as Revit names it

**Date:** 2026-09-24
**Result:**
- Model text is named as Revit's own IFC export names it, for example "Model Text:10" Trebuchet MS:1448731", with `ObjectType` "Model Text:10" Trebuchet MS", rather than "GenericModel-1448731".
- A model text element is in the Generic Models category but is not a family instance. Its reference list names a text type, which has no name entry (RE-38) and no type-definition record of the category, so neither RE-38 nor RE-44 found it.
- The text type is a type object (`01 00 00 00 · u64 id`, tag `db 0f`) that frames its own name with tag 0x0149 (as material names are framed, RE-58) and then a font with tag 0x07eb. The font is what marks it a text type (`partition_names::find_text_type_names`).
- Measured against Revit's export of Snowdon Towers, matched by `Tag`:

| | before | after |
|---|---:|---:|
| model texts with Revit's `Name` and `ObjectType` | 0 of 7 | 7 of 7 |

- No other element changes. On all 13 other local files the IFC is byte-identical apart from the timestamp. On Snowdon, a per-element comparison finds only these 7.
- Measured on Snowdon only (Revit 2024). No other local file has model text.

-----

## 1. What the probe shows

`examples/probe_re67_model_text_types.rs` lists every Generic Models record whose reference list names no id with a name entry, meaning it is not a family instance, together with the text types its list names:

| records | text type named | its name |
|---:|---|---|
| 6 | 1448732 | "10" Trebuchet MS" |
| 1 | 2239044 | "18" Trebuchet MS" |

Snowdon has 7 such records, and each names exactly one text type. Both types carry the font "Trebuchet MS", framed `ff ff ff ff eb 07 · u32 n · UTF-16` after their name.

## 2. The rule

`partition_schema_mvp::attach_model_text_types` gives a Generic Model with no type or family the one text type its list names. It then names the element `Model Text:<type>:<ElementId>`, with `FamilyNameSource` `system_family_by_type_kind`. An element naming no text type, or more than one, keeps its `GenericModel-ElementId` name. The rule runs on Revit 2024 only, where it is measured.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re67_model_text_types -- MODEL.rvt
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/names_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

The witness observations do not change.
