"""Print the breaking changes between two versions of the Python API's stubs.

    python3 tools/ci/python-api-compat.py <base.pyi> <head.pyi>

The stubs, `python/rvt/__init__.pyi`, are the Python API's listing: CI's
stubtest step fails when they and the built wheel disagree. A change is
breaking when a public name the base had is gone or changed kind (a function
made `async`, or a method made static, included), when a parameter is
removed, renamed, moved, made keyword-only or made positional-only, when one
loses its default or its default changes, when a new parameter has no
default, or when an `@overload` the base offered is gone. A positional-only
parameter is matched by its place, and `*args` and `**kwargs` by their role,
since no caller can name them. An explicit re-export (`from m import X as X`)
is a public name, and a key gone from a TypedDict (a returned dict's shape) a
removed one. Exits 1 when there is any.
"""

import ast
import sys


def params(fn):
    """(name, kind, default) for each parameter but `self`/`cls`; `default` is
    the default's source text, `None` when there is none. `*args` and
    `**kwargs` are named by their role, since no caller can name them."""
    a = fn.args
    positional = a.posonlyargs + a.args
    defaults = [None] * (len(positional) - len(a.defaults)) + [ast.unparse(d) for d in a.defaults]
    kinds = ["positional-only"] * len(a.posonlyargs) + ["positional"] * len(a.args)
    out = [(p.arg, k, d) for p, k, d in zip(positional, kinds, defaults)]
    if a.vararg:
        out.append(("*", "variadic", ""))
    out += [(p.arg, "keyword", None if d is None else ast.unparse(d)) for p, d in zip(a.kwonlyargs, a.kw_defaults)]
    if a.kwarg:
        out.append(("**", "variadic", ""))
    return [p for p in out if p[0] not in ("self", "cls")]


def public(name):
    return not name.startswith("_") or (name.startswith("__") and name.endswith("__"))


def decorator_names(node):
    names = set()
    for d in node.decorator_list:
        if isinstance(d, ast.Name):
            names.add(d.id)
        elif isinstance(d, ast.Attribute):
            names.add(d.attr)  # typing.overload, x.setter, x.deleter
    return names


def api(path):
    """Each public name: (kind, signatures). A callable keeps one signature
    per definition, so every `@overload` is compared."""
    tree = ast.parse(open(path, encoding="utf-8").read())
    out = {}

    def members(prefix, body, in_class):
        for node in body:
            if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and public(node.name):
                decorators = decorator_names(node)
                # A pyo3 class is constructed through `__new__`; either spelling
                # in the stubs is the class's constructor.
                name = prefix + ("__init__" if node.name == "__new__" else node.name)
                if decorators & {"property", "setter", "getter", "deleter"}:
                    out[name] = ("property", None)
                    continue
                if "staticmethod" in decorators:
                    kind = "static method"
                elif "classmethod" in decorators:
                    kind = "class method"
                else:
                    kind = "method" if in_class else "function"
                if isinstance(node, ast.AsyncFunctionDef):
                    kind = "async " + kind
                signature = params(node)
                if name in out and out[name][1] is not None and "overload" in decorators | out[name][2]:
                    out[name][1].append(signature)
                else:
                    out[name] = (kind, [signature], decorators)
            elif isinstance(node, ast.ClassDef) and public(node.name):
                out[prefix + node.name] = ("class", None)
                members(prefix + node.name + ".", node.body, True)
            elif isinstance(node, ast.AnnAssign) and isinstance(node.target, ast.Name) and public(node.target.id):
                out[prefix + node.target.id] = ("attribute", None)
            elif isinstance(node, ast.Assign):
                for target in node.targets:
                    if isinstance(target, ast.Name) and public(target.id):
                        out[prefix + target.id] = ("attribute", None)
            elif isinstance(node, (ast.ImportFrom, ast.Import)):
                # A stub's explicit re-export, `from m import X as X`.
                for alias in node.names:
                    if alias.asname and alias.asname == alias.name.rsplit(".", 1)[-1] and public(alias.asname):
                        out[prefix + alias.asname] = ("re-export", None)

    members("", tree.body, False)
    return {name: entry[:2] for name, entry in out.items()}


def article(kind):
    return ("an " if kind[0] in "aeiou" else "a ") + kind


def signature_breaks(name, base_params, head_params):
    found = []
    head_by_name = {p[0]: (i, p) for i, p in enumerate(head_params)}
    for i, (pname, pkind, pdefault) in enumerate(base_params):
        if pkind == "positional-only":
            if i >= len(head_params) or head_params[i][1] not in ("positional-only", "positional"):
                found.append(f"{name}: positional-only parameter {i + 1} ({pname}) removed or moved")
            elif pdefault is not None and head_params[i][2] is None:
                found.append(f"{name}: positional-only parameter {i + 1} ({pname}) lost its default")
            elif pdefault is not None and head_params[i][2] != pdefault:
                found.append(f"{name}: positional-only parameter {i + 1} ({pname}) default changed from {pdefault} to {head_params[i][2]}")
            continue
        if pname not in head_by_name:
            found.append(f"{name}: parameter {pname} removed or renamed")
            continue
        j, (_, hkind, hdefault) = head_by_name[pname]
        if pkind in ("positional", "keyword") and hkind == "positional-only":
            found.append(f"{name}: parameter {pname} made positional-only")
        elif pkind == "positional" and (hkind != "positional" or j != i):
            found.append(f"{name}: parameter {pname} moved or made keyword-only")
        if pdefault is not None and hdefault is None:
            found.append(f"{name}: parameter {pname} lost its default")
        elif pdefault is not None and pkind != "variadic" and hdefault != pdefault:
            found.append(f"{name}: parameter {pname} default changed from {pdefault} to {hdefault}")
    base_names = {p[0] for p in base_params}
    for j, (hname, hkind, hdefault) in enumerate(head_params):
        if hdefault is not None or (hkind == "positional-only" and j < len(base_params)):
            continue
        if hname not in base_names:
            found.append(f"{name}: new parameter {hname} has no default")
    return found


def breaks(base, head):
    found = []
    for name, (kind, base_sigs) in sorted(base.items()):
        if name not in head:
            found.append(f"{name}: removed")
            continue
        head_kind, head_sigs = head[name]
        if head_kind != kind:
            found.append(f"{name}: was {article(kind)}, is {article(head_kind)}")
            continue
        if base_sigs is None:
            continue
        if len(base_sigs) == 1 and len(head_sigs) == 1:
            found += signature_breaks(name, base_sigs[0], head_sigs[0])
            continue
        # Overloads: every base signature must still be offered as it was.
        for sig in base_sigs:
            if sig not in head_sigs:
                found.append(f"{name}: overload ({', '.join(p[0] for p in sig)}) removed or changed")
    return found


if __name__ == "__main__":
    found = breaks(api(sys.argv[1]), api(sys.argv[2]))
    for line in found:
        print(f"breaking: {line}")
    if found:
        sys.exit(1)
    print("The Python API has no breaking change against the base.")
