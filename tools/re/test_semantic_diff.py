"""Failure modes of tools/re/semantic_diff.py, a pure function of two JSON documents (#1334).

Three ways a normalizer can fail, each written before the code that has to survive it:

- it hides a change that looks cosmetic and is not: 0.35 and 0.3500001 feet are different values;
- it reports a change where nothing moved: a reorder, or offsets that shifted for unrelated reasons;
- it collapses a bulk change and hides a different value inside the group.

Run: python3 tools/re/test_semantic_diff.py
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import semantic_diff as sd  # noqa: E402


def prop(name, value, kind="IFCLENGTHMEASURE"):
    return {"ifc_type": kind, "name": name, "value": value}


def pset(name, *properties):
    return {"kind": "properties", "name": name, "properties": list(properties)}


def wall(number, offset=100, **extra):
    """A row and its detail: a wall with the position fields a decoder moves for unrelated reasons."""
    ident = str(number)
    provenance = {
        "category": "IFCWALL",
        "class_name": "SWall",
        "body_source": "partition_element_record_bbox",
        "offset": offset + number,
        "record_offset": offset + number,
        "stream": "Partitions/46",
        "native_record": {"copy": "only", "offset": offset * 10 + number, "stream": "Partitions/46"},
    }
    row = {"id": ident, "level": "Level 1", "name": f"Wall:{ident}", "predefined_type": "NOTDEFINED", "provenance": provenance}
    detail = {
        "id": ident,
        "name": row["name"],
        "level": {"name": "Level 1", "index": 0, "elevation_feet": 0.0},
        "location_feet": [1.0, 2.0, 3.0],
        "predefined_type": "NOTDEFINED",
        "provenance": dict(provenance),
        "property_sets": [pset("Pset_WallCommon", prop("Width", 0.35), prop("FireRating", "1HR", "IFCLABEL"))],
        "trust_markers": {"BodySource": "partition_element_record_bbox"},
        "file": {"revit_release": 2024},
    }
    detail.update(extra)
    return row, detail


def dump(walls, **parts):
    out = {
        "rows": [row for row, _ in walls],
        "details": {row["id"]: detail for row, detail in walls},
        "rooms": [],
        "levels": [{"name": "Level 1", "index": 0, "elevation_feet": 0.0, "element_count": len(walls)}],
        "summary": {"file": {"revit_release": 2024}, "elements": {"total": len(walls), "folded_into_bodies": {}}},
        "diagnostics": {"notes": ["a", "b"], "unread_streams": []},
    }
    out.update(parts)
    return out


class SemanticDiff(unittest.TestCase):
    def test_a_change_that_looks_cosmetic_is_still_a_change(self):
        base = dump([wall(1)])
        row, detail = wall(1)
        detail["property_sets"] = [pset("Pset_WallCommon", prop("Width", 0.3500001), prop("FireRating", "1HR", "IFCLABEL"))]
        head = dump([(row, detail)])
        diff = sd.diff_dumps(base, head)
        self.assertEqual(len(diff["findings"]), 1)
        finding = diff["findings"][0]
        self.assertEqual(finding["kind"], "changed")
        self.assertEqual(finding["before"]["value"], 0.35)
        self.assertEqual(finding["after"]["value"], 0.3500001)
        self.assertEqual(finding["count"], 1)
        self.assertEqual(diff["elements"]["changed"], 1)
        # An integer and the equal float are one value, and a boolean is not the integer one.
        self.assertTrue(sd.same(1, 1.0))
        self.assertFalse(sd.same(True, 1))

    def test_a_reorder_and_moved_offsets_change_nothing(self):
        walls = [wall(1), wall(2), wall(3)]
        base = dump(walls, rooms=[{"id": "9", "number": "1", "area_square_feet": 10.5, "level_elevation_feet": 0.0, "name": "A"},
                                  {"id": "8", "number": "2", "area_square_feet": 20.5, "level_elevation_feet": 0.0, "name": "B"}])
        moved = []
        for number in (3, 2, 1):
            row, detail = wall(number, offset=5000)
            detail["property_sets"] = [pset("Pset_WallCommon", prop("FireRating", "1HR", "IFCLABEL"), prop("Width", 0.35))]
            moved.append((row, detail))
        head = dump(
            moved,
            rooms=[{"id": "8", "number": "2", "area_square_feet": 20.5, "level_elevation_feet": 0.0, "name": "B"},
                   {"id": "9", "number": "1", "area_square_feet": 10.5, "level_elevation_feet": 0.0, "name": "A"}],
            diagnostics={"notes": ["b", "a"], "unread_streams": []},
        )
        diff = sd.diff_dumps(base, head)
        self.assertEqual(diff["findings"], [])
        self.assertEqual(diff["coverage"]["position_only"], 3)
        self.assertIn("no semantic difference", diff["summary"])
        # A move of the offsets together with a real change lists the change and not the offsets.
        row, detail = wall(1, offset=9000)
        row["name"] = "Wall:renamed"
        again = sd.diff_dumps(base, dump([(row, detail), moved[1], moved[0]]))
        self.assertEqual([(f["path"], f["after"]) for f in again["findings"]], [("name", "Wall:renamed")])

    def test_a_bulk_collapse_keeps_the_different_value(self):
        base_walls = [wall(number) for number in range(1, 402)]
        head_walls = []
        for number in range(1, 402):
            row, detail = wall(number)
            value = 0.35 if number == 200 else 0.3
            detail["property_sets"].append(pset("Pset_Extra", prop("Thickness", value)))
            head_walls.append((row, detail))
        diff = sd.diff_dumps(dump(base_walls), dump(head_walls))
        findings = [f for f in diff["findings"] if f["kind"] == "added" and f["path"].endswith("[Thickness]")]
        self.assertEqual(sorted((f["count"], f["after"]["value"]) for f in findings), [(1, 0.35), (400, 0.3)])
        odd = next(f for f in findings if f["count"] == 1)
        self.assertEqual(odd["ids"], ["200"])
        bulk = next(f for f in findings if f["count"] == 400)
        self.assertNotIn("200", bulk["ids"])
        self.assertEqual(sum(f["count"] for f in findings), 401)
        self.assertIn("400 IFCWALL elements", diff["summary"])
        self.assertIn("1 IFCWALL element:", diff["summary"])


if __name__ == "__main__":
    unittest.main()
