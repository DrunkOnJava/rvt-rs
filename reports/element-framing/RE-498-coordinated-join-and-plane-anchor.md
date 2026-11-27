# RE-498: Two rules for faces whose stored edges part by a hair, fitted to one chair family: the coordinated join and the plane anchor

**Date:** 2026-11-27
**Issues:** #2085.
**Artefacts:** a contributor's chair family on one Revit 2024 save, with Revit's own IFC export of the same save, read on their machine (counts only; not in Measure); on Measure, the BIM4LCA office and housing files (VTT, CC BY-SA 4.0; Revit 2024) and RE1 (Revit 2025, MIT), each with Revit's own IFC4 export, Autodesk's sample projects of Revit 2015 to 2027, which have no Revit export, the families, the `rac_advanced` pairs and the held-out models.
**Status:** positive on the one family both were fitted to, where Revit's export keeps the end each rule names at every observation, measured with a 4,000,000-value graph budget; at `main`'s default limit of 1,000,000 the chairs' saved graph (1,789,525 values) is refused before either rule is reached. On Measure's models the plane anchor draws one body more, a cooktop on each of Autodesk's `rac_basic` saves of 2023 to 2027, which no Revit export checks; the coordinated join takes no face; no other export changes. Both on by default (Griffin: the coordinated join on 2026-11-26, the plane anchor on 2026-11-27).

## 1. What refuses these faces

**Two cylinder or revolved rings that part at the same corner.** A face's ring joins its loop's edges end to end within 1e-7 in its parameters, and on a cylinder or revolved face a ring whose stored edges part by more is refused (`disconnected UV boundary` or `disconnected parametric trim in model coordinates`). On the chairs, two neighbouring rings hold a gap at the same stored end of the one edge they share, with their other edges leading to separate `HermiteSurf` faces; one ring reads the gap `last/first` and the other `first/first` or `last/last`. At all 8,960 such cylinder corners the two views name the same stored point, and Revit's export holds it ([#2085, issuecomment-7076146484](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7076146484)); the revolved faces show the same pairing at eight stored corner pairs ([issuecomment-7077046366](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7077046366)).

**A flat face beside a curved one.** A flat face meshes the edge it shares with a curved face from the curved face's samples, projected onto the plane (`refined_plane_edge`), and where one lies off the plane the flat face is refused as `shared curved edge leaves adjacent plane`, and with it the body. On the chairs one end sample of the curved face leaves the plane at four distinct shared edges of the family's two plane faces, 896 end samples across the 448 placements. Revit's export holds the plane's own saved sample at all 896 and the curved face's at none, within 1e-5 ft, and the two lie about 3.88e-4 ft apart ([issuecomment-7077046366](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7077046366)).

## 2. The rules

- **The coordinated join** (`coordinated_keeps`, `COORDINATED_LIMIT`): where two cylinder or two revolved rings each hold a stored gap under 3.75e-4 ft at the same corner, in the pattern above, each ring keeps the stored point both views name and the loop's polygon meets there. A gap in any other pattern, or at or over the limit, is refused as before.
- **The plane anchor** (`refined_plane_edge`, `PLANE_ANCHOR_LIMIT`): at a flat face's edge shared with a curved face, an end sample of the curved face's that leaves the plane is taken as the plane's own saved sample there, where the two lie within 3.9e-4 ft, just over the chairs' 3.88e-4. The samples between must still lie on the plane.

## 3. Measured

**On the chairs** (the contributor's arms, [issuecomment-7182584020](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7182584020), at `0bccdc09`, with the research branch's 4,000,000-value graph budget): every gap the join takes is under 3.75e-4 ft in its own measure (96 gap views, 80 on cylinders and 16 on revolved faces), and Revit's export holds only the named end in all 17,920 cylinder and 3,584 revolution observations. The share of Revit's surface samples the reader's mesh covers goes from 67.55–67.63% with both rules off to 71.41–71.61% with the coordinated join (48 cylinder, 14 planar and eight revolved faces recovered), and to 84.30–84.33% with the plane anchor as well (two planar faces more). No face meshed before is refused, and the four control meshes are byte-identical. No chair is drawn whole in any arm: each still stops at a Hermite trim, which `main` does not read (RE-460). At `main`'s default limit of 1,000,000 values none of this is reached: the chairs' saved graph holds 1,789,525 ([issuecomment-7183799604](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7183799604)), and with that limit all 448 stop at the graph budget and emit no triangle ([issuecomment-7057648442](https://github.com/DrunkOnJava/rvt-rs/issues/2085#issuecomment-7057648442)). `rvt-ifc --saved-mesh-graph-values` raises it.

**On Measure's models** ([Measure 46058172915](https://github.com/DrunkOnJava/rvt-rs/actions/runs/46058172915) at `18d37a14`, which differs from this change only in comments, against `main` at `6d33d7d2`, every model Measure holds with the samples, held-out files, `rac_advanced` and families): 139 of 144 exports and 115 of 120 diagnostics are identical. The five that differ are the `rac_basic` saves of 2023 to 2027, one body each, and the same-project gates pass. On each of Autodesk's `rac_basic` saves of 2023 to 2027 the body that moves goes from its record box to its saved mesh, and `main` had refused it as `shared curved edge leaves adjacent plane`, the plane anchor's case. On the 2024 save it is a Miele induction cooktop (ElementId 680539): its 7,152-triangle mesh spans its record box to the millimetre (0.794 by 0.534 by 0.096 m) and holds 31 per cent of the box's volume, 0.01252 of 0.0407 m³ ([Measure 46062940579](https://github.com/DrunkOnJava/rvt-rs/actions/runs/46062940579), whose 2024 export hashes as the first run's). No Revit export of that sample exists in Measure, so the body is checked against nothing but its own box. With the coordinated join off ([Measure 46060234028](https://github.com/DrunkOnJava/rvt-rs/actions/runs/46060234028) at `74719370`), every one of the 144 exports the two runs share hashes as with it on, so the join takes no face on Measure's models: the cooktop is the plane anchor's.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=18d37a148473f7d897af0f53179e83624f5ca293 -f base=6d33d7d2ccba12771be3796e06fbdd93392106b2 \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true -f racadv=true -f families=true -f timing=true
gh workflow run measure.yml --ref main -f ref=18d37a148473f7d897af0f53179e83624f5ca293 -f base=6d33d7d2ccba12771be3796e06fbdd93392106b2 \
  -f samples=true -f sample_years="2024 2024" -f models=sample-2024-racbasicsampleproject -f keep_ifc=true -f scorers=none
```

The cooktop's figures are read from the second run's kept export with IfcOpenShell (world coordinates): its triangles, the axis-aligned box of its vertices against its record box's, and the volume of its closed mesh (the signed sum over its triangles) over the box's.

## 5. Not claimed

- That either rule is right on any file with a Revit export and a face it takes: the chairs are not in Measure, and the cooktop has no export.
- That either rule reaches the chairs at the default graph limit: it does not.
- That 3.75e-4 ft and 3.9e-4 ft are the limits Revit applies. Both are fitted to one family on one save.
