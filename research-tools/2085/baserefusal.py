import sys, re
base, listing = sys.argv[1:3]
tags = {l.split()[1] for l in open(listing) if len(l.split()) >= 2}
# SavedMeshRefusal is an IFCPROPERTYSINGLEVALUE in a set related to the object; read it by a text pass.
ent = {}; rel = []
for line in open(base, errors="replace"):
    m = re.match(r"^#(\d+)=\s*([A-Z0-9]+)\((.*)\);\s*$", line)
    if m: ent[m.group(1)] = (m.group(2), m.group(3))
objs = {i: a for i, (t, a) in ent.items() if t not in ("IFCPROPERTYSINGLEVALUE", "IFCPROPERTYSET", "IFCRELDEFINESBYPROPERTIES") and any("'%s'" % t2 in a for t2 in tags)}
for i, (t, a) in ent.items():
    if t == "IFCRELDEFINESBYPROPERTIES":
        refs = re.findall(r"#(\d+)", a); rel.append(refs)
for oid, a in objs.items():
    tag = next(t2 for t2 in tags if "'%s'" % t2 in a)
    texts = []
    for refs in rel:
        if oid in refs[:-1]:
            ps = ent.get(refs[-1]);
            if ps and ps[0] == "IFCPROPERTYSET":
                for pid in re.findall(r"#(\d+)", ps[1]):
                    p = ent.get(pid)
                    if p and p[0] == "IFCPROPERTYSINGLEVALUE" and "SavedMeshRefusal" in p[1]:
                        texts.append(re.search(r"IFC[A-Z]+\('([^']*)'\)", p[1]).group(1))
    print(f"  {tag} {ent[oid][0]}: base SavedMeshRefusal {texts or 'none'}")
