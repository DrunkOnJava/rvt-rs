"""Authored mathematical fixtures, not decoded Revit geometry. No native API."""
import argparse
import json
import math
import os


def world(point, fixture):
    """Rigid transform: rotate about X, then translate; millimetres throughout."""
    x, y, z = point
    a = math.radians(fixture["tilt_x_degrees"])
    t = fixture["translation_mm"]
    return [x + t[0], math.cos(a)*y - math.sin(a)*z + t[1],
            math.sin(a)*y + math.cos(a)*z + t[2]]


def definitions():
    items = []
    for name, start, end, tilt, translation in [
            ("REV-A", 0, 360, 0, [0, 0, 0]),
            ("REV-B", 30, 120, 0, [0, 0, 0]),
            ("REV-BT", 30, 120, 37, [1200, -800, 350])]:
        f = dict(id=name, start_degrees=start, end_degrees=end,
                 inner_radius_mm=300, outer_radius_mm=500, height_mm=100,
                 tilt_x_degrees=tilt, translation_mm=translation)
        f["profile_points_mm"] = [world(p, f) for p in
            [[300, 0, 0], [500, 0, 0], [500, 0, 100], [300, 0, 100]]]
        f["axis_points_mm"] = [world(p, f) for p in [[0, 0, 0], [0, 0, 1000]]]
        a = math.radians(tilt)
        f["sketch_normal"] = [0, -math.cos(a), -math.sin(a)]
        f["analytic_volume_m3"] = math.radians(end-start)*0.5*(0.5**2-0.3**2)*0.1
        f["reference_sector_endpoints_mm"] = {
            label: [world([r*math.cos(math.radians(angle)),
                          r*math.sin(math.radians(angle)), z], f)
                    for r, z in [(300, 0), (500, 0), (500, 100), (300, 100)]]
            for label, angle in [("start", start), ("end", end)]}
        f["native_status"] = "not_created"
        items.append(f)
    return dict(schema="rectangular-revolution-fixtures-v1", units="mm",
                status="authoring_spec_not_native_evidence",
                reference_convention="right-handed rotation about directed axis; verify against native result",
                fixtures=items)


def reference_mesh(f, step_degrees=1.0):
    """Closed manifold polyhedral approximation; independent authoring reference."""
    start, end = f["start_degrees"], f["end_degrees"]
    n = int(math.ceil((end-start)/step_degrees))
    closed = abs(end-start-360) < 1e-10
    rings = n if closed else n+1
    vertices = []
    for j in range(rings):
        angle = math.radians(start+(end-start)*j/n)
        for radius, z in [(300, 0), (500, 0), (500, 100), (300, 100)]:
            vertices.append(world([radius*math.cos(angle), radius*math.sin(angle), z], f))
    triangles = []
    def quad(a, b, c, d):
        triangles.extend([[a, b, c], [a, c, d]])
    for j in range(n):
        nxt = (j+1) % rings
        for k in range(4):
            quad(j*4+k, nxt*4+k, nxt*4+(k+1)%4, j*4+(k+1)%4)
    if not closed:
        quad(0, 1, 2, 3)
        last = n*4
        quad(last+3, last+2, last+1, last)
    return vertices, triangles


def write_reference(output):
    if not os.path.isdir(output):
        os.makedirs(output)
    spec = definitions()
    for f in spec["fixtures"]:
        vertices, triangles = reference_mesh(f)
        path = os.path.join(output, f["id"]+"-reference.obj")
        with open(path, "w") as stream:
            stream.write("# Authored reference, millimetres. NOT Revit output.\n")
            for p in vertices:
                stream.write("v %.12g %.12g %.12g\n" % tuple(p))
            for t in triangles:
                stream.write("f %d %d %d\n" % tuple(i+1 for i in t))
    with open(os.path.join(output, "fixture-definitions.json"), "w") as stream:
        json.dump(spec, stream, indent=2, sort_keys=True)
        stream.write("\n")
    return spec


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output")
    args = parser.parse_args()
    write_reference(args.output)
