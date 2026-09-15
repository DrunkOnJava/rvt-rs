import copy
import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('compare_snapshots', Path(__file__).parents[1] / 'compare_snapshots.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SnapshotComparisonTests(unittest.TestCase):
    def setUp(self):
        self.snapshot = {'schema_version': 1, 'source_file': 'base.rvt', 'build': '2027',
                         'elements': [{'id': 5, 'unique_id': 'uid-5', 'parameters': [
                             {'id': -1001203, 'storage_type': 'String', 'has_value': False, 'raw_value': None}]}]}

    def test_unset_and_empty_are_distinct(self):
        after = copy.deepcopy(self.snapshot)
        after['elements'][0]['parameters'][0].update(has_value=True, raw_value='')
        changes = module.compare(self.snapshot, after)['changes']
        self.assertEqual(len(changes), 1)
        self.assertEqual(changes[0]['parameters'][0]['id'], -1001203)

    def test_duplicate_identity_rejected(self):
        after = copy.deepcopy(self.snapshot)
        after['elements'].append(after['elements'][0])
        with self.assertRaisesRegex(ValueError, 'Duplicate unique_id'):
            module.compare(self.snapshot, after)

    def test_parameter_order_does_not_change_semantics(self):
        self.snapshot['elements'][0]['parameters'].append({'id': 2, 'raw_value': 'other'})
        after = copy.deepcopy(self.snapshot)
        after['elements'][0]['parameters'].reverse()
        self.assertEqual(module.compare(self.snapshot, after)['changes'], [])


if __name__ == '__main__':
    unittest.main()
