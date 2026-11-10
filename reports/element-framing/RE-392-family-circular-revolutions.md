# RE-392: Circular family revolutions and independent native-face validation

**Date:** 2026-11-09.
**Issues:** #156; follows RE-327 and RE-391.

## Result

Three fresh Revit 2020 circular-profile revolutions reconstruct from saved
records and pass an independent comparison with Revit's own evaluated faces
and point projections. The largest sampled gap is 0.011423 mm. Native solid
volumes and rebuilt mesh volumes satisfy the 0.01% relative-error bar.
The checks are sampled surface checks, not a complete Hausdorff-distance proof.

The IFC results are separate: the full torus passes; the partial torus fails
with a roughly 141 mm surface discrepancy and 24.976% volume error. The tilted
partial IFC does not finish tessellation within the 120-second replay budget;
it has no geometry-match verdict. None of those outcomes is relabelled as an
IFC pass. This report records the byte proof; the shared reader and analytic exporter are described in RE-393 and RE-394.

## Fixtures and native references

ARC-A revolves two semicircles, whose common centre lies 400 mm from the axis
and whose radius is 100 mm, through 360 degrees. ARC-B uses 30-120 degrees.
ARC-BT rotates B by 37 degrees about X and translates it (1200,-800,350) mm.
Revit 2020.2.9, build 20220517_1515(x64), authors new family documents through
public API calls. Recipes never feed reconstruction. IFC references use
explicit IFC4DTV and tessellation detail 1, as in RE-391.

| RFA | SHA-256 of cleaned package RFA |
| --- | --- |
| ARC-A.rfa | `aeb7b55ba14f82765bac982f155c97c74442662494eebdeb00520c7287ea81c9` |
| ARC-B.rfa | `bb951c9194e73310f53530809345a48052f1a323748fa068172a15ce3f0c0dbd` |
| ARC-BT.rfa | `02a9c03966aaeb3035afa9603ca3bc32096b251e1d252f82c301a8a84449faf8` |

