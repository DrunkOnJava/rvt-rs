#!/usr/bin/env python3
"""RE-443: replay public RVT bytes and 107 bodies against native IFC, without Revit.

Build rvt-ifc and probe_collapsed_ruled with --profile ci. Install the repository's
tools/ci/requirements-ifc.txt. Then:
  python3 tools/re/replay_collapsed_ruled.py --work target/collapsed-ruled \
    --reference-zip /path/to/collapsed-ruled-native-v1.zip
The native reference is not shared: redistribution terms are not established.
Its hashes and the local measurements are retained; no reference is downloaded.
--fine adds the slower, supplementary fine-reference measurement; the default
measurement remains the verdict for saved meshes, as in Measure.
"""
import argparse
from concurrent.futures import ProcessPoolExecutor
import hashlib
import json
import os
from pathlib import Path
import subprocess
import urllib.request
import zipfile

import ifcopenshell
import ifcopenshell.util.placement
import ifcopenshell.util.unit
import numpy as np

import fine_mesh
from ifc_common import MESH_FAILURES, site_frame
import mesh_bodies_vs_ifc as score

ROOT = Path(__file__).resolve().parents[2]
PIN = json.loads((ROOT / 'research/collapsed-ruled.json').read_text())


def verified(path, pin):
    if path.stat().st_size != pin['bytes']:
        raise ValueError(f'Wrong byte count: {path.name}')
    if hashlib.sha256(path.read_bytes()).hexdigest() != pin['sha256']:
        raise ValueError(f'Wrong SHA-256: {path.name}')
    return path


def fetch(path, pin):
    if path.exists():
        return verified(path, pin)
    with urllib.request.urlopen(pin['url'], timeout=60) as response:
        data = response.read(pin['bytes'] + 1)
    if len(data) != pin['bytes'] or hashlib.sha256(data).hexdigest() != pin['sha256']:
        raise ValueError(f'Download differs from pin: {path.name}')
    path.write_bytes(data)
    return path


def selected(model, tags):
    out = {}
    for product in model.by_type('IfcElement'):
        if product.Tag not in tags:
            continue
        if product.Tag in out:
            raise ValueError(f'Duplicate element tag: {product.Tag}')
        out[product.Tag] = product
    if set(out) != tags:
        raise ValueError(f'Missing elements: {sorted(tags-set(out))}')
    return out


def candidate_triangles(model, products):
    """Read exact emitted triangle coordinates, retaining coincident parts."""
    frame = site_frame(model)
    scale = ifcopenshell.util.unit.calculate_unit_scale(model)
    out = {}
    for tag, product in products.items():
        reps = [r for r in product.Representation.Representations if r.RepresentationIdentifier == 'Body']
        if len(reps) != 1 or len(reps[0].Items) != 1:
            raise ValueError(f'Unexpected body representation: {tag}')
        item = reps[0].Items[0]
        if not item.is_a('IfcTriangulatedFaceSet') or item.PnIndex is not None:
            raise ValueError(f'Expected recovered triangle body: {tag}')
        points = np.asarray(item.Coordinates.CoordList)
        mat = ifcopenshell.util.placement.get_local_placement(product.ObjectPlacement)
        world = points @ mat[:3, :3].T + mat[:3, 3]
        out[tag] = frame(world * scale)[np.asarray(item.CoordIndex) - 1]
    return out


def check_probe(path):
    probe = json.loads(path.read_text())
    if probe['file_sha256'] != PIN['source']['sha256'] or probe['surfaces'] != PIN['expected']['surfaces']:
        raise ValueError('Wrong probe file or surface count')
    expected = {s['owner']: s['body_sha256'] for s in PIN['symbols']}
    if {row['owner'] for row in probe['rows']} != set(expected):
        raise ValueError('Missing symbol witnesses')
    controls = 0
    for row in probe['rows']:
        if row['body_sha256'] != expected[row['owner']] or row['pointer_hex'] != '00000000' or row['profile_class'] != 'GArc':
            raise ValueError('Byte witness differs')
        if len(row['controls']) != 7 or not all(c['refusal'] for c in row['controls']):
            raise ValueError('Missing refusal control')
        if not row['faces'] or not all(f['rectangular'] and f['nonrectangular_refusal'] == 'unqualified collapsed ruled trim' for f in row['faces']):
            raise ValueError('Missing rectangular trim evidence')
        controls += len(row['controls']) + len(row['faces'])
    return probe, controls


