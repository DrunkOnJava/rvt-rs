#!/usr/bin/env python3
"""Cut an IFC down to a few elements, as text (RE-430).

Keeps the closure of the entities whose `Tag` is one of the given ones, with the project and the site and what
they reference, the property relationships that name a kept element (their property sets come with them) and
the aggregations of which a kept element is the whole. It is for an export too big to load whole with
IfcOpenShell (a cut of a Snowdon Towers sample's 118 furniture and sanitary terminals scores in minutes where
the whole file does not fit): the DATA section is read once as text and no model is built. The entities have to
be one to a line, as rvt-rs's writer and Revit's both write them.

Usage:

    python3 tools/re/cut_ifc.py <in.ifc> <out.ifc> <Tag> [<Tag> ...]

The cut of the same Tags from rvt-rs's export and from Revit's is what `tools/re/mesh_bodies_vs_ifc.py` scores.
"""

import re
import sys

ENTITY = re.compile(r"^#(\d+)\s*=\s*([A-Z0-9_]+)\s*\((.*)\)\s*;\s*$")
REF = re.compile(r"#(\d+)")


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


def read(path):
    """The header lines, the footer lines, and index -> (line, class) of every entity in the DATA section."""
    header, footer, entities = [], [], {}
    state = "header"
    with open(path, encoding="utf-8", errors="replace") as handle:
        for raw in handle:
            line = raw.rstrip("\n")
            if state == "header":
                header.append(line)
                if line.strip() == "DATA;":
                    state = "data"
            elif state == "footer" or line.strip() == "ENDSEC;":
                state = "footer"
                footer.append(line)
            else:
                match = ENTITY.match(line)
                if match:
                    entities[int(match.group(1))] = (line, match.group(2))
    return header, footer, entities


def main():
    if len(sys.argv) < 4:
        print(__doc__, file=sys.stderr)
        return 2
    source, destination, *tags = sys.argv[1:]
    wanted = set(tags)
    header, footer, entities = read(source)
    print(f"{len(entities)} entities read", file=sys.stderr)

    roots = []
    for index, (line, kind) in entities.items():
        if kind in ("IFCPROJECT", "IFCSITE"):
            roots.append(index)
        elif kind.startswith("IFC") and not kind.startswith("IFCREL") and kind not in ("IFCPROPERTYSET", "IFCPROPERTYSINGLEVALUE"):
            args = top_level_args(ENTITY.match(line).group(3))
            if len(args) > 7 and re.fullmatch(r"'\d+'", args[7] or "") and args[7].strip("'") in wanted:
                roots.append(index)
    print(f"{len(roots)} roots", file=sys.stderr)

    # The aggregations whose whole is a kept root: their parts are kept too.
    by_whole = {}
    for index, (line, kind) in entities.items():
        if kind == "IFCRELAGGREGATES":
            args = top_level_args(ENTITY.match(line).group(3))
            if len(args) >= 6:
                wholes = REF.findall(args[4])
                if wholes:
                    by_whole.setdefault(int(wholes[0]), []).append((index, [int(x) for x in REF.findall(args[5])]))

    # The property relationships that name a root: their property sets come with them.
    root_set = set(roots)
    for index, (line, kind) in entities.items():
        if kind == "IFCRELDEFINESBYPROPERTIES":
            args = top_level_args(ENTITY.match(line).group(3))
            if len(args) >= 6 and any(int(x) in root_set for x in REF.findall(args[4])):
                roots.append(index)

    keep = set()
    stack = list(roots)
    while stack:
        index = stack.pop()
        if index in keep or index not in entities:
            continue
        keep.add(index)
        stack.extend(int(x) for x in REF.findall(entities[index][0]))
        for relation, parts in by_whole.get(index, []):
            stack.append(relation)
            stack.extend(parts)
    print(f"{len(keep)} entities kept", file=sys.stderr)

    with open(destination, "w", encoding="utf-8") as out:
        out.write("\n".join(header) + "\n")
        for index in sorted(keep):
            out.write(entities[index][0] + "\n")
        out.write("\n".join(footer) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
