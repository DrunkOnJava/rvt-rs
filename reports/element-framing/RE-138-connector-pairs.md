# RE-138: which element each end of a duct or pipe is joined to

**Date:** 2026-10-01
**Issues:** #528, #96; Discussion #112
**Artefacts:** `RE1-Mechanical.rvt` (Revit 2025, MIT; 25 ducts and 6 pipes) and `RE1-Plumbing.rvt` (63 pipes), with Revit's own IFC2X3 exports of both; the other two RE1 models for the negative. Measured on GitHub's hosted runners with the Measure workflow, runs 36872401031 (the lists) and 36874925630 (the export).
**Probe and scorers:** `examples/probe_re138_connector_pairs.rs`, `tools/re/connector_pairs_vs_ifc.py` (the lists against Revit's connections) and `tools/re/ports_vs_ifc.py` (rvt-rs's export against Revit's).
**Status:** positive, partial. The lists reproduce 94 per cent of Revit's connections that involve a duct or pipe on both models; the export writes 90 and 95 per cent of them, with no flow direction.
**Credit:** jakobhirn-bit (Discussion #112), whose reading of a pipe's connector entries, keyed by the connected element, led RE-131 and RE-134 to the lists read here.

## 1. The lists

A duct's or pipe's object holds, after its `RbsCurveConnectorManager` anchor (RE-134), its connectors' joins as counted lists of two references:

```text
+0   u32 2
+4   u64 the duct or pipe itself   u32 connector index   u32 1
+20  u64 the element joined to it  u32 connector index   u32 1
```

Revit's export writes each connector as an `IfcDistributionPort` named `<In or Out>Port_<ElementId>_<index>` and each join as an `IfcRelConnectsPorts`, so a list is a connection of Revit's by the same two numbers on each side.

The lists come in two blocks, the earlier at 1,021 to 1,130 bytes from the anchor and the later at 1,217 to 1,362 on both models. They give the same joins with the curve's own index stored differently:

- **In the later block the curve's index is Revit's.** Every list of the later block that names a connection of Revit's does so as stored.
- **In the earlier block it is inverted** (0 for the connector Revit numbers 1, and 1 for 0). Every list of the earlier block that names a connection of Revit's does so inverted.
- **The other element's index is Revit's in both.**
- On some curves the earlier block holds one-reference lists instead of two, so the later block is the only copy; a join with only an earlier-block list is read inverted. Reading every join by its number of copies, which was tried first, gave 64 and 71 per cent, because a join with one list is not always the earlier one.

## 2. Result: the lists

`partition_connector_pairs::scan_connector_pairs` reads the lists of every duct and pipe. Against the connections of Revit's export whose ports are named for a duct or pipe (`tools/re/connector_pairs_vs_ifc.py`):

| model | ducts and pipes | pairs read | Revit's connections involving one | read | not read | read, not in Revit's export |
|---|---:|---:|---:|---:|---:|---:|
| RE1 Mechanical | 31 | 56 | 53 | **50 (94.3%)** | 3 | 6 |
| RE1 Plumbing | 63 | 113 | 108 | **102 (94.4%)** | 6 | 11 |

Of the pairs read that Revit's export does not hold, 5 of 6 on Mechanical and 4 of 11 on Plumbing are pairs in which neither element has a port in Revit's export; the others are pairs where Revit has a port for one element of the pair. Why Revit's export has no port for those elements is not established.

## 3. Result: the export

rvt-rs now writes a port for each connector in a join whose two elements it exports, an `IfcRelConnectsPortToElement` for each port and an `IfcRelConnectsPorts` for each join (`tools/re/ports_vs_ifc.py`, against Revit's export):

| model | Revit's ports | Revit's connections | involving a duct or pipe | rvt-rs's ports | rvt-rs's connections | of Revit's, written | written, not in Revit's | ports not tied to their element |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| RE1 Mechanical | 150 | 73 | 59 | 106 | 53 | **53 (89.8%)** | 0 | 0 |
| RE1 Plumbing | 260 | 126 | 118 | 226 | 113 | **112 (94.9%)** | 1 | 0 |
| RE1 Electrical | 53 | 0 | 0 | 0 | 0 | | | |
| RE1 Architecture | 0 | 0 | 0 | 0 | 0 | | | |

- Every port rvt-rs writes is tied to the element its name gives, and IfcOpenShell's IFC4 schema validation of both files finds nothing (0 findings).
- **A port is tied only to a distribution element.** IFC4's `IfcRelConnectsPortToElement` relates a port to an `IfcDistributionElement`. On RE1 Mechanical three of the lists' joins are to equipment rvt-rs writes as an `IfcBuildingElementProxy`; written, they were three schema errors in IfcOpenShell's validation, and they were 56 of the 59 instead of 53 before that rule. Revit's own IFC2X3 export puts ports on such proxies (5 of RE1 Mechanical's 150), which its schema allows.
- Nothing else in the Measure outputs changes: the other scorers' outputs are identical to main's, and Core Interior's export, which has no duct or pipe, is byte-identical.
- The ports are named `Port_<ElementId>_<index>`, with no `In` or `Out`.

## 4. What this does not claim

- **No flow direction.** Revit names a port `InPort_` and gives it `.SINK.`, or `OutPort_` and `.SOURCE.`; across the 134 named ports of RE1 Mechanical the prefix and the direction agree on all 134, and the same connector index is In on some pipes and Out on others, so the direction is the system's flow, which the lists do not carry. rvt-rs writes `.NOTDEFINED.`. A fitting's or equipment's own direction lives in its family.
- **Only connections that involve a duct or pipe.** Fitting to fitting and fitting to equipment connections, which are in Revit's export (14 of RE1 Mechanical's 73 and 8 of Plumbing's 126 involve no `IfcFlowSegment`), are not read. RE-141 reads them: a fitting's joins are lists of the same form with the fitting first.
- **Not the 3 and 6 connections that were not read** on the two models. They are connections of a duct or pipe whose lists, as read, do not give the join; what is different about them is not established.
- **Port placement.** A port has no `ObjectPlacement`; Revit's has one at the connector's origin.
- Revit 2025 only: the layout is measured on the RE1 models.
