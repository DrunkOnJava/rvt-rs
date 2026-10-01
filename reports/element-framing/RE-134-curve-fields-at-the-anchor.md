# RE-134: the curve fields at a duct's or pipe's connector anchor

**Date:** 2026-09-30
**Issues:** #96; Discussion #112
**Artefacts:** `RE1-Mechanical.rvt` (Revit 2025, MIT, `d5597201…9c014b`; 25 ducts and 6 pipes) and `RE1-Plumbing.rvt` (Revit 2025, MIT, `03689df2…f9e54c`; 63 pipes), with Revit's own IFC exports of both, all from `Drshelden/IFC-ECS`. Measured on GitHub's hosted runners with the Measure workflow, run 36736022239 on commit e30bf45, whose probe and scorers are byte-identical to this PR's.
**Probe:** `examples/probe_re134_duct_connectors.rs`, scored by `tools/re/curve_fields_vs_ifc.py`. Duct bodies are scored by `tools/re/duct_bodies_vs_ifc.py` in every Measure run.
**Status:** positive for where a duct's or pipe's type, width and height are stored; negative for the round versus rectangular switch, which the redistributable corpus cannot isolate.
**Credit:** jakobhirn-bit (Discussion #112) found the anchor and the size that follows it (RE-131) and named the missing switch.

## 1. Where the fields are

A duct is an `RbsDuctCurve` (tag 3371 on Revit 2025) and a pipe an `RbsPipeCurve` (3432), both deriving from `RbsSingleCurve`. Revit 2025's schema declares these fields for `RbsCurve` (tag 688, deriving from `HostObj`), in this order: `m_pConnectorManager`, `m_pCurveDriver`, `m_dWidthOrDiameter`, `m_dHeight`, `m_vNormal`, `m_dOffsetStart`, `m_dOffsetEnd`, `m_idType`, `m_idSystemCategory`, `m_idSegment`, and ten more. In the file `m_pConnectorManager` is written as the tag of `RbsCurveConnectorManager` (3361), `0xFF`×4 and the next tag, the anchor RE-131 reads a size after. The values after it follow that order of fields, with `m_pCurveDriver` not between them; that `RbsSingleCurve` derives from `RbsCurve` is inferred from this, not read:

| from the anchor | type | field | measured |
|---:|---|---|---|
| +0 | `u16`, `0xFF`×4, `u16` | `m_pConnectorManager` | the anchor |
| +8 | `f64` | `m_dWidthOrDiameter` | ducts: two of Revit's three extrusion dimensions, 25 of 25; pipes: equal to the height, 69 of 69 |
| +16 | `f64` | `m_dHeight` | as above |
| +24 | 3 × `f64` | `m_vNormal` | seen, not compared: a vertical duct (-1, 0, 0), a pipe (0, 0, 1) |
| +48, +56 | 2 × `f64` | `m_dOffsetStart`, `m_dOffsetEnd` | seen, not compared: on the vertical duct 435175 they are its two ends' elevations (10.2677 and 9.7881 ft), on pipe 441731 both 9.9675 ft |
| +64 | `u64` | `m_idType` | the ElementId of the type: Revit's type `Tag`, 94 of 94 |
| +72 | `u64` | `m_idSystemCategory` | `0xFF`×8 (unset) on the ducts and pipes seen |
| +80 | `u64` | `m_idSegment` | a pipe segment's ElementId on pipe 441731 (191066, beside its type 191067), unset on the ducts seen |
| +514, +532, +550 | UTF-16 | three copies of the size text (the duct design properties hold three size strings) | ducts: the text at +514 is width × height in millimetres, 25 of 25 |

## 2. Finding an element's anchor

An element's anchor is the nearest one before its first connector entry (RE-131), within 2,000 bytes. The nearest entry lies 1,292 or 1,324 bytes after a duct's anchor and 1,253 to 1,289 after a pipe's; all 25 ducts and 69 pipes have one. RE1 Mechanical has 31 anchors (25 + 6) and Plumbing 63, so the anchors are one per curve.

## 3. Result

`tools/re/curve_fields_vs_ifc.py`, on RE1 Mechanical and then RE1 Plumbing:

```
ducts in the probe: 25; pipes: 6
ducts with an anchor before their entries: 25 of 25
ducts also in Revit's export: 25
type id at anchor+64 is the Tag of Revit's type: 25 of 25
pipes with an anchor before their entries: 6 of 6
pipes also in Revit's export: 6
type id at anchor+64 is the Tag of Revit's type: 6 of 6
duct width and height at anchor+8 and +16 are two of Revit's profile and depth: 25 of 25
duct size text at anchor+514 is width x height in millimetres: 25 of 25
pipe height equals width at the anchor: 6 of 6
pipe nominal sizes, millimetres: {20: 5, 32: 1}

ducts in the probe: 0; pipes: 63
pipes with an anchor before their entries: 63 of 63
pipes also in Revit's export: 63
type id at anchor+64 is the Tag of Revit's type: 63 of 63
pipe height equals width at the anchor: 63 of 63
pipe nominal sizes, millimetres: {10: 1, 15: 38, 20: 12, 40: 8, 50: 2, 100: 2}
```

The type link closes RE-129's negative. That report found no link from a pipe or duct to the element that holds its type's name in the record, its reference list or its name entries, and RE-130 got the 69 pipes' types from the reference list instead. The link is in the curve's own object: `m_idType` at +64 is the type's ElementId for all 94, the 25 ducts included, whose type `233113-DUCT-Tees` is `IfcDuctSegmentType` `Rectangular Duct:233113-DUCT-Tees` with `Tag` 53292 in Revit's export.

The pipe sizes are nominal, not the outside diameter RE-131 reads from the record's box: pipe 441731 is 32 mm here and its box is 31.75 mm across (0.10417 ft).

## 4. Duct bodies are already Revit's, on RE1

For each of the 25 ducts of RE1 Mechanical, the world-space box of the extruded rectangle rvt-rs writes on main is Revit's (`tools/re/duct_bodies_vs_ifc.py`, in the same run):

```
ducts in Revit's export with a body: 25; rvt-rs writes one for 25 of them
world box within 0.01 ft of Revit's: 25 of 25 (worst 0.00000 ft)
```

Revit sometimes writes the profile's two sides the other way round from rvt-rs and turns the extrusion to suit, and the box is the same. An axis-aligned duct's solid is its record box, so on RE1 there is no duct geometry left to recover; a duct that is turned in plan or sloped would need its ends (connector entries) and its width and height (above), and RE1 has none.

## 5. What is not established: the family

Revit names every duct and duct type `Rectangular Duct:<type>[:<id>]`, and the file stores no name for that family in anything read here. All 25 ducts are rectangular in Revit's export and no redistributable model has a round or an oval duct, so the switch cannot be measured:

- A round duct is stored as width = height = diameter, as every pipe here is (69 of 69), so at the anchor it looks like a square duct, and 11 of the 25 ducts are square (`300x300`).
- The size text is `WxH` for all 25. A round duct's would carry the project's round-duct prefix or suffix (`RbsDuctSettingsElem` declares `m_strRoundDuctSizePrefix` and `m_strRoundDuctSizeSuffix`), which nothing here reads; whether an oval duct's text differs from a rectangular one's depends on the project's separator and suffix settings, which were not read either.
- The only byte position after a connector entry's point where duct entries and pipe entries take disjoint values is its first `u32` (1 on ducts, 3 on pipes). It separates the two kinds of element, and reads as the connector's domain (HVAC or piping), not its shape.

So rvt-rs keeps naming ducts `Duct-<ElementId>`. What would isolate the switch is a model with round, oval and rectangular ducts and Revit's IFC of it. A model authored for the purpose can be licensed like the RE1 set.

## 6. Not measured

- Sloped or turned pipes and ducts, and pipe and duct fittings: every RE1 pipe is along a principal axis (RE-131) and every duct's box is its solid (section 4), and the fittings' geometry is in their families.
- `m_vNormal` and the elevation offsets, which are seen but not compared with Revit's export.
- Connector references: each curve's manager also lists, near +1,060 from the anchor, the elements at its connectors and their connector indices (a duct's own id, then the fitting joined to it). They are how Revit's `IfcRelConnectsPorts` (73 in RE1 Mechanical, 126 in Plumbing) would be written, and they are decoded in RE-138.
