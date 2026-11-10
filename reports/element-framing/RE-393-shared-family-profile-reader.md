# RE-393: Shared line/arc sketch decoding and bounded revolution reading

**Date:** 2026-11-10. **Issue:** #156. Follows RE-327, RE-391 and RE-392.

## What changes

`FormDocument::sketch_curves(sketch_id)` now returns typed driven `Curve3`
values with their element/plane identities. Both extrusion and revolution
readers use it. It follows `CurveElem.m_pCurveDriver` and then `m_pCrv`, checking
the saved graph edges. The existing GLine/GArc parser is reused; auxiliary
centre-marker geometry is not selected as a profile curve. Extrusion coordinate
conventions and stored-loop comparisons are retained.

`FormDocument::revolution_ids()` inventories forms; `revolution(id)` decodes
sketch membership, flags, the marked axis and the two angle parameters. It
returns a typed `Revolution` with a radial/axial `Profile`, family-space axis
and radial directions, angles and source identities. `Revolution::point_at`
evaluates this profile at a sweep angle. `Revolution::mesh()` produces a closed
triangle mesh entirely in Rust. No Python recipe, IFC or native API value is an
input to that decoding or meshing.

The reader has no rectangle/torus dimension tables or fixed element IDs. It
uses the existing line/arc evaluation and loop assembly for one profile, and
samples that profile for a bounded convex-loop cap. A line-only profile uses
at most 0.25-degree sweep steps; profiles with arcs use at most 0.75-degree
profile and sweep steps. Sampling cannot exceed 4,096 profile points or
1,000,000 mesh vertices. Convexity is checked against every sampled edge, and
axis-crossing/off-plane profiles are refused.

## Scope and refusals

Measured: the six standalone Revit 2020 rectangular and circular fixtures
from RE-391/RE-392. Generic line/arc mixtures use the same code, but no native
mixed-profile fixture has yet been measured; no coverage claim is made for it.
The new reader accepts one positive, unflipped revolution with one closed
convex loop clear of the axis. Holes, nonconvex loops, voids, flipped flags,
old-axis flags, missing/ambiguous driver links, repeated referenced records,
invalid arc frames/radii, unsupported primitive classes and ambiguous angle
parameters are refused. Repaired profile gaps are also refused.

The existing extrusion API remains available and uses the shared curve path.
The research API and compiled probe expose it through `unstable-research`;
the default IFC adapter uses it as described in RE-394. The shared path also
serves the existing extrusion exporter, whose regression checks are below.

## Native-file replay

`probe_decoded_revolutions` opens each RFA, frames its native records, checks
complete graph consumption, invokes the Rust library and writes typed profile
and mesh JSON. It does not read prior evidence JSON or a fixture recipe.
`verify_reader.py` runs it and the independent evidence probe on all six files.
It compares decoded profile samples, axis and angles against the separate
Python byte interpretation; their largest profile-sample difference is below
0.000000001 mm. Every Rust mesh has closed, oriented edge incidence and positive
volume, with relative volume error below the fixed 0.01% bar.

| Fixture | Rust vertices | Rust triangles | Largest sampled native distance/bound (mm) | Rust volume error (%) |
| --- | ---: | ---: | ---: | ---: |
| REV-A | 5760 | 11520 | 0.00152019982 | 0.00031730951 |
| REV-B | 1446 | 2888 | 0.00151543804 | 0.00031730951 |
| REV-BT | 1446 | 2888 | 0.00151543805 | 0.00031730951 |
| ARC-A | 230400 | 460800 | 0.0114229811 | 0.00571144613 |
| ARC-B | 58082 | 116160 | 0.0114229811 | 0.00571144613 |
| ARC-BT | 58082 | 116160 | 0.0114229811 | 0.00571144613 |

For rectangular fixtures the measured mesh is the Rust output, compared in
both directions against native IFC with the existing scorer. The small extra
cap vertices in the partial rectangles are a consequence of the generic cap
fan; the native comparison remains within the original bar.

For circular fixtures the native references are the unchanged Revit face
samples and captured projections from RE-392. All native face samples are
measured to the Rust mesh. The 800 captured query locations must also lie on
the Rust mesh within 1e-8 m. For each such location, its nearest Rust point is
within (query-to-Rust distance + native projection distance) of the captured
native point; the replay reports the conservative maximum of this bound.
This is a sampled bidirectional check, not a full Hausdorff proof. The original
IFC discrepancy and tilted-IFC timeout remain separate limitations; they have
not been turned into IFC passes. Cleanup subsequently used native save/reopen
+checks to establish unchanged geometry; see the archive metadata receipt.

## Regression and negative controls

The RE-327 extrusion probe was run before and after on 14 installed Revit
2020/2024 sample families. Its output is byte-identical in every case: 30 decoded
extrusions across 9 files; the other 5 files have no decoded standalone
extrusions in this probe. This is not a claim that every form in those files
is supported. All input hashes remain unchanged.

Twenty-two controls mutate decoded native records, not source files: nine cases
on REV-B and those nine plus four arc cases on ARC-B. The compiled reader
refuses missing curves/axes, repeated records, voids, duplicate angles, flipped
curves, old-axis flags, missing driver edges, negative radii, nonunit arc frames,
filled arcs, an unsupported curve class and small gaps at internal profile joins.
The gap controls shorten one native line/arc without changing the closing join;
the revolution reader checks every join at 1e-9 ft before returning a profile. These are refusal tests, not native
measurements of the mutated variants. The source RFAs remain unchanged.

The final exporter integration retains 46 byte-identical exports on 23
existing files in default and optional family-extrusion modes (RE-394).
The new standalone bodies are intentional additions. All 302 configured
integration checks pass; required local gates are recorded in RE-394.
No workflow, dependency or build-file changes are included.

## Reproduction and provenance

See `tools/oracle/runner/revolutions/README.md` for the two-probe replay. It
requires no Revit installation once the native fixture/oracle files exist.
The rectangular bundle is already public under RE-391. The combined cleaned
six-fixture package has a manifest and one-command replay;
see RE-392 and the runner README.

Sources are the existing clean-room line/arc and profile code, RE-391/RE-392's
synthetic bytes, and their independent native references. The native oracle
is validation only. No Autodesk SDK implementation, ODA source, decompilation,
NDA material or copied/translated revit-ifc code is used.
