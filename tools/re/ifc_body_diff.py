#!/usr/bin/env python3
"""Compare the bodies of two IFC exports of one model, object by object, as text (RE-430).

Pairs every object that has a GlobalId and a representation by GlobalId and compares its class, its
`BodySource` and `SavedMeshRefusal` properties and a fingerprint of each IfcTriangulatedFaceSet it draws (the
SHA-256 of its point list and its index list). Any other item is compared by its IFC type alone: a box's size is
not read. No geometry kernel runs and no model is built, so an export of 270 MB is read in minutes: the DATA
section is parsed line by line, which needs the entities one to a line, as rvt-rs's writer writes them.

It prints how many objects are in one file only; how many change `BodySource`, from what to what, by class;
how many were drawn from a saved mesh in the base and are not in the head; among the objects drawn from a saved
mesh in both files, how many draw a different mesh; and, of the others, how many differ in geometry or in the
text of their refusal, grouped by the change in the refusal. That is what reads "no element drawn before
changes" and "every element that moves is a box becoming a saved mesh" for a model Revit has no export of.

With an output path it writes one line, GlobalId, Tag and class, for each object drawn from a saved mesh in the
head and not in the base.

Usage:

    python3 tools/re/ifc_body_diff.py <base.ifc> <head.ifc> [<newly-drawn.txt>]

Measure keeps both sides' exports with `-f keep_ifc=true` and an explicit `-f base=<sha>`.
"""

import collections
import hashlib
import re
import sys

ENTITY = re.compile(r"^#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\)\s*;\s*$")
REF = re.compile(r"#(\d+)")
GLOBAL_ID = re.compile(r"'[0-9A-Za-z_$]{22}'")
NUMERIC_TAG = re.compile(r"'\d+'")
SAVED_MESH = "saved_graphics_mesh"
PROPERTIES = ("BodySource", "SavedMeshRefusal")


def top_level_args(body):
    """The arguments of a STEP entity, split at the commas that are neither inside a string nor a list."""
    args, depth, quote, current = [], 0, False, []
    i = 0
    while i < len(body):
        c = body[i]
        if quote:
            current.append(c)
            if c == "'":
                if i + 1 < len(body) and body[i + 1] == "'":
                    current.append("'")
                    i += 1
                else:
                    quote = False
        elif c == "'":
            quote = True
            current.append(c)
        elif c == "(":
            depth += 1
            current.append(c)
        elif c == ")":
            depth -= 1
            current.append(c)
        elif c == "," and depth == 0:
            args.append("".join(current).strip())
            current = []
        else:
            current.append(c)
        i += 1
    args.append("".join(current).strip())
    return args


def refs(text):
    return [int(x) for x in REF.findall(text)]


