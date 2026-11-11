# RE-443: a zero ruled-profile pointer selects its saved point

2026-11-11. Positive, bounded to a point and an arc, a unit envelope and a
rectangular trim. Public RVT bytes and Revit's own IFC confirm the rule.
Measurements below are local runs, not a Measure workflow run.

## Finding and scope

A `RuledSurf` can hold one zero `m_pProfileCurve1` or `m_pProfileCurve2`
pointer. Its corresponding `m_Point1` or `m_Point2` is then the constant
profile at that end of the surface. The old reader required two curves and
refused the whole saved body when one of these patches occurred.

On Autodesk's public Revit 2023 basic MEP project, this recovers **107
bodies: 105 duct fittings and two sanitary terminals**. They use 76 distinct
source patches in seven symbols. Default IFC export changes from 176 to 69
bounding-box bodies and from 5,179 to 5,286 saved-mesh bodies, compared with
base `3abb42f0fc9e2dc913f46e93e1ef03590d6e552f`. The body remains an
`IfcTriangulatedFaceSet`; this does not introduce an analytic IFC surface.

The admitted combination is exactly point/arc or arc/point with surface
envelope `[[0,0],[1,1]]` and a single rectangular face trim. Other combinations,
both pointers zero, missing or invalid points, unresolved nonzero pointers,
clipped envelopes and nonrectangular or holed trims remain refused. RE-430's
general trim reader does not widen this new surface's measured scope.
Standalone family revolutions and their partial-arc refusals are unchanged.

## Input and byte witness

The source is downloaded directly from Autodesk; no RVT is redistributed:

- URL: <https://revit.downloads.autodesk.com/download/2023RVT_RTM/Docs/InProd/rmebasicsampleproject.rvt>
- Bytes: `30797824`
- SHA-256: `5b2331579140cc621d23b5642d24b2b7bbb6727e26610156962909d725e44e6a`

`research/collapsed-ruled.json` pins this file, all seven source-record body
hashes, the 107 target element ids and the native-reference package.
`probe_collapsed_ruled` emits every surface's stream, record location,
body-relative offsets, raw bytes, decoded values and refusal controls.

One example is owner **417088**, saved-graphics channel **103**,
`Partitions/12`, group-record offset **49751**. Its decoded body is 21,506
bytes with SHA-256
`19d6bff15ca92792e82ec6af689e1b9a8ca75a7b74fa5a8d4ee907673d9e122d`.
Surface object 138 starts at body offset 13753:

| Field | Body offset | Bytes (hex) | Read value |
|---|---:|---|---|
| `m_pProfileCurve2` | 13792 | `00000000` | zero pointer |
| `m_Point2.x` | 13820 | `0000000000000000` | 0 ft |
| `m_Point2.y` | 13828 | `bef2a3fc283fdabf` | -0.410104986876643 ft |
| `m_Point2.z` | 13836 | `fef2a3fc283fda3f` | 0.41010498687664654 ft |

The other profile is a `GArc`. The point is `(0,-125,125)` mm within floating
roundoff, matching a collapsed control-net edge of native element 419137 to
`6.62e-15` ft. This representative local-coordinate check is supplementary;
the full placed bodies are measured independently below. The probe derives
the point offsets from the recorded object end, then reads the actual 24
bytes as three little-endian doubles and checks them against the graph.

The other source owners are 492340, 539304, 543268, 717794, 718444 and 718635.
Both orders occur: 24 patches have the point first and 52 have it second.
The probe is deliberately specific to this pinned project; it makes no
12-release-corpus claim.

## Evaluation and refusal controls

For normalized `u,v`, each curve is evaluated across its stored parameter
range; the point profile is constant. The existing ruled interpolation
`S(u,v) = (1-v) P1(u) + v P2(u)` then gives the patch. At its collapsed end,
the `u` derivative vanishes. Removing the positive factor `v` (point first)
or `1-v` (point second) gives the one-sided limiting normal, still dependent
on `u`; the stored orientation is applied as before. A singular or nonfinite
normal remains an error. The trim checker permits an absent horizontal side
only at the explicitly saved pole and checks that side collapses spatially.

The byte probe evaluates each of 76 patches, including its pole, and meshes
its real face. Seven mutations of each real graph must fail: missing point,
invalid point, unresolved nonzero pointer, both pointers zero, clipped
envelope, point/line and point/ellipse. One additional mutation of each face
moves an interior trim sample off the rectangle and must fail with
`unqualified collapsed ruled trim`. **608 refusal controls pass**.
These are graph mutations of the measured file, not invented positive files.

## Native reference and body measurements

