# RE-455: where Revit's export stops a beam short of its record box: the column face on ten BIM4LCA models, and what the 13 beams it does shorten name

**Date:** 2026-11-13
**Issues:** #1839 (a Revit 2027 concrete frame whose beams run past Revit's), #1829; follows RE-122 (a beam stops at the near face of the column it names, Revit 2023) and RE-328 (a beam's body from its line and its record box)
**Artefacts:** the BIM4LCA models that Measure reads with Revit's own IFC4 export of each (VTT's ARCH archive, CC BY-SA 4.0; `research/bim4lca-measure.tsv`): the concrete and the timber office, the concrete and the timber housing, as built and at building permit, and the two terrain models. Ten models, 300 beams, each with a body in Revit's export.
**Probe and scorer:** `probes/examples/probe_1839_beam_column_ends.rs`, `tools/re/beam_ends_vs_ifc.py`
**Runs:** Measure [43450655901](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43450655901) (the probe at `c5ea03ff`, columns only) and [43455872839](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43455872839) (`870b4276`, columns, walls and other beams), both `-f nordic=true -f base=none -f scorers=none`; no export changes.
**Status:** Negative. On these models a beam's record box already ends where Revit's body ends for the beams that name a column, so the face of the column has nothing to cut and the rule cannot be tested on them; and none of the three kinds of element a beam's list can name (columns, walls, other beams) explains the 13 beams Revit does shorten. A test of the column face needs a model whose boxes run to the lines' ends, such as the reporter's.

## 1. The question

