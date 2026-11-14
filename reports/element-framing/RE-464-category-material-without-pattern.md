# RE-464 — A category's object style with no pattern holds its material 10 bytes earlier

**Date:** 2026-11-14
**Issues:** #1805 (a host type's layer copies on Revit 2008 to 2023); follows RE-91, RE-115, RE-180, RE-181
**Artefacts:** Autodesk's sample projects of 2015 to 2023 (`-f samples=true`), and `rac_advanced`'s saves of 2015 to 2020 with Revit's own IFC exports (`racadv-*`).
**Result:** positive. Revit 2015 to 2023 write a category's object-styles entry in two forms.
- **With a pattern:** the RE-115 frame, which `rac_basic` holds.
- **With no pattern:** a null object where the `PatternHelper` stands. This is every host category of `rac_advanced`, `rme_advanced`, `rme_basic`, `rst_advanced` and `rst_basic` of those releases.

The reader now reads both. The second form's material is the one `rac_basic`'s entries hold for the same category ("Default Wall" 24, "Default Floor" 758).

## 1. The two forms

Probe `probe_1805_host_type_layer_copies` (branch `research/1805-layer-copies` at `1084de89`, Measure [43560071868](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43560071868)) prints, for Walls, Floors, Roofs and Ceilings, the first 44 bytes of each place the category's `i32` is followed by `ff ff ff ff` with `i32 -3000010` within 60 bytes. On the 2019 `rac_basic`, Floors:

```text
60 7b e1 ff  ff ff ff ff  01 00 00 00  01 00 00 00  ff ff ff ff ca 01 01 00 00 00 00 00 00 00  36 39 d2 ff  f6 02 00 00
category     unset        1 (copy 2: 2) 1            PatternHelper object and 8 bytes           mark         material 758
```

On the 2019 `rac_advanced`, Floors:

```text
60 7b e1 ff  ff ff ff ff  01 00 00 00  01 00 00 00  00 00 00 00  36 39 d2 ff  f6 02 00 00
category     unset        1 (copy 2: 2) k (1 to 5)   null object  mark         material 758
```

Each entry is held twice, the copies differing only in the 1-or-2 word and the `u32` after it, and both copies name the same material.

| Autodesk samples, 2015 to 2023 | files | patterned (mark at +30) | unpatterned (mark at +20) |
|---|---:|---|---|
| `rac_basic` | 9 | all 4 categories: Walls 24, Floors 758, Roofs 25, Ceilings unset | none |
| `rac_advanced`, `rme_advanced`, `rme_basic`, `rst_advanced`, `rst_basic` | 25 | none | all 4 categories |

Before this change, the reader found no material on any of the 25: none of their 100 category entries was read.

## 2. What the export does with it

A layer that names no material of its own takes its category's material (RE-91, RE-115). On the files with the second form, those layers had none.

## 3. Measured against Revit's exports

Measure [43563790221](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43563790221): `7ffbde22` against `28b5ed83` (v0.23.0), unfiltered, with nordic, r2025, samples, holdout and racadv.
- 22 of 126 exports move; all 102 `diagnostics.json` files are byte-identical.
- `same-project gates` passes.
- Only material and layer-set lines move. On each scored model that moves, every newly written material is Revit's:

| model | export | elements with Revit's material set (`element_materials_vs_ifc`) | layer names Revit's, in order | whole layer sets within 0.001 ft |
|---|---|---|---|---|
| `racadv-2015` to `racadv-2020` (each) | IFC2X3, three.js repository | 5,252 to 5,256 of 5,293 to 5,297 written (4 more, all equal; its 4 `IfcWallStandardCase` with no material: 0) | 667 to 671 | 215 of 215 to 219 of 219 |
| held-out Warmond (2021) | IFC4 | 423 to 462 of 423 to 462 written (39 more, all equal; its 38 `IfcWall` with no material: 0) | 1,062 to 1,101 | 235 of 236 to 274 of 275 |
| held-out Gdynia (2021) | IFC2X3 CV2.0 | 154 to 155 of 157 to 158 written (1 more, equal) | 217 to 218 | 107 of 108 to 108 of 109 |

The other 14 exports that move are Autodesk samples of 2015 to 2023 with no Revit export: `rac_advanced` of every year, `rme_advanced` and `rme_basic` of 2015, 2019 and 2023. Their layers that take their category's material now carry it, as their `rac_basic` siblings' already did.

The held-out Revit 2020 file of #1805 (Frigento) does not move. That file is read as counts only and never shapes a reader, so why its four floors still read no category material is left there (#1805).

## Reproduce

```bash
gh workflow run measure.yml --ref research/1805-layer-copies -f ref=1084de89e7354649b30d745ee3c9f7bf169b9d7f -f base=none -f samples=true -f models="sample-2015-* sample-2016-* sample-2017-* sample-2018-* sample-2019-* sample-2020-* sample-2021-* sample-2022-* sample-2023-*" -f scorers=none -f probe=probe_1805_host_type_layer_copies -f probe_scorer=-
```
