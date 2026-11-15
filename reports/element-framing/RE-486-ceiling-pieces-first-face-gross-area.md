# RE-486: A ceiling whose saved face is in pieces takes the area of the piece that holds the first triangle of its saved mesh's top face, the piece Revit's export names

**Date:** 2026-11-15
**Issue:** #1209 (W7: coverings' `GrossArea` on cut ceilings and ceilings in pieces); follows RE-480 (a one-piece saved face is Revit's area), RE-465 (a ceiling in pieces: the piece its first-drawn lines lie on), RE-297 (a cut ceiling keeps the area left after the cut).
**Artefacts:** the BIM4LCA office (concrete and timber, as built and at building permit, Revit 2024) with Revit's own IFC4 exports; the BIM4LCA housing, the Revit 2025 models, Core Interior, RE1, the Autodesk samples of Revit 2015 to 2027 and the held-out projects (counts only) for what else moves.
**Scorer and probe:** `tools/re/ceiling_piece_order_vs_ifc.py` at `0049be011b22` and `probe_1209_ceiling_mesh_order` with `tools/re/ceiling_mesh_order_vs_ifc.py` at `31bca2a89c12`, both archived under `refs/archive/2026-11-15/cited/`. Neither lands.
**Commits:** the rule at `a2833a069bd5` (the first version, `21eb0ea8a883`, wrote nothing: section 3); the diagnostic build `052bc6e9db00`, archived under `refs/archive/2026-11-15/cited/`.
**Status:** positive. On the four BIM4LCA office files, the 16 ceilings in pieces that had no `GrossArea` now have Revit's: 16 of 16 equal, and nothing else moves. One ceiling of Autodesk's 2024 Snowdon sample gains one too, and nothing else in the 120 exports moves (Measure [43733137320](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43733137320), section 4). All five registered rows are met (section 5).

## 1. Which piece Revit's export names

revit-ifc 24.2's `BodyExporter.ExportBody` (CLEANROOM.md source 5, `Exporter/BodyExporter.cs` at `e5caa9b81d`; cited, not copied) makes one extrusion per piece of the element's geometry and sets the body's `ScaledArea` from each extrusion as it creates it, so the area left is the last one's. `GrossAreaCalculator` reads `ScaledArea` (RE-480, section 1). A ceiling in pieces therefore gets the area of the piece its body exporter creates last.

`ceiling_piece_order_vs_ifc.py` reads, for each `IfcCovering` ceiling of Revit's export with a `GrossArea`, every body item's profile area (the item's volume over its depth, through an `IfcBooleanResult`'s first operand where there is one), groups the items into pieces by area and sets each piece beside `GrossArea` (Measure [43713821158](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43713821158)):

| file | ceilings in pieces | `GrossArea` one piece's | the piece holding the highest STEP id | the last item listed | the largest piece |
|---|---:|---:|---:|---:|---:|
| concrete office, as built | 19 | 19 | 19 | 19 | 11 |
| concrete office, permit | 20 | 20 | 20 | 20 | 12 |
| timber office, as built | 19 | 19 | 19 | 19 | 11 |
| timber office, permit | 20 | 20 | 20 | 20 | 11 |