A reporter's Revit 2027 concrete frame has 396 of 719 beams longer than Revit's by a median of 296 mm, and none shorter (#1839). rvt-rs draws a beam along its location line, which runs to the middle of the columns it frames into; RE-122 found Revit's export stops a beam at the near face of the column its reference list names. That rule ships for Revit 2023 and is off for 2024 and later, because most steel beams of Snowdon Towers' structural sample stop a stored join cutback short of the face. Does the face say where Revit stops a beam on the models we can score?

## 2. The record box already ends where Revit's body ends

The probe lists every placed structural framing record with its box along the longer plan axis, the columns its list names that hold each end, and the bounds RE-122's rule gives. The scorer meshes Revit's beam in its own frame and sets its length beside the box's.

| Revit's length against the record box's (within 1 mm) | beams |
| --- | ---: |
| the same | 273 |
| shorter | 25 (13 beams: some are in both saves of a project) |
| longer | 2 |

- **Only the timber office names columns.** The beams of the other eight models name none. On each timber office file 63 of the 71 beams name a column, and the rule gives bounds on 60.
- **The rule is a no-op where it reaches.** Of the 120 beams (60 on each file) the rule gives bounds, Revit's length is the box's on 118, and on 116 of those the bounds are the box's own ends: the box stops at the column's near face already.
- **Beam 3360608 is cut where Revit does not cut it.** On both files its box and Revit's are 18.940 ft, and the rule would cut it to 18.184 ft, 230 mm short.
- **One beam is both shortened and reached:** 3346643 on the permit save (box 18.834 ft, Revit 18.479 ft, the rule's bounds 18.479 ft). That is exact, and it is one beam.
- **One beam is longer than its box:** 3350262 on the permit save (box 8.660 ft, Revit 9.605 ft; on the as-built save the box is 9.605 ft). The other is beam 2125408 of the timber housing as built (box 8.559 ft, Revit 8.917 ft, which names no column).

## 3. What the 13 beams Revit shortens name

The shortening is 2 mm (3 beams), 50 mm (2), 96 mm (6), 108 mm (1) and 2,320 mm (1). The second run asks, for columns, walls and other beams, how many each beam's list names, how many hold an end (the element's box contains the end along the axis and the beam's middle across it, and overlaps the beam in height), and where the beam would stop if the one element that holds the end cut it at its near face.

| the 13 beams | beams | cut at the element's near face |
| --- | ---: | --- |
| name a column that holds an end | 1 (3346643) | exact |
| name a wall that holds an end | 6 (the 96 mm group of the timber housing) | too far on 6 of 6, by 0.061 and 0.062 ft |
| name a wall that holds no end | 1 (the 2,320 mm beam of the concrete housing) | no bounds |
| name no column, wall or beam | 5 (3 at 2 mm, 2 at 50 mm) | no bounds |

- **Ten beams have a wall that holds an end, and the wall's near face is Revit's stop for none of them.** It gives bounds to all ten: on the six beams Revit shortens by 96 mm it cuts too far, since Revit's beam ends 0.061 to 0.062 ft (about 19 mm) inside the wall's box; and it cuts four beams that Revit leaves at the box's length or longer, 2123311 of the concrete housing (100 mm) and 2218451, 2218461 and 2125408 of the timber housing (5 mm). The walls' layers were not read, so what the 19 mm is stays open.
- **The column kind gives bounds to 2 beams of the timber office and is right on one:** 3346643 (exact) and 3360608 (230 mm too short, section 2).
- **No beam names another beam** in these ten models, so that kind has nothing to say.

## 4. What it means

- **The column face cannot be tested on the models we can open.** The only beams that name a column have their box at its face already, and the one that Revit shortens is one beam. For 2024 and later the rule stays off, and RE-122's finding on Snowdon Towers (steel beams a stored cutback short of the face, concrete beams at it) stays the only evidence.
- **The reporter's pattern is not in these ten models.** A box that runs to the line's ends, inside the column, is what the reporter's model shows; here the box is the body. The test is a run of the probe and the scorer on that model: the scorer prints counts only without `--list`, and its first tables say how many beams Revit shortens, how many a column gives bounds to, and how many of those the face makes exact.
- **A wall is not a column.** Where a wall holds a beam's end, Revit's stop is not at the wall's box on any of the ten beams, and the walls' layers were not read.

## 5. The reporter's model: the same table, and the face is not where Revit stops these beams either (2026-11-13)

The reporter ran the probe (the Windows build of `probe-binaries.yml` run 43460256185) and this scorer (at `d90c1fa8`, without `--list`) on the Revit 2027 concrete frame of #1839, against Revit's export of the same save, and posted its summary lines: counts and lengths, no ElementId.

- **915 beams, all with a body in Revit's export, none square in plan.** Against the record box, Revit's beam is longer on 188, the same on 189 (the rule gives bounds on 76 of them, none on 113) and shorter on 538 (bounds on 361, none on 177), by a median 256 mm (quartiles 129 and 433 mm, longest 1,294 mm).
- **The 2023 rule is not Revit's on the 361 beams it cuts.** Exact on 2, cut too far on 310, too little on 49. It shortens them by a median 559 mm where Revit shortens them by 261 mm (Revit's length minus the rule's: median 108 mm, quartiles 20 and 503), and it would cut 76 beams that Revit leaves at the box's length.
- **By the kind of element a shortened beam's list names:** columns 477 (an end held on 397, bounds on 364: exact 3, too far 309, too little 52), other beams 468 (held on 410, bounds on 365: exact 12, too far 192, too little 161), walls 17 (held on none).
- **The 188 beams Revit makes longer are a case of their own:** 176 are steel roof members that name no column, longer by a median 13 mm. By the family names in Revit's export: concrete 529 shorter, 179 the same, 12 longer; steel roof and other 9, 10 and 176.

The near face of a column's record box is not where Revit stops these beams, nor is a beam's or a wall's. The reporter's guess is that some of the model's columns are shear walls, with long or L-shaped sections, and that a turned column's box reaches past its real face. This scorer cannot say what the stop is, which is why section 6 asks what Revit's own beam ends lie on in Revit's own export.

## 6. What the ends of Revit's own beams lie on (scorer addition)

The scorer now also asks Revit's own export what its beams stop on, without the record box or the probe's naming: each Revit beam's mesh is taken in the world frame, its two end caps are found as the ends of its principal axis, and the centre of each cap is looked up among the faces of Revit's columns, beams and members, walls and slabs (within 1 mm of a face and inside it, the beam's own faces excluded). The ends are counted by how Revit's length stands to the record box's and by the kind of element they lie on, column first.

On the ten BIM4LCA models (Measure [43480745216](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43480745216), `-f nordic=true -f base=none -f scorers=none`, the scorer from this branch's commit):

| Model | Revit stands to the record | Ends | Column | Beam | Wall | Slab | Nothing |
|---|---|---:|---:|---:|---:|---:|---:|
| timber office, as built | the same (71 beams) | 142 | 108 | 0 | 20 | 0 | 14 |
| timber office, permit | the same (69) | 138 | 105 | 0 | 19 | 0 | 14 |
| timber office, permit | shorter (1), longer (1) | 2, 2 | 2, 1 | 0 | 0, 1 | 0 | 0 |
| concrete housing, both | the same (21), shorter (1) | 42, 2 | 0 | 0 | 3, 2 | 0 | 39, 0 |
| timber housing, as built | the same (29), shorter (11), longer (1) | 58, 22, 2 | 0 | 16, 0, 0 | 0 | 0 | 42, 22, 2 |
| timber housing, permit | the same (30), shorter (11) | 60, 22 | 0 | 16, 0 | 0 | 0 | 44, 22 |
| concrete office, as built and permit; terrain models | the same | 16 each | 0 | 0 | 8 or 0 | 0 | 8 or 16 |

- **A positive control.** On the timber offices, where the 2023 rule is exact on 59 of the 60 beams it cuts, 108 of 142 ends (105 of 138 at permit) of the beams Revit leaves at the record box's length lie on a column's face in Revit's own export: where the record box ends at the column's face, so does Revit's mesh, and the scorer finds it. The other ends lie on walls (20 and 19) or on nothing (14 and 14).
- **The 11 timber housing beams Revit shortens (a median 96 mm, quartiles 2 and 96) end on nothing** in this set (22 of 22 ends), and the one concrete housing beam it shortens, by 2,320 mm, ends on two walls. No beam Revit shortens ends on a column face in these models except the timber office's one at permit (2 ends).

The question for the reporter's model is the same, now asked of Revit's own geometry: whether the ends of the 538 beams Revit shortens lie on the true face of a column (or of a wall, the shear walls the reporter suspects), where the record box's face is not that. The scorer prints counts only without `--list`.

## 7. Not changed

- Nothing is exported differently. `beam_column_trims` stays a Revit 2023 rule.
- The 13 beams are left as they are. A class is written as a rule only where the cut length is Revit's within 1 mm on every beam of it, the class has at least 10 beams and it cuts none that Revit leaves; none does.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref research/1839-beam-column-ends -f ref=<sha of the probe> -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit nordic-housing-*" \
  -f scorers=none -f probe=probe_1839_beam_column_ends -f probe_scorer=beam_ends_vs_ifc
python3 tools/re/beam_ends_vs_ifc.py <probe.txt> <Revit's export>.ifc [--list]
```
