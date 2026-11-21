# RE-497: A ring's stored edge ends within 0.00001 ft along the surface are one point

**Date:** 2026-11-21
**Issues:** #2085 (this change, #2120); #2108, #2111.
**Artefacts:** the BIM4LCA office and housing files (VTT, CC BY-SA 4.0; Revit 2024), each with Revit's own IFC4 export; RE1 Architecture, Plumbing, Mechanical and Electrical (Revit 2025, MIT); Autodesk's sample projects of Revit 2015 to 2027, which have no Revit export in Measure; the held-out models. Outside Measure, kept local: Revit's IFC4 export of Snowdon Towers Architectural (bldrs-ai/test-models, no licence).
**Probes:** `probe_2085_trim_breaks` with `native_saved_mesh::ring_gaps`, on `research/2085-ring-join` (`6be738a5`, and the candidate `5be3e1eb`), kept as `refs/archive/2026-11-21/research-2085-ring-join`; `probe_2098_face_cracks` on `measure/2085-touch-slice-cracks` and `measure/2085-touch-main-cracks`.
**Status:** positive. On `main` the join draws two office mullions, six bodies across four files, that were refused; each is within 0.000003 ft of Revit's body and at its volume. Four walls gain their five base quantities, every one equal to Revit's. No drawn body changes, nothing is newly refused, and no "different" count rises. On the research branch it meshes 152 of the housing's 351 and 84 of the office's 142 faces refused as disconnected. Gaps of 0.00001 ft or more stay refused, as Griffin chose (2026-11-21).

## 1. What refuses these faces

A plane or cylinder face's ring is built from its loop's edges, each joined to the ring at whichever of its ends lies within 1e-7 of the ring's last point in the surface's parameters. Where none does, the face is refused as `disconnected UV boundary`, and so is a plane beside a cylinder so refused, since a plane meshes such a neighbour to share their edge (`refined_plane_edge` calls `cylinder_face`).

On the BIM4LCA housing and office (concrete, as built) the census found every such ring already open with the edges' points as the file stores them, before any refinement. It found no seam: no gap of 1e-2 or more in the parameters with under 1e-4 ft between the points.

The gaps, each failing ring by its largest step along the surface, were:
- **Under 0.00001 ft:** 134 of the housing's 302 plane rings and 39 of its 65 cylinder rings; 80 of the office's 123 and 4 of its 12. Many cylinder rings part only in their angle, by 1e-7 to 4e-6, with the points under 1e-7 ft apart.
- **From 0.00001 to 0.0001 ft:** 16 plane and 24 cylinder rings on the two files. Not read.
- **From 0.0001 to 0.0015 ft:** where a piece of edge looks missing. 158 of the housing's 165 are in three graphs with the same counts and face numbers. 68 of the 165 part by exactly 0.1 mm: 60 along one parameter, 8 along both.

On the contributor's save, 36 of the 84 cylinder rings on the chairs part by under 0.00001 ft, and the other 48 by 0.0001 to 0.001 ft ([#2085](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7054296537)).

## 2. The rule

A ring whose edges do not all meet within 1e-7 in the parameters joins where every gap's step along the surface is under 0.00001 ft (`RING_TOUCHING`). The step is a chord on a plane and a helix on a cylinder, so a step across a seam is a whole turn long and never qualifies. Each such pair of ends becomes one point, the ring's own: the next edge's stored end is dropped, so its first span starts up to 0.00001 ft off it. No edge is added. A ring stays refused if one of its edges is shorter than the bound, or if both of an edge's ends lie within it of the ring's last point.

Griffin chose the bound (2026-11-21) over joining gaps up to 0.25 mm with straight edges labelled as filled in, and kept it when told it leaves 48 of the contributor's 84 chair rings refused. A candidate that bridged every gap up to 0.001 ft was measured first (`551ca747`; [#2085](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7055157063)): it meshed more faces, newly drew a faucet 1.7% over Revit's volume, and drew edges the file does not hold.

## 3. Measured