The pinned RVT was exported without saving it, using Revit **23.1.90.15**,
IFC exporter **23.4.1.0**, IFC4 Design Transfer View, a fresh Fine 3D view,
no section box, `TessellationLevelOfDetail=1`, `UseActiveViewGeometry=true`
and no linked files. The full native IFC SHA-256 is
`629bf5a6e29865eba806d810d10f013c82ecc75588c68b429ce1c20a4e160cda`.
The public API invoked export; an API face capture is not the geometry oracle.

The reference package contains only `README.txt` and `native-mep-107.ifc`.
The IFC is a geometry-only subset of that export: all 107 representation and
placement trees were checked equal to the original, with units and the site
frame retained. Unrelated elements, properties and owner metadata are left
out. It is not presented as an untouched full-project export.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Reference IFC | 281892 | `4faf2915673bcb3ff48137b1ee7350449afef141a0e4ab2f6c8c760e7c874cf5` |
| Reference ZIP | 52288 | `77322bed93a88f027f473cd71578fa27aaeabf9402eaab99a445a1a138d6f93a` |

The replay reads the emitted triangle coordinates directly, preserving
coincident parts, and meshes the native reference with the repository-pinned
IfcOpenShell 0.9.0.post1. Both are transformed to their site frame. Every
unique vertex and every triangle centroid is scored against the other
surface in both directions. Volume is summed over all parts, without
deduplication. Missing elements, duplicate tags or failed native meshing
abort the replay rather than disappearing from the denominator.

| Native reference tessellation | Bodies | Maximum two-way sampled distance | Maximum relative volume difference |
|---|---:|---:|---:|
| Default (primary comparison) | 107 | 0.962523 mm | 0.178475% |
| Fine, supplementary: duct fittings | 105 | 0.131547 mm | See all-body maximum below |
| Fine, supplementary: sanitary terminals | 2 | 0.720662 mm | See all-body maximum below |
| Fine, supplementary: all | 107 | 0.720662 mm | 0.019159% |

**None of the 107 bodies passes the strict 0.001 ft two-way threshold in
the default comparison.** Fine reference tessellation brings 105 of 107
inside that threshold. It does not replace the default verdict. The replay's
explicit bounds are 0.0033 ft / 0.2% by default; fine bounds are 0.0005 ft
for fittings, 0.0025 ft for terminals and 0.025% for volume. These are
regression bounds for this sample, not a new project-wide accuracy claim.

The two sanitary residuals lie at an existing plane/cylinder junction, away
from the new ruled patches. This change does not fix that junction or claim
the full bodies exact. The replay reports both classes separately; the
native export calls the sanitary elements `IfcFlowTerminal`.

## Reproduce without Revit

Use the checkout containing this report and record its immutable commit
with `git rev-parse HEAD` alongside the output. The replay fetches the
source and native-reference archive from the URLs in the JSON pin.

```sh
cargo build --locked --profile ci --bin rvt-ifc
cargo build --locked --profile ci -p rvt-probes --example probe_collapsed_ruled
python3 -m pip install --require-hashes -r tools/ci/requirements-ifc.txt
python3 tools/re/replay_collapsed_ruled.py --work target/collapsed-ruled
```

The sample downloads on demand with exact size and SHA-256 checks. The ZIP
and its IFC member are checked too. `--reference-zip` and `--sample` accept
already downloaded copies but still verify their pins. `--rvt-ifc` and
`--probe` override binary
paths when using a separate Cargo target directory. Add `--fine` for the
slower supplementary run. Scoring uses up to four processes by default;
`--workers 1` reduces memory use, with the same per-body arithmetic. The
optional fine run can take tens of minutes. Results, per-element errors and the byte witnesses
are written under `--work`. No Revit installation or automation is needed.

## Regression comparison

Against the base above, 12 project models were exported before and after:
Autodesk 2023 structural basic, architectural advanced/basic, public MEP and
Golden Nugget; Autodesk 2025 electrical, facades and architectural; BIM4LCA
concrete office as built and timber housing at permit; RE1 Mechanical and
Plumbing. Only the intended 107 MEP bodies change their representation or
placement. Eight other project IFCs are byte-identical. Architectural basic,
Golden Nugget and the office change only 6, 9 and 15 `SavedMeshRefusal`
messages respectively, identifying the still-unsupported profiles or trims.
This local selection is not the full upstream Measure corpus.

All six existing REV-A, REV-B, REV-BT, ARC-A, ARC-B and ARC-BT IFC outputs
are byte-identical to the base, including the unsupported partial-arc cases.
The input files' hashes are unchanged. The public replay starts from a fresh
Autodesk download and independently reproduces the 107-body measurement.
