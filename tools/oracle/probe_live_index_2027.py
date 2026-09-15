#!/usr/bin/env python3
"""Research-only active-partition routing from native 2027 index observations.

Does not select the last/highest partition. Requires redundant ElemTable ids,
complete duplicated DocumentIncrementTable framing, and a unique route to a
present partition with nonzero stored record count. API truth is loaded only
for the final comparison. No production capability is promoted by this probe.
"""
import argparse
from collections import Counter
import json
import hashlib
from pathlib import Path
import struct

from compare_streams import load_dump
import probe_owner_frames_2027 as frames


def u32(data, offset):
    if offset < 0 or offset + 4 > len(data):
        raise ValueError('truncated u32')
    return struct.unpack_from('<I', data, offset)[0]


def increment_table(data):
    if data[:2] != bytes.fromhex('56 05'):
        raise ValueError('unsupported DocumentIncrementTable tag')
    cursor = 2
    tables = []
    for _ in range(2):
        count = u32(data, cursor)
        cursor += 4
        if count > len(data) // 85:
            raise ValueError('increment count exceeds bounded input')
        rows = []
        for index in range(count):
            start = cursor
            route_count = u32(data, cursor)
            cursor += 4
            if route_count > (len(data) - cursor) // 8:
                raise ValueError('route count exceeds bounded input')
            routes = [struct.unpack_from('<iI', data, cursor + i * 8) for i in range(route_count)]
            cursor += route_count * 8
            string_units = u32(data, cursor)
            cursor += 4
            end = cursor + string_units * 2
            if end + 77 > len(data):
                raise ValueError('truncated increment string/tail')
            data[cursor:end].decode('utf-16-le')  # validate encoding; do not expose user names
            cursor = end
            repeated = [u32(data, cursor + off) for off in (32, 36, 40, 44, 48, 52, 60, 64, 72)]
            if any(value != index for value in repeated):
                raise ValueError('unvalidated increment identity layout')
            if data[cursor + 76] not in (0, 1):
                raise ValueError('unvalidated increment flag')
            rows.append({'index': index, 'routes': routes,
                         'stored_record_count': u32(data, cursor + 24),
                         'compaction_flag_candidate': u32(data, cursor + 56),
                         'offset': start, 'end': cursor + 77,
                         '_bytes': data[start:cursor + 77]})
            cursor += 77
        tables.append(rows)
    if len(tables[0]) != len(tables[1]) or any(a['_bytes'] != b['_bytes'] for a, b in zip(*tables)):
        raise ValueError('duplicated increment tables disagree')
    if data[cursor:] != b'\x00' * 8:
        raise ValueError('unvalidated trailing increment-table data')
    return [{k: v for k, v in row.items() if k != '_bytes'} for row in tables[0]]


def element_index(data):
    if data[:2] != bytes.fromhex('e6 05'):
        raise ValueError('unsupported ElemTable tag')
    count = u32(data, 2)
    if len(data) - count * 40 != 30:
        raise ValueError('unsupported exact 40-byte ElemTable layout')
    rows = {}
    for index in range(count):
        offset = 30 + index * 40
        record = data[offset:offset + 40]
        element_id = struct.unpack_from('<Q', record, 16)[0]
        increment = u32(record, 28)
        # Reference fields at +4/+8 vary; their semantics remain opaque.
        # Preserve every index row, validating redundancy only when a queried
        # record owner needs a physical route. Unknown internal rows cannot
        # establish ownership or a route.
        validated = u32(record, 36) == element_id and u32(record, 32) == increment
        if element_id in rows:
            raise ValueError('ambiguous duplicate ElemTable identity')
        rows[element_id] = {'element_id': element_id, 'increment': increment, 'offset': offset, 'validated_route_fields': validated,
                            'validated_identity': u32(record,36) == element_id,
                            'uninterpreted_revision_field': u32(record,32)}
    return rows


