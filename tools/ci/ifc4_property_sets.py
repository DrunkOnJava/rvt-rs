#!/usr/bin/env python3
"""The standard property sets of one IFC4 file against IFC4's definitions, as
a short, stable summary (#822).

IFC reserves the `Pset_` prefix for the property sets it defines. For every
property set named `Pset_` that the file relates to an object
(IfcRelDefinesByProperties) or gives a type object (HasPropertySets), this
counts what IFC4's definitions do not allow, in a fixed order:

    sets <n> on <m> objects and types
    not an IFC4 set: <set> <count>
    not a property of its IFC4 set: <set>.<property> <count>
    on an entity IFC4 does not apply it to: <set> on <entity> <count>
    another kind of property than IFC4 declares: <set>.<property> <kind> for <declared> <count>
    another value type than IFC4 declares: <set>.<property> <type> for <declared> <count>
    not one of the enumeration's values: <set>.<property> <value> <count>
    findings <n>

The definitions are buildingSMART's list of IFC4's property set definitions,
the one its validation service checks rule PSE001 against, pinned by commit
and sha256 in tools/ci/gen_standard_property_sets.py. A set scoped to a
predefined type (`IfcCovering/CEILING`) applies to an object with that
predefined type, or a user-defined one of that name. Quantity sets are not
checked. Measure (tools/ci/measure-reference-models.sh) runs this on every
export.

Usage: ifc4_property_sets.py MODEL.ifc [--definitions PSET_DEFINITIONS.csv] [--fail]

With --definitions the CSV is read from that path, and fetched to it first
when it is absent; its sha256 must be the pinned one either way. --fail exits
1 when there is any finding.
"""

import ast
import collections
import csv
import io
import os
import sys

import ifcopenshell

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_standard_property_sets as pinned  # noqa: E402


def definitions(path):
    """Set name -> (applicable (entity, scope) pairs, property name -> definition)."""
    if path and os.path.exists(path):
        text = pinned.source_text(path)
    else:
        text = pinned.source_text(None)
        if path:
            with open(path, "wb") as out:
                out.write(text.encode("utf-8"))
    csv.field_size_limit(sys.maxsize)
    out = {}
    for row in csv.DictReader(io.StringIO(text)):
        applies = []
        for applicable in ast.literal_eval(row["applicable_entities"]):
            entity, _, scope = applicable.partition("/")
            applies.append((entity.strip(), scope.strip() or None))
        properties = {p["property_name"]: p for p in ast.literal_eval(row["property_definitions"])}
        out[row["Name"]] = (applies, properties)
    return out


def lineage(schema, entity):
    """The entity's name and its supertypes' names, `None` for an unknown one."""
    try:
        declaration = schema.declaration_by_name(entity)
    except (RuntimeError, ValueError):
        return None
    names = []
    while declaration is not None:
        names.append(declaration.name().upper())
        declaration = declaration.supertype()
    return names


def applies_to(applies, schema, entity, predefined, object_type):
    names = lineage(schema, entity)
    if names is None:
        return True
    for applicable, scope in applies:
        if applicable.upper() not in names:
            continue
        if scope is None or predefined == scope or (predefined == "USERDEFINED" and object_type == scope):
            return True
    return False


def occurrence_of(schema, type_entity):
    """The occurrence entity of a type object's entity: IfcWallType -> IfcWall."""
    for suffix in ("Type", "Style"):
        if type_entity.endswith(suffix) and lineage(schema, type_entity[: -len(suffix)]):
            return type_entity[: -len(suffix)]
    return None


