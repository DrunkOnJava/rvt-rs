# RE-121 — A door's record box does not give its body (negative)

**Date:** 2026-09-28
**Issues:** #227 (doors and windows); follows RE-94 (door openings) and RE-120
**Artefacts:**
- `2024_Core_Interior.rvt` (Revit 2024, MIT, magnetar-io) and its reference export `2024_Core_Interior_slim.ifc`;
- `RE1-Architecture.rvt` (Revit 2025, MIT, Drshelden/IFC-ECS) with Revit's export;
- `Snowdon Towers Sample Architectural.rvt` (Revit 2024, local only) with Revit's IFC4 export;
- `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023, local only) with Revit's IFC4 exports.

**Result:** negative. rvt-rs draws a door as its record box, and on every release that box reaches past Revit's door body across the wall, on one side, by up to 0.9 m. One rule gives Revit's body wherever the body is centred on its wall: across the wall, keep the side nearer the wall's centreline and reflect it onto the far side. On Snowdon Towers the body is not centred on 124 of 126 hosted doors, and nothing read so far says which case a door is in, so nothing ships.

## 1. The observation

On Exemplo_data, door 337290's box ends 0.89 m past Revit's body on one side of its wall, and at Revit's body on the other side. Along the wall and in height, rvt-rs's box is Revit's. On the other doors of the two 2023 projects the excess is 0.59 to 0.9 m, and on RE1 Architecture (2025) 0.20 to 0.73 m, always on one side. That is the reach of a door leaf swung open: the record box is the element's model box, and it appears to hold the swing.

## 2. The mirror rule, measured

`tools/re/door_bodies_vs_ifc.py` (tolerance 0.01 ft, internal feet, host wall from rvt-rs's own opening relations):

| file | release | hosted doors | mirror rule gives Revit's | Revit's body centred on the wall |
|---|---:|---:|---:|---:|
| 2024_Core_Interior | 2024 | 132 | 132 | 132 |
| RE1-Architecture | 2025 | 4 | 4 | 4 |
| modelo_bim | 2023 | 1 | 1 | 1 |
| Exemplo_data | 2023 | 10 | 8 | 9 |
| Snowdon Towers Architectural | 2024 | 126 | 0 | 2 |

The rule holds exactly where the body is centred. It fails on the two Exemplo_data doors that are otherwise:
- one differs along the wall too (0.063 and 0.124 ft), so its box holds more than the swing;
- one is a double door whose body sits 1.376 ft off the wall's centre.

Snowdon Towers' door families sit their frame at one face of the wall. Door 631418 (internal feet across the wall):

| | from | to |
|---|---:|---:|
| host wall | -100.625 | -99.625 |
| rvt-rs's box | -103.542 | -99.583 |
| Revit's body | -100.748 | -100.063 |

Its box holds the swing on one side and reaches past the far face on the other. Door 742710's box holds a swing on both sides.

## 3. What would decide it

Where the body sits across the wall is a property of the door family's geometry, not of the instance's box or its host. A door's type openings (RE-94) give its width and height, not its frame's depth or offset. The body needs either the family's own geometry or a stored frame depth and offset; neither is read.

So rvt-rs keeps drawing the record box, as the fail-closed rule asks: a centred-body guess would be exact on 145 doors and wrong on 126 with no signal telling them apart.

## 4. Windows

The same test on windows (`--class IfcWindow`) is negative too:

| file | hosted windows | mirror rule gives Revit's | Revit's body centred on the wall |
|---|---:|---:|---:|
| modelo_bim | 10 | 10 | 10 |
| Exemplo_data | 9 | 5 | 5 |
| 2024_Core_Interior | 6 | 0 | 6 |
| Snowdon Towers Architectural | 68 | 0 | 2 |

Core Interior's windows are centred, yet their boxes reach 0.801 ft past the body on one side and 4.823 ft on the other: they hold more than a swing. Snowdon's differ from the body on every face.

## 5. Reproduce

| file | sha256 |
|---|---|
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| 2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |
| Snowdon Towers Sample Architectural_IFC4.ifc | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
./target/ci/rvt-ifc 2024_Core_Interior.rvt -o core.ifc
python3 tools/re/door_bodies_vs_ifc.py core.ifc 2024_Core_Interior_slim.ifc --list 10
```
