# RE-52 — Stair runs' treads and risers

**Date:** 2026-09-23
**Result:**
- **Positive.** A straight stair run's element data carries its plan sketch: two boundary lines from its first riser to its last, and a line across the run at each riser, as bounded line records (RE-49) at the run's base elevation.
- **Positive.** The run type a run's reference list names has its own serialised object, holding its tread thickness, riser thickness, nosing length and structural depth, flags for its parts and construction, and its name.
- With the stair's riser height (RE-47), those give the run's side view, treads and risers. rvt-rs now draws 34 of Snowdon Towers' 43 runs that way instead of as their record box, which spans the whole stair's height.
- The 30 steel-pan runs, with slanted risers, equal Revit's own geometry of each flight to 1e-5 ft: every point rvt-rs draws is one of Revit's, and every point of Revit's lies on or in rvt-rs's.
- The 4 runs with upright risers hold every point of Revit's except one riser's foot, but their nosing is drawn square: Revit shapes it with the type's nosing profile, which is not read.
- Each run with a run type now carries that type's name. It equals the type part of Revit's own `ObjectType` on all 49 flights Revit exports: 43 Tags, since the copies of a multistory stair share one.

**Probe:** `examples/probe_re52_stair_treads.rs` lists each run type's values and flags and each run's outcome, and with `--json` writes each drawn run's sketch and side view for scoring.

**Oracles:**
- Snowdon Towers Sample Architectural (local only) with Revit's IFC4 export of it. Revit's flights are meshed by IfcOpenShell 0.8.5 and moved from the export's shared coordinates to the model's by the site placement the export states: origin (417622.2672, 78713.9831, 237.8964) m, reference direction (0.898675, 0.438615).
- The VIM export of the 2027 edition of Snowdon (`vimaec/vim-hackathon`, local only), for the 6 runs whose Tag Revit's IFC gives to a copy of the stair on another floor. The VIM holds each flight's mesh under its ElementId.
- No licensed file in CI has a stair: Core Interior, Einhoven and the RE1 models have none. Unit tests pin the byte layouts and both constructions.

-----

## 1. The run's sketch

After its element-data header and ElementId (RE-44), run 621141's data holds 99 bounded line records (`04 00 08 01 · f64 start · f64 end · f64×3 origin · f64×3 unit direction`), all at z = −16.917 ft, the run's base:

| order | line | meaning |
|---:|---|---|
| 1 | (−108.979, 25.688) → (−108.979, 33.938) | first boundary line |
| 2 | (−105.313, 25.688) → (−105.313, 33.938) | second boundary line |
| 3–12 | x −108.979 → −105.313 at y = 25.688 + k · 0.917 | the 10 riser lines |
| later | the same lines again, and a line per tread's front and back | not used |

`run_sketch` reads:
- the climb direction from the first line;
- the width from the first later line parallel to it;
- a riser at every line across the run that spans it from boundary to boundary.

It requires the first boundary line to end on the last riser. On Snowdon, 41 of the 43 runs give a sketch. Runs 1209815 and 1211948 are the two spiral runs, whose first line is not straight.

A run's riser lines number its own risers. Three single-run stairs record one more riser line than the flight riser count RE-47 gives them: 1563804, 1647253 and 2234674. They are not drawn.

## 2. The run type

A run's reference list names one type-definition record of category `OST_StairsRuns` (−2000919), as a wall's names its wall type (RE-28). For run 621141 that is 613941. That record has no element data of its own. Its serialised object sits inside element 629898's data:

```text
01 00 00 00 · u64 613941
+0x47  db 0f        object tag
+0x59  f64          structural depth, ft
+0x61  f64          0 on every type measured
+0x69  f64          tread thickness, ft
+0x71  f64          riser thickness, ft
+0x79  f64          nosing length, ft
+0xb5  u8           each tread runs on under the riser above (1 on 613941 only)
+0xbd  u8 × 4       monolithic, treads, risers, slanted risers
+0xc1  u32 n · UTF-16 × n   the type's name
```

(Offsets are past the start of the ElementId.) The byte at +0xb9 is 3 on 1557288 and 0 elsewhere. It is not read.

Snowdon's eight run types, read this way:

