# RE-365: Revit 2017 to 2023 steel beams as their type's I section

**Date:** 2026-11-06
**Issues:** #1190 (geometry beyond the box on 2015 to 2023); follows RE-103 (a steel beam's type stores its I section), RE-113 (the 32-bit value block), RE-362 (2015 to 2023 beams along their lines), #1514.
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored. BIMcollab's Revit 2015 example, fetched with `tools/ci/fetch-bimcollab.sh`, counts only.
**Scorer:** `tools/re/save_pair_bodies.py`; `probes/examples/probe_re365_i_sections_32.rs`.
**Status:** positive against the same samples saved by a later release. **No beam of a 2017 to 2023 file is scored here against a Revit export**, and a later save cannot catch an error it shares. This builds on RE-362: an I section is drawn along the line that reads.

## 1. Before

RE-362 left 11 of `rst_basic`'s 323 beams on 2019 to 2023 with the 2024 save's box and another volume, because `type_i_sections_at` stopped at 2024. `rst_basic` 2023 drew no `IfcIShapeProfileDef`, and its 2024 save draws 18.

## 2. The 32-bit entry

The value is there, under the same BuiltInParameter (`-1005502` is `42 a8 f0 ff`), narrower:

```
2024  f64 · ff×8 · i64 parameter, owner u64 before a 56×ff 3×00 mark (RE-103)
2023  f64 · ff×4 · i32 parameter, owner u32 before a 28×ff 3×00 mark (RE-113)
```

By `probe_re365_i_sections_32`, each type's width, depth, web and flange against its 2024 save, to the probe's six decimals: `rst_basic` 22 of 22 on 2019 to 2023, and 15 of 15 on the types 2017 and 2018 share with it, of their 22.

## 3. The change

`length_places_within` reads either layout (the 64-bit one keeps #1489's record-bounded value blocks, the 32-bit one ends blocks at marks alone, as measured here), and `type_i_sections_at` takes the narrow one on `I_SECTION_32BIT_REVIT_VERSIONS` (2017 to 2023). The 32-bit branch calls `attach_beam_sections`, for `StructuralFraming` only: a column's I section is placed from its transform, which is not measured on these releases. The rule that decides whether a beam's body holds its section (RE-103) is unchanged.

## 4. Bodies against a later save

Beams drawn along their lines, paired by `Tag`, to 3 mm on the world box and 0.1% on the volume, this change against RE-362:

| save | reference | RE-362 | this change |
|---|---|---:|---:|
| `rst_basic` 2019 to 2023 (each) | 2024 | 312 of 323 | 323 of 323 |
| `rst_basic` 2017, 2018 (each) | 2024 | 298 of 323 | 309 of 323 |
| `rst_basic` 2016 | 2024 | 285 of 323 | 285 of 323 |
| `rst_advanced` 2019 | 2027, not pinned | 48 of 54 | 54 of 54 |
| `rac_advanced` 2015 to 2023 (each) | 2024 | 33 of 33 | 33 of 33 |

The 14 left on `rst_basic` 2017 and 2018, and the 38 on 2016, are RE-362's: the saves' raw record boxes differ. `rst_advanced` 2019's reference is a local copy of Autodesk's 2027 sample, which no pin holds.

## 5. What else changes

- **Across all 38 sample saves of 2015 to 2024,** 83 beams change, on `rst_basic` 2017 to 2023 (11 each) and `rst_advanced` 2019 (6). Each goes from `IfcRectangleProfileDef` to `IfcIShapeProfileDef`, and gains `SectionShape`, `SectionWebThickness` and `SectionFlangeThickness`. No other property, storey, GlobalId or element changes.
- **Column cuts stay.** `rst_basic` 2023 draws 11 I sections where 2024 draws 18: RE-122 keeps the other 7 as cut boxes.
- **Byte identity.** [Measure run 42521217353](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42521217353) hashes the IFC of 118 models and samples, this change against RE-362's head: 110 are identical, and the 8 that differ are Autodesk's `rst_basic` of 2017 to 2023 and `rst_advanced` of 2019, the 8 files that hold the 83 beams above.
- **Cost.** About 0.1 s on a 1.4 s export (`rst_basic` 2023) and on a 1.9 s export (`rst_advanced` 2019), three runs each: the value blocks' scan.

## 6. Not measured, not read

- Against a Revit export: see the status above.
- **Revit 2015 and 2016.** `-1005502` has no hit on `rst_basic` 2015 or 2016, or `rst_advanced` 2015. Their sections' values sit in the same `f64 · ff×4` form under positive ids of the document's own (`0x0fdf4c` on `rst_basic` 2016, `0x04a1` on `rst_advanced` 2015), so reading them would first need what each id means, and no built-in id says. `rst_advanced` 2015 therefore stays at 48 of 54 against its 2027 copy.
- **BIMcollab's Revit 2015 example** (#1514). `-1005502` has no hit there either, so the 17 of its 91 beams whose volume differs from Revit's export do not change. They are not drawn along a line in any case: none of the 91 records holds one.
- A column's I section on 2015 to 2023, a channel, tee or angle on any release.

## 7. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2015 2024
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
S=partition_beam_axis,partition_beam_box_along_axis
python3 tools/re/save_pair_bodies.py samples/2023-rstbasicsampleproject.ifc samples/2024-rstbasicsampleproject.ifc --sources $S
cargo run --profile ci -p rvt-probes --example probe_re365_i_sections_32 -- samples/2023-rstbasicsampleproject.rvt 2023 > 2023.txt
cargo run --profile ci -p rvt-probes --example probe_re365_i_sections_32 -- samples/2024-rstbasicsampleproject.rvt 2024 > 2024.txt
diff 2023.txt 2024.txt
```
