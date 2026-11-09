# Rectangular revolution fixtures for #156

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
