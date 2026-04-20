# IFC4 export validation evidence

rvt-rs ships a pure-Rust STEP writer for IFC4 (no IfcOpenShell
runtime dependency). Claiming "spec-valid IFC4" is cheap; demonstrating
it is the point of this doc.

## What we validate automatically (every commit)

Three layers of IFC conformance are enforced as CI jobs on every
push and pull request. See [`.github/workflows/ci.yml`](../.github/workflows/ci.yml).

### Layer 1 — Rust-side structural assertions

**Job**: `ifc-smoke` (IFC-42).

Builds `tests/ifc_synthetic_project.rs` (end-to-end: decoded elements
→ `build_ifc_model` → `write_step` → `.ifc` file) and grep-asserts
the committed fixture carries the expected entity counts:

| Entity | Count |
|---|---|
| `IFCPROJECT` | 1 |
| `IFCBUILDINGSTOREY` | 3 |
| `IFCWALL` | 4 |
| `IFCSLAB` | 1 |
| `IFCDOOR` | 1 |
| `IFCWINDOW` | 2 |
| `IFCMATERIAL` | 2 |
| `IFCPROPERTYSET` | 2 |
| `IFCOPENINGELEMENT` | 1 |
| `IFCRELVOIDSELEMENT` | 1 |
| `IFCRELFILLSELEMENT` | 1 |
| `IFCEXTRUDEDAREASOLID` | 7 |

Catches writer regressions that drop elements, mis-count openings,
or silently truncate STEP output.

### Layer 2 — IfcOpenShell independent validation

**Job**: `ifcopenshell-validate` (IFC-41).

Installs [IfcOpenShell](https://github.com/IfcOpenShell/IfcOpenShell) 0.8.x
from PyPI and loads the committed
[`tests/fixtures/synthetic-project.ifc`](../tests/fixtures/synthetic-project.ifc)
through its full IFC4 schema parser. Fails the CI run if any of:

- The file's declared schema is not `IFC4`.
- The spatial hierarchy isn't the expected 1 project / 1 site /
  1 building / 3 storeys shape.
- Per-element counts (walls/slabs/doors/windows) drift.
- Any `IfcOpeningElement` has a dangling `IfcRelVoidsElement` or
  `IfcRelFillsElement` reference (the substring-counting in Layer 1
  can't see dangling refs — Python-side IfcOpenShell materialises
  them and would raise during `open()`).

This is the independent third-party gate. rvt-rs's own tests assert
what the writer emits; IfcOpenShell asserts what the IFC4 spec
accepts.

### Layer 3 — 357 lib unit tests covering every emission path

**Job**: `test` (unit tests).

Per-feature coverage enforced at compile time + runtime:

| IFC4 feature | Tests file | Count |
|---|---|---|
| STEP escaping / Unicode / GUIDs | `src/ifc/step_writer.rs` | 8 |
| Spatial hierarchy | `src/ifc/step_writer.rs` | 5 |
| Per-element entity emission | `src/ifc/step_writer.rs` | 4 |
| Material layer set (IFC-28/29) | `src/ifc/step_writer.rs` | 2 |
| Material profile set (IFC-30) | `src/ifc/step_writer.rs` | 2 |
| Property-set emission (IFC-31/33) | `src/ifc/step_writer.rs` | 2 |
| IfcQuantity variants (IFC-32) | `src/ifc/entities.rs` | 1 |
| Opening / fill (IFC-37/38) | `src/ifc/step_writer.rs` | 3 |
| IfcProfileDef subclasses (IFC-24) | `src/ifc/step_writer.rs` | 9 |
| IfcMember routing (IFC-10) | `src/ifc/category_map.rs` | 7 |
| IfcRevolvedAreaSolid (IFC-18) | `src/ifc/step_writer.rs` | 1 |
| IfcBooleanResult (IFC-19) | `src/ifc/step_writer.rs` | 2 |
| IfcFacetedBrep (IFC-20) | `src/ifc/step_writer.rs` | 1 |
| IfcRepresentationMap (IFC-21) | `src/ifc/step_writer.rs` | 3 |
| IfcFixedReferenceSweptAreaSolid (IFC-17) | `src/ifc/step_writer.rs` | 1 |
| ForgeUnit → IfcSIUnit / IfcConversionBasedUnit (IFC-39/40) | `src/ifc/entities.rs` + `step_writer.rs` | 10 |
| Category → IFC type mapping (IFC-01) | `src/ifc/category_map.rs` | 14 |

357 total lib tests as of 2026-04-20. Every test runs on every push
(ubuntu + macOS + windows matrix).

## What's deferred — buildingSMART Model Validator

The [buildingSMART Model Validator](https://www.buildingsmart.org/compliance/certified-software/certification-tools/) is the canonical conformance tool for IFC4.
Running it produces a multi-page PDF report enumerating rule
pass/fail per buildingSMART MVD ("Model View Definition").

Integrating it into CI is possible but requires:

1. Acquiring a buildingSMART validator license (commercial / research).
2. Packaging the validator as a container or self-hosted runner step.
3. Parsing its PDF / XML output into a CI-friendly exit code.

This is tracked as IFC-43 in the task board. Until the validator is
wired into CI, the evidence trail is:

- **Automated**: Layers 1-3 above.
- **Spot-check**: `tests/fixtures/synthetic-project.ifc` has been
  manually loaded in [BlenderBIM](https://blenderbim.org) 0.0.240509
  without errors and displays the expected spatial hierarchy +
  per-element geometry. Spot-checks are not a CI substitute — they
  cover the same scenarios as Layer 2 and can only catch regressions
  a human notices visually.

## Reproducing the evidence locally

```bash
# Layer 1 + 3: Rust-side tests
cargo test --release

# Layer 2: IfcOpenShell validation (requires Python + pip)
pip install 'ifcopenshell>=0.8.0,<0.9.0'
python - <<'PY'
import ifcopenshell
f = ifcopenshell.open("tests/fixtures/synthetic-project.ifc")
assert f.schema == "IFC4"
print("project:", f.by_type("IfcProject")[0].Name)
print("storeys:", [s.Name for s in f.by_type("IfcBuildingStorey")])
print("walls:", len(f.by_type("IfcWall")))
PY

# Regenerate the fixture from current source (verifies the fixture
# stays in sync with the writer):
DUMP_IFC=1 cargo test --release --test ifc_synthetic_project
```

## Known limitations

rvt-rs emits spec-valid IFC4 STEP for the entity classes it supports
(see [compatibility.md §4](compatibility.md#4-ifc4-export-coverage)
for the full per-class table). It does NOT claim:

- **IFC2X3 / IFC4X3 output**. IFC4 only, by design.
- **buildingSMART certified-software status**. No certification
  cycle has been run. The spec-conformance we assert is what
  IfcOpenShell's parser accepts + what our tests assert — not a
  buildingSMART-endorsed certification.
- **Preservation of Revit-specific BIM semantics** that IFC doesn't
  express natively (e.g. Revit parameter formulas, family types as
  first-class entities). Where IFC4 has a semantic gap, rvt-rs emits
  `IfcPropertySet` / `IfcTypeObject` approximations; round-trip
  fidelity to Revit's proprietary model is out of scope.