def read(path):
    """GlobalId -> (class, Tag, BodySource, SavedMeshRefusal, fingerprints) of every object with a representation."""
    entities = {}
    with open(path, encoding="utf-8", errors="replace") as handle:
        for line in handle:
            match = ENTITY.match(line.rstrip("\n"))
            if match:
                entities[int(match.group(1))] = (match.group(2), match.group(3))

    def kind_of(index):
        return entities.get(index, ("",))[0]

    # An object is an IfcProduct whose seventh attribute, Representation, is a product definition shape.
    objects = {}
    for index, (kind, body) in entities.items():
        if not kind.startswith("IFC") or kind.startswith("IFCREL"):
            continue
        args = top_level_args(body)
        if len(args) > 7 and GLOBAL_ID.fullmatch(args[0] or "") and any(kind_of(r) == "IFCPRODUCTDEFINITIONSHAPE" for r in refs(args[6])):
            objects[index] = (args[0].strip("'"), kind, args)

    properties = collections.defaultdict(dict)
    for index, (kind, body) in entities.items():
        if kind != "IFCRELDEFINESBYPROPERTIES":
            continue
        args = top_level_args(body)
        for definition in refs(args[5]):
            if kind_of(definition) != "IFCPROPERTYSET":
                continue
            for single in refs(top_level_args(entities[definition][1])[4]):
                if kind_of(single) == "IFCPROPERTYSINGLEVALUE":
                    prop = top_level_args(entities[single][1])
                    if prop[0].strip("'") in PROPERTIES:
                        for target in refs(args[4]):
                            properties[target][prop[0].strip("'")] = prop[2]

    out = {}
    for index, (gid, kind, args) in objects.items():
        prints = []
        for shape in refs(args[6]):
            if kind_of(shape) != "IFCPRODUCTDEFINITIONSHAPE":
                continue
            for representation in refs(top_level_args(entities[shape][1])[2]):
                if kind_of(representation) != "IFCSHAPEREPRESENTATION":
                    continue
                for item in refs(top_level_args(entities[representation][1])[3]):
                    if kind_of(item) == "IFCTRIANGULATEDFACESET":
                        face_set = top_level_args(entities[item][1])
                        points = entities[refs(face_set[0])[0]][1]
                        prints.append(hashlib.sha256((points + "|" + face_set[3]).encode()).hexdigest()[:16])
                    elif kind_of(item):
                        prints.append(kind_of(item))
        tag = args[7].strip("'") if NUMERIC_TAG.fullmatch(args[7] or "") else ""
        out[gid] = (kind, tag, properties[index].get("BodySource", ""), properties[index].get("SavedMeshRefusal", ""), tuple(prints))
    return out


def label(entry, gid):
    return entry[1] or gid


def main():
    if len(sys.argv) not in (3, 4):
        print(__doc__, file=sys.stderr)
        return 2
    base, head = read(sys.argv[1]), read(sys.argv[2])
    both = sorted(base.keys() & head.keys())
    print(
        f"objects with a representation: base {len(base)}, head {len(head)}; "
        f"only in base {len(base.keys() - head.keys())}, only in head {len(head.keys() - base.keys())}"
    )

    changed = [g for g in both if base[g][2] != head[g][2]]
    print(f"BodySource changed: {len(changed)}")
    moves = collections.Counter((base[g][0], base[g][2][:60], head[g][2][:60]) for g in changed)
    for (cls, before, after), count in moves.most_common():
        print(f"  {count:5} {cls}: {before} -> {after}")
    lost = [g for g in changed if SAVED_MESH in base[g][2] and SAVED_MESH not in head[g][2]]
    print(f"drawn from a saved mesh in the base and not in the head: {len(lost)} {[label(base[g], g) for g in lost[:20]]}")

    drawn = [g for g in both if SAVED_MESH in base[g][2] and base[g][2] == head[g][2]]
    differ = [g for g in drawn if base[g][4] != head[g][4]]
    print(f"objects drawn from a saved mesh in both: {len(drawn)}; their meshes differ: {len(differ)} {[label(base[g], g) for g in differ[:20]]}")

    others = [g for g in both if base[g][2] == head[g][2] and SAVED_MESH not in base[g][2] and (base[g][4] != head[g][4] or base[g][3] != head[g][3])]
    print(f"other objects whose geometry or refusal differs: {len(others)}")
    geometry = [g for g in others if base[g][4] != head[g][4]]
    print(f"  of them, geometry differs: {len(geometry)} {[label(base[g], g) for g in geometry[:20]]}")
    refusals = collections.Counter((base[g][0], base[g][3][:70], head[g][3][:90]) for g in others if base[g][3] != head[g][3])
    for (cls, before, after), count in refusals.most_common():
        print(f"  {count:5} {cls}: {before or '(none)'} -> {after or '(none)'}")

    newly = [g for g in changed if SAVED_MESH in head[g][2] and SAVED_MESH not in base[g][2]]
    print(f"newly drawn from a saved mesh: {len(newly)}")
    if len(sys.argv) == 4:
        with open(sys.argv[3], "w", encoding="utf-8") as out:
            for g in newly:
                out.write(f"{g}\t{head[g][1]}\t{head[g][0]}\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
