"""Verify exported reference meshes using topology, volume and centroid integrals."""
import collections
import json
import math
import os
import sys


def dot(a, b):
    return sum(x*y for x, y in zip(a, b))


def cross(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]


def check(folder):
    with open(os.path.join(folder, "fixture-definitions.json")) as stream:
        spec = json.load(stream)
    results = []
    for f in spec["fixtures"]:
        vertices, triangles = [], []
        with open(os.path.join(folder, f["id"]+"-reference.obj")) as stream:
            for line in stream:
                if line.startswith("v "):
                    vertices.append([float(v)/1000 for v in line.split()[1:]])
                elif line.startswith("f "):
                    triangles.append([int(i)-1 for i in line.split()[1:]])
        edges = collections.Counter()
        volume, moment = 0.0, [0.0]*3
        for t in triangles:
            a, b, c = [vertices[i] for i in t]
            v = dot(a, cross(b, c))/6
            volume += v
            for k in range(3):
                moment[k] += v*(a[k]+b[k]+c[k])/4
            for i, j in zip(t, t[1:]+t[:1]):
                edges[i, j] += 1
        assert all(n == 1 and edges[j, i] == 1 for (i, j), n in edges.items()), "open or misoriented mesh"
        assert volume > 0, "inward or degenerate mesh"
        a, b = math.radians(f["start_degrees"]), math.radians(f["end_degrees"])
        analytic = 0.5*(.5**2-.3**2)*.1*(b-a)
        # Integrate r^2 dr dtheta dz independently of the mesh construction.
        radial_moment = 2*(.5**3-.3**3)/(3*(.5**2-.3**2))
        x = radial_moment*(math.sin(b)-math.sin(a))/(b-a)
        y = radial_moment*(math.cos(a)-math.cos(b))/(b-a)
        z = .05
        tilt = math.radians(f["tilt_x_degrees"])
        expected = [x, y*math.cos(tilt)-z*math.sin(tilt), y*math.sin(tilt)+z*math.cos(tilt)]
        expected = [v+t/1000 for v, t in zip(expected, f["translation_mm"])]
        centroid = [m/volume for m in moment]
        volume_error = abs(volume-analytic)/analytic
        centroid_error = math.sqrt(sum((p-q)**2 for p, q in zip(expected, centroid)))
        assert volume_error < .0001, "volume differs from analytic sector"
        assert centroid_error < .00002, "wrong angular sector or transform"
        assert abs(analytic-f["analytic_volume_m3"]) < 1e-14
        results.append(dict(id=f["id"], triangles=len(triangles), volume_m3=volume,
            analytic_volume_m3=analytic, relative_volume_error=volume_error,
            centroid_error_m=centroid_error, closed_oriented_manifold=True))
    return dict(status="passed", scope="mathematical reference only; no Revit validation", results=results)


if __name__ == "__main__":
    print(json.dumps(check(sys.argv[1]), indent=2, sort_keys=True))