The cleaned MIT fixture package is [family-revolutions-revit2020-v2.zip](https://github.com/BIM4GIB/rvt-rs/releases/download/fixtures-revolutions-2020-v2/family-revolutions-revit2020-v2.zip)
(SHA-256 `6d68bedd29224158c2783fd66ae3072d7f4d931a68540db68856ee12587912c7`).
It contains all six families and native references. The package manifest verifies
all files; `metadata-cleanup.json` records native save/reopen checks after replacing
inherited appearances with a plain generic appearance. Native volumes, face
counts and evaluated sample points stayed exactly unchanged. Reference values
are retained and their source hashes bind them to these verified copies.
Run `verify_package.py` as described in the runner README.

## Saved geometry selection and coordinates

Resolve the revolution's sketch membership and marked axis as in RE-391.
For a profile CurveElem, follow `m_pCurveDriver` to CurveElemDriver, then its
`m_pCrv` graph edge to GArc. The same record also contains a GLine reached by
`CurveElem.m_oArcCntr`: its parameter limits are zero and it is the arc-centre
marker, not the profile. Selecting a primitive only by presence/class is unsafe.
The decoder checks edge source, pointer offset/token, target tag and object id.

For these files, the existing GArc evaluation rule is confirmed:

`p(t) = m_center + m_radius * (cos(t)*m_xVec + sin(t)*m_yVec)`.

Centre/radius are in feet; basis vectors are unit and orthogonal; limits are
radians. The two semicircles can each have limits near [0,pi] while the second
has reversed basis vectors. Their directed endpoints close the profile.
The tilted fixture establishes that the saved arc centre and frame are already
in family coordinates; the sketch plane transform must not be applied again.

The bounded proof accepts two co-centred, equal-radius, oriented arcs forming
one circle in the axis plane, clear of the axis, with one positive unflipped
revolution. It does not establish arbitrary arc/line mixtures, ellipses,
splines, holes, voids, flipped sketches, reversed ranges or other releases.
Equal-limit full-circle encodings are refused here; these fixtures do not
measure that representation.

## Byte observations

All offsets below are decimal, zero-based in inflated `Partitions/0`, written
by rvt-dump as `Partitions_0.decomp`. IDs are observations, not reader constants.
Offsets below refer to the cleaned v2 archive; native resaving moved the record
locations. Fresh probes and the independent dump verify 117 record bodies and 12,405
field spans across the three files, including every row below.

| File | Element / field | Inflated offset | Raw bytes | Decoded value |
| --- | --- | ---: | --- | --- |
| ARC-B | 2685 / CurveElem.m_pCurveDriver | 657528 | `ffffffff7a03` | `token 4294967295; class 890` |
| ARC-B | 2685 / CurveElem.m_oArcCntr | 657544 | `030000004d06` | `token 3; class 1613` |
| ARC-B | 2685 / CurveElemDriver.m_pCrv | 657706 | `040000003907` | `token 4; class 1849` |
| ARC-B | 2685 / GLine.m_endParams | 657823 | `00000000000000000000000000000000` | `[0.0,0.0]` |
| ARC-B | 2685 / GArc.m_center | 658059 | `40f54ffd53fff43f000000000000000000000000000080bc` | `[1.3123359580052494,0.0,-2.7755575615628914e-17]` |
| ARC-B | 2685 / GArc.m_radius | 658051 | `40f54ffd53ffd43f` | `0.32808398950131235` |
| ARC-B | 2685 / GArc.m_endParams | 657987 | `0000000000000000182d4454fb210940` | `[0.0,3.141592653589793]` |
| ARC-B | 2686 / GArc.m_xVec | 658654 | `000000000000f0bf0000000000000000f9a9f1d24d6298bc` | `[-1.0,0.0,-8.459899447643689e-17]` |
| ARC-B | 2686 / GArc.m_yVec | 658678 | `f9a9f1d24d62983c0000000000000000000000000000f0bf` | `[8.459899447643689e-17,0.0,-1.0]` |
| ARC-BT | 2685 / GArc.m_center | 658059 | `40f54ffd53ff144040f54ffd53ff04c098f6a57d695ff23f` | `[5.2493438320209975,-2.6246719160104988,1.1482939632545932]` |
| ARC-BT | 2685 / GArc.m_yVec | 658027 | `0000000000000000b8be55941142e3bf137aa20e6c8ee93f` | `[0.0,-0.6018150231520485,0.7986355100472927]` |
| ARC-BT | 2687 / GLine.m_dirVec | 659151 | `0000000000000000b6be55941142e3bf147aa20e6c8ee93f` | `[0.0,-0.6018150231520483,0.7986355100472928]` |

## Native geometry validation

Reconstruction uses only probe JSON. The shared sketch checks retain complete
record consumption, membership, flags and angle-parameter consistency checks.
Arc and sweep steps are at most 0.75 degrees; seams/caps close with oriented
edge incidence. The analytic circular-sweep volume uses decoded dimensions.

The separate native oracle exports a 21 by 21 UV grid per face, retaining
points inside the trimmed face, through Face.Evaluate. It also projects 400
reconstructed vertices and 400 triangle centres onto the native faces. The
query hash and RFA hash bind the returned measurements to the reconstruction.
No face sample, projection or API volume becomes a format-decoder input.
Projection failure or an unsampled face is refused. A cap limits neither the
native UV samples nor the 800 requested projections.

| Fixture | Native samples to rebuilt triangles (mm) | Rebuilt queries to native faces (mm) | Native solid volume error (%) | Result |
| --- | ---: | ---: | ---: | --- |
| ARC-A.rfa | 1.3377216e-13 | 0.0114229811 | 5.34474504e-05 | pass |
| ARC-B.rfa | 0.00115065262 | 0.0114229811 | 0.00235269838 | pass |
| ARC-BT.rfa | 0.00115065262 | 0.0114229811 | 0.00235269838 | pass |

The fixed bars are 0.3048 mm for sampled surface distances and 0.01% for
relative volume. All rebuilt meshes have about 0.005712% volume error.
There are 4,796 evaluated native face points and 2,400 projections in total.

## IFC and display-mesh controls

The IFC scorer uses IfcOpenShell 0.9.0.post1, world coordinates, linear deflection
0.00003 m and angular deflection 0.03 rad. It measures up to 4,000 evenly spaced
vertices in each direction and 4,000 rebuilt triangle centres against all of
the other mesh's triangles, using the repository's distance scorer. Volumes
use complete meshes. Counts/settings appear in each result. The opposite-sense
control uses 400 native vertices. These sampling limits are explicit, not an
all-vertices claim. RE-391's rectangular replay still uses all its vertices.

The earlier 0.000003048 m meshing setting was impractically slow on the curved
surfaces. The bounded setting leaves the acceptance bars unchanged. Full torus:
176,400 IFC vertices / 352,800 triangles; largest sampled gap 0.016762 mm;
IFC volume error 0.007460%. Partial torus: 50,261 vertices / 100,518 triangles;
largest sampled gap 141.440 mm; IFC volume error 24.975668%. Its opposite-sense
control misses by 842.344 mm. The tilted IFC is explicitly unmeasured after
the child tessellator exceeds 120 seconds and is terminated.

The full IFC uses IfcAdvancedBrep. Both partial exports use
IfcSurfaceCurveSweptAreaSolid with an IfcCircleProfileDef. ARC-B's meshed radial
range is approximately 400-600 mm and z range 0-200 mm; the native solid and
saved-byte reconstruction are 300-500 mm and -100 to +100 mm. This identifies
a discrepancy in the native-IFC/consumer chain; it does not assign its cause to
Revit or IfcOpenShell. The [IFC4 surface-curve sweep definition](https://standards.buildingsmart.org/IFC/RELEASE/IFC4/ADD1/HTML/schema/ifcgeometricmodelresource/lexical/ifcsurfacecurvesweptareasolid.htm)
is relevant to a separate investigation of the profile frame.

Face.Triangulate(1.0) is not an exact-face oracle either: all three maximum-detail
native display meshes differ by approximately 0.386 mm and have about 0.065%
mesh-volume error. Those failures are retained in local evidence; the independent
Face.Evaluate/Face.Project checks above pass without changing geometry or bars.

## Replay and checks

`tools/oracle/runner/revolutions/README.md` documents native authoring, oracle
capture and offline replay. The replay checks source bytes, three native-face
comparisons, the separate IFC outcomes, 18 malformed-evidence refusals and a
wrong-reference volume control. JSON mutations are refusal tests, not native
observations of additional formats.

The complete cleaned archive passes its four-stage replay and all 302
configured integration checks. RE-393 covers the compiled reader and RE-394
covers the final analytic exporter, its regression checks and local gates.
Workflows and dependencies are unchanged.

## Sources and limitations

Sources are synthetic file bytes, the existing clean-room GArc reader and
RE-327/RE-391, documented public Revit API signatures for authoring/oracles,
and Revit's native export and face measurements. No Autodesk SDK implementation,
ODA code, decompilation, NDA material, or copied/translated revit-ifc code is
used. This is measured Revit 2020 research, not general revolution support.
The fixture package is cleaned and replayable; private native launch tooling is excluded.
