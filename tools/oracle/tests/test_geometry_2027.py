import json
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).parents[1]))
from probe_geometry_2027 import decode_span, wall_surface_mesh


class NativeGeometryTests(unittest.TestCase):
    def test_measured_wall_spans_and_malformed_boundaries(self):
        path=Path(__file__).parents[3]/'tests/fixtures/oracle-2027/wall-geometry-spans.json'
        for case in json.loads(path.read_text()):
            data=bytes.fromhex(case['hex'])
            curve=decode_span(data,0,case['kind'])
            if case['kind']=='line':
                for key in ('start','end'):
                    for got,want in zip(curve[key],case['api_curve'][key]):
                        self.assertAlmostEqual(got,want,places=10)
            mesh,surfaces=wall_surface_mesh(data,0,case['kind'])
            if case['kind']=='line':
                self.assertEqual(len(mesh['vertices']),8)
            self.assertAlmostEqual(surfaces[0]['bounds'][3]-surfaces[0]['bounds'][1],case['api_height'])
            with self.assertRaises(ValueError):
                wall_surface_mesh(data[:-1],0,case['kind'])
            malformed=bytearray(data)
            malformed[(68 if case['kind']=='line' else 101)+32]=0
            with self.assertRaises(ValueError):
                wall_surface_mesh(malformed,0,case['kind'])
