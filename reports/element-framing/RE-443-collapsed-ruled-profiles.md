# RE-443: a zero ruled-profile pointer selects its saved point

2026-11-11; admission narrowed 2026-11-12. Positive, bounded to a point and
an arc, a unit envelope, a rectangular trim and the source duct-fitting category. Public RVT bytes and Revit's own IFC confirm the rule.
Native-reference measurements below are local runs against a reference that
is **not shared**. The separate Measure findings are attributed below.

## Finding and scope

A `RuledSurf` can hold one zero `m_pProfileCurve1` or `m_pProfileCurve2`
pointer. Its corresponding `m_Point1` or `m_Point2` is then the constant
profile at that end of the surface. The old reader required two curves and
refused the whole saved body when one of these patches occurred.

On Autodesk's public Revit 2023 basic MEP project, this recovers **105 duct
fittings**. Two sanitary terminals, **543319 and 593903**, keep their record
boxes and are negative controls. The byte investigation covers 76 distinct
source patches in seven symbols, including the refused terminals. Default
IFC export changes from 176 to 71 bounding-box bodies and from 5,179 to
5,284 saved-mesh bodies, compared with base
`09b18bd338b8c2fdb9a7d29943970735d416a73c`. The body remains an
`IfcTriangulatedFaceSet`; this does not introduce an analytic IFC surface.

The admitted combination is exactly point/arc or arc/point with surface
envelope `[[0,0],[1,1]]` and a single rectangular face trim. Other combinations,
both pointers zero, missing or invalid points, unresolved nonzero pointers,
clipped envelopes and nonrectangular or holed trims remain refused. RE-430's
general trim reader does not widen this new surface's measured scope.
A complete body containing a collapsed ruled patch is admitted only when its
own element record's `m_builtinCategory` is `OST_DUCT_FITTING` (-2008010).
The exporter reads that category from the existing production walk; it does
not use an IFC class, fixture id or family name. Missing or conflicting
categories fail closed. Other categories keep their existing body with
`SavedMeshRefusal = collapsed ruled profile outside the measured duct fittings`.
An already-partial body retains its existing decoder refusal. An attach
without a production walk cannot qualify a collapsed body. Unrelated saved
meshes, standalone family revolutions and partial-arc refusals are unchanged.

## Input and byte witness

The source is downloaded directly from Autodesk; no RVT is redistributed:

- URL: <https://revit.downloads.autodesk.com/download/2023RVT_RTM/Docs/InProd/rmebasicsampleproject.rvt>
- Bytes: `30797824`
- SHA-256: `5b2331579140cc621d23b5642d24b2b7bbb6727e26610156962909d725e44e6a`

`research/collapsed-ruled.json` pins this file, all seven source-record body
hashes, the 105 positive target ids, two terminal refusal controls and the
native-reference package.
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
The same probe changes all 105 fittings' IFC classes to building-element
proxies and the two terminals' IFC classes to duct fittings before attachment:
all 105 still take saved meshes and both terminals keep boxes with the named
refusal. This checks source-category admission independently of IFC labels.
The replay separately checks both unmodified exported terminals' boxes,
`BodySource` and exact refusal text.

## Native reference and body measurements

**Not shared:** redistribution terms for this export of Autodesk's sample
have not been established. The public archive was withdrawn on 2026-11-12;
the manifest retains its hashes and size, but no download URL. The figures
below are retained as local measurements, not publicly reproducible native
comparisons. The source RVT and byte probe remain publicly reproducible.

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
out. It is not presented as an untouched full-project export. The archive
is unchanged: it still contains both terminal references, but the replay
scores only the 105 admitted fittings. The terminals are refusal controls,
not positive geometry targets.

| Artifact | Bytes | SHA-256 |
|---|---:|---|
| Reference IFC | 281892 | `4faf2915673bcb3ff48137b1ee7350449afef141a0e4ab2f6c8c760e7c874cf5` |
| Reference ZIP | 52288 | `77322bed93a88f027f473cd71578fa27aaeabf9402eaab99a445a1a138d6f93a` |

The table below records the revised one-cell generator grid measured on
2026-11-12. The replay reads the emitted triangle coordinates directly, preserving
coincident parts, and meshes the native reference with the repository-pinned
IfcOpenShell 0.9.0.post1. Both are transformed to their site frame. Every
unique vertex and every triangle centroid is scored against the other
surface in both directions. Volume is summed over all parts, without
deduplication. Missing elements, duplicate tags or failed native meshing
abort the replay rather than disappearing from the denominator.

| Native reference tessellation | Bodies | Maximum two-way sampled distance | Maximum relative volume difference |
|---|---:|---:|---:|
| Default (primary comparison): duct fittings | 105 | 0.696493 mm | 0.178479% |
| Fine, supplementary: duct fittings | 105 | 0.131137 mm | 0.019083% |

