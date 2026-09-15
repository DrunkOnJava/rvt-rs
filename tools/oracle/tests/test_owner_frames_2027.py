import importlib.util
import struct
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location('owner_frames', Path(__file__).parents[1] / 'probe_owner_frames_2027.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class OwnerFrameTests(unittest.TestCase):
    def fixture(self):
        element_id = 2**32 + 123
        body = b'\x01\x00' + b'\x00' * 14 + module.CARRIER + struct.pack('<Q', element_id)
        body += struct.pack('<qI', -1001203, 1) + b'A\x00'
        return struct.pack('<QII', element_id, 0, len(body)) + body + struct.pack('<I', len(body))

    def test_preserves_64bit_id_and_requires_both_lengths(self):
        data = self.fixture()
        result = module.extract(data)
        self.assertEqual(result[0]['element_id'], 2**32 + 123)
        self.assertEqual(result[0]['value'], 'A')
        self.assertEqual(module.extract(data[:-4] + b'\x00' * 4), [])
        for end in range(len(data)):
            self.assertEqual(module.extract(data[:end]), [])

    def test_mismatched_repeated_id_is_rejected(self):
        data = bytearray(self.fixture())
        data[38] ^= 1
        self.assertEqual(module.extract(data), [])


if __name__ == '__main__':
    unittest.main()
