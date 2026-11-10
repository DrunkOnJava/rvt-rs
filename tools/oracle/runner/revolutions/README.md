# Family revolution fixtures for #156

The three synthetic RFA files, their native IFC references and evidence are
available in the [MIT-licensed fixture archive](https://github.com/BIM4GIB/rvt-rs/releases/download/fixtures-revolutions-2020-v1/rectangular-revolutions-revit2020-v1.zip).
Extract it into a fixture directory and use the replay command below. Each
RFA/IFC has a licence sidecar with its SHA-256. No Revit launch is performed by
these tools; native authoring requires an existing Revit Python host.

`rectangular_fixtures.py OUTPUT` writes a JSON recipe and three mathematical OBJ
reference meshes (millimetres). `verify_reference.py OUTPUT` independently checks
their oriented edge incidence, tetrahedral volume and centroid against sector
integrals. These are authored references, not Revit files or decode evidence.

- REV-A: radii 300/500 mm, height 100 mm, full turn.
- REV-B: same profile, 30 to 120 degrees.
- REV-BT: B rotated 37 degrees about X, then translated (1200,-800,350) mm.

The reference is right-handed about the directed axis. The saved/native result
must establish Revit's angular convention, rather than inheriting that assumption.

After native approval, put this directory on an existing Revit Python host's
import path and call:

```python
from create_revit_fixtures import create_fixtures
create_fixtures(uiapp, template_path, new_output_directory)
```

Use the release's unhosted Metric Generic Model.rft. The script creates new
background family documents, writes three RFAs and native-authoring.json, then
closes only its own documents. It refuses an existing output directory. It
uses the documented public NewRevolution API to author geometry. A separate
native oracle captures face triangles, solid volume and centroid; those values
never feed the byte decoder or reconstruction. Native authoring was exercised
on Revit 2020.2.9 on 2026-11-09.

Run the offline probe on each resulting file:

```text
cargo run --profile ci -p rvt-probes --example probe_family_revolutions -- REV-A.rfa
```

The probe currently accepts standalone RFA files, retains duplicate saved records,
and reports semantic field values, body/field spans and angle parameter values.
Its matching GElement records are locations only, not a decoded-mesh verdict.
Unsupported records are reported. This probe makes no solid-match verdict;
the separate measurement command below reconstructs and compares geometry.

`export_revit_ifc.export_fixtures(uiapp, family_directory, new_output_directory)`
creates one new metric host per family, places it at the origin, saves the host
and exports native IFC4 Design Transfer (`IFCVersion.IFC4DTV`). It was exercised
in the same release. Generic `IFC4` is not interchangeable with this setting.

```text
python tools/oracle/runner/revolutions/measure_rectangular.py REV-B-probe.json REV-B-native.ifc results
```

The measurement script uses numpy and IfcOpenShell (the repository pins
0.9.0.post1). This replay was exercised with Python 3.11.4, NumPy 2.4.3 and
IfcOpenShell 0.9.0.post1. It rebuilds exclusively from probe fields and checks against the
IFC using the existing repository distance scorer. It refuses cases outside
one positive rectangular revolution with unflipped lines and one marked axis.
Results include exact source field bytes and positions. A failed comparison
writes its report and exits nonzero.

To replay all three files without Revit, build the two readers and run:

```text
cargo build --locked --profile ci --bin rvt-dump
cargo build --locked --profile ci -p rvt-probes --example probe_family_revolutions
python tools/oracle/runner/revolutions/verify_bundle.py --probe target/ci/examples/probe_family_revolutions --dump target/ci/rvt-dump --fixtures FIXTURE_DIRECTORY --output NEW_RESULTS_DIRECTORY
```

On Windows append `.exe` to both executable paths. The fixture directory must
contain `REV-{A,B,BT}.rfa` and `REV-{A,B,BT}-native.ifc`. The replay creates fresh
probe JSON, checks every emitted record body and field-byte prefix against
`rvt-dump`, then measures the reconstructions. Licence-sidecar hashes are checked
when supplied. It also refuses nine mutations of real probe evidence and a
deliberately wrong IFC pairing. These mutations test refusal behaviour; they
are not native observations of additional Revit variants. A successful run
writes `verification.json`; any failure exits nonzero. Inputs remain unchanged.

Measured: all three rebuild and all three pass the strict IFC bar with the
explicit Design Transfer exports. The initial generic IFC4 full turn was a
coarse faceted BRep; IFC4DTV preserves an advanced BRep and resolves the mismatch
without relaxing the bar. `export_settings_matrix.py` reproduces the six
controlled format/detail exports. See
`reports/element-framing/RE-391-family-rectangular-revolutions.md`.

Record independently authored releases separately from upgrades. The fixture
archive is an external release asset; native binaries are not added to the
source tree.


## Circular profiles (RE-392)

`curved_fixtures.definitions()` describes ARC-A (full torus), ARC-B (30-120
 degrees) and ARC-BT (B tilted and translated). All use two bounded semicircles,
400 mm major radius and 100 mm minor radius. In an existing Revit Python host:

```python
from curved_fixtures import definitions
from create_revit_fixtures import create_fixtures
from export_revit_ifc import export_fixtures
spec = definitions()
create_fixtures(uiapp, template_path, new_family_directory, spec)
export_fixtures(uiapp, new_family_directory, new_ifc_directory,
                [f["id"] for f in spec["fixtures"]])
```

Run the existing byte probe on each family. Reconstruction follows the saved
CurveElem -> CurveElemDriver -> m_pCrv link; the auxiliary arc-centre GLine is
not a profile. The proof refuses anything outside two oriented bounded arcs
forming one circle clear of a marked axis. See
[RE-392](../../../../reports/element-framing/RE-392-family-circular-revolutions.md).

For an independent native-face oracle, first generate queries offline:

```text
python tools/oracle/runner/revolutions/measure_native_faces.py queries ARC-B-probe.json ARC-B-queries.json
```

Then, in the authorised native host, call
`sample_native_faces.sample_faces(uiapp, family_path, query_path, new_oracle_path)`.
Name the result `ARC-B-faces.json` and repeat for A and BT. This opens/closes only
its own family, exports trimmed-face UV samples and native projections, records
hashes, and verifies the source and pre-existing document state are unchanged.
It does not launch Revit. Face samples, projections and volumes are validation
outputs; they never become decoder inputs.

The offline replay expects the three RFAs, three `-native.ifc` files, three
`-queries.json` files and three `-faces.json` files together:

```text
python tools/oracle/runner/revolutions/verify_curved.py --probe target/ci/examples/probe_family_revolutions --dump target/ci/rvt-dump --fixtures FIXTURE_DIRECTORY --output NEW_RESULTS_DIRECTORY
```

The native-face comparisons pass for all three. IFC is a separate result:
ARC-A passes, ARC-B differs by approximately 141 mm and 25% volume, and ARC-BT
exceeds the 120-second tessellation budget on the recorded environment. The
replay reports `verified_native_faces_with_recorded_ifc_discrepancies`; that is
not an all-IFC-pass verdict. A timeout sets `comparison_completed=false` and
provides no geometry-match verdict. A comparison that completes retains its
actual pass/fail result. The IFC child is killed on timeout.

Curved IFC comparison uses 30-micrometre linear deflection, 0.03-radian angular
deflection and up to 4,000 evenly spaced points per direction/triangle-centre
check. Volumes use complete meshes. The 0.3048 mm / 0.01% acceptance bars remain
fixed. Native-face checks use all captured UV samples and all 800 projections.
The original rectangular replay retains its existing full-vertex checks.

The cleaned MIT fixture package is [family-revolutions-revit2020-v2.zip](https://github.com/BIM4GIB/rvt-rs/releases/download/fixtures-revolutions-2020-v2/family-revolutions-revit2020-v2.zip)
(SHA-256 `6d68bedd29224158c2783fd66ae3072d7f4d931a68540db68856ee12587912c7`).
It contains all six families and native references. The package manifest verifies
all files; `metadata-cleanup.json` records native save/reopen checks after replacing
inherited appearances with a plain generic appearance. Native volumes, face
counts and evaluated sample points stayed exactly unchanged. Reference values
are retained and their source hashes bind them to these verified copies.
Run `verify_package.py` as described in the runner README.


## Compiled shared reader (RE-393)

The Rust library now exposes `FormDocument::sketch_curves`, shared by extrusions
and revolutions, and `FormDocument::revolution` with `Revolution::mesh`. The
compiled probe reads RFA bytes directly and writes typed profiles and closed
meshes in feet; it does not consume the Python probe JSON.

```text
cargo build --locked --profile ci -p rvt-probes --example probe_decoded_revolutions --example probe_family_revolutions
cargo run --locked --profile ci -p rvt-probes --example probe_decoded_revolutions -- ARC-B.rfa NEW_OUTPUT_DIRECTORY
python tools/oracle/runner/revolutions/verify_reader.py --reader target/ci/examples/probe_decoded_revolutions --evidence-probe target/ci/examples/probe_family_revolutions --rectangular RECTANGULAR_FIXTURES --curved CURVED_FIXTURES --output NEW_RESULTS_DIRECTORY
```

Append `.exe` to executable paths on Windows. The fixture folders are the same
ones described above. Six decoded meshes are measured against the independent
native references; profile samples are cross-checked against the Python byte
interpretation. Twenty-two mutated-native-graph controls exercise actual Rust
refusals. The replay never launches Revit or changes source fixtures.

Measured coverage is the six standalone 2020 files. One positive unflipped
revolution, one closed convex line/arc loop, clear of its axis, is the supported
scope. Mixed line/arc profiles need native fixtures before they are counted as
measured coverage. See [RE-393](../../../../reports/element-framing/RE-393-shared-family-profile-reader.md)
for the reader and the following section for the default exporter integration.

## Default IFC export (RE-394)

The normal `rvt-ifc FAMILY.rfa -o FAMILY.ifc` path writes a supported standalone
Revit 2020 revolution as an analytic `IfcRevolvedAreaSolid`. The shared reader
supplies a radial/axial line-and-arc profile, axis, origin and start/end angles.
The exporter supports REV-A, REV-B, REV-BT and ARC-A, confirmed by native IFC.
Profiles containing arcs require a full turn; ARC-B and ARC-BT are refused with
a skipped note. The reader retains all six as research evidence.
The IFC preserves those lines and arcs; a display tessellation is not used as
export input. Its saved ElementId is the Tag and its `BodySource` is
`family_revolution`. The body is unclassified: modes requiring mapped IFC types
do not gain a proxy. Project instances and the optional family-extrusion path
are unchanged. See [RE-394](../../../../reports/element-framing/RE-394-analytic-family-revolutions.md)
for eligibility, analytic mapping and measured OpenCascade readback.

After the shared-reader replay above, measure the actual written IFC:

```text
cargo build --locked --bin rvt-ifc
python tools/oracle/runner/revolutions/verify_export.py --exporter target/debug/rvt-ifc --reader-checks READER_RESULTS_DIRECTORY --rectangular RECTANGULAR_FIXTURES --curved CURVED_FIXTURES --output NEW_EXPORT_RESULTS_DIRECTORY
```

The replay requires NumPy, IfcOpenShell with OpenCascade, and Pygments for
IfcOpenShell's EXPRESS-rule validator. It checks IFC schema/EXPRESS rules,
analytic profile segments, start angle and tilted placement, identity and
diagnostics. Each of the four supported IFCs is read in a bounded child process
twice: once with default OpenCascade settings to check ordinary import, and once with
20-micrometre linear / 0.03-radian angular deflection for native comparisons.
Each import has a 120-second limit. The acceptance limits remain 0.3048 mm
sampled distance and 0.01% relative volume error; the fine mesh must be closed
and oriented. Timings, output sizes, hashes, settings and results are saved.

It never launches Revit. The existing native IFC limitations of the curved
reference captures remain separate; partial arc captures are research leads,
not export confirmation. The exporter replay asserts their refusal. The reverse
distance is a sampled bound, not a whole-surface guarantee.
The replay validates the final analytic output.
The viewer uses its existing, coarser display tessellation independently.

Set `RVT_RECTANGULAR_REVOLUTIONS_DIR` and `RVT_CURVED_REVOLUTIONS_DIR` to these
fixture folders when running the Rust integration tests. The native CLI
contract checks analytic output, a finite display mesh, source immutability,
placeholder behavior and the three mapped-type mode refusals.

## Complete six-fixture package replay

The fixture package includes cleaned copies of all six circular and
rectangular files. Extract it into one directory; keep its
`rectangular/`, `curved/` and `manifest.json` together. The package's
`REPRODUCE.md` includes setup and expected results. No private launch tooling,
Revit session, installed sample files or local previous-run output is required.

Tested with Python 3.11, IfcOpenShell 0.9.0.post1, NumPy 2.4.3 and Pygments 2.21.0:

```text
python -m pip install ifcopenshell==0.9.0.post1 numpy==2.4.3 pygments==2.21.0
cargo build --locked --profile ci --bin rvt-ifc --bin rvt-dump
cargo build --locked --profile ci -p rvt-probes --example probe_family_revolutions --example probe_decoded_revolutions
python tools/oracle/runner/revolutions/verify_package.py --bin-dir target/ci --fixtures EXTRACTED_PACKAGE --output NEW_RESULTS_DIRECTORY
```

The runner verifies every file hash, then runs the rectangular byte/native-IFC
proof, circular byte/native-face proof (retaining the known native IFC
mismatches), shared Rust reader with 22 refusal controls, and final analytic
IFC exporter (four bodies and two explicit partial-arc refusals). The pinned
archive and member hashes are in `research/family-revolutions-measure.tsv`;
Measure wiring is a separate maintainer change. The archive is unchanged, so
its original expected-six-export description is superseded by this scope.
The runner's final `verification.json` records the four stage statuses
and executable hashes. `--bin-dir` accepts a custom Cargo target directory;
Windows `.exe` names are selected automatically. Each output directory must
be new. Replay changes no fixture and launches no Revit.
