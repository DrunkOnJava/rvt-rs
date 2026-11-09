# RE-391: Rectangular family revolutions: axis, angles and native IFC comparison

**Date:** 2026-11-09
**Issues:** #156; follows RE-327 and RE-344.

Measured on three independently authored families in Revit 2020.2.9,
20.2.90.12, build `20220517_1515(x64)`.

## Result

The saved records rebuild all three rectangular revolutions as closed meshes.
All three pass the stated comparison against Revit's IFC4 Design Transfer
exports. The initial generic IFC4 reference was too coarse for the full turn;
an explicit export-settings experiment resolves that mismatch without changing
the reconstruction or tolerance. No general revolution support or production
exporter change is claimed.

For these fixtures, the chain is:

`RevolutionElem.m_sketchId → CurveElem / SketchMembership.m_groupId →
RevolutionCurveData.m_axis → GLine → right-handed turn about the directed axis`.

The parameters RE-344 already located, -1001802 and -1001803, match the
commanded start and end angles in radians. The four profile lines and marked
axis line are already in family coordinates, in feet. Applying the sketch
plane transform again would double-transform REV-BT. The proof uses their
saved positions directly and verifies their common sketch-plane identity.

## Fixtures and source identity

Each file has one positive revolution: a rectangle with radii 300/500 mm and
height 100 mm. REV-A commands 0–360 degrees; REV-B commands 30–120 degrees;
REV-BT is B rotated 37 degrees about X, then translated (1200,-800,350) mm.
All are new documents, not upgrades or versions of a customer model.

| File | SHA-256 |
| --- | --- |
| REV-A.rfa | `8aaa02dbf1fc7f06cf56088a567b5ab8e15e544486b7fd848b914d632e982223` |
| REV-B.rfa | `2f26d428c79ffcee2bd31f1e542d4f15994ecd0f2d84dcdb7a0f18c792d2224b` |
| REV-BT.rfa | `28fa900ce2fa6fddad8494022f54cea8e18f5c9329e30308f503f924716258bb` |

