import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('compare_streams', Path(__file__).parents[1] / 'compare_streams.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class StreamComparisonTests(unittest.TestCase):
    def test_absent_inflated_data_is_not_reported_as_equality(self):
        before = {'streams': [{'name': 'raw', 'raw_sha256': 'a', 'raw_length': 1, 'members': []}]}
        after = {'streams': [{'name': 'raw', 'raw_sha256': 'b', 'raw_length': 1, 'members': []}]}
        change = module.compare(before, after)[0]
        self.assertEqual(change['status'], 'changed')
        self.assertIsNone(change['inflated_candidates_equal'])

    def test_unicode_length_is_code_units_not_codepoints(self):
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            needle = 'A😀'
            (directory / 'member').write_bytes((3).to_bytes(4, 'little') + needle.encode('utf-16-le'))
            manifest = {'streams': [{'name': 'Partitions/0', 'members': [
                {'status': 'inflated', 'path': 'member', 'prepared_offset': 16}]}]}
            occurrence = module.locate(directory, manifest, needle)[0]
            self.assertEqual(occurrence['inflated_member_offset'], 4)
            self.assertTrue(occurrence['u32_length_prefix_matches'])


if __name__ == '__main__':
    unittest.main()