The rows were registered on [#2085](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7056154482) before any run was read.

**On the research branch** (the probe, where the contributor's figures were read), `5be3e1eb` against `6be738a5` ([44854748630](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44854748630) against [44840566884](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44840566884)):
- **Housing (concrete, as built):** faces refused as `disconnected UV boundary` fall from 351 to 199; the other 152 mesh, and none takes a new reason.
- **Office (concrete, as built):** 142 → 58, with 84 meshing.
- Paired by graph and face, no face that meshes on `6be738a5` is refused, and no other refused face changes its reason.
- Every ring still refused (194 and 51) has a step of 0.00001 ft or more. None is held back by a short edge or by both of an edge's ends touching the ring.
- **On the contributor's save** ([#2085](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7056173710)), the same probe on `5be3e1eb`: 36 of the 84 cylinder faces refused as disconnected mesh and 48 stay refused, with no meshed face lost. The chairs' coverage of Revit's surface within 0.01 ft rises from 65.6–65.8% to 67.55–67.63%, against 70.3–70.5% for the bridging candidate. All 448 chairs stay partial.

**On `main`,** the join alone (`42fad511` against `0d0fc927`):
- **Cracks** ([44857637927](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44857637927), the crack probe on `818ccf1d` against `1eea412b`, on BIM4LCA and Autodesk's samples). Of the 5,927 bodies the probe tracks, 2 change, and none gains a boundary vertex over 0.001 ft. The newly drawn mullions are not among them; their evidence is their fit to Revit's export, below. Both stay refused, on the next face's reason: RE1 Architecture's 356990, now `a trim loop does not close in the surface's parameters`, and Snowdon Towers' 1135940, now `shared curved edge leaves adjacent plane`. 12 of the 77 exports differ.

- **Full Measure** ([44857635338](https://github.com/DrunkOnJava/rvt-rs/actions/runs/44857635338)): 13 of 120 exports differ, the held-out AA house's among them (not kept), and 11 of 102 `diagnostics.json`.
- **Bodies** (`ifc_body_diff.py` on the kept exports): no drawn body's geometry changes, and none stops being drawn. 67 objects change their refusal text, each naming `disconnected UV boundary` in full on `main`. They stay refused, and three reasons rise:
  - `disconnected parametric trim in model coordinates` on 60: 15 terminals on each office file, which keep `disconnected UV boundary` from another face;
  - `a trim loop does not close in the surface's parameters` on 1: RE1 Architecture's furniture 356990, which keeps it too;
  - `shared curved edge leaves adjacent plane` on 6: Snowdon Towers' terminal 1135940, and one proxy in each `rac_basic` save.
- **Newly drawn:** six bodies, the BIM4LCA office's curtain mullions 3029185 and 3029301 (both on each concrete file, 3029301 on each timber file). Each also named the reason on `main`. Read on every vertex against Revit's export, each is within 0.000003 ft both ways, with volume ratio 1.0000. The bodies, walls and refusal texts are read from the kept exports by `research-tools/2085/` on `refs/archive/2026-11-21/measure-2085-reads` (`dfb7cb34`).
- **Walls:** four walls gain their five base quantities, read from their saved meshes once those close (RE-388). They are 1347016 on the housing at permit, and 3011915, 3454517 and 3455071 on the office as built. `qtos_vs_ifc`'s "equal" rises by 5 and 15 there; "different" (0 and 1) and "values Revit does not write" (0) do not move.
- **Lines that follow the mullions:**
  - `saved_meshes_vs_ifc` counts them, each within its box;
  - `oriented_boxes_vs_ifc` loses the boxes they were (2 → 0, and 1 → 0 on the timber files);
  - `schema.txt`'s swept solids fall by as many;
  - `mesh_bodies_vs_ifc`'s mullion totals move.

  The office terminals' refusal text gains `disconnected parametric trim in model coordinates`, the next face's reason on `main`.
- **Held-out:** all 18 models' counts are identical. The same-project gates pass on `42fad511`.

## 4. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=5be3e1eb436de24b4f415a73acc1d1b6f070ae14 -f base=none -f nordic=true -f models='nordic-housing-concrete-as-built nordic-office-as-built' -f probe=probe_2085_trim_breaks -f scorers=none
gh workflow run measure.yml --ref main -f ref=42fad51151acf66414bb3ebe16e58cda514af986 -f base=0d0fc927d13936b8712966bdf4857f1fa0becb51 -f nordic=true -f r2025=true -f samples=true -f holdout=true -f keep_ifc=true
gh workflow run measure.yml --ref main -f ref=818ccf1dcda5646d85726d3fe518a42bcd6c43b9 -f base=1eea412b7926c1b29ed6d8c7906f65ef7a6305a7 -f nordic=true -f samples=true -f probe=probe_2098_face_cracks -f scorers=none
```

`5be3e1eb` is kept as `refs/archive/2026-11-21/research-2085-ring-join`; `818ccf1d` and `1eea412b` as `refs/archive/2026-11-21/measure-2085-touch-slice-cracks` and `-touch-main-cracks`.

## 5. Not claimed

- **Gaps from 0.00001 to 0.0001 ft:** not read; they stay refused.
- **Gaps from 0.0001 ft up:** whether the file stores the missing piece of edge is not read; they stay refused.
- **The contributor's chairs:** none draws whole; four other faults remain (#2085).
