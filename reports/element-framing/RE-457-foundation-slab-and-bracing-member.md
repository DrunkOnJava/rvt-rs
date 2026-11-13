# RE-457: a floor of the Structural Foundations category is an IfcSlab and a horizontal bracing an IfcMember, where rvt-rs wrote an IfcFooting and an IfcBeam

**Date:** 2026-11-13
**Issues:** #1841 (a Revit 2027 structural model whose 59 foundation slabs Revit writes as `IfcSlab` and rvt-rs as `IfcFooting`, and one framing member Revit writes as `IfcMember` and rvt-rs as `IfcBeam`), #1829
**Artefacts:** a reporter's Revit 2027 structural model with Revit's IFC4 export of the same save (confidential; their counts only, read through the Revit API and from the export); Autodesk's Snowdon Towers Structural sample (Revit 2024) and `rst_basic` (Revit 2023 to 2027); Autodesk's exporter source (revit-ifc, CLEANROOM.md source 5: cited, not quoted)
**Probe:** `probes/examples/probe_1841_framing_usage.rs` (the reporter ran its Windows build from `probe-binaries.yml`)
**Runs:** Measure [43465228601](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43465228601) (the exports of both sides kept for Snowdon Towers Structural, `rac_basic` and `rst_basic` of 2024, at `d11e99cb`) and [43465218624](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43465218624) (unfiltered, the same head)
**Status:** the floor rule is read on the open models (section 5); both rules wait on the reporter's counts after a release, since no open model with a Revit export has a foundation or a bracing.

## 1. The question

The reporter's model has 60 elements whose IFC class differs from Revit's export: 59 foundation slabs (`Losa de cimentación`) that Revit writes as `IfcSlab` and rvt-rs as `IfcFooting`, and one framing instance that Revit writes as `IfcMember` and rvt-rs as `IfcBeam`. `category_map` maps the whole `StructuralFoundation` category to `IfcFooting` and the whole `StructuralFraming` category to `IfcBeam`. What in the file tells the elements apart, and what is Revit's rule?

## 2. What Revit's exporter does

- **A floor of the category.** In the exporter's source, `Exporter.cs` sends every `Floor` to `FloorExporter` whatever its category and only a `WallFoundation` to `FootingExporter`; the class then comes from the class table. The reporter's model uses Revit's shipped template (`exportlayers-ifc-IAI`, no category mapping of its own), which maps Floors to `IfcSlab` and the Structural Foundations category to `IfcSlab` with `BASESLAB`. Their 59 are Revit floors in that category: all 59 are `IfcSlab` in Revit's export, none sets "Export to IFC As", their 4 `IfcSlabType` carry `BASESLAB`, and the occurrences carry `$` (a Revit 2027 save, whose occurrences state none, #1692).
- **A bracing.** `ExporterUtil.GetSpecificCategoryForElement` swaps a framing instance's category for the sub-category of its structural usage before the class is looked up, so a bracing need not be a beam. On the reporter's model, of 915 framing instances, the 485 with stored structural usage 3 (Girder in Revit's schedule) and the 429 with 6 (Other) are `IfcBeam` in Revit's export, and the one with 8 (Horizontal Bracing) is `IfcMember`; the stored value equals the API's in all three, as they read it.

## 3. What the file says

- **The floor.** An element's record names its own class (`RevitClass`, RE-76). Snowdon Towers Structural has 90 `IfcFooting`: 77 family instances, 9 `ContFooting` (wall footings) and 4 of `RevitClass` `Floor` in the category Structural Foundations. `rst_basic` has 36: 32 family instances and 4 `ContFooting`. The 4 floors are the case; the others are footings.
- **The usage.** A framing instance's `m_structUsage` (`InsertableInst`, a 4-byte integer) is in its native record. The open samples carry the values 3, 4, 5 and 6 (`rst_basic` 2023 to 2027: 5, 198, 102 and 65; Snowdon Towers Structural: 313, 602, 25 and 2) and all are `IfcBeam` in rvt-rs; no Revit export of them says otherwise, and 8 is in none. The two members of `rst_basic` without a value are stair supports.

## 4. The change

- A floor of the Structural Foundations category (`RevitClass` `Floor`) is written as `IfcSlab` with `BASESLAB` (`category_map::FOUNDATION_SLAB`); a family instance or a `ContFooting` of the category stays `IfcFooting`.
- A framing instance whose stored structural usage is 8 is written as `IfcMember` (`category_map::HORIZONTAL_BRACING_MEMBER`), with no `PredefinedType` of its own, since none is measured; every other value stays `IfcBeam`. The usage is read in the pass that already reads the beams' rotations, so the export makes no further pass over the file.

## 5. Rows, registered on #1841 before the Measure of this change was dispatched, and what the runs read

1. **Holds.** The 4 floors of the category in Snowdon Towers Structural (tags 680685, 680724, 784139 and 784189) go from `IfcFooting` to `IfcSlab` with `BASESLAB` on the occurrence (a Revit 2024 file), with the same GlobalIds; its 86 other `IfcFooting` and every other element are as before (1,282 elements, 4 differ).
2. **Holds on the 2024 save.** `rst_basic` of 2024 differs in no element (454 of 454; its 36 `IfcFooting` stay). The other saves are read on the unfiltered run.
3. **One more file holds a floor of the category.** `rac_basic` of 2024 has one (tag 766598, type `150mm Foundation Slab`), which row 3 allowed; it is the only other element of the kept exports that changes (453 elements, 1 differs).
4. **What the 5 gain besides the class.** The `Floor` system family and the name `Floor:<type name>:<ElementId>` as Revit names a floor, an `IfcSlabType` named `Floor:<type name>` that carries `BASESLAB` (row 6 below), and the property sets of a floor (`Pset_SlabCommon` with `IsExternal` false, `Identity Data`, `Pset_EnvironmentalImpactIndicators`, `Pset_ReinforcementBarPitchOfSlab`). No other property of theirs changes.
5. **Same-project gates:** pending.
6. **Holds.** Each of the 5 has an `IfcSlabType` with `BASESLAB`; on a Revit 2027 save the occurrence states none by the rule for slabs (#1692), and the type keeps the value, as the reporter's 59 do. The first kept run (43463267911, the floor commit alone) showed these floors with no type object, which is why the system family name is derived for them.
7. **The member rule.** No open sample carries usage 8 (their values are 3 to 6), so no `IfcMember` is added and no `IfcBeam` is lost; the read adds no pass over the file and the unfiltered run, pending, shows whether any export differs for it.

## 6. Not changed, and what is left out

- The `PredefinedType` of the member: the reporter's export holds it and it is not read here yet.
- Any other structural usage value. Only 8 is observed to move; the values 3 to 6 of the samples stay `IfcBeam`, and a bracing or kicker-bracing usage of another value has no observation.
- The bodies and quantities of the foundation slabs: only their class and predefined type change.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<sha of this change> -f base=<its merge-base with main> -f samples=true -f nordic=true -f r2025=true -f racadv=true -f holdout=true -f bimcollab=true -f families=true
gh workflow run probe-binaries.yml -f probe=probe_1841_framing_usage   # for a model that cannot leave its machine
```
