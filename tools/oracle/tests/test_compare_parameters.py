import importlib.util
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('compare_parameters', Path(__file__).parents[1] / 'compare_parameters.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ParameterComparisonTests(unittest.TestCase):
    def test_duplicate_values_are_not_set_deduplicated(self):
        snapshot = {'version': '2027', 'elements': [{'parameters': [
            {'id': -1001203, 'raw_value': 'same'}]}, {'parameters': [
            {'id': -1001203, 'raw_value': 'same'}]}]}
        decoded = {'revit_version': 2027, 'scan_complete': True, 'failed_candidates': [], 'observations': [
            {'occurrence': {'parameter_id': -1001203, 'value': 'same'}}]}
        result = module.compare(snapshot, decoded)
        self.assertFalse(result['matches'])
        self.assertEqual(result['missing'][0]['count'], 1)
        decoded['observations'].append(decoded['observations'][0])
        self.assertTrue(module.compare(snapshot, decoded)['matches'])


if __name__ == '__main__':
    unittest.main()
