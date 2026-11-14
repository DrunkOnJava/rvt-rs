# RE-468: a gutter is an IfcFlowSegment, typed by an IfcPipeSegmentType GUTTER

**Date:** 2026-11-14
**Issues:** #1989; follows RE-319 (conduits, conduit fittings and gutters) and RE-402 sections 6.1 and 7 (the BIM4LCA housing, where "ducts" were found missing)
**Artefacts:** the BIM4LCA housing (the six files; Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them) and office; BIM-Speed's Vitoria `ALD26 Before` (Revit 2018, IFC2X3 export, held out, CC BY 4.0)
**Runs:** probe [43583970009](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43583970009) (`probe_re45_ifc_export_parameters` on the housing and office); Measure [43579535220](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43579535220) (`duct_bodies_vs_ifc` and `class_sets_vs_ifc` on `main`); RUN_FINAL (the rule)
**Status:** Positive. The "ducts" #1989 asks about are three gutters; rvt-rs wrote all three, in the wrong class.

## 1. What #1989 found, and what it is

RE-402 reports that `duct_bodies_vs_ifc` reads 3 ducts in Revit's export of the housing and 1 in rvt-rs's, the box of the one 3.0 ft off. The 3 are not ducts. They are the `Gutter` elements 1567789, 1567790 and 1629154 (family `HLV-1.125`), which the scorer counts with the flow segments. On `main` the scorer reads "ducts in Revit's export with a body: 3; rvt-rs writes one for 1 of them; world box within 0.01 ft of Revit's: 1 of 1 (worst 0.00000 ft)": the offset is gone since the scorer's site-frame repair, and rvt-rs writes all three, but as other classes.

## 2. What Revit's export writes

Each of the three gutters, in each of the six housing files, is an `IfcFlowSegment` with no `PredefinedType` (IFC4's `IfcFlowSegment` declares none), named `Gutter:HLV-1.125:<ElementId>`, with a body, typed by one `IfcPipeSegmentType` `GUTTER` (`Gutter:HLV-1.125:459261`), and holding `Pset_EnvironmentalImpactIndicators.Reference` and `Pset_PipeSegmentTypeCommon.Reference`; no quantity set. rvt-rs wrote 1567789 and 1567790 as `IfcCovering` `ROOFING` and 1629154 as `IfcPipeSegment`.

## 3. Why

Two of the gutters carry an "Export to IFC As" of `IfcCovering` with `ROOFING`, as do the roofing elements around them (the probe lists more than a dozen elements with the same pair); the third carries none. Revit's export writes all three the same way, so it acts on neither: the `IfcCovering` value is ignored on a gutter (as a wall sweep's is, RE-192), and the default for a gutter is `IfcFlowSegment`. The office's 24 gutters carry `IfcPipeSegment` with `GUTTER` and Revit writes them as `IfcPipeSegment` `GUTTER` on both files, so that value is acted on. Vitoria's 2018 IFC2X3 export writes its 2 gutters as `IfcFlowSegment`, which IFC2X3 has no subtype to refine; RE-319 took `IfcPipeSegment` from their type objects (2 of 2) and from IFC4's `GUTTER`, and the housing is the first IFC4 export that holds a gutter without the value.

## 4. What rvt-rs writes

A gutter is an `IfcFlowSegment` (no `PredefinedType`), typed by an `IfcPipeSegmentType` `GUTTER`, unless its own or its type's "Export to IFC As" names `IfcPipeSegment`, when it is an `IfcPipeSegment` with the element's own or its type's predefined type where that is an `IfcPipeSegmentTypeEnum` enumerator (#1713). An `IfcCovering` value on a gutter is not acted on, and the export reports it as an unapplied value like any other.

## 5. Measured

RESULTS_SLOT

## 6. Not shown

- **Names and property sets.** Revit names the gutters `Gutter:HLV-1.125:<ElementId>` and gives them the two property sets above; rvt-rs writes `Gutter-<ElementId>` and its own sets. Gutter type names are not read (RE-319).
- **Other flow elements.** Whether an unparameterised conduit is also an `IfcFlowSegment` in an IFC4 export is not measured: no IFC4 export holds one.
- **The office's gutters without a value.** None: all 24 of its gutters carry `IfcPipeSegment`.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=<head> -f base=main -f nordic=true \
  -f models="nordic-housing-* nordic-office-as-built" -f scorers="class_sets_vs_ifc duct_bodies_vs_ifc predefined_types_vs_ifc type_objects_vs_ifc psets_vs_ifc"
gh workflow run measure.yml --ref main -f ref=<head> -f base=none -f nordic=true \
  -f models="nordic-housing-concrete-as-built nordic-office-as-built" -f scorers=none \
  -f probe=probe_re45_ifc_export_parameters -f probe_scorer=-
```

`<head>` is the merge commit of the pull request that carries this report.