def pole_witness(probe, native):
    witness = next(r for r in probe['rows'] if r['owner'] == 417088 and r['surface'] == 138)
    point = np.array(witness['point_feet'])
    scale = ifcopenshell.util.unit.calculate_unit_scale(native) / 0.3048
    product = selected(native, {'419137'})['419137']
    poles = []
    for surface in native.traverse(product.Representation):
        if not surface.is_a('IfcBSplineSurface'):
            continue
        net = np.array([[p.Coordinates for p in row] for row in surface.ControlPointsList])
        for edge in (net[0], net[-1], net[:, 0], net[:, -1]):
            if np.linalg.norm(edge - edge[0], axis=1).max() < 1e-10:
                poles.append(edge[0] * scale)
    distance = float(np.linalg.norm(np.array(poles) - point, axis=1).min())
    if distance > 1e-9:
        raise ValueError('Representative saved point differs from native spline pole')
    return dict(owner=417088, surface=138, native_tag='419137', distance_feet=distance,
                scope='Representative local-coordinate pole in the corresponding native element; full bodies are measured separately in their placed site frames.')


def measure_body(payload):
    tag, a, b, native_class, fine = payload
    pa = np.unique(np.concatenate([a.reshape(-1, 3), a.mean(axis=1)]), axis=0)
    pb = np.unique(np.concatenate([b.reshape(-1, 3), b.mean(axis=1)]), axis=0)
    va, vb = float(score.volume(a)), float(score.volume(b))
    if not np.isfinite([va, vb]).all() or min(va, vb) <= 0:
        raise ValueError(f'Invalid body volume: {tag}')
    da, db = float(score.surface_distance(pa, b)), float(score.surface_distance(pb, a))
    ratio = va / vb
    expected = PIN['expected']
    distance_limit = expected['default_max_distance_ft']
    volume_limit = expected['default_max_relative_volume_error']
    if fine:
        distance_limit = expected['fine_fitting_max_distance_ft'] if native_class == 'IfcDuctFitting' else expected['fine_terminal_max_distance_ft']
        volume_limit = expected['fine_max_relative_volume_error']
    if not np.isfinite([da, db, ratio]).all() or max(da, db) > distance_limit or abs(ratio-1) > volume_limit:
        raise ValueError(f'Outside measured bound: {tag}: distances={da}, {db}; volume ratio={ratio}')
    return dict(tag=tag, native_class=native_class, candidate_to_native_ft=da,
                native_to_candidate_ft=db, volume_ratio=ratio,
                candidate_samples=len(pa), native_samples=len(pb))


