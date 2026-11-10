# RE-400: a mullion with another lying over it, and the neighbours of a short mullion, are read like the others

**Date:** 2026-11-10
**Issues:** #1046 (base quantities Revit computes from its joined geometry, W7), #1016 (quantities); follows RE-268 (which end faces of a curtain mullion Revit's solid keeps), RE-243 and RE-248
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them); revit-ifc is not read again here
**Runs:** Measure [42885661997](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42885661997) (`tools/re/mullion_ends_vs_boxes.py` with RE-268's cascade split by what the neighbour is and by the mullion's own length, commit `0b106c9fb353`, on `main` at `2713ebcb`, saved meshes off; section 2), [42886890399](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42886890399) (this change's scorer and the quantity census on its code, same flag; section 4) and [42887741985](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42887741985) (unfiltered: this change against `main`; section 4)
**Status:** Positive on the office: a curtain mullion drawn as an extrusion of its box has its `NetSurfaceArea` written on 926 more mullions as built (7,530 of 7,795, from 6,604) and 924 more at permit (7,558 of 7,823, from 6,634), Revit's on all of them, none different. The 263 mullions on each file that RE-268's reading would still get right on 256 and wrong on 7 stay out. The rule rests on the office, one project in two states; no other model has Revit's `NetSurfaceArea` of a mullion to score it against.

## 1. What RE-268 left out

RE-268 reads which ends of a mullion Revit's `NetSurfaceArea` counts from where the boxes of its curtain wall's mullions meet, and writes `perimeter x length + kept ends x section area`. It leaves a mullion out when one of its ends is not read, or when an end meets, within 6 cm, a mullion that was left out or is shorter than its own section. Of the 7,795 mullions of the office as built (7,823 at permit) that Revit draws as one extrusion of their box, 1,189 (1,187) were left out, and RE-268 section 4's table said its reading would have been Revit's on 1,182 and wrong on 7 of them. It kept them out because "no field read tells" the 7 from the others. That table did not say what the 7 had in common, and the left-out mullions were not told apart by what they met.

## 2. The 1,189 by what the neighbour is

The scorer of RE-268 now prints, for each mullion it leaves out because of a neighbour, what that neighbour is and how long the mullion is. On the office as built (and at permit, where it differs):

| left out by RE-268 because | mullions | its count is Revit's | wrong |
|---|---:|---:|---:|
| an end of its own has another mullion lying over it (PAR_SAME or PAR_IN), all 0.3 m or longer | 159 (157) | 159 (157) | 0 |
| an end meets a mullion shorter than its section, and nothing else that was left out | 767 | 767 | 0 |
| an end meets a mullion that has another lying over it, or one across it inside its box (2 of them also a short one) | 254 | 252 | 2 |
| an end of its own has a mullion across it holding the end's point off its axis and its faces (PERP_IN) | 9 | 4 | 5 |

The 767 are 444 mullions 0.3 m or longer and 323 under 0.15 m, and the rule is Revit's on every one. The 2 wrong of the third row are horizontal pieces 0.1 m long of one curtain wall whose ends butt against an upright that lies over another upright; 252 mullions that meet the same kind of neighbour are right, 149 of them under 0.15 m long. Nothing read separates the 2 from them. The 5 wrong of the fourth row are why a PERP_IN end stays unread: the count is right on 4 of 9.

## 3. What is written

`partition_schema_mvp::kept_ends` (`mullion_ends.rs`) now:

- reads a mullion that has another lying over one of its ends (overlapped) on its own: the mullion that lies over it counts for nothing at the end's point, and the overlapped mullion is not left out for what is near it. It remains a mullion that keeps its neighbours out, as RE-268 had it, since the 2 wrong above are next to such mullions;
- no longer leaves a mullion out because a mullion shorter than its section is near one of its ends. The short mullion keeps its own count, as it did.

A mullion with an end that has a mullion across it holding the point off its axis and faces (PERP_IN), or one oblique to it, one whose axis is not read, and one that meets any of those or an overlapped mullion (unless it is overlapped itself) is left out, as before. `tools/re/mullion_ends_vs_boxes.py` reads the same rule and prints the mullions written by what the rule read, with the curtain walls they stand on.

## 4. Measured

**The scorer on the office** (run 42886890399, `rvt-ifc --no-saved-meshes` so that the mullions are extruded boxes the scorer reads, as since #1191 a mullion is drawn from its saved mesh by default; the code is the same either way, the rule reads each mullion's record box):

| | as built | at permit |
|---|---:|---:|
| mullions with a `NetVolume` (drawn as an extrusion of their box) | 7,795 | 7,823 |
| `NetSurfaceArea` written, Revit's value, before (RE-268's rule) | 6,604 | 6,634 |
| `NetSurfaceArea` written, Revit's value, now | 7,530 | 7,558 |
| different from Revit's | 0 | 0 |
| left out | 263 | 263 |
| of those left out, the rule's count would be right, wrong | 256, 7 | 256, 7 |

By what the rule read, the mullions written, with the curtain walls they stand on: as RE-268 read them 6,604 (211 walls; 6,634 on 212 at permit), overlapped 159 (40 walls; 157 on 39) and neighbours of a short mullion 767 (106 walls, both files); none different. The census (`qtos_vs_ifc`, same run) has `Qto_MemberBaseQuantities.NetSurfaceArea` equal on 7,532 of 10,003 as built (7,560 of 10,031 at permit) with none different, and 2,471 missing on both: the 2,208 mullions Revit does not draw as an extrusion of their box (RE-243 section 2) and the 263.

**This change against `main`, unfiltered** (run 42887741985, `-f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true`): pending; this section is filled in when the run completes.

## 5. Not shown

- **Other projects.** The office is the one model whose Revit export writes a mullion's `NetSurfaceArea`. RE1 Architecture's 10 mullions are in an export made with base quantities off (RE-242), so the census counts what rvt-rs writes there apart, and Snowdon Towers Architectural's Revit export, which is local only, holds 1,625 mullions in 66 curtain walls and no `Qto_MemberBaseQuantities` on any of them. The 106 curtain walls that carry the short-neighbour mullions and the 40 that carry overlapped ones are 106 and 40 of one building's 220.
- **The 263 left out.** 9 have a PERP_IN end of their own (4 right, 5 wrong), 254 meet a mullion with an overlapped or PERP_IN end (252 right, 2 wrong). They stay out until something read tells the 7 from the others.
- **The 2,208 mullions on curved or edited curtain walls** that Revit does not draw as an extrusion of their box: RE-268 section 4 reads them right on 1,664 and wrong on 136, so none is written.
- **Why a mullion that something lies over is read right.** The 159 are Revit's with the overlapping mullion counted for nothing at the end, which says what Revit's solid keeps, not why. A project whose exporter joins overlapping mullions another way would show it as a different value.

## 6. Reproduce

The scorers come from `--ref`, a branch (`measure/mullion-cascade`, tip `0b106c9fb353`, for the first run, also kept under `refs/archive/2026-11-10/cited/0b106c9fb353`; `main` for the others once this change has landed); `-f ref` is the code that makes the exports.

```bash
gh workflow run measure.yml --ref measure/mullion-cascade -f ref=2713ebcbb4c0aac8190272a2460f39bf6820eec9 -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers=mullion_ends_vs_boxes -f ifc_flags=--no-saved-meshes
gh workflow run measure.yml --ref main -f ref=bb0e149bf7a57d2a6b6ea3b5fd88c778c90d64c5 -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers="mullion_ends_vs_boxes qtos_vs_ifc" -f ifc_flags=--no-saved-meshes
gh workflow run measure.yml --ref main -f ref=bb0e149bf7a57d2a6b6ea3b5fd88c778c90d64c5 -f base=2713ebcbb4c0aac8190272a2460f39bf6820eec9 \
    -f nordic=true -f r2025=true -f samples=true -f racadv=true -f bimcollab=true -f holdout=true
```