**None of the 105 fittings passes the strict 0.001 ft two-way threshold in
the default comparison.** All 105 pass with fine reference tessellation.
It does not replace the default verdict. The replay's explicit bounds
remain 0.0033 ft / 0.2% by default and 0.0005 ft / 0.025% with fine reference
tessellation. These are regression bounds for this sample, not a new
project-wide accuracy claim.

The two sanitary terminals missed the strict threshold even with fine
reference tessellation: their previous sampled residual was 0.720662 mm
at an existing plane/cylinder junction. They are now refused, not accepted
under a looser terminal bound. Their native export class is `IfcFlowTerminal`;
the decision uses their source record category, not that export class.

## Reproduce without Revit

Use the checkout containing this report and record its immutable commit
with `git rev-parse HEAD` alongside the output. Download the RVT from the
Autodesk URL above and verify its size and SHA-256 against the manifest.
The byte probe needs only that public source:

```sh
cargo build --locked --profile ci --bin rvt-ifc
cargo build --locked --profile ci -p rvt-probes --example probe_collapsed_ruled
cargo run --locked --profile ci -p rvt-probes --example probe_collapsed_ruled -- /path/to/rmebasicsampleproject.rvt
```

The native comparison additionally requires a local copy of the **not shared**
reference archive matching the retained hash. It cannot be reproduced from
public downloads alone:

```sh
python3 -m pip install --require-hashes -r tools/ci/requirements-ifc.txt
python3 tools/re/replay_collapsed_ruled.py --work target/collapsed-ruled \
  --reference-zip /path/to/collapsed-ruled-native-v1.zip
```

The sample downloads on demand with exact size and SHA-256 checks. The local
ZIP and its IFC member are checked too. Omitting `--reference-zip` fails
explicitly without downloading a reference. `--sample` accepts a local RVT
but still verifies its pin. `--rvt-ifc` and
`--probe` override binary
paths when using a separate Cargo target directory. Add `--fine` for the
slower supplementary run. Scoring uses up to four processes by default;
`--workers 1` reduces memory use, with the same per-body arithmetic. The
optional fine run can take tens of minutes. Results, per-element errors and the byte witnesses
are written under `--work`. No Revit installation or automation is needed.

## Regression comparison

Against `09b18bd3`, 12 project models were exported before and after:
Autodesk 2023 structural basic, architectural advanced/basic, public MEP and
Golden Nugget; Autodesk 2025 electrical, facades and architectural; BIM4LCA
concrete office as built and timber housing at permit; RE1 Mechanical and
Plumbing. Only the intended 105 MEP fittings change their representation; placements
are unchanged. Eight other project IFCs are byte-identical. Architectural basic,
Golden Nugget and the office change only 6, 9 and 15 `SavedMeshRefusal`
messages respectively, identifying the still-unsupported profiles or trims.
This local selection is not the full upstream Measure corpus.

All six existing REV-A, REV-B, REV-BT, ARC-A, ARC-B and ARC-BT IFC outputs
are byte-identical to the base, including the unsupported partial-arc cases.
The input files' hashes are unchanged. The native replay now scores the 105
fittings and checks two terminal refusals; it requires the not-shared reference
locally.

## Maintainer's Measure of the reviewed head (2026-11-12)