def measure(candidate, native_path, native_model, fine, workers):
    label = 'fine' if fine else 'default'
    print(f'Measuring {len(candidate)} bodies against {label} native tessellation...', flush=True)
    MESH_FAILURES.clear()
    reference = score.meshes(str(native_path), tags=set(candidate), model=native_model,
                             settings=fine_mesh.settings() if fine else None)
    if set(reference) != set(candidate) or MESH_FAILURES:
        raise ValueError(f'Incomplete native meshing: {MESH_FAILURES}')
    payloads = ((tag, candidate[tag], reference[tag][2], reference[tag][0], fine)
                for tag in sorted(candidate, key=int))
    rows = []
    with ProcessPoolExecutor(max_workers=workers) as pool:
        for row in pool.map(measure_body, payloads):
            rows.append(row)
            if len(rows) % 25 == 0 or len(rows) == len(candidate):
                print(f'  {label}: {len(rows)}/{len(candidate)} bodies checked', flush=True)
    return dict(reference_meshing='fine' if fine else 'default', elements=len(rows),
                max_distance_mm=max(max(r['candidate_to_native_ft'], r['native_to_candidate_ft']) for r in rows)*304.8,
                max_volume_difference_percent=max(abs(r['volume_ratio']-1) for r in rows)*100, rows=rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work', type=Path, required=True)
    parser.add_argument('--sample', type=Path)
    parser.add_argument('--reference-zip', type=Path,
                        help='Required local reference archive; not shared (redistribution terms not established).')
    suffix = '.exe' if os.name == 'nt' else ''
    parser.add_argument('--rvt-ifc', type=Path, default=ROOT/'target/ci'/f'rvt-ifc{suffix}')
    parser.add_argument('--probe', type=Path, default=ROOT/'target/ci/examples'/f'probe_collapsed_ruled{suffix}')
    parser.add_argument('--fine', action='store_true')
    parser.add_argument('--workers', type=int, choices=range(1, 9), default=min(4, os.cpu_count() or 1),
                        help='Scoring processes, 1 to 8 (default: up to 4). Each body uses the same arithmetic.')
    args = parser.parse_args()
    args.work.mkdir(parents=True, exist_ok=True)
    result_path = args.work/'verification.json'
    # A failed rerun must not leave an earlier success looking current.
    result_path.unlink(missing_ok=True)
    if args.reference_zip is None:
        parser.error('The native reference is not shared: redistribution terms are not established. Supply a local --reference-zip; its hash must match research/collapsed-ruled.json.')
    sample = verified(args.sample, PIN['source']) if args.sample else fetch(args.work/'sample.rvt', PIN['source'])
    archive = verified(args.reference_zip, PIN['archive'])
    with zipfile.ZipFile(archive) as packed:
        member = packed.getinfo(PIN['reference']['file'])
        if member.file_size != PIN['reference']['bytes']:
            raise ValueError('Wrong reference member size')
        data = packed.read(member)
    reference_path = args.work/PIN['reference']['file']
    if hashlib.sha256(data).hexdigest() != PIN['reference']['sha256']:
        raise ValueError('Wrong reference member SHA-256')
    if reference_path.exists():
        # Preserve its timestamp: the shared native-mesh cache keys on it.
        verified(reference_path, PIN['reference'])
    else:
        reference_path.write_bytes(data)
    with (args.work/'probe.json').open('w') as out:
        subprocess.run([str(args.probe.resolve()), str(sample.resolve())], stdout=out, check=True)
    probe, controls = check_probe(args.work/'probe.json')
    candidate_path = args.work/'candidate.ifc'
    subprocess.run([str(args.rvt_ifc.resolve()), str(sample.resolve()), '-o', str(candidate_path.resolve()), '--large-model'], check=True, env=dict(os.environ, SOURCE_DATE_EPOCH='0'))
    candidate_model = ifcopenshell.open(str(candidate_path))
    native_model = ifcopenshell.open(str(reference_path))
    tags = set(PIN['targets'])
    selected(native_model, tags)
    candidate = candidate_triangles(candidate_model, selected(candidate_model, tags))
    os.environ.setdefault('RVT_MESH_WORKERS', '1' if os.name == 'nt' else '4')
    os.environ.setdefault('RVT_MESH_CACHE', str(args.work/'mesh-cache'))
    result = dict(source_sha256=PIN['source']['sha256'], reference_sha256=PIN['reference']['sha256'],
                  byte_surfaces=probe['surfaces'], refusal_controls=controls,
                  representative_pole=pole_witness(probe, native_model),
                  default=measure(candidate, reference_path, native_model, False, args.workers))
    if args.fine:
        result['fine'] = measure(candidate, reference_path, native_model, True, args.workers)
    result_path.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({k:({a:b for a,b in v.items() if a!='rows'} if isinstance(v,dict) else v) for k,v in result.items()}, indent=2))


if __name__ == '__main__':
    main()