def route_increment(index, rows, present, visiting=()):
    if index < 0 or index in visiting or index >= len(rows):
        raise ValueError('cyclic/out-of-range increment route')
    row = rows[index]
    if row['stored_record_count'] > 0:
        if index not in present:
            raise ValueError('active increment has no physical partition')
        return {index}
    destinations = set()
    for target, count in row['routes']:
        if target < 0:
            continue
        if count != 0 or target <= index:
            raise ValueError('unvalidated non-forward redirect')
        destinations.update(route_increment(target, rows, present, visiting + (index,)))
    return destinations


def analyze(root, max_string_units):
    if not 1 <= max_string_units <= 8 * 1024 * 1024:
        raise ValueError('invalid explicit string budget')
    frames.MAX_STRING_UNITS = max_string_units
    for dump in sorted((root / 'analysis').iterdir()):
        if not dump.is_dir() or not (dump / 'manifest.json').exists():
            continue
        _, manifest = load_dump(dump)
        if manifest['revit_version'] != 2027:
            raise ValueError('unsupported native version')
        streams = {s['name']: s for s in manifest['streams']}
        def single(name):
            members = streams[name]['members']
            if len(members) != 1 or members[0]['status'] != 'inflated':
                raise ValueError('index stream has unsupported member segmentation')
            return (dump / members[0]['path']).read_bytes()
        increments = increment_table(single('Global/DocumentIncrementTable'))
        index = element_index(single('Global/ElemTable'))
        present = {int(name.split('/')[1]) for name in streams if name.startswith('Partitions/')}
        selected, rejected = [], []
        for name, stream in streams.items():
            if not name.startswith('Partitions/'):
                continue
            partition = int(name.split('/')[1])
            for member in stream['members']:
                if member['status'] != 'inflated':
                    raise ValueError('failed partition inflate prevents complete research comparison')
                for observation in frames.extract((dump / member['path']).read_bytes()):
                    item = {'stream': name, 'member': member['path'], **observation}
                    row = index.get(observation['element_id'])
                    if row is None:
                        rejected.append({'reason': 'absent_from_current_element_index', **item})
                        continue
                    if not row['validated_route_fields']:
                        raise ValueError('queried owner has unsupported index identity/revision fields')
                    targets = route_increment(row['increment'], increments, present)
                    if len(targets) != 1:
                        raise ValueError('current element has ambiguous/unresolved physical partition')
                    target = next(iter(targets))
                    if partition != target:
                        rejected.append({'reason': 'historical_partition', 'selected_partition': target, **item})
                    else:
                        selected.append({'logical_increment': row['increment'], **item})
        snapshot_bytes = (root / 'snapshots' / f'{dump.name}.json').read_bytes()
        snapshot = json.loads(snapshot_bytes)
        expected = Counter((e['id'], p['id'], p['raw_value']) for e in snapshot['elements']
                           for p in e['parameters'] if p['id'] in frames.PARAMETER_IDS and p['raw_value'] is not None)
        actual = Counter((x['element_id'], x['parameter_id'], x['value']) for x in selected)
        report = {'schema_version': 1, 'status': 'research_not_production_live_selection',
                  'source_sha256': manifest['source_sha256'],
                  'snapshot_sha256': hashlib.sha256(snapshot_bytes).hexdigest(),
                  'max_string_units': max_string_units,
                  'matches_api_live_owner_value_multiset': expected == actual,
                  'expected_count': sum(expected.values()), 'selected_count': len(selected),
                  'increment_rows': increments,
                  'opaque_index_rows': [r for r in index.values() if not r['validated_route_fields']], 'observations': selected, 'rejected': rejected}
        (root / 'analysis' / f'{dump.name}-live-index-research.json').write_text(json.dumps(report, indent=2, ensure_ascii=False))
        print(dump.name, len(selected), len(rejected), expected == actual)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run_directory', type=Path)
    parser.add_argument('--max-string-units', type=int, default=8192)
    args = parser.parse_args()
    analyze(args.run_directory, args.max_string_units)