def main(argv):
    args = argv[1:]
    fail = "--fail" in args
    if fail:
        args.remove("--fail")
    path = None
    if "--definitions" in args:
        i = args.index("--definitions")
        path = args[i + 1]
        del args[i : i + 2]
    if len(args) != 1:
        sys.exit(__doc__)
    defined = definitions(path)
    f = ifcopenshell.open(args[0])
    schema = ifcopenshell.ifcopenshell_wrapper.schema_by_name(f.schema)

    held = []  # (property set, entity it is checked against, predefined type, object type)
    for rel in f.by_type("IfcRelDefinesByProperties"):
        pset = rel.RelatingPropertyDefinition
        if not pset.is_a("IfcPropertySet") or not (pset.Name or "").startswith("Pset_"):
            continue
        for obj in rel.RelatedObjects or ():
            held.append((pset, obj, obj.is_a(), getattr(obj, "PredefinedType", None), getattr(obj, "ObjectType", None)))
    for type_object in f.by_type("IfcTypeObject"):
        occurrence = occurrence_of(schema, type_object.is_a())
        for pset in type_object.HasPropertySets or ():
            if pset.is_a("IfcPropertySet") and (pset.Name or "").startswith("Pset_"):
                user = getattr(type_object, "ElementType", None)
                predefined = getattr(type_object, "PredefinedType", None)
                held.append((pset, type_object, occurrence, predefined, user))

    unknown_set = collections.Counter()
    unknown_property = collections.Counter()
    misapplied = collections.Counter()
    wrong_kind = collections.Counter()
    wrong_type = collections.Counter()
    wrong_value = collections.Counter()
    checked = set()
    for pset, obj, entity, predefined, object_type in held:
        name = pset.Name
        definition = defined.get(name)
        if definition is None:
            unknown_set[name] += 1
            continue
        applies, properties = definition
        if entity is not None and not applies_to(applies, schema, entity, predefined, object_type):
            misapplied[(name, entity)] += 1
        # A set related to several objects is one set: its properties are
        # counted once.
        if pset.id() in checked:
            continue
        checked.add(pset.id())
        for prop in pset.HasProperties or ():
            declared = properties.get(prop.Name)
            if declared is None:
                unknown_property[(name, prop.Name)] += 1
                continue
            if prop.is_a() != declared["property_type"]:
                wrong_kind[(name, prop.Name, prop.is_a(), declared["property_type"])] += 1
                continue
            data_type = declared.get("data_type")
            if not isinstance(data_type, dict):
                continue
            instance = data_type.get("instance") or ""
            if prop.is_a("IfcPropertySingleValue") and prop.NominalValue is not None and instance.startswith("Ifc"):
                if prop.NominalValue.is_a() != instance:
                    wrong_type[(name, prop.Name, prop.NominalValue.is_a(), instance)] += 1
            if prop.is_a("IfcPropertyEnumeratedValue") and data_type.get("values"):
                allowed = set(data_type["values"])
                for value in prop.EnumerationValues or ():
                    if str(value.wrappedValue) not in allowed:
                        wrong_value[(name, prop.Name, str(value.wrappedValue))] += 1

    objects = {obj.id() for _, obj, _, _, _ in held}
    print(f"sets {len({pset.id() for pset, *_ in held})} on {len(objects)} objects and types")
    lines = []
    for set_name, count in sorted(unknown_set.items()):
        lines.append(f"not an IFC4 set: {set_name} {count}")
    for (set_name, prop), count in sorted(unknown_property.items()):
        lines.append(f"not a property of its IFC4 set: {set_name}.{prop} {count}")
    for (set_name, entity), count in sorted(misapplied.items()):
        lines.append(f"on an entity IFC4 does not apply it to: {set_name} on {entity} {count}")
    for (set_name, prop, kind, declared), count in sorted(wrong_kind.items()):
        lines.append(
            f"another kind of property than IFC4 declares: {set_name}.{prop} {kind} for {declared} {count}"
        )
    for (set_name, prop, value_type, declared), count in sorted(wrong_type.items()):
        lines.append(
            f"another value type than IFC4 declares: {set_name}.{prop} {value_type} for {declared} {count}"
        )
    for (set_name, prop, value), count in sorted(wrong_value.items()):
        lines.append(f"not one of the enumeration's values: {set_name}.{prop} {value} {count}")
    for line in lines:
        print(line)
    print(f"findings {len(lines)}")
    return 1 if fail and lines else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
