"""The representation items an export wrote for one element, through mapped items.

    python items.py <ifc> <Tag>
"""
import collections
import sys
import ifcopenshell

model = ifcopenshell.open(sys.argv[1])
element = next(e for e in model.by_type("IfcProduct") if str(getattr(e, "Tag", "")) == sys.argv[2] and e.Representation)
seen = collections.Counter()

def walk(item, depth=0):
    seen[item.is_a()] += 1
    if item.is_a("IfcMappedItem"):
        for sub in item.MappingSource.MappedRepresentation.Items:
            walk(sub, depth + 1)
    if item.is_a("IfcBooleanResult"):
        walk(item.FirstOperand, depth + 1); walk(item.SecondOperand, depth + 1)

for rep in element.Representation.Representations:
    print("representation", rep.RepresentationIdentifier, rep.RepresentationType, len(rep.Items))
    for item in rep.Items:
        walk(item)
print(dict(seen))
