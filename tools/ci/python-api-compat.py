"""Print the breaking changes between two versions of the Python API's stubs.

    python3 tools/ci/python-api-compat.py <base.pyi> <head.pyi>

The stubs, `python/rvt/__init__.pyi`, are the Python API's listing: CI's
stubtest step fails when they and the built wheel disagree. A change is
breaking when a public name the base had is gone or changed kind, when a
parameter is removed, renamed, moved or made keyword-only, when one loses its
default, or when a new parameter has no default. A key gone from a TypedDict
(a returned dict's shape) is a removed name. Exits 1 when there is any.
"""

import ast
import sys


def params(fn):
    a = fn.args
    positional = a.posonlyargs + a.args
    defaults = [False] * (len(positional) - len(a.defaults)) + [True] * len(a.defaults)
    out = [(p.arg, "positional", d) for p, d in zip(positional, defaults)]
    if a.vararg:
        out.append(("*" + a.vararg.arg, "variadic", True))
    out += [(p.arg, "keyword", d is not None) for p, d in zip(a.kwonlyargs, a.kw_defaults)]
    if a.kwarg:
        out.append(("**" + a.kwarg.arg, "variadic", True))
    return [p for p in out if p[0] not in ("self", "cls")]


def public(name):
    return not name.startswith("_") or (name.startswith("__") and name.endswith("__") and name != "__version__")


def api(path):
    tree = ast.parse(open(path, encoding="utf-8").read())
    out = {}

    def members(prefix, body):
        for node in body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and public(node.name):
                decorators = {d.id for d in node.decorator_list if isinstance(d, ast.Name)}
                kind = "property" if "property" in decorators else "function"
                out[prefix + node.name] = (kind, params(node) if kind == "function" else None)
            elif isinstance(node, ast.ClassDef) and public(node.name):
                out[prefix + node.name] = ("class", None)
                members(prefix + node.name + ".", node.body)
            elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and public(node.target.id):
                out[prefix + node.target.id] = ("attribute", None)
            elif isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and public(target.id):
                        out[prefix + target.id] = ("attribute", None)

    members("", tree.body)
    return out


def breaks(base, head):
    found = []
    for name, (kind, base_params) in sorted(base.items()):
        if name not in head:
            found.append(f"{name}: removed")
            continue
        head_kind, head_params = head[name]
        if head_kind != kind:
            found.append(f"{name}: was a {kind}, is a {head_kind}")
            continue
        if kind != "function":
            continue
        head_by_name = {p[0]: (i, p) for i, p in enumerate(head_params)}
        for i, (pname, pkind, pdefault) in enumerate(base_params):
            if pname not in head_by_name:
                found.append(f"{name}: parameter {pname} removed or renamed")
                continue
            j, (_, hkind, hdefault) = head_by_name[pname]
            if pkind == "positional" and (hkind != "positional" or j != i):
                found.append(f"{name}: parameter {pname} moved or made keyword-only")
            if pdefault and not hdefault:
                found.append(f"{name}: parameter {pname} lost its default")
        base_names = {p[0] for p in base_params}
        for hname, _, hdefault in head_params:
            if hname not in base_names and not hdefault:
                found.append(f"{name}: new parameter {hname} has no default")
    return found


if __name__ == "__main__":
    found = breaks(api(sys.argv[1]), api(sys.argv[2]))
    for line in found:
        print(f"breaking: {line}")
    if found:
        sys.exit(1)
    print("The Python API has no breaking change against the base.")