| type | depth | tread | riser | nosing | mono | treads | risers | slanted | under | name |
|---:|---:|---:|---:|---:|:-:|:-:|:-:|:-:|:-:|---|
| 168810 | 2.5 in | 2 in | 1/4 in | 1 in | 0 | 1 | 1 | 0 | 0 | 2" Tread 1" Nosing 1/4" Riser |
| 168815 | 2.5 in | 2 in | 1/4 in | 3/4 in | 1 | 0 | 0 | 1 | 0 | 3/4" Nosing |
| 176831 | 6.5 in | 2 in | 1/4 in | 3/4 in | 1 | 0 | 0 | 1 | 0 | 3/4" Nosing 6 1/2" Depth (no run uses it) |
| 613941 | 2.5 in | 1/4 in | 1/4 in | 1 in | 0 | 1 | 1 | 1 | 1 | 1/4" Tread 1" Nosing 1/4" Riser |
| 809789 | 2.5 in | 2 in | 1/4 in | 1 in | 0 | 1 | 0 | 0 | 0 | Spiral |
| 951855 | 24 in | 2 in | 1/4 in | 3/4 in | 1 | 0 | 0 | 1 | 0 | 3/4" Nosing (Solid Base) |
| 1557288 | 2.5 in | 2.5 in | 1/4 in | 3 in | 0 | 1 | 0 | 0 | 0 | 2 1/2" Tread Square Nosing No Riser |
| 1565035 | 2.5 in | 2 in | 1/4 in | 1 in | 0 | 1 | 1 | 0 | 0 | 2" Tread 1" Nosing 1/4" Riser (Wood) |

Every value agrees with the type's name. For every flight, the name equals the type part of the `ObjectType` Revit writes, which is `Non-Monolithic Run:` or `Monolithic Run:` followed by the name. That family part is not stored; like "Basic Wall" (#322), Revit supplies it in its UI language.

Two readings rest on few types:
- The monolithic flag is set on the three monolithic types. The two that runs use are the two Revit exports as `Monolithic Run`.
- The slanted flag is set on 613941 and on the monolithic types, whose risers Revit draws slanted.
- The +0xb5 flag is set on 613941 alone, the one type whose treads run on under the riser above. With one type it cannot be told apart from any other setting only that type has, so only the two constructions measured are drawn.

## 3. The side view

For a run of n risers with riser lines u₀ = 0 < u₁ < … < uₙ₋₁ along the climb, riser height h, tread thickness t, riser thickness r and nosing N, riser k stands at riser line k − 1, behind the nosing, and tread k spans risers k and k + 1. The last riser meets the landing or floor above. Two constructions are measured:

- **Slanted risers, treads under them (613941, the steel pan).** Riser and tread are one folded plate. The riser's front runs from (uₖ₋₁ + N, (k − 1)h) to the tread's nosing (uₖ₋₁, kh), and its back is r·√(h² + N²)/h behind it. The tread's front and back are the risers' faces carried through it.
- **Upright risers behind the tread below (168810, 1565035).** Riser k is r thick at uₖ₋₁ + N, from the underside of tread k − 1 (the floor, for the first) to the underside of tread k. Tread k spans from uₖ₋₁ to uₖ + N.

The outline is one closed ring in the run's vertical plane. It is extruded across the run as an `IfcExtrudedAreaSolid` whose `Position` states the plane: Z across the run, X up.

Monolithic runs, runs without risers, the other two combinations of the flags, and dimensions no run can have are not drawn. They keep their record box.

## 4. Measured against Revit's geometry

For each drawn run, every vertex of the drawn run was compared with the vertices of Revit's mesh of the same flight, and every vertex of Revit's mesh with the drawn run: inside or on its outline across its width.

| runs | construction | drawn vertices on Revit's | Revit's vertices on or in the drawn run |
|---:|---|---|---|
| 30 | slanted (613941) | all, to 1e-5 ft | all, to 1e-5 ft |
| 3 | upright (168810: 832703, 1240811, 2270381) | all but the nosing's lower front edge, 0.083 ft proud | all |
| 1 | upright (168810: 1240916) | the same | all but 4: its first riser continues 0.167 ft (one tread thickness) below the landing |

Of the 30 slanted runs, 24 were measured against Revit's IFC4 export and all 30 against the VIM. Revit's IFC gives the Tag of each of the other 6 (621691, 621695, 701091, 701095, 882251 and 882255) to a copy of its stair 13.333 ft higher, a multistory stair. The VIM holds each at its own elevation. Run 832703 is measured against the IFC only, because the VIM is the 2027 edition, where that run has 6 in risers and an extra tread.

The upright type's nosing is shaped by a nosing profile family: Revit's tread is 0.0625 ft thick at its front edge and chamfers back under the nosing. rvt-rs draws the tread its full 2 in thickness to its front edge.

Run 1240916 starts on a landing. Revit carries its first riser one tread thickness below the landing's top, behind the landing's edge. The runs starting on a floor do not. With one run showing it, this is not drawn.

Revit's IFC export and rvt-rs's IFC export of the file, both meshed by IfcOpenShell, agree on all 34 drawn runs to 3e-6 ft. An `IfcFixedReferenceSweptAreaSolid` with the same profile was tried first. IfcOpenShell 0.8.5 turns it 180° about the sweep direction when that runs along −X or −Y, against the frame IFC4 defines for it, so the stated `Position` of an extrusion is used instead.

## 5. What this does not do

- Monolithic runs (3 on Snowdon), runs without risers (3), spiral runs (2) and runs whose riser lines outnumber their risers (3) keep their record box.
- The nosing profile and landing-start riser in §4.
- Landings and stringers keep their record boxes.
- Nothing is read from a release other than 2024 (RE-47's layouts).
