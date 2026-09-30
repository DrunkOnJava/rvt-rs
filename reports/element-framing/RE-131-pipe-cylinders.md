# RE-131: pipes as cylinders

**Date:** 2026-09-30
**Issues:** #96 (MEP); follows RE-130 (pipe types)
**Artefacts:** RE1 Mechanical and Plumbing (Revit 2025, MIT), each against Revit's own IFC export. Every run is on GitHub's hosted runners with the Measure workflow: the probes are 36719977712 and 36720663075, the production path against `main` is 36723473598.
**Probe:** `examples/probe_re131_pipe_sections.rs`; scored by `tools/re/pipe_bodies_vs_ifc.py`.
**Status:** positive on Revit 2025 where the ends are proven. Sloped pipes and other releases are not measured here.
**Credit:** the entry layout and the radius derivation are jakobhirn-bit's (Discussion #112), measured by them on Autodesk's Snowdon Towers plumbing sample.

## 1. What a pipe's element record holds

Nothing that says how a pipe sits in its box. On RE1 the records of pipes 441731 and 441849 are 335 bytes each: a header, the bounding box and the references. Revit's own IFC writes each pipe as an `IfcFlowSegment` whose body is an `IfcExtrudedAreaSolid` over an `IfcCircleProfileDef`: a start point, a direction, a length and a radius. rvt-rs wrote the box.

## 2. Where the ends are

The partition holds connector entries: a `u64` element, a `u32` connector index, the `u32` 1, and three `f64`, the end's point. The probe looked, in every partition, for three `f64` inside each pipe's box. On RE1 the entries hold exactly the points at which Revit's IFC starts and ends each pipe's extrusion, for the four pipes read by hand (441731, 441849, 441892, 442026) and then for all 69 (section 3). The same entries recur in the neighbouring partition (`Partitions/79` holds 441731's start entry, at the same point).

Two things to know. At a connected end the entry's element is the one connected there: pipe 442026's two ends are keyed by 442040 (index 2) and 442191 (index 1), two Closet Bend fittings that Revit's export places 0.1413 ft beyond the pipe's ends on its axis, each beside the end it is keyed for, and pipe 441892's second end is keyed by 442037. Pipes 441731 and 441849 have both entries keyed by the pipe itself; what sits at their ends was not checked. And the two entries are not fixed apart: 441731's are 58 bytes apart (2465867 and 2465925 of `Partitions/78`), 442026's are 74 apart in one place (4326840 and 4326914) and 3,889 apart in another (3944640 and 3948529). So an entry is found by where its point falls, never by whose it is. Other elements' connectors lie at the same points too: at 765622 an entry of element 427568 (index 11) sits 0.009 ft along the axis from 441731's start.

## 3. The radius, and the proof

No field stores the outside radius. A cylinder of radius `r` and length `L` along the unit vector `u` has the half extent `|L·u_q|/2 + r·sqrt(1 − u_q²)` on axis `q`, so each axis that is not nearly the pipe's own gives `r`. `partition_pipe_axes::pipe_body` takes a pair of points only if those radii agree to 0.002 ft and the box rebuilt from the cylinder is the record box to 0.002 ft on all six sides. `best_pipe_body` takes the furthest-apart pair that passes, and none where two different cylinders of that length do.

Applied to the probe's hits (points inside the box that follow a `u32` 1) for every pipe in RE1, against Revit's cylinder for the same ElementId: 6 of 6 on Mechanical and 63 of 63 on Plumbing solved, none wrong, none unsolved; ends within 0.01 ft, radius within 0.001 ft. An earlier rule, which paired a pipe's own start entry with the first entry after it, found 26 of Plumbing's 63 pipes and got one wrong (444104: a connector 0.009 ft from the start was taken as the end, and the radii still agreed). The rebuild is what rejects such a pair.

## 4. What changes

Run 36723473598, this branch against `main`, scored per pipe by `pipe_bodies_vs_ifc.py`:

| | `main` | RE-131 |
|---|---:|---:|
| RE1 Plumbing pipes with a circle profile | 0 of 63 | **63 of 63** |
| radius within 0.001 ft of Revit's | 0 of 63 | **63 of 63** |
| axis ends within 0.01 ft of Revit's | 21 of 63 | **63 of 63** |
| length within 0.01 ft of Revit's | 21 of 63 | **63 of 63** |
| RE1 Mechanical pipes: circle profile and radius | 0 of 6 | **6 of 6** |
| RE1 Mechanical pipes: axis ends and length | 2 of 6 | **6 of 6** |

The worst differences are under 0.00001 ft. The 23 pipes `main` had right (21 of Plumbing's 63 and 2 of Mechanical's 6) are presumably the vertical ones, since its box was extruded along Z; that is inferred, not listed. The export diagnostics count the new body: `partition_pipe_axis` is 63 on Plumbing and 6 on Mechanical, and the record-box bodies fall from 124 to 61 and from 74 to 68. The IFC of Core Interior, Einhoven, RE1 Architecture and RE1 Electrical is identical to `main`'s, and no other scorer's output changes on any model.

## 5. Not measured

- **Sloped pipes.** All 69 pipes on RE1 run along a principal axis, so for them the box alone fixes the cylinder and the CI oracle cannot exercise the derivation. The proof is what guards the general case: a pair that does not reproduce the box on all six sides is not taken. jakobhirn-bit's figure for Snowdon Towers' plumbing sample, 3,006 of 3,056 pipes passing, is theirs on a file that cannot be run in CI; it is not claimed here.
- **Other releases.** The layout is measured on Revit 2025 here, so `PIPE_AXIS_SUPPORTED_REVIT_VERSIONS` lists 2025. jakobhirn-bit reads the same entries on 2024 and 2026; those are theirs.
- **Ducts and pipe fittings** keep their boxes: 25 of RE1 Mechanical's `IfcFlowSegment` are ducts, whose family (rectangular, round or oval) follows a shape that is not read here, and fittings are elbows and tees, which jakobhirn-bit reports as swept blends their shape builder cannot draw yet.
- **Which end is which.** The scorer takes the closer pairing of the two ends, so the entry rvt-rs calls the start may be Revit's end.
