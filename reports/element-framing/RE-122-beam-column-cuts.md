# RE-122 — Beams stop at the columns their records name

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023); follows RE-29 (walls named in a record's reference list), RE-49 (beam lines and the support trims they do not carry) and RE-119
**Artefacts:**
- `255ribeiro/intro_ifc` `modelo_bim.rvt` (Revit 2023, local only, no licence) with Revit's own IFC4 export;
- Autodesk's `Snowdon Towers Sample Structural.rvt` (Revit 2024, local only) with `vimaec/vim-hackathon` `Snowdon.r2027.vim`, a VIM export of a later edition (MIT repository; model content Autodesk's, local only).

**Result:** positive on Revit 2023, shipped: a beam's reference list names the column each of its ends lies in, and Revit's export stops the beam at that column's near face. On Revit 2024 the naming holds too, but most steel beams stop a stored join cutback short of the face, and when Revit applies the cutback is not decoded, so 2024 beams are unchanged. Where the cutbacks are stored is recorded here.

## 1. Revit 2023

modelo_bim's eight beams (four at 3.2 to 4.0 m, four foundation beams at -0.8 to 0.0 m, RE-119) span between four columns. Each record box runs to the column centres; Revit's export stops each beam at the column faces, 0.3 m short at each end.

`examples/probe_re122_beam_column_refs.rs` lists, for each beam end, the columns whose record box holds it in plan and overlaps the beam in height, and the ones the beam's reference list names:

- Each upper beam end is held by one column, and the beam names it.
- Each foundation beam end is held by two columns, the one below the Level and the one standing on it (their boxes touch at 0.0 m). The beam names only the one below.

So the name is the signal, as it is for walls joined to walls (RE-29). `element_record_beam_cuts::beam_column_trims` cuts each end held by exactly one named column back to that column's near face; an end two named columns hold declines the beam.

| modelo_bim | Revit's box (1e-3 m) |
|---|---:|
| beams, record box (before) | 0 of 8 |
| beams, RE-122 | 8 of 8 |

Columns, walls and slabs are unchanged. Exemplo_data and Einhoven have no beams. Revit 2024 and 2025 records are not trimmed.

## 2. Revit 2024: named columns and stored cutbacks

`examples/probe_re122_beam_supports.rs` extends RE-49's probe on Snowdon Towers' structural model. Of the 942 beams, it takes the level beams on a model axis whose VIM mesh is comparable (RE-49's filters). For each end it finds the columns whose record box holds it:

| beam end | ends |
|---|---:|
| no column holds it, mesh at the line's end | 1,000 |
| no column holds it, mesh trimmed | 139 |
| a column holds it, not named by the beam | 5 |
| one named column, mesh at its face | 83 |
| one named column, mesh 0.042 ft (1/2") short of its face | 249 |
| one named column, mesh 0.125 ft (1-1/2") short of its face | 30 |
| one named column, mesh at the line's end | 22 |

**Where the cutbacks are stored.** A beam's element data stores two `f64`, the start's and the end's join cutbacks in line order, 0x2a bytes after 16 bytes of `ff` followed by `02 00 00 00 02 00 00 00` (at +0x445 on most beams, +0x46f on others). The probe finds that pair on every one of the 246 beams with an end at a named column. On 172 of them, the measured gaps are the stored pair: on 37 the two ends differ and the pair is in line order, never reversed.

On the other 74 the mesh ends at the face although the pair holds 1/2":
- 32 concrete beams;
- 14 bar joists;
- 28 W-shape steel beams.

The column's family does not separate the 28: they frame into W-shape columns, as 160 of the matching W-shape beams do. The VIM is a later edition, and its beams may have been edited.

So on 2024 the face alone is exact on 83 ends and a cutback off on 279. The stored cutback would be exact on the 172 beams but wrong by 1/2" on the 74, and on the 22 untrimmed ends either rule is wrong by half a column. Nothing read so far says which case a beam is in, so 2024 is left as RE-49 left it: the full line.

## 3. Reproduce

| file | sha256 |
|---|---|
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |
| Snowdon Towers Sample Structural.rvt | `6dc833f9fe9fa702607265f2e3d581a6d2a928ea56133db8225c9b5e0f79a44f` |
| Snowdon.r2027.vim | `3e7de6364b4592f2281f7453053f7fa7b9f5ac0e703d41ff0cdd5c278eed387e` |

```bash
cargo run --profile ci --example probe_re122_beam_column_refs -- modelo_bim.rvt
./target/ci/rvt-ifc modelo_bim.rvt -o modelo.ifc
cargo run --profile ci --example probe_re122_beam_supports -- \
    "Snowdon Towers Sample Structural.rvt" Snowdon.r2027.vim "Snowdon Towers Sample Structural"
```
