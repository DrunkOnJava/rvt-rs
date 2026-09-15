import sys
import unittest
from pathlib import Path
sys.path.insert(0, str(Path(__file__).parents[1]))
from probe_live_index_2027 import route_increment, increment_table, element_index


class LiveIndexTests(unittest.TestCase):
    def test_compaction_redirects_old_logical_revisions(self):
        rows = [{'stored_record_count': 0, 'routes': [(-1, 2415), (3, 0)]},
                {'stored_record_count': 0, 'routes': [(-1, 1), (2, 0)]},
                {'stored_record_count': 0, 'routes': [(-1, 1), (3, 0)]},
                {'stored_record_count': 2415, 'routes': [(-1, 2415)]}]
        self.assertEqual(route_increment(0, rows, {3}), {3})
        self.assertEqual(route_increment(1, rows, {3}), {3})
        self.assertEqual(route_increment(2, rows, {3}), {3})

    def test_missing_active_partition_is_not_highest_partition_fallback(self):
        with self.assertRaises(ValueError):
            route_increment(0, [{'stored_record_count': 1, 'routes': []}], {3})

    def test_backward_or_out_of_range_route_rejected(self):
        for destination in (0, 9):
            with self.assertRaises(ValueError):
                route_increment(0, [{'stored_record_count': 0, 'routes': [(destination, 0)]}], set())

    def test_ambiguous_route_remains_ambiguous(self):
        rows = [{'stored_record_count': 0, 'routes': [(1, 0), (2, 0)]},
                {'stored_record_count': 1, 'routes': []},
                {'stored_record_count': 1, 'routes': []}]
        self.assertEqual(route_increment(0, rows, {1, 2}), {1, 2})

    def test_truncated_index_tables_rejected(self):
        for parser in (increment_table, element_index):
            with self.assertRaises(ValueError):
                parser(b'\x56\x05\x01')

class NativeRevisionCounterexampleTests(unittest.TestCase):
    def test_measured_storage_revision_is_not_always_equal_to_other_field(self):
        import json
        import struct
        path=Path(__file__).parents[3]/'tests/fixtures/oracle-2027/storage-revision-rows.json'
        for case in json.loads(path.read_text()):
            # Single-record wrapper is synthetic; the 40-byte row is unchanged
            # from the native file, with source coordinates recorded separately.
            header=bytearray(30);header[:2]=bytes.fromhex('e605');struct.pack_into('<I',header,2,1)
            row=element_index(bytes(header)+bytes.fromhex(case['hex']))[case['owner']]
            self.assertTrue(row['validated_identity'])
            self.assertEqual(row['increment'],case['stored_record_increment'])
            self.assertEqual(row['uninterpreted_revision_field'],case['other_field'])
            if case['variant'].endswith('thickness'):
                self.assertFalse(row['validated_route_fields'])
                self.assertEqual((row['increment'],row['uninterpreted_revision_field']),(1,0))