Native authoring uses the installed English Metric Generic Model template
(SHA-256 `b79ffdbe9b8592982e92c1769f252336ed7dd64bf8153545934e7f78990a2e57`).
The recipe and scripts are under `tools/oracle/runner/revolutions/`.
The [fixture archive](https://github.com/BIM4GIB/rvt-rs/releases/download/fixtures-revolutions-2020-v1/rectangular-revolutions-revit2020-v1.zip)
is redistributable under MIT and includes all six RFA/IFC files, licence
sidecars and replay evidence. It can be downloaded without Revit; it is not
yet wired into an automatic CI corpus job. Archive SHA-256:
`57bbc9c7be6f53ea98e4c73daaa6a9ea703a0f320eac36f69ac7af898c8e0072`. Hashes above identify the
cleaned copies: only the RFA BasicFileInfo save path was replaced, at equal
byte length, with a neutral synthetic path. Every other OLE stream is identical
to native output. Revit reopened all three copies and returned exactly the
same mesh hashes and solid volumes. The probes and comparisons were rerun.
IFC header/person metadata was neutralised; geometry arrays were unchanged.

## Byte evidence

Offsets below are decimal, zero-based in `Partitions/0` after inflation, as
written by `rvt-dump` to `Partitions_0.decomp`; they are not OLE-file offsets.
Form id is 2684, sketch id 2683, profile curve ids 2685–2688, axis curve id
2689 in these files. Identical ids are an authoring consequence, not lookup
constants in the reconstruction.

| File | Element / field | Offset | Raw little-endian bytes | Value |
| --- | --- | ---: | --- | --- |
| A, B, BT | 2684 / m_sketchId | 772037 | `7b0a0000` | 2683 |
| A | 2684 / end value | 772045 | `112d4454fb211940` | 6.28318530717958 |
| A | 2684 / start value | 772057 | `0000000000000000` | 0 |
| B, BT | 2684 / end value | 772045 | `65732d3852c10040` | 2.0943951023931953 |
| B, BT | 2684 / start value | 772057 | `65732d3852c1e03f` | 0.5235987755982988 |
| A, B, BT | end parameter id | 772053 | `b5b6f0ff` | -1001803 |
| A, B, BT | start parameter id | 772065 | `b6b6f0ff` | -1001802 |
| A | 2689 / RevolutionCurveData.m_axis | 775105 | `01` | true |
| B, BT | 2689 / RevolutionCurveData.m_axis | 774969 | `01` | true |
| B | 2689 / GLine.m_dirVec | 775087 | `00000000000000000000000000000000000000000000f03f` | (0,0,1) |
| BT | 2689 / GLine.m_dirVec | 775087 | `0000000000000000b6be55941142e3bf147aa20e6c8ee93f` | (0,-0.6018150231520483,0.7986355100472928) |
| BT | 2689 / GLine.m_origin | 775063 | `dfeff7fbfd7e0f4040f54ffd53ff04c098f6a57d695ff23f` | (3.9370078740157477,-2.6246719160104988,1.1482939632545932) ft |

The four profile curves have `m_axis=false`; the axis has `m_axis=true`.
Both have `m_bOldAxis=false` in every fixture. Detailed probe JSON retains
their field spans and values, including the curve membership and endpoints.
All 123 record bodies and 13,281 field spans emitted across the three files
were compared byte-for-byte with the existing `rvt-dump` output; all matched.

## Reconstruction and oracle

`measure_rectangular.py` consumes only the Rust byte-probe JSON for the
rebuild. It does not consume the authoring recipe, dimensions, Revit API
geometry, or IFC geometry while constructing it. It resolves sketch membership,
requires one marked line axis and four unflipped profile lines, joins by
endpoints and verifies a rectangle in one radial half-plane. It rotates that
profile about the directed axis with the measured start/end values. Closure
and oriented edge incidence are checked; the mesh step is 0.25 degrees.

The oracle is Revit's own IFC4 Design Transfer export of a new metric host containing
one fixture instance at the origin. Settings: base quantities on, space
boundaries 0, `TessellationLevelOfDetail=1`, and explicit API format
`IFCVersion.IFC4DTV`. The IFC declares Autodesk Revit
2020 (ENU). Full application build is recorded above. No instance placement
is inferred from IFC to correct the reconstruction.

Measurements used Python 3.11.4 and NumPy 2.4.3. IfcOpenShell 0.9.0.post1
tessellates each IFC using the repository's
`fine_mesh.settings()`: linear deflection 0.000003048 m, angular deflection
0.03 rad, world coordinates. Existing `mesh_bodies_vs_ifc.surface_distance`
measures every reference vertex, every reconstructed vertex and every
reconstructed triangle centre. The bar is 0.001 ft (0.3048 mm) in all three
distance checks and relative volume errors below 0.01% for both IFC and rebuilt
mesh against the analytic volume. This is sampled surface
agreement, not a continuous Hausdorff bound.

| Fixture | Revit IFC geometry | IFC vertices → rebuild, mm | Rebuild vertices → IFC, mm | Rebuild centres → IFC, mm | IFC volume error vs analytic | Strict pass |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| A | IfcAdvancedBrep | 0.001190 | 0.001520 | 0.001055 | 0.0002527% | yes |
| B | IfcExtrudedAreaSolid | 0.001190 | 0.001515 | 0.001052 | 0.0002523% | yes |
| BT | IfcExtrudedAreaSolid | 0.001190 | 0.001515 | 0.001052 | 0.0002523% | yes |

Analytic volumes reconstructed from the records: A 0.05026548245743664 m³;
B and BT 0.01256637061435917 m³. A has 4,520 IFC vertices / 9,040 triangles;
B and BT each have 1,136 / 2,268. Rebuilds have 5,760 / 11,520 for A and
1,444 / 2,884 for each partial turn. Every listed vertex is tested.

Turning the partial profiles in the opposite sense puts an IFC vertex
742.938671 mm from the reconstructed surface in both B and BT. This is an
explicit discriminator for rotation direction; volume alone cannot provide it.

### Export-settings control: the initial reference was too coarse

The first export used generic `IFCVersion.IFC4`, not an explicit Design
Transfer setting. It contained an `IfcFacetedBrep`, with 40 unique angular positions
on the outer radius and 32 on the inner. Its vertices have radii 0.3/0.5 m
within 7.8e-16 m and heights 0/0.1 m. The largest outer angular gap is 9 degrees.
The chord sagitta `0.5*(1-cos(9 degrees/2)) = 0.0015413331334 m` agrees with
the measured reverse distance. Fine tessellation cannot recover a true curved
surface from these already faceted IFC faces. That observation alone did not
establish a Revit capability limitation. Six controlled exports of the same
unchanged host isolate the export settings:

| API format / settings | Native representation | Largest vertex distance, mm | Strict pass |
| --- | --- | ---: | --- |
| IFC4 / detail 0.25 | faceted BRep | 24.471742 | no |
| IFC4 / detail 1 | faceted BRep | 1.541333 | no |
| IFC4 / detail 1 / Fine 3D view | faceted BRep | 1.541333 | no |
| IFC4DTV / detail 1 | advanced BRep | 0.001520 | yes |
| IFC4DTV / detail 1 / Fine 3D view | advanced BRep | 0.001520 | yes |
| IFC4RV / detail 1 | tessellated | 1.541333 | no |

The explicit Design Transfer setting resolves the mismatch. The rebuild,
mesh step, IFC meshing settings and comparison tolerances are unchanged.
All three references were regenerated with IFC4DTV and remeasured: 3/3 strict
passes. B and BT remain true extruded sector profiles despite being authored
as Revit revolutions. The coarse exports remain local controls, not the primary
oracle or a claimed reconstruction failure.

`export_settings_matrix.export_matrix(uiapp, host_path, new_output_directory)`
reproduces the experiment without saving the host. Autodesk documents that
[IFC4 Design Transfer can preserve advanced BReps](https://help.autodesk.com/cloudhelp/2020/ENU/Revit-DocumentPresent/files/GUID-E029E3AD-1639-4446-A935-C9796BC34C95.htm).

Separately labelled public-API solid volumes are 0.0502653797880 m³ (A) and
0.0125663091297 m³ (B/BT). These support the intended native shape but do not
replace the IFC comparison, and were not used to derive decoder rules.

## Reproduction

In an approved Revit 2020 Python host, call `create_fixtures(uiapp, template,
new_family_directory)`, then `export_fixtures(uiapp, family_directory,
new_ifc_directory)` from the corresponding authoring modules. Both refuse
an existing output directory and close only their own new documents.

```text
cargo build --locked --profile ci -p rvt-probes --example probe_family_revolutions
target/ci/examples/probe_family_revolutions REV-B.rfa > REV-B-probe.json
python tools/oracle/runner/revolutions/measure_rectangular.py REV-B-probe.json REV-B-native.ifc results
```

Repeat for A and BT: all three pass with the Design Transfer references.
The initial generic-IFC4 full-ring control still exits nonzero. Use UTF-8
redirection when capturing JSON on Windows.

`verify_bundle.py` automates the full three-file replay with fresh probe output,
checks all body hashes and field bytes against `rvt-dump`, checks input hashes,
and runs the measurements and negative controls. Its command is in the
[runner README](../../tools/oracle/runner/revolutions/README.md). It needs only
the two compiled readers, Python/IfcOpenShell and the six fixture files; Revit
is needed only to author new native fixtures, not to replay these findings.

## Scope, checks and remaining work

- Three synthetic families, one release, one rectangular profile each. No
  holes, curved profiles, reversed axes, flipped sketches, voids, Boolean
  subtraction, arbitrary angular ranges or independently authored older builds.
- All three records decode; all three rebuild; all three meet the strict
  IFC bar. No complete saved-mesh decode is claimed by the GElement inventory.
- Nine modified-evidence cases are refused: repeated records, no axis, flipped
  curve, missing profile edge, reversed angles, incomplete decode, unread body
  bytes, duplicate angle IDs and an inconsistent angle summary. Pairing B's
  reconstruction with A's native IFC also fails. The JSON mutations are
  proof-script refusal checks, not native fixtures for those variants.
- Local gate components passed: workspace clippy with warnings denied, strict
  library/binary check, rustdoc, all-target/all-feature workspace build, 27 doc
  tests, 301 integration checks, MCP network audit, 15 MCP checks and all three
  basic examples. Windows cargo-fmt exceeds the process command-length limit;
  the same workspace target roots were checked with rustfmt in batches instead.
  Integration checks used the repository's optimised `ci` profile. External
  corpus checks were not enabled; hosted CI is tracked by the draft PR. These local checks were rerun after
  rebasing onto `5247e7ba107add935c3cc3ed60bb9ee95dbeeb8e`.
- At fixed `SOURCE_DATE_EPOCH=0`, upstream `5247e7ba107add935c3cc3ed60bb9ee95dbeeb8e`
  and changed `rvt-ifc` builds in separate build directories produced
  byte-identical outputs on 11 inputs: three tier-1 synthetic projects, these
  three families and their native hosts, and installed 2020/2024 basic sample
  families. All input hashes remained unchanged. This checks existing output,
  including its existing limitations; it adds no production geometry claim.
- The library change only exposes record source locations to research
  callbacks. This draft adds no production revolution reader or export flag.

Sources: authored saved bytes and each file's `Formats/Latest`; RE-327's
sketch/membership infrastructure; RE-344's angle parameter IDs; Revit's own
IFC exports; documented public API for authoring and separately labelled
oracle capture. Credit STE1200's team and Discussion #112 for the pipeline
target. No Autodesk/ODA implementation source or decompiled internals were
used, and no revit-ifc code was copied or translated.