78 ceilings in pieces, 73 uncut and 5 cut (the cut ones, by the `.rvt`'s `CutPairGStep` as in RE-480, are 1, 3, 0 and 1; Revit writes them as plain pieces too, with no `IfcBooleanResult`). On every one, `GrossArea` is the piece that holds the item with the highest STEP id, which is the item listed last. The largest piece is it on 45. A layered ceiling lists one item per piece per layer, and the order of the pieces can differ between layers (2094020 lists A, B and then B, A), so the piece named is the last layer's last.

## 2. The saved mesh names the same piece

A ceiling's saved mesh stores the slab one face per primitive: the top face first, then the bottom, then the sides. Where the slab is in pieces, each face's pieces are connected parts of that primitive (triangles sharing a vertex 1e-6 ft apart). `probe_1209_ceiling_mesh_order` prints, for each part of each primitive, the index of its first triangle, its height and its area facing down and up; its scorer sets each candidate rule beside Revit's `GrossArea` to 0.1% (Measure [43714434788](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43714434788), the same four files):

| the part of the saved mesh taken | Revit's piece on |
|---|---:|
| the part of the first primitive that holds its first triangle | 78 of 78 |
| the first part of the first upward face | 78 of 78 |
| the first part of the first downward face | 74 of 78 |
| the largest part | 45 of 78 |
| the last part of the first downward face | 4 of 78 |

The 5 cut ceilings among the 78, set beside the per-ceiling values of RE-480's run (Measure [43701677731](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43701677731)), give the same: the first primitive's first part is Revit's on 5 of 5. On these ceilings the first primitive is the top face, so the first two rules are one.

## 3. The rule

In the saved-mesh pass, a ceiling that still holds no `Qto_CoveringBaseQuantities.GrossArea` and whose saved face is not one piece (RE-480's `one_piece_face_area` gives none) gets one from `first_piece_face_area` when:

- its saved mesh decodes with no diagnostic (`partial_refusal`);
- the mesh's first primitive is one horizontal face: every triangle's outward normal vertical (|n_z| at least 0.999 of its length), all at one height (0.001 ft) and facing one way, and in more than one connected part;
- among the horizontal triangles of the other primitives facing the other way, grouped by height, some connected part has the same area to 0.1% as the part the rule takes.

It takes the area of the part of the first primitive that holds the primitive's first triangle and writes it as `GrossArea` (`add_covering_areas`), into the ceiling's set or a new one.

rvt-rs writes a ceiling sketched in pieces as one `IfcCovering` per piece, all with its ElementId (#331), where Revit's export writes one covering whose body lists the pieces. `coverings_without_area` therefore takes a ceiling per ElementId, only where none of its coverings holds `GrossArea`, and the area goes on the first of them, the element's own covering, which the export writes before its pieces.

**The first version wrote nothing.** It took a ceiling only where its ElementId named exactly one covering without `GrossArea`, and every ceiling in pieces names 2 to 9, so its full Measure ([43715338673](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43715338673)) moved no export. A diagnostic build that prints, for each ceiling without `GrossArea`, its coverings and where the rule refuses (`052bc6e9db00`, archived under `refs/archive/2026-11-15/cited/`, Measure [43721003786](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43721003786), the four office files) found the 16 ceilings in pieces, 4 on each file, with 2 to 9 coverings each. On all 16 the rule itself takes an area: no decoder diagnostic that `partial_refusal` counts (the 5 cut ones included, which RE-480's probe counted as not decoded whole: the two tests disagree on them, and why is not read here), every triangle of the first primitive level and facing one way, in 2 to 9 parts, and an opposite part of the same area to about 1e-15. Nothing a ceiling already holds is replaced, so RE-465's values and RE-480's stay as they are. A ceiling whose first primitive is not one horizontal face, whose mesh decodes with a diagnostic, or whose taken part has no partner face of the same area gets none.

## 4. Measured

Measure [43733137320](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43733137320) exports 120 models at `b000d8ab` and at its merge base, `main` at `156e2b25`, with nordic, r2025, samples and holdout, and runs every scorer. The filtered run [43721768570](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43721768570) (`a2833a06`, the office files, `psets_vs_ifc` and `qtos_vs_ifc`) gave the same office counts.

**The office.**

| file | `GrossArea` equal, before | after | different | missing, after |
|---|---:|---:|---:|---:|
| concrete office, as built | 86 of 90 | 90 of 90 | 0 | 0 |
| concrete office, permit | 71 of 75 | 75 of 75 | 0 | 0 |
| timber office, as built | 84 of 88 | 88 of 88 | 0 | 0 |
| timber office, permit | 70 of 74 | 74 of 74 | 0 | 0 |

- **Which ceilings.** The 16 new values are the 16 ceilings RE-480 left without one (its section 5): the 5 cut ceilings whose face is in pieces (1, 3, 0 and 1) and the 11 uncut ones RE-465's rule leaves out (3, 1, 4 and 3), of 2 to 9 pieces each. Every covering rvt-rs writes on the four files that Revit gives a `GrossArea` now has Revit's.
- **Totals.** Each file's totals move by these 4 alone: equal +4 and missing −4. `different` stays at 1, and the values Revit does not write stay at 0, 0, 3 and 3.
- **Other scorers.** `psets_vs_ifc` is identical. The MCP answers check one more element and find one more quantity equal to Revit's on each file.
- **Diagnostics.** `diagnostics.json` moves only in `exported.total_entities`, +4 on each file: one quantity per ceiling, in a set its covering already holds.

**The other models.**
- **Byte-identical.** 115 of the 120 exports are: the BIM4LCA housing, the Revit 2025 models, Core Interior, RE1, every held-out export and every sample save but one.
- **Snowdon.** The one that moves is Autodesk's Snowdon Towers Architectural sample of Revit 2024: one ceiling gains a `GrossArea` (`exported.total_entities` +1). No Revit export in CI checks it, and Revit's own IFC4 export of that file (local only, no licence) was made without base quantities, so it holds no `GrossArea`. Read against that export's body instead, by plain STEP outside Measure (the two exports kept by Measure [43735582243](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43735582243)): the area rvt-rs writes equals, to 8e-9 relative, the profile area of the piece that Revit's export creates last, the body item with the highest STEP id (6 items, 3 pieces). Section 1 shows that is the piece whose area Revit's `GrossArea` takes. That piece is also the largest of the three, so this one ceiling does not tell RE-486's rule from the largest piece's.
- **Gates.** `same-project gates` passes on `b000d8ab`.

## 5. The registered rows

Registered on #1209 before the run (2026-11-15, comment 6894478486), read from Measure 43733137320:

1. **The office.** Every ceiling the rule gives a `GrossArea` equals Revit's, and `different` and the values Revit does not write are unchanged. **Met:** 16 of 16, the 5 cut ones among them.
2. **The other files with a Revit export.** No value moves, or each new one equals Revit's. **Met:** none of their exports moves.
3. **The samples.** Ceilings that gain a value are counted per release, with the same value on every save of one project, and `same-project gates` passes. **Met:** one ceiling of the 2024 Snowdon sample, a project of one save in the corpus. No other sample save moves, and the gates pass.
4. **Held-out.** Counts only. **Met:** every held-out export is identical.
5. **No value is replaced.** RE-465's and RE-480's values are unchanged. **Met:** equal rises by exactly as much as missing falls on every office file, and the semantic diff shows `GrossArea` going from absent to a value, with no changed value.

## 6. Where it rests

- **One building.** The rule is read from the 78 ceilings in pieces of the BIM4LCA office, four saves of one building by Revit 2024, and the values it writes that a Revit export checks are that building's. It is listed on `docs/one-project-rules.md`.
- **An observed ordering, not a format fact.** No field of the saved mesh says which piece Revit's body exporter creates last. The rule rests on the order in which Revit saved these ceilings' triangles agreeing, on 78 of 78, with the order in which its exporter created the pieces. A Revit release or a ceiling kind that saves its pieces in another order would break it, and the second face check guards only against a part that is not a whole piece of the slab.

## 7. Reproduce

```bash
# which piece Revit's export names (the four office files)
gh workflow run measure.yml --ref main -f ref=0049be011b22750ee2e28ea589a7a0dd24d458a3 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit" -f scorers=ceiling_piece_order_vs_ifc
# the saved mesh's parts in file order
gh workflow run measure.yml --ref main -f ref=31bca2a89c12c6dae29af21f23d4e1551e3333bf -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit" -f scorers=none \
  -f probe=probe_1209_ceiling_mesh_order -f probe_scorer=-
# the change, on the four office files
gh workflow run measure.yml --ref main -f ref=a2833a069bd5df015a153f9abddbfe21562a1d8b -f base=main -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit" -f scorers="psets_vs_ifc qtos_vs_ifc"
# the change, on every model (b000d8ab is a2833a06 with main 156e2b25 merged in and the docs)
gh workflow run measure.yml --ref main -f ref=b000d8ab232ed9cc30be030acd4074e98385626f \
  -f base=156e2b257f610c8c57c82828571a892b687ebb77 -f nordic=true -f r2025=true -f samples=true -f holdout=true
```
