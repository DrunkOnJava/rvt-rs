# RE-141: the joins of duct and pipe fittings

**Date:** 2026-10-01
**Issues:** #528, #96; Discussion #112
**Artefacts:** `RE1-Mechanical.rvt` (Revit 2025, MIT; 25 ducts, 6 pipes, 35 fittings) and `RE1-Plumbing.rvt` (63 pipes, 54 fittings), with Revit's own IFC2X3 exports of both; the other two RE1 models for the negative. Measured on GitHub's hosted runners with the Measure workflow, runs 36900047336 (what holds a connector anchor), 36900900046 (the list scan) and 36902094526 (the export, head against main).
**Probes and scorers:** `examples/probe_re141_connector_anchors.rs`, `examples/probe_re141_list_scan.rs`, `tools/re/connector_pairs_vs_ifc.py` and `tools/re/ports_vs_ifc.py` (now also scoring every connection of Revit's export).
**Status:** positive, partial. A fitting's joins are in the same list form as a pipe's and are read; the export writes 90.4 and 95.2 per cent of all of Revit's port-to-port connections on the two models (72.6 and 88.9 per cent before), with no connection Revit does not hold on Mechanical and one on Plumbing, as before.
**Credit:** jakobhirn-bit (Discussion #112), whose reading of a pipe's connector entries led to the lists RE-138 reads and this reads again.

RE-138 left the connections of Revit's export that involve no duct or pipe unread (fitting to fitting, fitting to equipment), and its lead was that the pipes' joins to a fitting are stored on the pipe's side and a fitting's own are somewhere else. This finds where.

## 1. What a fitting's record holds, and what it does not

`probe_re141_connector_anchors.rs` dumps the whole record of the first duct and pipe fittings, and searches the schema for every class whose name holds `Connector` or `Port` (95 on RE1 Plumbing, among them `FamilyInstanceConnectorManager`, `RbsSystemConnectorManager` and `RbsCurveConnectorManager`).

- **A fitting's record holds the ids of the elements it joins, but no connector index.** Searching the four dumped RE1 Plumbing fittings for the ElementIds Revit joins them to finds every one, as a `u64`, in a list of ascending `u64` ElementIds inside the record: fitting 444103's holds 443721, 443932, 444009, 444103 (itself), 444104, 444106 and 444393, and Revit joins it to 444009, 444104 and 444393. The list is the record's reference list (#228), so it also holds the fitting's own id and elements that are not joins. Nothing in it says which connector a join is at.
- **The pipes' connector managers are not in the leading record chain.** Every `RbsCurveConnectorManager` anchor of RE1 Plumbing (63) and Mechanical (31) lies outside the partition's leading chain of records, so the joins RE-138 reads are in objects of their own, not in the pipe's record. No other connector class has an anchor of the `[tag][0xFF x 4][tag + 1]` form, so where a fitting's connector object is was not found by an anchor.

## 2. The joins are in the RE-138 list form, with the fitting first

`probe_re141_list_scan.rs` looks for the list RE-138 reads (`u32 2`, then two references of `u64` element, `u32` connector index, `u32 1`) at every offset of every partition, with a duct, pipe or fitting as the first reference and any element Revit's export names as the second.

Scored against the connections of Revit's export (`IfcRelConnectsPorts` between ports named `<In or Out>Port_<ElementId>_<index>`), the pairs by what the first reference is:

| model | first reference to second | pairs read | in Revit's export | in Revit's with the first index flipped |
|---|---|---:|---:|---:|
| RE1 Plumbing | fitting to fitting | 11 | 11 | 0 |
| | fitting to duct or pipe | 17 | 14 | 0 |
| | duct or pipe to fitting | 158 | 77 | 66 |
| | duct or pipe to other | 12 | 0 | 11 |
| RE1 Mechanical | fitting to fitting | 12 | 12 | 0 |
| | fitting to duct or pipe | 11 | 11 | 0 |
| | fitting to other (terminals, equipment) | 5 | 5 | 0 |
| | duct or pipe to fitting | 76 | 37 | 31 |
| | duct or pipe to other | 1 | 0 | 1 |

- **A fitting's list is written once and its own connector index is Revit's as stored.** Of the 56 distinct lists with a fitting first (28 on each model), none is Revit's with the index flipped. A duct's or pipe's lists are in two copies, the earlier with the index inverted (RE-138): the 66 and 31 pairs that are Revit's only when flipped.
- **Every connection of Revit's export that involves a fitting is read, on Plumbing, and all but two on Mechanical.** Plumbing: 99 of 99, with the index of each end exact. Mechanical: 60 of 62; the two not read join a fitting to elements (491631 and 490317) that are neither fittings nor ducts or pipes, so the list that names the join is one of theirs.
- **The 3 fitting-to-pipe pairs on Plumbing that are not Revit's** (fittings 447376, 447727 and 447766, to 447767, 447753 and 447767) all name an element Revit's export has no port for, the same kind of pair RE-138 found among the pipes' (an element Revit's export leaves out).

## 3. Result: the export

`partition_connector_pairs::scan_fitting_pairs` reads these lists for the file's fittings, and the export writes a port for each connector of a joined pair of distribution elements, as for ducts and pipes. Head against main on the Measure workflow (`tools/re/ports_vs_ifc.py`):

| model | Revit's connections | written before | written now | written, not in Revit's | involving a duct or pipe, before and now |
|---|---:|---:|---:|---:|---:|
| RE1 Mechanical | 73 | 53 (72.6%) | **66 (90.4%)** | 0 | 53 and 54 of 59 |
| RE1 Plumbing | 126 | 112 (88.9%) | **120 (95.2%)** | 1 | 112 and 112 of 118 |

- Every port rvt-rs writes is tied to the element its name gives, and IfcOpenShell's IFC4 schema validation of both files finds nothing (0 findings).
- Core Interior, Einhoven, RE1 Architecture and RE1 Electrical are byte-identical to main in every output.
- Ports on equipment are still left out: `IfcRelConnectsPortToElement` relates a port to an `IfcDistributionElement`, and rvt-rs writes the RE1 Mechanical equipment as an `IfcBuildingElementProxy` (RE-138).

## 4. What this does not claim

- **No flow direction, no port placement**, as RE-138.
- **Not the 7 (Mechanical) and 6 (Plumbing) connections left.** They are joins to equipment written as a proxy and joins that no list read here gives; which is which was not split.
- **The lists are found by their form alone**, anywhere in a partition, with the elements of the file as the only filter. Where a fitting's connector object sits, and what class it is, was not established, so a list's form can be matched by bytes that are not a connector's; none was on the two models (every pair read for a fitting is a connection of Revit's except three that name elements Revit's export leaves out).
- Revit 2025 only: the layout is measured on the RE1 models.
