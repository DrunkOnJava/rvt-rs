"""#2114: the stride and the CRC-32 rule on one class's Tags in two exports of one model."""
import os, re, sys, zlib, collections
def tags(path, cls):
    sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "tools", "re"))
    import ifcopenshell
    from saved_meshes_vs_ifc import body_sources
    model = ifcopenshell.open(path)
    return {e.Tag for e in model.by_type(cls) if e.Tag and body_sources(e) == "saved_graphics_mesh"}
def stride(t, cap=300):
    t = sorted(t, key=lambda x: (len(x), x)); step = max(1, -(-len(t) // cap)); return set(t[::step])
def crc(t, cap=300):
    t = sorted(t, key=lambda x: (len(x), x))
    return set(sorted(t, key=lambda x: (zlib.crc32(x.encode()), len(x), x))[:cap]) if len(t) > cap else set(t)
a, b = tags(sys.argv[1], sys.argv[3]), tags(sys.argv[2], sys.argv[3])
print(f"{sys.argv[3]} Tags: {len(a)} -> {len(b)} (added {len(b - a)}, removed {len(a - b)})")
for name, f in (("stride", stride), ("crc", crc)):
    x, y = f(a), f(b)
    print(f"  {name}: scored {len(x)} -> {len(y)}; the same {len(x & y)}; in or out {len(x ^ y)}")
