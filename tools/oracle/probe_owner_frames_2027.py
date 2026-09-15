#!/usr/bin/env python3
"""Research-only 2027 record framing; does not change production owner gates.

Usage: probe_owner_frames_2027.py RUN_DIRECTORY
Requires analysis/<variant>/ outputs from oracle_stream_dump and snapshots/.
API identities are used only after extraction to compare multisets.
"""
import argparse
from collections import Counter
import json
import hashlib
from pathlib import Path
import struct

CARRIER = bytes.fromhex('10 03 01 00 00 00')
MAX_HEADER_DISTANCE = 256
MAX_BODY_BYTES = 16 * 1024 * 1024
MAX_STRING_UNITS = 4096
PARAMETER_IDS = (-1001203, -1001405)


def record_frames(data):
    """Measured u64 id/u32 uninterpreted/u32 length/body/repeated u32 length."""
    frames = []
    start = 0
    while (carrier := data.find(CARRIER, start)) >= 0:
        start = carrier + 1
        if carrier + 14 > len(data):
            continue
        element_id = struct.unpack_from('<Q', data, carrier + 6)[0]
        if element_id == 0 or element_id >= 2**63:
            continue
        candidates = []
        for outer in range(max(0, carrier - MAX_HEADER_DISTANCE), carrier - 15):
            if struct.unpack_from('<Q', data, outer)[0] != element_id:
                continue
            length = struct.unpack_from('<I', data, outer + 12)[0]
            end = outer + 16 + length
            if not 2 <= length <= MAX_BODY_BYTES or end + 4 > len(data) or end < carrier + 14:
                continue
            if struct.unpack_from('<I', data, end)[0] != length:
                continue
            candidates.append({'element_id': element_id, 'record_offset': outer,
                               'body_start': outer + 16, 'body_end': end, 'body_length': length,
                               'repeated_id_offset': carrier + 6,
                               'class_tag': struct.unpack_from('<H', data, outer + 16)[0]})
        if len(candidates) == 1:
            frames.extend(candidates)
    return frames


def extract(data):
    result = []
    for frame in record_frames(data):
        for parameter_id in PARAMETER_IDS:
            needle = struct.pack('<q', parameter_id)
            start = frame['body_start']
            while (offset := data.find(needle, start, frame['body_end'])) >= 0:
                start = offset + 1
                if offset + 12 > frame['body_end']:
                    continue
                count = struct.unpack_from('<I', data, offset + 8)[0]
                end = offset + 12 + count * 2
                if count > MAX_STRING_UNITS or end > frame['body_end']:
                    continue
                try:
                    value = data[offset + 12:end].decode('utf-16-le')
                except UnicodeDecodeError:
                    continue
                result.append({'parameter_id': parameter_id, 'value': value,
                               'parameter_offset': offset, **frame})
    return result


def analyze(root):
    from compare_streams import load_dump
    out = root / 'analysis'
    for dump in sorted(out.iterdir()):
        if not dump.is_dir() or not (dump / 'manifest.json').exists():
            continue
        _, manifest = load_dump(dump)
        if manifest['revit_version'] != 2027:
            raise ValueError('This research probe only measures the 2027 profile')
        result = []
        failed_candidates = []
        for stream in manifest['streams']:
            if not stream['name'].startswith('Partitions/'):
                continue
            for member in stream['members']:
                if member['status'] != 'inflated':
                    failed_candidates.append({'stream': stream['name'], **member})
                    continue
                for observation in extract((dump / member['path']).read_bytes()):
                    result.append({'stream': stream['name'], 'member': member['path'], **observation})
        snapshot_bytes = (root / 'snapshots' / f'{dump.name}.json').read_bytes()
        snapshot = json.loads(snapshot_bytes)
        expected = Counter((element['id'], parameter['id'], parameter['raw_value'])
                           for element in snapshot['elements'] for parameter in element['parameters']
                           if parameter['id'] in PARAMETER_IDS and parameter['raw_value'] is not None)
        actual = Counter((item['element_id'], item['parameter_id'], item['value']) for item in result)
        report = {'status': 'research_not_production_decoder',
                  'source_sha256': manifest['source_sha256'],
                  'snapshot_sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
                  'failed_candidates': failed_candidates,
                  'matches_api_owner_value_multiset': expected == actual and not failed_candidates, 'observations': result}
        (out / f'{dump.name}-owner-frame-research.json').write_text(json.dumps(report, indent=2, ensure_ascii=False))
        print(dump.name, len(result), expected == actual)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run_directory', type=Path)
    analyze(parser.parse_args().run_directory)
