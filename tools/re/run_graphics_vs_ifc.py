#!/usr/bin/env python3
"""Which of a stair run's own solids is the flight Revit's export draws (#357).

Takes the output of `examples/probe_357_run_graphics.rs` (each run's
primitives' vertices, by the `Geometry` object they come from, in Revit's
internal feet) and Revit's IFC export. For every `IfcStairFlight`, or landing's
`IfcSlab`, whose Tag is a run or landing the probe printed, Revit's body is meshed with IfcOpenShell and
moved into Revit's internal frame through the export's site placement. For
each of the run's `Geometry` objects, and for all of them together, this
reports, in feet:

- covered: how far the farthest of Revit's vertices lies from the nearest
  vertex of the run's (0 when every one of Revit's vertices is the run's);
- extra: how far the farthest of the run's vertices lies from Revit's.

A solid that is the flight has both near 0.

Needs IfcOpenShell and NumPy.

Usage:

    python3 tools/re/run_graphics_vs_ifc.py <probe.txt> <revit.ifc> [--list]
"""

import collections
import sys

import ifcopenshell
import ifcopenshell.geom
import ifcopenshell.util.placement as placement
import ifcopenshell.util.unit as unit
import numpy

FT = 0.3048


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    vertices = collections.defaultdict(lambda: collections.defaultdict(list))
    summary = {}
    for line in open(args[0], encoding="utf-8", errors="replace"):
        p = line.split()
        if p and p[0] == "vertex":
            vertices[p[1]][int(p[2])].append([float(v) for v in p[3:6]])
        elif p and p[0] == "graphics":
            summary[p[1]] = " ".join(p[2:])
    model = ifcopenshell.open(args[1])
    scale = unit.calculate_unit_scale(model) / FT
    sites = model.by_type("IfcSite")
    inverse = numpy.linalg.inv(placement.get_local_placement(sites[0].ObjectPlacement)) if sites else numpy.eye(4)
    settings = ifcopenshell.geom.settings()
    settings.set("use-world-coords", True)
    print(f"runs with graphics: {len(vertices)}; runs printed: {len(summary)}")
    for tag, line in sorted(summary.items()):
        print(f"  {tag}: {line[:200]}")
    # A landing is an IfcSlab in Revit's export (#824).
    for flight in list(model.by_type("IfcStairFlight")) + list(model.by_type("IfcSlab")):
        tag = str(flight.Tag or "")
        if tag not in vertices:
            continue
        try:
            shape = ifcopenshell.geom.create_shape(settings, flight)
        except Exception as error:
            print(f"  {tag}: Revit's body not meshed ({error})")
            continue
        world = numpy.array(shape.geometry.verts, dtype=float).reshape(-1, 3)
        homogeneous = numpy.c_[world / unit.calculate_unit_scale(model), numpy.ones(len(world))]
        revit = (inverse @ homogeneous.T).T[:, :3] * scale
        revit = numpy.unique(numpy.round(revit, 6), axis=0)
        objects = dict(vertices[tag])
        objects["all"] = [v for vs in vertices[tag].values() for v in vs]
        print(f"{flight.is_a()} {tag}: Revit {len(revit)} vertices, z {revit[:, 2].min():.4f} to {revit[:, 2].max():.4f}")
        for name, points in objects.items():
            ours = numpy.array(points)
            d = numpy.linalg.norm(revit[:, None, :] - ours[None, :, :], axis=2)
            covered = float(d.min(axis=1).max())
            extra = float(d.min(axis=0).max())
            print(f"  object {name}: {len(ours)} vertices, covered {covered:.4f} ft, extra {extra:.4f} ft")


if __name__ == "__main__":
    main()
