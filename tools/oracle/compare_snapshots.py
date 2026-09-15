#!/usr/bin/env python3
"""Report Revit API semantic changes by UniqueId and integer parameter id.

Usage: compare_snapshots.py BEFORE.json AFTER.json [-o REPORT.json]
The API snapshot is an oracle; this report alone does not measure RVT decoding.
"""
import argparse
import json
from pathlib import Path


def index_unique(items, key):
    result = {}
    for item in items:
        value = item[key]
        if value in result:
            raise ValueError(f'Duplicate {key}: {value}')
        result[value] = item
    return result


def compare(before, after):
    if before['schema_version'] != after['schema_version']:
        raise ValueError('Snapshot schema versions differ')
    left = index_unique(before['elements'], 'unique_id')
    right = index_unique(after['elements'], 'unique_id')
    changes = []
    for uid in sorted(left.keys() | right.keys()):
        a, b = left.get(uid), right.get(uid)
        if a is None or b is None:
            changes.append({'unique_id': uid, 'kind': 'added' if a is None else 'removed', 'element': b if a is None else a})
            continue
        fields = {}
        for key in sorted((a.keys() | b.keys()) - {'parameters'}):
            if a.get(key) != b.get(key):
                fields[key] = {'before': a.get(key), 'after': b.get(key)}
        ap, bp = index_unique(a['parameters'], 'id'), index_unique(b['parameters'], 'id')
        parameters = []
        for pid in sorted(ap.keys() | bp.keys()):
            p, q = ap.get(pid), bp.get(pid)
            if p != q:
                parameters.append({'id': pid, 'before': p, 'after': q})
        if fields or parameters:
            changes.append({'unique_id': uid, 'element_id': b['id'], 'kind': 'changed', 'fields': fields, 'parameters': parameters})
    return {'schema_version': 1, 'before_source': before['source_file'], 'after_source': after['source_file'],
            'before_build': before['build'], 'after_build': after['build'],
            'element_count_before': len(left), 'element_count_after': len(right), 'changes': changes}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('-o', '--output', type=Path)
    args = parser.parse_args()
    result = compare(json.loads(args.before.read_text()), json.loads(args.after.read_text()))
    text = json.dumps(result, indent=2, ensure_ascii=False) + '\n'
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end='')


if __name__ == '__main__':
    main()
