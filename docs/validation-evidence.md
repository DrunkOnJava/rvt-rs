# IFC4 export validation evidence

rvt-rs ships a pure-Rust STEP writer for IFC4 (no IfcOpenShell
runtime dependency). Claiming "spec-valid IFC4" is cheap; demonstrating
it is the point of this doc.

## What CI checks on every pull request

The `ifcopenshell-validate` job ("IfcOpenShell validates exports" in
[`.github/workflows/ci.yml`](../.github/workflows/ci.yml)) is part of
`CI passed`, the check every merge needs. It exports with the `rvt-ifc` the
pull request builds and checks the result against readers rvt-rs did not
write:

1. **The IFC4 property set table is buildingSMART's.** The table the writer
   uses to place a property in a standard `Pset_` is generated from
   buildingSMART's definitions (pinned by commit and sha256) and
   IfcOpenShell's IFC4 schema. The committed table must be the generator's
   output (#817).
2. **The tier-1 projects export valid IFC4.** The three synthetic projects
   in `corpus/tier1/` (architectural 2024, MEP 2024, structural 2023, made
   by `gen-fixture`) are exported and validated by `ifcopenshell.validate`
   against the IFC4 schema, and by
   [`tools/ci/ifc_schema_arity.py`](../tools/ci/ifc_schema_arity.py)
   against each entity's declared attribute count and `PredefinedType`.
3. **A real project's export is internally consistent.**
   `Revit_IFC5_Einhoven.rvt` (Revit 2023, from
   [`magnetar-io/revit-test-datasets`](https://github.com/magnetar-io/revit-test-datasets),
   MIT) is exported, and
   [`tools/ci/validate-real-ifc.py`](../tools/ci/validate-real-ifc.py)
   cross-checks it:
   - `IFC4` schema, one `IfcProject`, `IfcSite` and `IfcBuilding`, and at
     least one `IfcBuildingStorey`.
   - The entity counts `rvt-ifc --diagnostics` reports against
     IfcOpenShell's object graph.
   - `IfcRelContainedInSpatialStructure` coverage: one relation per
     non-empty storey, covering every exported building element.
   - A unit assignment.
   - Geometry and material limitations stay explicit in the diagnostics,
     and where the export has geometry, matching `IfcProductDefinitionShape`
     coverage.
4. **Real exports validate in full.** The Einhoven export and the
   `2024_Core_Interior.rvt` export (Revit 2024, thousands of property
   values) go through `ifcopenshell.validate` and the arity check.
5. **Independent readers agree with Revit's own export.** IfcOpenShell and
   IFClite, a separate Rust STEP reader with no IfcOpenShell code, each read
   Revit's IFC export of Core Interior and must report the counts rvt-rs is
   gated against. The verdict of the three witnesses (rvt-rs reading the
   `.rvt`, the two readers reading Revit's `.ifc`) must pass on the claimed
   surface, and the fresh observations must replay byte for byte against
   the ones committed under `research/witness/`
   ([`verification-protocol.md`](verification-protocol.md)).
6. **Predefined types and placements match Revit's.** On the Core Interior
   export, every `PredefinedType` written must be the value Revit's exporter
   writes for that type (#220), and two pinned elements' placement must
   compose to where Revit puts them (#232).
7. **The full project export, on a narrow surface.** The same three
   witnesses also run against Revit's full Core Interior export. The claimed
   fields are deliberately few; what rvt-rs cannot claim yet is excluded by
   name, with its tracking issue.

## What is measured against Revit's own exports

The Measure workflow (`.github/workflows/measure.yml`) exports the six
redistributable reference models (Core Interior, Einhoven and the four RE1
models) and scores each export against Revit's own IFC export of the same
model with the scorers under `tools/re/`: element counts, ElementIds,
bodies, materials, storeys, openings and more. It is not part of
`CI passed`. It runs when the measurement itself changes and is dispatched
for a change to what the exporter writes. The reports under
`reports/element-framing/` cite the runs they rest on.

## Reproducing the evidence locally

```bash
# IfcOpenShell imports, and the arity check on the committed synthetic IFC files
tools/check-local.sh --ifcopenshell

# IfcOpenShell on the committed synthetic project (Python, with the
# versions CI pins)
pip install --require-hashes -r tools/ci/requirements-ifc.txt
python - <<'PY'
import ifcopenshell
f = ifcopenshell.open("tests/fixtures/synthetic-project.ifc")
assert f.schema == "IFC4"
print("project:", f.by_type("IfcProject")[0].Name)
print("storeys:", [s.Name for s in f.by_type("IfcBuildingStorey")])
print("walls:", len(f.by_type("IfcWall")))
PY
```

The real-project checks need the magnetar corpus in `_project_corpus/Revit`
([`CONTRIBUTING.md`](../CONTRIBUTING.md) says how to fetch it).

## What is not checked

- **buildingSMART's Validation Service** is not run in CI. Of its rules,
  only the standard property sets are mirrored, through the generated table
  above.
- **Viewers.** Once, `tests/fixtures/synthetic-project.ifc` was loaded in
  [BlenderBIM](https://blenderbim.org) 0.0.240509 without errors and showed
  the expected spatial hierarchy and geometry. That is a spot check, not a
  gate.

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