The maintainer compared `3d3caf78` with `3abb42f0` in unfiltered Measure
[43133538738](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43133538738)
and retained exports in filtered run
[43140323326](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43140323326).
His [read of the results](https://github.com/DrunkOnJava/rvt-rs/pull/1797#issuecomment-6812683878)
reports 108 of 127 exports byte-identical, ten gaining saved bodies, and nine
changing refusal-note text only. The new basic MEP save gains the 107 bodies;
Snowdon Towers HVAC gains 32 duct fittings. Office and architectural-basic
body sources, meshes and placements remain unchanged. Same-project gates
pass for 15 samples; the semantic check is NOT MEASURED on the measurement
branch by design. A held-out download failure passed on rerun.

Measure has no Revit IFC export of these samples. It confirms the export
scope and body-source changes, not the native-reference accuracy figures.

On that head, the 2024 basic MEP IFC grows from 103.5 to 159.2 MB, with
1,633,146 triangles in the 107 new face sets (15,263 on average; 15,588
maximum). Snowdon Towers HVAC grows from 65.5 to 69.4 MB for 32 new bodies
(3,610 triangles on average). These are the maintainer's measurements of
the original grid, before the point-profile subdivision revision below.


## Historical point-profile subdivision revision (`5a0f8d1f`, 2026-11-12)

This section records the earlier 107-body scope before the category
restriction. Its terminal counts and costs are not the current admission.

The original ruled grid subdivided both directions. For a point/arc patch,
every generator from the arc to the point is straight. A strip between two
arc samples is planar, so subdividing it along the generator adds triangles
without reducing the arc's chord error. The point-profile path now takes
one cell in `v`, retaining the reviewed `u` grid and all trim knots. It still
resolves the face's ruled component before choosing a grid; unresolved
neighbours and every existing admission refusal remain in force. Other
ruled faces keep their original grid.

An arc-only `u` grid was tried but not retained: native tag 419137 exceeded
the existing fine-reference bounds (0.2655 mm and 0.02724% volume). Keeping
the reviewed arc sampling removes the redundant generator subdivisions
without relaxing any measured bound. The graph reader and both probes use
the `Fv` fields introduced by #1800; mutable probe controls convert to JSON
only at the mutation boundary.


All 76 measured source faces now have two `v` knots; their combined triangle
count falls from 87,596 to 1,852. Local builds on upstream
`09b18bd338b8c2fdb9a7d29943970735d416a73c` compare the reviewed subdivision
with this revision. Both the 2023 and 2024 basic MEP saves retain all 107
recovered bodies, whose combined triangle count falls from 1,633,146 to
66,098 (15,263 to 618 on average; maximum 15,588 to 859). Their placements,
body sources and refusal notes are unchanged, and no other representation
changes.

| Local export | Base without point profiles | Reviewed grid | Revised grid |
|---|---:|---:|---:|
| Basic MEP 2023 | 101,091,346 bytes | 156,722,967 bytes | 105,088,486 bytes |
| Basic MEP 2024 | 103,550,587 bytes | 159,182,251 bytes | 107,547,770 bytes |
| Snowdon Towers HVAC 2024 | 65,514,494 bytes | 69,364,648 bytes | 65,858,488 bytes |

HVAC retains its 32 recovered duct fittings: 115,512 triangles become 5,856
(3,610 to 183 on average; maximum 4,748 to 212). Its other representations,
placements, body sources and refusals are unchanged. The 2024 inputs and
BIM4LCA office are the public files pinned by the existing research manifests.

The default native-reference maximum sampled distance was 0.962523 mm
with the reviewed grid and is 0.720662 mm with the revision. Removing grid
vertices changes the vertices and triangle centroids the scorer samples;
this lower sampled maximum is not proof of a more accurate underlying
surface. No replay bound was relaxed. All 107 bodies still miss the strict
0.001 ft threshold with default reference tessellation; fine tessellation
still puts the 105 fittings inside it, leaving the two sanitary residuals.

Using `tools/re/mesh_tjunctions.py` at its 0.003 ft tolerance on the 107 MEP
bodies, the near-edge junction count falls from 30,308 to four; no individual
body's count increases. This is the helper's geometric diagnostic, not a
claim that every body is watertight or that the sanitary junction residual
is fixed.


The fresh 12-project comparison against `09b18bd3` reproduces the original
scope: only the intended 107 MEP bodies change representation; eight other
IFCs are byte-identical and three change only the same 6, 9 and 15 refusal
notes. All six existing REV/ARC outputs are byte-identical to that base,
including the partial-arc refusals, and all source hashes are unchanged.
In the focused comparison, architectural basic 2024 and the BIM4LCA office
are byte-identical between the reviewed and revised grids. HVAC's recovered
bodies have zero counted junctions, down from 2,572, with none increasing.

These are local revision checks, not a new unfiltered Measure result. The
maintainer's previous corpus verdict and its native-reference limitation
remain attributed to the reviewed head above.

## Duct-fitting admission revision (2026-11-12)

Following review of `5a0f8d1f`, the exporter narrows body admission to the
source duct-fitting category. The two sanitary terminals remain useful byte
witnesses but fail the fine native-reference body threshold, so they are
explicit negative controls. No tessellation or pointer rule changes in this
revision. The native-reference archive and its hashes are unchanged.

Local comparison against `5a0f8d1f` on the 2023 and 2024 basic MEP saves finds
exactly two representation changes: 543319 and 593903 return to their original
record boxes, with the new category refusal. All 105 fittings retain identical
representations and placements. Their 64,380 triangles (613.143 average, 636
maximum) are unchanged; the previous 66,098 included the two refused terminals.
Snowdon Towers HVAC retains all 32 recovered fittings and its 5,856 triangles.
Its IFC is byte-identical to `5a0f8d1f`, as are architectural basic 2024 and
the BIM4LCA concrete office as built.

| Local export | Base `09b18bd3` | Reviewed `5a0f8d1f` | Duct fittings only |
|---|---:|---:|---:|
| Basic MEP 2023 | 101,091,346 bytes | 105,088,486 bytes | 105,006,306 bytes |
| Basic MEP 2024 | 103,550,587 bytes | 107,547,770 bytes | 107,465,590 bytes |
| Snowdon Towers HVAC 2024 | 65,514,494 bytes | 65,858,488 bytes | 65,858,488 bytes |

The 12-project and six-family local comparisons against `09b18bd3` retain
the scope described above, with 105 MEP fittings replacing the previous 107
positive targets. The default and fine native replays score those 105 only;
both terminal controls keep boxes with the named refusal. The 608 graph
refusals and all 107 IFC-class override controls pass. These are local checks
of the category revision, not a new unfiltered Measure result.
