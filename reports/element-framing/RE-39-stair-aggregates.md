# RE-39 — Stairs as IFC aggregates of their runs, landings and stringers

**Date:** 2026-09-23
**Result:**
- **Positive.** A stair's runs, landings and stringers each name their stair's ElementId in their reference list. rvt-rs now exports each stair as an `IfcStair` with no body of its own and an `IfcRelAggregates` to those parts, as Revit's own IFC4 export does.
- On Snowdon Towers Architectural every aggregate relation rvt-rs writes is one Revit's export has. The same 26 stairs aggregate in both, and all 228 parts rvt-rs attaches are in the matching Revit aggregate.
- **Measured and left out:** railings. 70 of them name a stair, Revit aggregates 65 of those, and nothing read so far tells the other 5 apart, so railings stay standalone elements (§3).

**Oracle:** Snowdon Towers Sample Architectural (`bldrs-ai/test-models`, local only, `.rvt` sha256 `33271010…`) and Revit's IFC4 export of it (`ecfcb04e…`). No licensed file in CI has stairs, so the evidence is local.

**Probe:** `examples/probe_re39_stair_parts.rs` counts, per part category, the placed instances whose reference list names exactly one placed stair.

-----

## 1. The link

`probe_re39_stair_parts` on Snowdon Towers (27 placed stairs):

| part | name one stair | none | more than one |
|---|---:|---:|---:|
| runs (`OST_StairsRuns` −2000919) | 42 | 0 | 0 |
| landings (`OST_StairsLandings` −2000920) | 17 | 0 | 0 |
| stringers (`OST_StairsStringerCarriage` −2000123) | 170 | 0 | 1 |
| railings (`OST_StairsRailing` −2000126) | 70 | 61 | 0 |

The stringer that names two stairs stays a standalone `IfcMember` (fail closed). Revit's export puts it under stair 746421.

## 2. What exports

| entity | exported | in Revit's export |
|---|---:|---:|
| `IfcStair` (no body) | 27 | 27 of 27 |
| `IfcStairFlight` | 42 | 42 of 43 |
| `IfcSlab` `.LANDING.` | 17 | 17 |
| `IfcMember` `.STRINGER.` | 170 | 170 |

Against Revit's `IfcRelAggregates`, rvt-rs writes 26 stair aggregates for the same 26 stairs. Every part it attaches is in the matching Revit aggregate: 169 stringers, 42 flights and 17 landings. Revit's aggregates hold 67 parts more:
- the 65 railings of §3;
- the stringer that names two stairs;
- one run that has no placed-instance frame rvt-rs can read.

A part reaches the spatial structure through its stair and is not also contained in a storey (IFC4's decomposition rule). The IfcOpenShell validator reports no issue on the 127,070-instance export, and the schema-arity gate passes.

`IfcStair` and `IfcStairFlight` leave their `PredefinedType` unset. Revit writes per-instance values (`HALF_TURN_STAIR`, `STRAIGHT`, …) that no decoded byte gives.

## 3. Railings

Every one of the 70 railings that names a stair names exactly one. Revit aggregates 65 of them under that stair and exports the other 5 standalone. The two groups agree on everything compared:
- the frame's reference lists and second list;
- the flags at `+0x46`;
- the sentinel slots;
- the ElemTable owner field (RE-31), which is the stair for only 10 of the 65.

Railings therefore stay standalone `IfcRailing` elements, contained in their storey as before.

## 4. What this does not claim

- Roofs are §5.
- Stair bodies are their parts' record bounding boxes. A run's box is an envelope of its treads, not the treads.
- Revit 2024 and 2025 only.

## 5. Roofs

The 20 placed `OST_Roofs` (−2000035) instances on Snowdon Towers are all `IfcRoof` `Tag`s in Revit's IFC4 export, which writes them in two shapes:
- 13 are an `IfcRoof` with a body of its own;
- 7 are a bodiless `IfcRoof` aggregating one `IfcSlab` `.ROOF.` with the same `Tag`.

No decoded byte says which shape a roof gets. rvt-rs therefore writes each roof as one `IfcRoof` with the record's box. All 20 `Tag`s are Revit's. For the 7 decomposed roofs, the entity structure is simpler than Revit's. Core Interior, the RE1 models and the structural sample have no roof records, and their exports are unchanged.

