#!/usr/bin/env python3
"""Generate schema-only facts; never include parameter values or owner state."""
import argparse
import hashlib
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('snapshot', type=Path)
parser.add_argument('output', type=Path)
parser.add_argument('--schema-sha256', required=True)
args = parser.parse_args()
raw = args.snapshot.read_bytes()
source = json.loads(raw)
items = {}
for entry in source['parameter_catalog']:
    parameter_id = entry['parameter_id']
    if parameter_id >= 0:
        continue
    fact = {'parameter_id': parameter_id, 'caption': entry['name'],
            'storage_type': entry['storage_type'], 'spec_type_id': entry['spec'],
            'group_type_id': entry['group'], 'is_shared': entry['is_shared']}
    if fact['is_shared']:
        raise ValueError('Built-in parameter unexpectedly shared')
    if parameter_id in items and items[parameter_id] != fact:
        raise ValueError(f'Conflicting definition for {parameter_id}')
    items[parameter_id] = fact
out = {'format': 'rvt-builtin-definition-catalog/v1', 'schema_sha256': args.schema_sha256,
       'revit_version': 2027, 'caption_locale': 'en-US',
       'evidence': 'Revit API definition metadata; no owner state or parameter values',
       'source_snapshot_sha256': hashlib.sha256(raw).hexdigest(),
       'definitions': [items[key] for key in sorted(items)]}
args.output.write_text(json.dumps(out, indent=2, ensure_ascii=False)+'\n')
