#!/usr/bin/env python3
"""Fail when the saved-graphics field list and the readers it serves differ (#1694).

`SAVED_GRAPHICS_FIELDS` (src/research/native_saved_scene.rs) names the only
fields of a graphics record's objects that the saved-graphics pass builds. A
reader that takes a field the list does not name reads it as absent, and no
export need show it (a route no model exercises, the pre-2023 one, never
does). So the names are checked here: every `"m_..."` string literal of the
modules that read a graph object on the way from a record to a mesh must be in
the list, and every name in the list but `m_id` (which the check of a record
against its identity reads) must be a literal of one of them.

The one reader that depends on the set of fields and not a name,
`default_representation` in native_graphics_traversal.rs, counts the fields of
a pre-2023 `InstanceInfo`; this check cannot see that, and the doc comment of
the constant says so.

Usage:

    python3 tools/ci/saved-graphics-fields.py [repository root]
"""

import pathlib
import re
import sys

LIST_MODULE = "src/research/native_saved_scene.rs"
# The modules whose literals the list is made of, as the constant's doc
# comment names them; the module that holds the list is one of them.
READERS = (
    "src/research/native_saved_mesh.rs",
    "src/research/native_graphics_traversal.rs",
    "src/research/native_parametric_mesh.rs",
    "src/research/native_parametric_surface.rs",
    "src/research/native_empty_faces.rs",
    "src/research/partition_instance_transforms.rs",
    LIST_MODULE,
)
# Read by the decode itself and not by a reader: the check of a record against
# its identity (src/research/native_document.rs).
DECODE_READS = {"m_id"}
NAME = re.compile(r'"(m_[A-Za-z0-9_]+)"')
CONSTANT = re.compile(r"pub const SAVED_GRAPHICS_FIELDS: &\[&str\] = &\[(.*?)\];", re.S)


def main():
    root = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else pathlib.Path(".")
    scene = (root / LIST_MODULE).read_text()
    constant = CONSTANT.search(scene)
    if constant is None:
        raise SystemExit(f"{LIST_MODULE}: no SAVED_GRAPHICS_FIELDS constant")
    listed = set(NAME.findall(constant.group(1)))
    read = {}
    for path in READERS:
        text = (root / path).read_text()
        if path == LIST_MODULE:
            text = CONSTANT.sub("", text)
        for name in NAME.findall(text):
            read.setdefault(name, set()).add(path.rsplit("/", 1)[-1])
    missing = sorted(set(read) - listed)
    extra = sorted(listed - set(read) - DECODE_READS)
    for name in missing:
        print(f"::error::{name} is read by {', '.join(sorted(read[name]))} and not in SAVED_GRAPHICS_FIELDS")
    for name in extra:
        print(f"::error::{name} is in SAVED_GRAPHICS_FIELDS and no reader names it")
    if missing or extra:
        raise SystemExit(1)
    print(f"saved-graphics fields: {len(listed)} names, every literal of {len(READERS)} readers listed")


if __name__ == "__main__":
    main()
