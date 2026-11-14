# RE-466: a host type whose layer copies disagree takes its current copy on Revit 2020

Date: 2026-11-14. Status: positive, on one release. Issue: [#1805](https://github.com/DrunkOnJava/rvt-rs/issues/1805). Follows RE-446 (the current copy on Revit 2024 and later) and RE-464 (the category material read on Revit 2015 to 2023).

## 1. The question

RE-446 takes the copy of a host type's layers in its current partition (`native_document::current_partitions`, RE-145 and RE-216) where its copies disagree, on Revit 2024 and later only. On Revit 2008 to 2023 such a type stayed dropped, because on the one held-out file of those releases with such a type (`bimspeed-frigento-2020`, Revit 2020) the current copy gave 5 floors layers of Revit's widths and, on 4 of them, materials Revit's export does not give.

## 2. Why the materials differed

Measure [43557579235](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43557579235), [43558313581](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43558313581) and [43559093841](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43559093841), counts only: each of the 4 floors has one layer with no material of its own, which in Revit takes its category's material, and the category-material reader read none for Floors on that file. Its object-styles entry has no pattern: the mark 20 bytes past the category, where RE-115's entry has it at 30. RE-464 found that form on Autodesk's samples (every host category of the `rac_advanced`, `rme` and `rst` saves of 2015 to 2023) and the reader takes it since PR #1997. The held-out file shaped neither the reader nor this rule; it is read only as counts.

## 3. The held-out file now

Measure [43577510927](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43577510927), probe `probe_1805_host_type_layer_copies` at `ef1cc84e` (branch `research/1805-layer-copies`, main with #1997 merged), scorer `tools/re/layer_copies_vs_ifc.py`, held-out counts only:

- types whose copies disagree 2, both with a current partition; 5 floors of them, each with a layer set in Revit's export;
- widths equal to Revit's: first copy 4, latest 5, current 5; widths and materials equal: first 0, latest 5, current 5 (the probe compares the layers that have a width; see section 5 for the membranes);
- the 4 layers with no material of their own: category material Revit's 4, not Revit's 0, not read 0;
- the Floors object-styles entry: 2 entries of the form with no pattern, naming 1 material, set.

## 4. Where it rests

None of Autodesk's 34 sample projects of 2015 to 2023 has a host type whose copies disagree (Measure [43560071868](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43560071868): `copies_disagree 0` on all 34). So the held-out Revit 2020 file is the only file of 2008 to 2023 that exercises the rule, and only 2020 takes it (`CURRENT_COPY_32BIT_REVIT_VERSIONS`), as RE-353 takes a release only where a file of it measures the rule. The other releases of 2008 to 2023 keep dropping such a type.

## 5. What the export shows

Measure [43578558197](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43578558197) (`7952aec6`, unfiltered, with nordic, r2025, samples, holdout and racadv): the held-out 2020 file is the one export of 126 that moves, and 102 of 102 diagnostics are identical. On that file, counts only:

- layer sets written 12 to 17; whole sets within 0.001 ft of Revit's 12 of 12 to 17 of 17; layer names equal to Revit's, in order, 23 to 28;
- `Pset_SlabCommon.PitchAngle` equal 10 to 15 of 18, different 0; properties equal 497 to 502, different 0;
- elements with Revit's material set 98 to 99, and IfcSlab with a different set 2 to 6.

The six different sets (two on `main` already, four of these floors) are rvt-rs's materials inside Revit's: each has fewer layers than Revit's, every name rvt-rs writes is one of Revit's, and each is Revit's, names in order, once the layers Revit's export writes 0 thick are left out (runs [43582471166](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43582471166) and [43582862119](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43582862119), `tools/re/layer_sets_vs_ifc.py`, 6 of 6). Those are membranes, which rvt-rs leaves out because Revit's IFC4 export of Snowdon Towers does (RE-53): Revit's exports disagree about them, and rvt-rs keeps leaving them out. The held-out scrub had dropped the line `layer sets whose count differs from Revit's`, which is why the layer-set counts above looked whole.

## 6. Reproduce

```
python3 -I .claude/scripts/measure-dispatch.py --ref ef1cc84e8837c342cab1a2836707b9b3a6a2c378 --base none \
  --scorers-ref research/1805-layer-copies -f holdout=true -f models=holdout-bimspeed-frigento-2020 \
  -f scorers=none -f probe=probe_1805_host_type_layer_copies -f probe_scorer=layer_copies_vs_ifc
```
