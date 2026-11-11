# RE-438: the ends beside an oblique mullion, or a partner beside a mullion across inside, are read, and a partner joint centred under a mullion across is hidden

**Date:** 2026-11-11
**Issues:** #1767 (the 4 `NetSurfaceArea` values still different on the timber office at permit after RE-436), #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-268 (which ends are kept, from the boxes), RE-400 and RE-403 (what it leaves out), RE-404 (the same from the meshes), RE-432, RE-433 and RE-436
**Artefacts:** the BIM4LCA concrete office as built and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of the two as built (the permit exports are read through the scorer's list of the values that differ)
**Scorers:** `tools/re/qtos_vs_ifc.py`; the end tables come from two throwaway builds that print what lies at each end of each mullion (commits `1d00cc0e2253` and `72f6a699f2f0`, archived under `refs/archive/2026-11-11/cited/`)
**Runs:** Measure [43086563944](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43086563944) (the first build, the three office files), [43090090858](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43090090858) (the second, which adds the geometry of what lies across each end); the change's own run is in section 5
**Status:** Not yet read: the unfiltered Measure of the change is running (section 5).

## 1. The question

After RE-436 the BIM4LCA timber office at permit still reads 4 `Qto_MemberBaseQuantities.NetSurfaceArea` values different (#1767): three pieces of one chain of 50 x 50 mm mullions on one curtain wall (3539474, 3539475 and 3135318) where rvt-rs counts 1, 2 and 1 joint ends more than Revit, and a short mitred piece (3029161). RE-268 reads which ends are kept from where the boxes of a wall's mullions meet, and leaves a mullion out when it cannot: 597 of the concrete office as built's 10,003 mullions (447 because an end meets a mullion that is itself left out or overlapped, 150 for an end of their own) and 18 more whose record names no axis, and Revit writes a value on every one. What does Revit do at an end that RE-268 does not read?

## 2. What the builds print

The first build prints, for each mullion, the neighbours RE-268 finds at each end of its box (a partner, a mullion lying over it, one across at its centre or holding its point inside, a parallel mullion holding it, an oblique one), the outcome (read, left out for an end of its own, left out for a neighbour, no axis), and for five mullions the neighbours' ids. The second adds, for each mullion across an end, its offset from the end's point and whether its face holds the whole end face, and the oblique neighbours' cosines. Neither moves an export: the concrete office as built hashes the same as `main`'s, and the second build's three exports the same as the first's. Revit's count of kept ends is read for each mullion as the surface of its mesh less Revit's `NetSurfaceArea`, over the section's area (the volume over the record's length: the mesh's longest extent is the wrong axis on a piece shorter than its section is wide): a whole number of end faces on 9,983 of the concrete office's mullions and 7,857 of the timber office's.

## 3. What Revit does with each end

Taking the mullions whose count is 0 or 2, so that each end is told (the ends of a count of 1 are not), the concrete office as built:

| what RE-268 finds at the end | ends Revit keeps | ends Revit hides |
|---|---:|---:|
| nothing (6,780 of them butt a side face of a mullion across, whether its face holds the whole end face or not) | 8,303 | 2 |
| a partner alone | 0 | 8,796 |
| a mullion across centred on the end alone | 0 | 29 |
| a lying-over mullion (`S`) or a parallel one holding the point (`L`), alone | 86 and 174 | 0 |
| **an oblique mullion alone** | **50** | 0 |
| **a partner and an oblique mullion** | **76** | 0 |
| **a partner and a mullion across holding the point inside** | **52** | 0 |
| a mullion across inside alone | 14 | 5 |

The timber office as built reads alike (a mullion across centred alone: 25 hidden and 2 kept). RE-268's rule (an end is hidden iff it has a partner xor a mullion across at its centre or holding its point inside) agrees with Revit on the first five rows and on a partner with a mullion across inside, but it hides a partner end that an oblique mullion lies at, and it leaves a mullion out for an oblique mullion or a mullion across inside at an end: 150 of the concrete office's. Of those 150, the end patterns are all in the table: **131 mullions have only ends that the table decides (an oblique mullion alone or with a partner, a partner with a mullion across inside, a lying-over or parallel one with an oblique one), and the new rule's count of kept ends is Revit's on every one**: 61 have no partner end and 70 have one (the timber office as built: 130, 61 and 69). The other 19 (23 on the timber office) have an end with a mullion across inside and no partner, which is Revit's to keep on 14 ends and to hide on 5 that read alike, 0.1 m pieces beside a partner.

Where a partner and a mullion across centred on the end meet, Revit hides the end (4 ends on 3 mullions, all in the timber office at permit, none in the as-built files): the two joints of the chain of #1767, 3539474 with 3539475 and 3539475 with 3135318, each a pair of partner ends with a bar across at the middle of the joint (3539479 and 3539478). RE-268's xor reads them kept, and that is 3 of the 4 values.

## 4. The change

`Neighbours::hidden` and `Neighbours::unread` (`src/research/partition_schema_mvp/mullion_ends.rs`): an end with a partner is hidden unless an oblique mullion or one across holding its point inside lies at it, which keeps it; with no partner, a mullion across centred on it hides it, and nothing else does. A mullion is left out for an end with a mullion across inside and no partner, or with an oblique mullion where one across holds the point (no end of the BIM4LCA offices has the second, so it stays unread), and no longer for an oblique mullion alone or a partner end with one across inside. Four failure-first tests of the pure helper `kept_ends`: a joint centred under a bar is hidden, an oblique brace keeps a partner end and does not leave its mullion out, a bar across inside leaves an end with no partner unread and one with a partner kept, and an oblique mullion where one across is centred is not read.

## 5. What Measure found

Not yet read: the unfiltered Measure of the change against `main` is running ([43092555284](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43092555284)). The rows it is held to are registered on #1636 (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6805957071).

## 6. Not done

- The 2 mullions of the concrete office as built and the 7 of the timber office that the rule would read wrong stay out (they are left out for a neighbour, as before): 6 are the end pieces of chains of 0.1 m pieces, whose free end butts a long mullion within 6 cm of long mullions that have another lying over their ends and whose end Revit hides where it keeps every other end of that kind, and 3 of the timber office have a mullion across centred on an end Revit keeps (312 and 272 mullions beside an overlapped mullion read right).
- The mullions with a lone inside end (19 and 23) stay out.
- The mesh path refuses a mullion whose box reading keeps an end that its mesh finds a partner at (RE-404 expects a hidden end for each partner), so a mullion with a partner end that the change releases is written only where its value comes from the boxes.

## 7. Reproduce

```bash
# the first and second builds (kept under refs/archive/2026-11-11/cited/1d00cc0e2253 and 72f6a699f2f0): the neighbours and outcome of every mullion, in each model's rvt-ifc.log (`mullion-kd`, `mullion-ac`, `mullion-ob` lines)
gh workflow run measure.yml --ref main -f ref=1d00cc0e22539b1c8ef0dd40130f0166a6fbf2b8 -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-timber-permit nordic-office-timber-as-built" -f scorers=none
gh workflow run measure.yml --ref main -f ref=72f6a699f2f0f6e648abbfeab9352a42c432569f -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-timber-permit nordic-office-timber-as-built" -f scorers=none
```
