#!/usr/bin/env python3
"""Compare unowned parser occurrences to API truth as multisets, preserving duplicates.

Usage: compare_parameters.py SNAPSHOT.json OCCURRENCES.json [-o REPORT.json]
Agreement proves occurrence/value counts only, never element ownership.
"""
import argparse
from collections import Counter
import json
from pathlib import Path

PARAMETER_IDS = {-1001203, -1001405}


def compare(snapshot, decoded):
    expected = Counter((parameter['id'], parameter['raw_value'])
                       for element in snapshot['elements']
                       for parameter in element['parameters']
                       if parameter['id'] in PARAMETER_IDS and parameter['raw_value'] is not None)
    actual = Counter((item['occurrence']['parameter_id'], item['occurrence']['value'])
                     for item in decoded['observations'])
    def entries(counts):
        return [{'parameter_id': pid, 'value': value, 'count': count}
                for (pid, value), count in sorted(counts.items())]
    version_matches = str(snapshot['version']) == str(decoded['revit_version'])
    return {'schema_version': 2, 'decoder_boundary_source': decoded.get('boundary_source'), 'comparison_scope': 'unowned_parameter_value_multiset',
            'ownership_validated': False, 'version_matches': version_matches,
            'matches': version_matches and expected == actual and not decoded['failed_candidates'] and decoded.get('scan_complete', False),
            'scan_complete': decoded.get('scan_complete', False), 'scan_issues': decoded.get('scan_issues', []),
            'max_string_units': decoded.get('max_string_units'),
            'expected_count': sum(expected.values()), 'decoded_count': sum(actual.values()),
            'missing': entries(expected - actual), 'unexpected': entries(actual - expected),
            'failed_candidates': decoded['failed_candidates']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('snapshot', type=Path)
    parser.add_argument('occurrences', type=Path)
    parser.add_argument('-o', '--output', type=Path)
    args = parser.parse_args()
    result = compare(json.loads(args.snapshot.read_text()), json.loads(args.occurrences.read_text()))
    text = json.dumps(result, indent=2, ensure_ascii=False) + '\n'
    if args.output:
        args.output.write_text(text)
    else:
        print(text, end='')
    raise SystemExit(0 if result['matches'] else 1)


if __name__ == '__main__':
    main()
