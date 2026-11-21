"""#2085: the walls whose base quantities differ between two exports, with what each gained or lost."""
import sys
import ifcopenshell
import ifcopenshell.util.element as U
def quantities(path):
    m = ifcopenshell.open(path); out = {}
    for w in m.by_type("IfcWall"):
        q = U.get_psets(w, qtos_only=True).get("Qto_WallBaseQuantities", {})
        out[w.Tag] = ({k: v for k, v in q.items() if k != "id"}, w.Name)
    return out
a, b = quantities(sys.argv[1]), quantities(sys.argv[2])
n = 0
for t in sorted(set(a) | set(b)):
    qa, qb = a.get(t, ({}, None))[0], b.get(t, ({}, None))[0]
    if qa != qb:
        n += 1
        gained = sorted(set(qb) - set(qa)); lost = sorted(set(qa) - set(qb)); changed = sorted(k for k in set(qa) & set(qb) if qa[k] != qb[k])
        print(f"wall {t} '{(b.get(t) or a.get(t))[1]}': gained {gained} lost {lost} changed {changed}")
print("walls differing:", n)
