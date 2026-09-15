#!/usr/bin/env python3
"""Compare oracle_stream_dump directories without inferring parameter ownership."""
import argparse
import hashlib
import json
from pathlib import Path


def load_dump(directory):
    directory = Path(directory)
    manifest = json.loads((directory / 'manifest.json').read_text())
    if manifest['schema_version'] != 1:
        raise ValueError('Unsupported stream manifest schema')
    for stream in manifest['streams']:
        for path, expected in [(stream['raw_path'], stream['raw_sha256'])] + [
            (member['path'], member['sha256']) for member in stream['members']
            if member['status'] == 'inflated'
        ]:
            data = (directory / path).read_bytes()
            if hashlib.sha256(data).hexdigest() != expected:
                raise ValueError(f'Hash mismatch: {directory / path}')
    return directory, manifest


def compare(before, after):
    left = {s['name']: s for s in before['streams']}
    right = {s['name']: s for s in after['streams']}
    changes = []
    for name in sorted(left.keys() | right.keys()):
        a, b = left.get(name), right.get(name)
        if a is None or b is None:
            changes.append({'name': name, 'status': 'added' if a is None else 'removed'})
            continue
        def hashes(stream):
            return [m['sha256'] for m in stream['members'] if m['status'] == 'inflated']
        ah, bh = hashes(a), hashes(b)
        changes.append({
            'name': name,
            'status': 'equal' if a['raw_sha256'] == b['raw_sha256'] else 'changed',
            'raw_length_before': a['raw_length'], 'raw_length_after': b['raw_length'],
            'inflated_candidates_equal': ah == bh if ah or bh else None,
            'inflated_candidate_count_before': len(ah), 'inflated_candidate_count_after': len(bh),
            'failed_candidates_before': sum(m['status'] == 'failed' for m in a['members']),
            'failed_candidates_after': sum(m['status'] == 'failed' for m in b['members']),
        })
    return changes


def locate(directory, manifest, text):
    needle = text.encode('utf-16-le')
    if not needle:
        raise ValueError('An empty needle is not informative')
    occurrences = []
    for stream in manifest['streams']:
        for member in stream['members']:
            if member['status'] != 'inflated':
                continue
            data = (directory / member['path']).read_bytes()
            start = 0
            while (offset := data.find(needle, start)) >= 0:
                prefix = data[max(0, offset - 16):offset]
                occurrences.append({
                    'stream': stream['name'], 'member_path': member['path'],
                    'prepared_member_offset': member['prepared_offset'],
                    'inflated_member_offset': offset, 'prefix_hex': prefix.hex(),
                    'u32_length_prefix_matches': offset >= 4 and int.from_bytes(data[offset-4:offset], 'little') == len(needle) // 2,
                })
                start = offset + 1
    return occurrences


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('--control', type=Path, help='No-edit save derived from the same baseline')
    parser.add_argument('--find-utf16', action='append', default=[])
    parser.add_argument('-o', '--output', type=Path)
    args = parser.parse_args()
    before_dir, before = load_dump(args.before)
    after_dir, after = load_dump(args.after)
    report = {'schema_version': 1, 'before_sha256': before['source_sha256'],
              'after_sha256': after['source_sha256'], 'streams': compare(before, after),
              'interpretation': 'Byte observations only. Candidate sequences do not establish ownership, current revision, or valid chunk boundaries.'}
    if args.control:
        _, control = load_dump(args.control)
        report['control_sha256'] = control['source_sha256']
        report['control_streams'] = compare(before, control)
        control_changed = {s['name'] for s in report['control_streams'] if s['status'] != 'equal'}
        for stream in report['streams']:
            stream['also_changed_in_no_edit_control'] = stream['name'] in control_changed
    report['utf16_occurrences'] = {text: {
        'before': locate(before_dir, before, text), 'after': locate(after_dir, after, text)
    } for text in args.find_utf16}
    output = json.dumps(report, indent=2, ensure_ascii=False) + '\n'
    if args.output:
        args.output.write_text(output)
    else:
        print(output, end='')


if __name__ == '__main__':
    main()
