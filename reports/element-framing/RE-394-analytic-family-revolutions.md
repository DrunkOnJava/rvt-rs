# RE-394: Analytic standalone family revolutions

**Date:** 2026-11-10. **Issue:** #156. Follows RE-391, RE-392 and RE-393.

## Result and scope

The default `rvt-ifc FAMILY.rfa -o FAMILY.ifc` path now writes a supported
standalone Revit 2020 revolution as an `IfcBuildingElementProxy` containing
an analytic `IfcRevolvedAreaSolid`. Its profile, axis, origin and angular
interval come from the shared saved-record reader in RE-393. No display mesh,
fixture recipe or native reference supplies the exported coordinates.

Eligibility requires `PartAtom`, no `ProjectInformation`, exactly one visible
positive revolution, no other recognized model form or nested instance, and
no already exported building element. The reader refuses voids, flipped or
ambiguous records, unsupported curves, holes and unsupported angular/profile
configurations. The exporter additionally refuses any profile containing an arc
unless its sweep is a full turn (within 1e-10 radians). The skipped note is
`partial revolution with an arc profile lacks native export confirmation`.
Export coverage is REV-A, REV-B, REV-BT and ARC-A, confirmed by Revit's own IFC;
ARC-B and ARC-BT remain reader research evidence and are not exported;
mixed line/arc profiles, project instances and other releases are unmeasured.
An unsupported candidate leaves the model unchanged and records a reason.

The source ElementId is the IFC Tag. `RvtFamilyFormGeometry.BodySource` is
`family_revolution`; `RevitClass` is `RevolutionElem`. Diagnostics count its
geometry without claiming a recovered building-product category. The default
`scaffold` policy permits it; `typed-no-geometry`, `geometry` and `strict`
still require mapped types. Placeholder and diagnostic-proxy outputs are
unchanged. Cancellation is checked during record reading and before attachment.

## Analytic mapping

The `IfcIndexedPolyCurve` preserves lines and three-point arcs. Its coordinates
are radius (X) and axial distance (Y), in metres. The local revolution axis is
+Y at the local origin. Position has X along the radial vector rotated by the
saved starting angle, Y along the directed axis, Z = X cross Y, and its origin
at the saved axis origin. Sweep is end minus start, in radians.

This follows buildingSMART's [IfcRevolvedAreaSolid definition](https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD1/HTML/schema/ifcgeometricmodelresource/lexical/ifcrevolvedareasolid.htm):
the profile and axis share the XY plane before Position is applied. Directions
and sweep retain 15 decimal places; a full turn is normalized to 2*pi rather
than truncated to six decimals. Profile coordinates use the writer's existing
micrometre precision. The adapter refuses merged rounded points, collinear
arc points and arcs that the profile writer would collapse into lines.

`PlacedRevolvedArea` leaves the existing `RevolvedArea` callers/output intact.
The viewer independently transforms its usual display mesh through this frame.
Existing project-instance and optional family-extrusion paths are unchanged.

## Reproduction and measured results

Use the six-file archive, SHA-256 and setup instructions in
[the runner README](../../tools/oracle/runner/revolutions/README.md#complete-six-fixture-package-replay).
Its manifest identifies each cleaned RFA and native reference. RE-392 records
the circular byte evidence; RE-393 describes the compiled reader checks.

`verify_package.py` rebuilds the evidence and reader results, then calls
`verify_export.py` on the actual exporter. It checks IFC schema/EXPRESS rules,
segment types, source identity, diagnostics, start angles and tilted placement.
Separate OpenCascade processes import each IFC with default settings and with
20-micrometre linear / 0.03-radian angular deflection. Each has a 120-second
limit. Fine meshes must be closed and oriented with positive volume, below
0.01% relative volume error and 0.3048 mm sampled distance.

Baseline measurements for the four retained exports, IfcOpenShell 0.9.0.post1:

| Fixture | IFC bytes | Default import (s) | Fine import (s) | Max sampled gap/bound (mm) | Volume error (%) |
| --- | ---: | ---: | ---: | ---: | ---: |
| REV-A | 9087 | 0.019 | 0.229 | 0.009989 | 0.00205396 |
| REV-B | 9087 | 0.015 | 0.038 | 0.009869 | 0.00201420 |
| REV-BT | 9088 | 0.017 | 0.050 | 0.009869 | 0.00201420 |
| ARC-A | 9051 | 0.064 | 93.915 | 0.031170 | 0.00575802 |
| ARC-B | Not exported | — | — | — | — |
| ARC-BT | Not exported | — | — | — | — |

Four supported exports pass; the two partial arc cases must be refused.
Timings are local observations, not a performance guarantee.
The full torus is 9,051 bytes and imports normally in 0.064 seconds; fine
tessellation remains substantially more expensive. An earlier 3-micrometre /
0.01-radian meshing experiment exceeded 120 seconds, so ordinary and fine
imports are reported separately without relaxing the geometry acceptance bars.

Rectangular references are native IFCs. ARC-A also agrees with Revit's own IFC
(RE-392: 0.017 mm, 0.0075% volume). Circular face captures are supplementary
research measurements, not export confirmation. ARC-B's native IFC differs by
141 mm and 25% volume; ARC-BT's native IFC tessellation exceeded 120 seconds
without a verdict. Both remain in the evidence package but are refused by the
exporter until a native export or saved mesh confirms them.

The exporter first collects RevolutionElem ids without decoding sketch graphs.
Without a revolution it returns before the supporting-record budget or graph
decoding, avoiding false skipped notes for sketch-heavy families.
Eight existing Revit 2020 families without revolutions produce no revolution
skipped note and retain their source hashes. None exceeds 2,000 supporting
records; the exact over-budget case has no saved-file fixture in this replay.

The configured CLI contract checks four exports and two partial-arc refusals:
analytic output, finite display mesh, source immutability, three mapped-type refusals and placeholder
behavior. All 302 integration checks pass. Required local gates also passed:
formatting, workspace clippy, strict compile, rustdoc, all-target/all-feature
build, 27 doc tests, 15 MCP checks, network audit, examples and probes.
Supplementary regression on 23 existing files gives 46 byte-identical exports
in default and optional family-extrusion modes. Those installed samples are
not required by the redistributable replay. Hosted CI and the full Measure
comparison must run on the revised head after publication.
