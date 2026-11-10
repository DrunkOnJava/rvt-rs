"""Authored circular-profile fixtures; never used by the byte reconstruction."""
import math

from rectangular_fixtures import world


def definitions():
    items = []
    for name, start, end, tilt, translation in [
            ("ARC-A", 0, 360, 0, [0, 0, 0]),
            ("ARC-B", 30, 120, 0, [0, 0, 0]),
            ("ARC-BT", 30, 120, 37, [1200, -800, 350])]:
        f = dict(id=name, start_degrees=start, end_degrees=end,
                 major_radius_mm=400, minor_radius_mm=100,
                 tilt_x_degrees=tilt, translation_mm=translation)
        # Two bounded semicircles: a closed profile clear of the axis.
        f["profile_arcs_mm"] = [
            dict(start=world(a, f), end=world(b, f), midpoint=world(m, f))
            for a, b, m in [
                ([500, 0, 0], [300, 0, 0], [400, 0, 100]),
                ([300, 0, 0], [500, 0, 0], [400, 0, -100])]]
        f["axis_points_mm"] = [world(p, f) for p in [[0, 0, 0], [0, 0, 1000]]]
        a = math.radians(tilt)
        f["sketch_normal"] = [0, -math.cos(a), -math.sin(a)]
        f["analytic_volume_m3"] = math.radians(end-start)*0.4*math.pi*0.1**2
        items.append(f)
    return dict(schema="curved-revolution-fixtures-v1", units="mm",
                status="authoring_spec_not_native_evidence", fixtures=items)
