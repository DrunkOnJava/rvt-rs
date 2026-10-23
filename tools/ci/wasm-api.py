"""The browser viewer's wasm API, as `rvt-wasm/src/lib.rs` exports it.

    python3 tools/ci/wasm-api.py compat <base lib.rs> <head lib.rs>
    python3 tools/ci/wasm-api.py docs <lib.rs> <README.md> <docs/viewer-api.md>

`compat` prints the breaking changes between two versions of the bindings
and exits 100 when there is any (as cargo-semver-checks does), so that a
crash, which exits 1, is not read as a verdict. JavaScript calls them by their `js_name` and
passes arguments by position, so a binding removed or renamed, a parameter
removed, moved or retyped, a new parameter that is not an `Option` (which
JavaScript may leave out), or a changed return type is breaking.
`docs` fails when the README's count of the bindings is not the count the
source exports, or when the table of `docs/viewer-api.md` (after its
`<!-- wasm-bindings -->` marker) does not list exactly the exported bindings.
"""

import re
import sys

BINDING = re.compile(
    r"#\[wasm_bindgen(?:\(([^)]*)\))?\]\s*(?:#\[[^\]]*\]\s*|///[^\n]*\n\s*)*pub fn ([a-z_0-9]+)\s*\(([^)]*)\)\s*(?:->\s*([^{]+?))?\s*\{",
    re.S,
)


def bindings(path):
    out = {}
    for m in BINDING.finditer(open(path, encoding="utf-8").read()):
        attr, fn, params, ret = m.groups()
        js = re.search(r"js_name\s*=\s*\"?([A-Za-z_0-9]+)\"?", attr or "")
        args = []
        for param in filter(None, (p.strip() for p in params.split(","))):
            name, _, ty = param.partition(":")
            args.append((name.strip(), " ".join(ty.split())))
        out[js.group(1) if js else fn] = (args, " ".join((ret or "()").split()))
    return out


def compat(base, head):
    found = []
    for name, (args, ret) in sorted(base.items()):
        if name not in head:
            found.append(f"{name}: removed or renamed")
            continue
        head_args, head_ret = head[name]
        for i, (pname, pty) in enumerate(args):
            if i >= len(head_args):
                found.append(f"{name}: parameter {pname} removed")
            elif head_args[i][1] != pty:
                found.append(f"{name}: parameter {i + 1} ({pname}) is {head_args[i][1]}, was {pty}")
        for pname, pty in head_args[len(args):]:
            if not pty.startswith("Option<"):
                found.append(f"{name}: new parameter {pname} ({pty}) cannot be left out")
        if head_ret != ret:
            found.append(f"{name}: returns {head_ret}, was {ret}")
    return found


def main(argv):
    if argv[1:2] == ["compat"]:
        found = compat(bindings(argv[2]), bindings(argv[3]))
        for line in found:
            print(f"breaking: {line}")
        if found:
            return 100
        print("The wasm API has no breaking change against the base.")
        return 0
    if argv[1:2] == ["docs"]:
        exported = set(bindings(argv[2]))
        failed = False
        claims = [int(n) for n in re.findall(r"the (\d+) JS-callable", open(argv[3], encoding="utf-8").read())]
        if claims != [len(exported)]:
            print(f"::error::README.md counts {claims} JS-callable bindings; rvt-wasm exports {len(exported)}")
            failed = True
        doc = open(argv[4], encoding="utf-8").read()
        rows = doc[doc.index("<!-- wasm-bindings -->"):].splitlines()[1:]
        listed = set()
        for row in rows[2:]:
            if not row.startswith("|"):
                break
            listed.add(row.split("|")[1].strip().strip("`"))
        for name in sorted(exported - listed):
            print(f"::error::docs/viewer-api.md does not list the binding {name}")
            failed = True
        for name in sorted(listed - exported):
            print(f"::error::docs/viewer-api.md lists {name}, which rvt-wasm does not export")
            failed = True
        if failed:
            return 1
        print(f"README.md and docs/viewer-api.md list the {len(exported)} bindings rvt-wasm exports.")
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv))
