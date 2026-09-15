#!/usr/bin/env python3
"""Measured native2027 curve spans; research only, no API inputs in decoding.

Class tags below are observations, not a resolved schema. Wall meshes describe
uncut side surfaces; floor profiles include supported linked Opening sketches.
Enumerates bounded frames and routes current storage using native index streams.
No API snapshots, Mark labels, or live-index comparison reports are inputs.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import struct
from compare_streams import load_dump
from probe_owner_frames_2027 import record_frames
from probe_live_index_2027 import increment_table, element_index, route_increment

MARKER = bytes.fromhex('04000801')
PROFILES = {3899: ('Wall', 'line'), 450: ('Wall', 'arc'), 2153: ('Floor', 'line')}


def decode_span(body, marker_offset, kind):
    if kind not in ('line','arc') or marker_offset<0 or body[marker_offset:marker_offset+4]!=MARKER:
        raise ValueError('unsupported curve kind or marker')
    count = 12 if kind == 'arc' else 8
    end = marker_offset + 4 + count * 8
    if end > len(body):
        raise ValueError('truncated curve span')
    values = struct.unpack_from(f'<{count}d', body, marker_offset + 4)
    if not all(math.isfinite(value) for value in values):
        raise ValueError('nonfinite curve span')
    low, high = values[:2]
    if high <= low:
        raise ValueError('empty/reversed parameter interval')
    if kind == 'line':
        origin, direction = values[2:5], values[5:8]
        if abs(sum(x*x for x in direction) - 1) > 1e-10:
            raise ValueError('nonunit line direction')
        return {'kind': 'line', 'start': [x + low*d for x, d in zip(origin, direction)],
                'end': [x + high*d for x, d in zip(origin, direction)],
                'origin': origin, 'direction': direction, 'parameters': [low, high]}
    x_axis, y_axis, radius, center = values[2:5], values[5:8], values[8], values[9:12]
    if radius <= 0 or any(abs(sum(v*v for v in axis)-1) > 1e-10 for axis in (x_axis,y_axis)) or abs(sum(x*y for x,y in zip(x_axis,y_axis))) > 1e-10:
        raise ValueError('invalid arc basis/radius')
    return {'kind': 'arc', 'center': center, 'basis_x': x_axis, 'basis_y': y_axis,
            'radius': radius, 'start_angle': low, 'end_angle': high}


def wall_surface_mesh(body, marker_offset, kind):
    # Adjacent surface sequence measured independently of API geometry values.
    start = marker_offset + (68 if kind == 'line' else 101)
    stride = 105 if kind == 'line' else 137
    surfaces = []
    for i in range(4):
        offset = start + stride*i
        if offset + stride > len(body) or body[offset+32] != 1:
            raise ValueError('unsupported wall surface sequence')
        values = struct.unpack_from(f'<{(stride-33)//8}d', body, offset+33)
        bounds = struct.unpack_from('<4d',body,offset)
        if not all(math.isfinite(x) for x in (*values,*bounds)):
            raise ValueError('nonfinite wall surface')
        if bounds[2] <= bounds[0] or bounds[3] <= bounds[1]:
            raise ValueError('invalid wall surface domain')
        surfaces.append({'bounds':bounds,'origin':values[:3], 'basis_x':values[3:6],
                         'basis_y':values[6:9], 'extra':values[9:]})
    if surfaces[0] != surfaces[3]:
        raise ValueError('wall reference surfaces disagree')
    domain = surfaces[0]['bounds']
    if any(s['bounds'] != domain for s in surfaces):
        raise ValueError('wall surface domains disagree')
    if any(abs(sum(x*x for x in s[k])-1)>1e-10 for s in surfaces for k in ('basis_x','basis_y')):
        raise ValueError('nonunit surface basis')
    lo,base,hi,top = domain
    count = 1 if kind == 'line' else max(1,math.ceil((hi-lo)/(2*math.pi)*1024))
    vertices=[]
    for i in range(count+1):
        u=lo+(hi-lo)*i/count
        for v in (base,top):
            for surface in surfaces[1:3]:
                o,x,y=surface['origin'],surface['basis_x'],surface['basis_y']
                if kind == 'line':
                    point=[o[j]+u*x[j]+v*y[j] for j in range(3)]
                else:
                    z=surface['extra'][:3]; radius=surface['extra'][3]
                    if radius<=0 or abs(sum(t*t for t in z)-1)>1e-10:
                        raise ValueError('invalid cylindrical surface')
                    point=[o[j]+radius*(math.cos(u)*x[j]+math.sin(u)*y[j])+v*z[j] for j in range(3)]
                vertices.append(point)
    triangles=[]
    def quad(a,b,c,d):
        triangles.extend([[a,b,c],[a,c,d]])
    for i in range(count):
        a=4*i;b=a+4
        quad(a,b,b+1,a+1)
        quad(a+2,a+3,b+3,b+2)
        quad(a,a+2,b+2,b)
        quad(a+1,b+1,b+3,a+3)
    quad(0,1,3,2)
    n=4*count;quad(n,n+2,n+3,n+1)
    return {'vertices':vertices,'triangles':triangles,
            'source':'measured_side_surface_domains_uncut_wall_hypothesis',
            'angular_segment_limit_radians':2*math.pi/1024 if kind=='arc' else None}, surfaces


def floor_extrusion(body, curves):
    # Last planar domain is a repeated top-surface description. Require its
    # complete earlier copy immediately followed by the parallel bottom plane.
    if len(body)<210:
        raise ValueError('short floor geometry')
    top=body[-105:]
    offset=body.find(top,0,len(body)-105)
    if offset<0 or body.find(top,offset+1,len(body)-105)>=0:
        raise ValueError('missing/ambiguous repeated floor plane')
    def plane(data):
        if len(data)!=105 or data[32]!=1:
            raise ValueError('unsupported floor plane')
        values=struct.unpack_from('<9d',data,33)
        bounds=struct.unpack_from('<4d',data)
        if not all(math.isfinite(x) for x in (*values,*bounds)):
            raise ValueError('nonfinite floor plane')
        return bounds,values
    domain,upper=plane(top)
    other,lower=plane(body[offset+105:offset+210])
    if domain!=other or upper[3:]!=lower[3:] or upper[:2]!=lower[:2]:
        raise ValueError('nonparallel floor extrusion')
    if abs(upper[5])>1e-10 or abs(upper[8])>1e-10 or upper[2]<=lower[2]:
        raise ValueError('unsupported nonhorizontal floor')
    remaining=[(list(c['start'][:2]),list(c['end'][:2])) for c in curves if c['kind']=='line']
    if len(remaining)!=len(curves):
        raise ValueError('unsupported curved floor profile')
    loops=[]
    def near(a,b): return math.dist(a,b)<1e-8
    while remaining:
        a,b=remaining.pop(0);loop=[a,b]
        while not near(loop[-1],loop[0]):
            matches=[(i,edge[1] if near(edge[0],loop[-1]) else edge[0]) for i,edge in enumerate(remaining) if near(edge[0],loop[-1]) or near(edge[1],loop[-1])]
            if len(matches)!=1:
                raise ValueError('open or ambiguous floor edge graph')
            i,point=matches[0];remaining.pop(i);loop.append(point)
        loops.append(loop[:-1])
    def area(loop): return sum(a[0]*b[1]-a[1]*b[0] for a,b in zip(loop,loop[1:]+loop[:1]))/2
    loops.sort(key=lambda loop:abs(area(loop)),reverse=True)
    if not loops or any(abs(area(loop))<1e-10 for loop in loops):
        raise ValueError('degenerate floor boundary')
    return {'kind':'vertical_extrusion','top_elevation':upper[2],'bottom_elevation':lower[2],
            'height':upper[2]-lower[2], 'profiles_xy':loops,
            'area':abs(area(loops[0]))-sum(abs(area(loop)) for loop in loops[1:]),
            'plane_byte_offset':offset,'repeated_plane_byte_offset':len(body)-105}


def semantic_reference_frames(data, allowed_tags, index):
    """Research dual-length frames for owner classes without the common carrier.

    Requires current-index owner membership and repeated outer lengths. Calling
    code also requires unique semantic class per owner and active routing.
    """
    for tag in allowed_tags:
        needle=struct.pack('<H',tag)
        cursor=0
        while (body_start:=data.find(needle,cursor))>=0:
            cursor=body_start+1
            if body_start<16:
                continue
            outer=body_start-16
            owner=struct.unpack_from('<Q',data,outer)[0]
            if owner not in index:
                continue
            length=struct.unpack_from('<I',data,outer+12)[0]
            end=body_start+length
            if length<2 or length>16*1024*1024 or end+4>len(data):
                continue
            if struct.unpack_from('<I',data,end)[0]!=length:
                continue
            yield {'element_id':owner,'record_offset':outer,'body_start':body_start,
                   'body_end':end,'body_length':length,'class_tag':tag}


def analyze(root):
    for dump in sorted((root/'analysis').iterdir()):
        if not dump.is_dir() or not (dump/'manifest.json').exists():
            continue
        _, manifest = load_dump(dump)
        if manifest['revit_version'] != 2027:
            raise ValueError('unsupported native version')
        variant = dump.name
        streams = {x['name']:x for x in manifest['streams']}
        def single(name):
            members=streams[name]['members']
            if len(members)!=1 or members[0]['status']!='inflated':
                raise ValueError('unsupported index segmentation')
            return (dump/members[0]['path']).read_bytes()
        index=element_index(single('Global/ElemTable'))
        increments=increment_table(single('Global/DocumentIncrementTable'))
        present={int(name.split('/')[1]) for name in streams if name.startswith('Partitions/')}
        records=[]
        references={}
        unsupported_reference_rows=[]
        for name,stream in streams.items():
            if not name.startswith('Partitions/'):
                continue
            for member in stream['members']:
                if member['status']!='inflated':
                    raise ValueError('failed partition inflate')
                data=(dump/member['path']).read_bytes()
                for reference in semantic_reference_frames(data,(2912,4489),index):
                    row=index[reference['element_id']]
                    if not row['validated_identity']:
                        unsupported_reference_rows.append(reference)
                        continue
                    targets=route_increment(row['increment'],increments,present)
                    if len(targets)!=1:
                        raise ValueError('ambiguous reference route')
                    if int(name.split('/')[1]) not in targets:
                        continue
                    key=(reference['element_id'],reference['class_tag'])
                    if key in references:
                        raise ValueError('ambiguous semantic reference frame')
                    references[key]={'stream':name,'member':member['path'],**reference}
                for record in record_frames(data):
                    row=index.get(record['element_id'])
                    if row is None or record['class_tag'] not in PROFILES:
                        continue
                    if not row['validated_identity']:
                        raise ValueError(f'unsupported owner index fields: {record} {row}')
                    targets=route_increment(row['increment'],increments,present)
                    if len(targets)!=1:
                        raise ValueError('ambiguous physical partition')
                    if int(name.split('/')[1]) in targets:
                        records.append({'stream':name,'member':member['path'],'index_record':row,**record})
        elements, seen = [], set()
        for record in records:
            key = (record['stream'], record['member'], record['record_offset'])
            if key in seen or record['class_tag'] not in PROFILES:
                continue
            seen.add(key)
            body = (root/'analysis'/variant/record['member']).read_bytes()[record['body_start']:record['body_end']]
            klass, kind = PROFILES[record['class_tag']]
            offsets = [i for i in range(len(body)-3) if body[i:i+4] == MARKER]
            # Measured wall location is the first curve; arc's final marker is
            # a separate degenerate axis, not an additional location curve.
            selected = offsets[:1] if klass == 'Wall' else offsets
            errors, curves = [], []
            for offset in selected:
                try:
                    curves.append({'byte_offset': offset, **decode_span(body, offset, kind)})
                except ValueError as exc:
                    errors.append({'byte_offset': offset, 'reason': str(exc)})
            meshes, surfaces = [], []
            if klass == 'Wall' and selected and not errors:
                try:
                    mesh, surfaces = wall_surface_mesh(body, selected[0], kind)
                    meshes.append(mesh)
                except ValueError as exc:
                    errors.append({'reason': str(exc)})
            elements.append({'id': record['element_id'], 'class': klass, 'curves': curves,
                             'geometry_status': 'partial_centerline' if klass == 'Wall' else 'partial_outer_edges_holes_unresolved',
                             'errors': errors, 'meshes': meshes, 'surfaces': surfaces,
                             'source': {**record, 'body_sha256': hashlib.sha256(body).hexdigest()}})
        for (owner,tag), opening in references.items():
            if tag!=2912:
                continue
            body=(dump/opening['member']).read_bytes()[opening['body_start']:opening['body_end']]
            if len(body)!=132 or struct.unpack_from('<Q',body,38)[0]!=owner:
                continue # unsupported opening profile, no inferred hole
            host=struct.unpack_from('<Q',body,105)[0]
            sketch_id=struct.unpack_from('<Q',body,121)[0]
            floor=next((e for e in elements if e['id']==host and e['class']=='Floor'),None)
            sketch=references.get((sketch_id,4489))
            if floor is None or sketch is None:
                continue
            sketch_body=(dump/sketch['member']).read_bytes()[sketch['body_start']:sketch['body_end']]
            curves=[]
            for offset in range(len(sketch_body)-3):
                if sketch_body[offset:offset+4]==MARKER:
                    curves.append({'byte_offset':offset,**decode_span(sketch_body,offset,'line')})
            floor.setdefault('opening_boundaries',[]).append({'opening_id':owner,'sketch_id':sketch_id,'curves':curves,'source':opening,'sketch_source':sketch})
            floor['curves'].extend(curves)
            floor['geometry_status']='profile_with_linked_opening_edges_solid_unresolved'
        for element in elements:
            if element['class']!='Floor':
                continue
            source=element['source']
            body=(dump/source['member']).read_bytes()[source['body_start']:source['body_end']]
            try:
                element['geometry']=floor_extrusion(body,element['curves'])
                element['geometry_status']='measured_vertical_extrusion_profile'
            except ValueError as exc:
                element['errors'].append({'reason':str(exc)})
        report = {'schema_version': 1, 'units': 'feet', 'status': 'research_measured_class_profiles_not_production',
                  'complete_document_geometry':False,
                  'limitations':['Measured native 2027 class profiles, not a resolved schema registry',
                                 'Uncut wall surface model; joined, hosted-cut, slanted, or attached final solids unvalidated',
                                 'Horizontal polygonal floor extrusion; unknown opening or sketch profiles may be incomplete',
                                 'Other native classes and unsupported record framings are not emitted'],
                  'source_sha256': manifest['source_sha256'], 'elements': elements,
                  'unsupported_reference_rows':unsupported_reference_rows,
                  'routing_evidence':'ElemTable +28 stored-record revision; +32 retained separately, equality not required (type-thickness holdout)' }
        output = root/'analysis'/f'{variant}-geometry-research.json'
        output.write_text(json.dumps(report, indent=2))
        print(variant, [(e['id'],len(e['curves']),len(e['errors'])) for e in elements])

if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('run_directory',type=Path)
    analyze(parser.parse_args().run_directory)
