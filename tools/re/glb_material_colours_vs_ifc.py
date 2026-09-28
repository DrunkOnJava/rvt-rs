#!/usr/bin/env python3
"""Compare the colours family instances are drawn in, in an rvt-gltf GLB,
with Revit's own IFC4 styles for the same elements.

Research tool for #355 (RE-82 materials in the GLB). A GLB node's name ends
in the element's ElementId, which is the IFC `Tag`. For each drawn family
instance whose GLB material is a model material (not a category or layer
colour), it reads Revit's material for that element and the surface style
colour Revit writes for it (on the material's representation, or on the
element's own geometry items), and counts where the material name and the
sRGB colour (within 0.01) agree.

Usage:

    python3 tools/re/glb_material_colours_vs_ifc.py <model.glb> <revit-export.ifc>

Needs IfcOpenShell (tested with 0.8.5).
"""

import collections
import json
import struct
import sys

import ifcopenshell
import ifcopenshell.util.element as ue

if len(sys.argv) != 3:
    print(__doc__, file=sys.stderr)
    sys.exit(2)
glb,ifc=sys.argv[1:3]
b=open(glb,'rb').read(); n=struct.unpack('<I',b[12:16])[0]; doc=json.loads(b[20:20+n])
def lin2srgb(c): return 12.92*c if c<=0.0031308 else 1.055*c**(1/2.4)-0.055
f=ifcopenshell.open(ifc)
def revit_colour(e):
    m=ue.get_material(e, should_skip_usage=True)
    mats=[]
    if m is None: return None,None
    if m.is_a('IfcMaterial'): mats=[m]
    elif m.is_a('IfcMaterialConstituentSet'): mats=[c.Material for c in m.MaterialConstituents or []]
    else: return None,None
    if len(mats)!=1: return 'multi',None
    mat=mats[0]
    for rep in mat.HasRepresentation or []:
        for r in rep.Representations:
            for it in r.Items:
                for st in it.Styles:
                    for s in (st.Styles if st.is_a('IfcPresentationStyleAssignment') else [st]):
                        if s.is_a('IfcSurfaceStyle'):
                            for x in s.Styles:
                                if x.is_a('IfcSurfaceStyleShading') or x.is_a('IfcSurfaceStyleRendering'):
                                    c=x.SurfaceColour; return mat.Name,(c.Red,c.Green,c.Blue)
    # Snowdon: styles sit on the element's own geometry items.
    cols=set()
    def walk(item,depth=0):
        if depth>6 or item is None: return
        for st in getattr(item,'StyledByItem',None) or []:
            for sty in st.Styles:
                for s2 in (sty.Styles if sty.is_a('IfcPresentationStyleAssignment') else [sty]):
                    if s2.is_a('IfcSurfaceStyle'):
                        for x in s2.Styles:
                            if x.is_a('IfcSurfaceStyleShading'):
                                cc=x.SurfaceColour; cols.add((round(cc.Red,3),round(cc.Green,3),round(cc.Blue,3)))
        if item.is_a('IfcMappedItem'):
            for it in item.MappingSource.MappedRepresentation.Items: walk(it,depth+1)
        if item.is_a('IfcBooleanResult'):
            walk(item.FirstOperand,depth+1)
    if e.Representation:
        for r in e.Representation.Representations:
            if r.RepresentationIdentifier=='Body':
                for it in r.Items: walk(it)
    if len(cols)==1: return mat.Name,next(iter(cols))
    return mat.Name,None
bytag={e.Tag:e for e in f.by_type('IfcElement') if e.Tag and not e.is_a('IfcOpeningElement')}
c=collections.Counter(); bad=[]
for node in doc['nodes']:
    ex=node.get('extras') or {}
    if 'mesh' not in node: continue
    tag=(node.get('name') or '').rsplit(':',1)[-1].rsplit(' ',1)[-1]
    mesh=doc['meshes'][node['mesh']]; mi=mesh['primitives'][0].get('material')
    if mi is None: continue
    mat=doc['materials'][mi]; name=mat.get('name') or ''
    if name.startswith('Category ') or name.startswith('Layer'): continue
    e=bytag.get(tag)
    if e is None or not (e.is_a('IfcDoor') or e.is_a('IfcWindow') or e.is_a('IfcFurniture') or e.is_a('IfcBuildingElementProxy') or e.is_a('IfcFlowTerminal') or e.is_a('IfcPlate') or e.is_a('IfcMember') or e.is_a('IfcRailing') or e.is_a('IfcFurnishingElement') or e.is_a('IfcLightFixture') or e.is_a('IfcSanitaryTerminal') or e.is_a('IfcColumn') or e.is_a('IfcBeam')): continue
    ours=tuple(round(lin2srgb(x),3) for x in mat['pbrMetallicRoughness']['baseColorFactor'][:3])
    rn,rc=revit_colour(e)
    if rc is None: c[('revit', rn if rn=='multi' else 'no colour')]+=1; continue
    same_name = rn==name
    same_col = all(abs(a-b)<=0.01 for a,b in zip(ours,rc))
    c[(e.is_a(), 'name '+('=' if same_name else '!='), 'colour '+('=' if same_col else '!='))]+=1
    if not same_col and len(bad)<6: bad.append((tag,e.is_a(),name,rn,ours,tuple(round(x,3) for x in rc)))
for k,v in sorted(c.items(),key=str): print(v,k)
for x in bad: print('  ',x)
