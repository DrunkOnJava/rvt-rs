#!/usr/bin/env python3
"""Measure's score cache: the output of a scorer or schema step, kept under a
key made of everything the step reads, so a step whose inputs have not changed
is not run again (.github/workflows/measure.yml, tools/ci/measure-reference-models.sh).

    measure-cache.py lookup --step S --dir D --dest FILE [--extra FILE]... -- SCRIPT ARG...
    measure-cache.py store  --step S --dir D --dest FILE [--extra FILE]... [--status N]
    measure-cache.py finish [--days N] [--max-mb N] [--report FILE]

`lookup` exits 0 on a hit (FILE and the extras are written from the cache and
`S cached` is added to D/timings.log), 1 on a miss (the key is remembered for
`store`) and 2 when the step is not cached, saying why in the report. `store`
keeps FILE and the extras (files the step also writes, such as the validation
findings) when the step finished with status 0 and its output shows no sign of
a failure that a rerun could mend. `finish` prunes the cache, prints what the
run did and, for the workflow, sets `changed` in $GITHUB_OUTPUT: whether a new
cache is worth saving.

The key is a sha256 over:
  - the step's name and the scorer's name as given, and every argument as given;
  - the content of each argument that is a file: the rvt-rs IFC and the glTF
    (the files the run writes beside the step, in D) and everything else
    (Revit's export, the validation findings, the property set definitions).
    An IFC in D is hashed without its FILE_NAME and IFCOWNERHISTORY lines,
    which carry the export time (the hash tools/ci/measure-reference-models.sh
    writes to ifc.sha256), so a rebuild that writes the same model hits;
  - the scorer and every module of tools/re and tools/ci it can reach, by
    import, importlib path or name (a module it only mentions is included);
  - the value of every environment variable the closure reads, except the two
    that only size the mesh cache (RVT_MESH_CACHE, RVT_MESH_WORKERS);
  - the files a scorer reads that its arguments do not name (EXTRA_INPUTS);
  - the Python and platform, the sha256 of tools/ci/requirements-ifc.txt, the
    installed version of every package it pins, the versions IfcOpenShell and
    numpy report, and RVT_IFC_FLAGS.

A step is not cached when its closure does something the key cannot see: lists
or walks a directory, takes a directory name from an argument, reads a file
it names itself, runs a program, draws a random number or a date, reads the
IFC's export time, or reads an environment variable by a computed name. The
models that Measure keeps only counts of (`holdout-*`, `bimcollab-*`, the
directories tools/ci/scrub-holdout-results.sh reduces) are never looked up or
stored, because the Actions cache of a public repository can be restored by a
workflow a fork's pull request changes, so what the scrub removes must not be in
it; each entry names its model, and `finish` removes any that names one of
those or none. Only status 0 is kept: a scorer that crashed, ran out of time or of
memory is run again.
"""

import argparse
import ast
import collections
import fnmatch
import hashlib
import importlib.metadata
import json
import os
import platform
import re
import shutil
import sys
import time

CI_DIR = os.path.dirname(os.path.abspath(__file__))
RE_DIR = os.path.join(os.path.dirname(CI_DIR), "re")
REQUIREMENTS = os.path.join(CI_DIR, "requirements-ifc.txt")

FORMAT = 1
NEUTRAL_ENV = {"RVT_MESH_CACHE", "RVT_MESH_WORKERS"}
HELD_OUT = ("holdout-*", "bimcollab-*")
MAX_ENTRY_BYTES = 4 * 1024 * 1024
STALE_SECONDS = 3 * 24 * 3600
# Output that shows a failure a rerun could mend, whatever the status says.
TRANSIENT = ("unreadable:", "a worker died", "Traceback (most recent call last)", "MemoryError", "BrokenProcessPool")
IFC_TIME_LINES = re.compile(rb"^FILE_NAME\(|IFCOWNERHISTORY")

BAD_MODULES = {"subprocess", "socket", "urllib", "requests", "http", "random", "uuid", "datetime", "tempfile", "glob"}
BAD_ATTRIBUTES = {
    "listdir": "fs-listing",
    "scandir": "fs-listing",
    "iterdir": "fs-listing",
    "rglob": "fs-listing",
    "walk": "fs-listing",
    "getcwd": "process",
    "expanduser": "process",
    "system": "process",
    "popen": "process",
    "OwnerHistory": "export-time",
    "CreationDate": "export-time",
    "file_name": "export-time",
    "time_stamp": "export-time",
    "TimeStamp": "export-time",
    "creation_date": "export-time",
}
OWN_DIRECTORY = {"os.path.abspath(__file__)", "__file__"}
# ifc_common lists the mesh cache directory that RVT_MESH_CACHE names, which
# only saves meshing and changes no scorer's output.
# gen_standard_property_sets fetches the property set definitions when the
# file is absent, and checks them against the sha256 it pins; its source is in
# the key, so the pin is.
ALLOWED_IN = {"ifc_common": {"fs-listing"}, "gen_standard_property_sets": {"process"}}


class Uncacheable(Exception):
    pass


def is_held_out(model):
    """The models tools/ci/scrub-holdout-results.sh reduces to counts before
    anything leaves the runner (its `holdout-*` and `bimcollab-*` directories)."""
    return any(fnmatch.fnmatch(model, pattern) for pattern in HELD_OUT)


def work_dir():
    path = os.environ.get("MEASURE_CACHE_WORK") or os.path.join(os.environ.get("RUNNER_TEMP", "/tmp"), "measure-cache-work")
    os.makedirs(path, exist_ok=True)
    return path


def cache_dir():
    path = os.environ.get("MEASURE_CACHE_DIR")
    if path:
        os.makedirs(path, exist_ok=True)
    return path


def log(line):
    with open(os.path.join(work_dir(), "report.log"), "a", encoding="utf-8") as report:
        report.write(line + "\n")


def sha(data):
    return hashlib.sha256(data).hexdigest()


def atomic_write(path, data):
    scratch = f"{path}.{os.getpid()}.{time.time_ns()}"
    with open(scratch, "wb") as out:
        out.write(data)
    os.replace(scratch, path)


# ---- file digests -----------------------------------------------------------


def file_digest(path, ifc_without_export_time):
    """The sha256 of the file, memoised for the run; for an IFC the run wrote,
    of its lines other than FILE_NAME and IFCOWNERHISTORY."""
    stat = os.stat(path)
    memo = os.path.join(work_dir(), "digests")
    os.makedirs(memo, exist_ok=True)
    name = sha(f"{os.path.realpath(path)} {stat.st_size} {stat.st_mtime_ns} {ifc_without_export_time}".encode())
    remembered = os.path.join(memo, name)
    if os.path.exists(remembered):
        with open(remembered, encoding="ascii") as handle:
            return handle.read()
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        if ifc_without_export_time:
            for line in handle:
                if not IFC_TIME_LINES.search(line):
                    digest.update(line if line.endswith(b"\n") else line + b"\n")
        else:
            for block in iter(lambda: handle.read(1 << 20), b""):
                digest.update(block)
    value = digest.hexdigest()
    atomic_write(remembered, value.encode("ascii"))
    return value


# ---- the scorer's closure ---------------------------------------------------


def local_modules():
    found = {}
    for directory in (RE_DIR, CI_DIR):
        for name in sorted(os.listdir(directory)):
            if name.endswith(".py"):
                found.setdefault(name[:-3], []).append(os.path.join(directory, name))
    return found


def parse(path):
    try:
        with open(path, "rb") as handle:
            return ast.parse(handle.read(), filename=path)
    except (OSError, SyntaxError) as error:
        raise Uncacheable(f"cannot read {os.path.basename(path)}: {type(error).__name__}")


def docstrings(tree):
    ids = set()
    for node in ast.walk(tree):
        if isinstance(node, (ast.Module, ast.ClassDef, ast.FunctionDef, ast.AsyncFunctionDef)):
            body = node.body
            if body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant) and isinstance(body[0].value.value, str):
                ids.add(id(body[0].value))
    return ids


def references(tree, known):
    skip = docstrings(tree)
    names = set()
    for node in ast.walk(tree):
        if isinstance(node, ast.Import):
            names.update(alias.name.split(".")[0] for alias in node.names)
        elif isinstance(node, ast.ImportFrom):
            if node.module:
                names.add(node.module.split(".")[0])
            names.update(alias.name for alias in node.names)
        elif isinstance(node, ast.Constant) and isinstance(node.value, str) and id(node) not in skip:
            base = os.path.basename(node.value)
            names.add(base[:-3] if base.endswith(".py") else node.value)
    return names & set(known)


def closure(script):
    """The script and every local module it reaches, as {path: tree}."""
    known = local_modules()
    script = os.path.abspath(script)
    if not os.path.isfile(script):
        raise Uncacheable("the scorer is not a file")
    seen = {}
    queue = [script]
    while queue:
        path = queue.pop()
        if path in seen:
            continue
        seen[path] = parse(path)
        for name in references(seen[path], known):
            own = os.path.join(os.path.dirname(path), name + ".py")
            queue.append(own if os.path.isfile(own) else known[name][0])
    return seen


def audit(trees):
    """What the closure does that the key could not see, and the environment
    variables it reads (a set of names; None for a computed name)."""
    kinds = set()
    env = set()
    for path, tree in trees.items():
        found = set()
        skip = docstrings(tree)
        handled = set()
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                found.update("process" for alias in node.names if alias.name.split(".")[0] in BAD_MODULES)
            elif isinstance(node, ast.ImportFrom):
                if node.module and node.module.split(".")[0] in BAD_MODULES:
                    found.add("process")
                if node.module == "os" and any(alias.name in ("environ", "getenv") for alias in node.names):
                    env.add(None)
            elif isinstance(node, ast.Attribute):
                if node.attr in BAD_ATTRIBUTES:
                    found.add(BAD_ATTRIBUTES[node.attr])
            elif isinstance(node, ast.Constant) and isinstance(node.value, str) and id(node) not in skip:
                if re.search(r"FILE_NAME|IFCOWNERHISTORY", node.value, re.I):
                    found.add("export-time")
            if isinstance(node, ast.Call):
                called = ast.unparse(node.func)
                first = node.args[0] if node.args else None
                if called in ("os.environ.get", "os.getenv", "os.environ.setdefault", "os.environ.pop"):
                    env.add(first.value if isinstance(first, ast.Constant) and isinstance(first.value, str) else None)
                    if called.startswith("os.environ."):
                        handled.add(id(node.func.value))
                elif called in ("os.path.dirname", "os.path.split"):
                    if first is None or ast.unparse(first) not in OWN_DIRECTORY:
                        found.add("dirname")
                elif called == "open" and isinstance(first, ast.Constant) and isinstance(first.value, str):
                    found.add("literal-path")
            elif isinstance(node, ast.Subscript) and ast.unparse(node.value) == "os.environ":
                key = node.slice
                env.add(key.value if isinstance(key, ast.Constant) and isinstance(key.value, str) else None)
                handled.add(id(node.value))
        for node in ast.walk(tree):
            if isinstance(node, ast.Attribute) and ast.unparse(node) == "os.environ" and id(node) not in handled:
                env.add(None)
        kinds |= found - ALLOWED_IN.get(os.path.basename(path)[:-3], set())
    return kinds, env


def closure_digest(trees):
    parts = []
    for path in sorted(trees):
        with open(path, "rb") as handle:
            parts.append(f"{os.path.relpath(path, os.path.dirname(CI_DIR))} {sha(handle.read())}")
    return sha("\n".join(parts).encode())


# ---- files a scorer reads that its arguments do not name --------------------


def declared_siblings(script, trees, ref):
    """unfilled_openings_vs_ifc's `siblings(path)`: the other Revit exports in
    the reference export's directory that share its first four words."""
    tree = trees[os.path.abspath(script)]
    for node in tree.body:
        if isinstance(node, ast.FunctionDef) and node.name == "siblings":
            namespace = {"os": os}
            exec(compile(ast.Module([node], []), script, "exec"), namespace)
            return namespace["siblings"](ref)
    raise Uncacheable("the scorer no longer has the siblings() Measure's cache reads")


# scorer -> (what it reads beside its arguments, found from the file arguments)
EXTRA_INPUTS = {
    # The export's diagnostics, beside the IFC (args[0]).
    "qtos_vs_ifc": lambda script, trees, files: [os.path.join(os.path.dirname(os.path.abspath(files[0])), "diagnostics.json")],
    "unfilled_openings_vs_ifc": lambda script, trees, files: declared_siblings(script, trees, files[1]),
}


# ---- the key ----------------------------------------------------------------


def fingerprint():
    """The interpreter and the libraries the scorers run on, worked out once a
    run: Python, the platform, requirements-ifc.txt and the installed version of
    every package it pins, and the versions IfcOpenShell and numpy report."""
    remembered = os.path.join(work_dir(), "fingerprint.json")
    if os.path.exists(remembered):
        with open(remembered, encoding="utf-8") as handle:
            found = json.load(handle)
    else:
        packages = []
        with open(REQUIREMENTS, "rb") as handle:
            requirements = handle.read()
        for line in requirements.decode("utf-8").splitlines():
            match = re.match(r"^([A-Za-z0-9_.\-]+)==", line)
            if match:
                try:
                    packages.append((match.group(1), importlib.metadata.version(match.group(1))))
                except importlib.metadata.PackageNotFoundError:
                    packages.append((match.group(1), None))
        reported = {}
        for name, attribute in (("ifcopenshell", "version"), ("numpy", "__version__")):
            try:
                value = getattr(__import__(name), attribute, None)
                reported[name] = value if isinstance(value, str) else None
            except Exception as error:
                reported[name] = f"unavailable: {type(error).__name__}"
        found = {
            "format": FORMAT,
            # The minor release, not the patch: the runner image a job lands on
            # moves the patch (3.12.15 to 3.12.16 in run 41769277902, which cost
            # two of its six shards every hit), and the libraries that decide
            # the scorers' output are pinned by hash in requirements-ifc.txt.
            "python": f"{sys.version_info.major}.{sys.version_info.minor}",
            "platform": [platform.system(), platform.machine(), *platform.libc_ver()],
            "requirements": sha(requirements),
            "packages": packages,
            "reported": reported,
        }
        atomic_write(remembered, json.dumps(found, sort_keys=True).encode("utf-8"))
    return {**found, "ifc_flags": os.environ.get("RVT_IFC_FLAGS", "")}


def make_key(args, script, script_args):
    outputs = {os.path.abspath(path) for path in args.extra}
    if os.path.abspath(args.dest) in outputs:
        raise Uncacheable("the output is also an input")
    directory = os.path.abspath(args.dir)
    trees = closure(script)
    kinds, names = audit(trees)
    stem = os.path.basename(script)[:-3]
    declared = EXTRA_INPUTS.get(stem)
    if declared:
        kinds.discard("dirname")
        kinds.discard("fs-listing")
    if None in names:
        kinds.add("computed-env")
    if kinds:
        raise Uncacheable("reads " + ", ".join(sorted(kinds)))
    tokens = []
    files = []
    for arg in script_args:
        full = os.path.abspath(arg)
        if full in outputs:
            tokens.append(["output", arg])
        elif os.path.isfile(arg):
            own = os.path.dirname(full) == directory
            tokens.append(["file", arg, file_digest(arg, own and arg.endswith(".ifc"))])
            files.append(arg)
        else:
            tokens.append(["literal", arg])
    extra = []
    if declared:
        for path in declared(script, trees, files):
            extra.append([os.path.basename(path), file_digest(path, False) if os.path.isfile(path) else None])
    environment = sorted((name, os.environ.get(name)) for name in names if name not in NEUTRAL_ENV)
    parts = [fingerprint(), args.step, script, closure_digest(trees), environment, tokens, extra]
    return sha(json.dumps(parts, sort_keys=True).encode())


def keyfile(args):
    return os.path.join(work_dir(), "keys", sha(f"{os.path.abspath(args.dir)} {args.step}".encode()))


def entry_path(root, key):
    return os.path.join(root, key[:2], key)


# ---- commands ---------------------------------------------------------------


def lookup(args):
    root = cache_dir()
    label = f"{os.path.basename(os.path.abspath(args.dir))} {args.step}"
    if not root:
        return 2
    if is_held_out(os.path.basename(os.path.abspath(args.dir))):
        log(f"not cached {label}: held out, only its counts leave the runner")
        return 2
    script, *script_args = args.command
    try:
        key = make_key(args, script, script_args)
    except Uncacheable as why:
        log(f"not cached {label}: {why}")
        return 2
    os.makedirs(os.path.dirname(keyfile(args)), exist_ok=True)
    atomic_write(keyfile(args), key.encode("ascii"))
    entry = entry_path(root, key)
    if os.path.isfile(os.path.join(entry, "out")):
        shutil.copyfile(os.path.join(entry, "out"), args.dest)
        for path in args.extra:
            kept = os.path.join(entry, "extra-" + os.path.basename(path))
            if os.path.isfile(kept):
                shutil.copyfile(kept, path)
        stale = time.time() - os.stat(entry).st_mtime > STALE_SECONDS
        os.utime(entry)
        with open(os.path.join(args.dir, "timings.log"), "a", encoding="utf-8") as timings:
            timings.write(f"{args.step} cached\n")
        log(f"hit {label} {key[:12]}{' stale' if stale else ''}")
        return 0
    log(f"miss {label} {key[:12]}")
    return 1


def store(args):
    root = cache_dir()
    label = f"{os.path.basename(os.path.abspath(args.dir))} {args.step}"
    if not root or is_held_out(os.path.basename(os.path.abspath(args.dir))) or not os.path.exists(keyfile(args)):
        return 0
    with open(keyfile(args), encoding="ascii") as handle:
        key = handle.read()
    with open(args.dest, "rb") as handle:
        out = handle.read()
    status = args.status
    if status is None:
        last = out.rstrip(b"\n").rsplit(b"\n", 1)[-1]
        status = int(last[5:]) if last.startswith(b"exit ") and last[5:].isdigit() else -1
    text = out.decode("utf-8", "replace")
    if status != 0:
        log(f"not kept {label} {key[:12]}: status {status}")
        return 0
    if len(out) > MAX_ENTRY_BYTES or any(mark in text for mark in TRANSIENT):
        log(f"not kept {label} {key[:12]}: output too large or shows a failure to repeat")
        return 0
    entry = entry_path(root, key)
    if os.path.isdir(entry):
        return 0
    scratch = os.path.join(root, f".new-{os.getpid()}-{time.time_ns()}")
    os.makedirs(scratch)
    shutil.copyfile(args.dest, os.path.join(scratch, "out"))
    for path in args.extra:
        if os.path.isfile(path):
            shutil.copyfile(path, os.path.join(scratch, "extra-" + os.path.basename(path)))
    with open(os.path.join(scratch, "meta.json"), "w", encoding="utf-8") as meta:
        json.dump({"model": os.path.basename(os.path.abspath(args.dir)), "step": args.step}, meta)
    os.makedirs(os.path.dirname(entry), exist_ok=True)
    try:
        os.rename(scratch, entry)
    except OSError:
        shutil.rmtree(scratch, ignore_errors=True)
        return 0
    log(f"kept {label} {key[:12]} {len(out)} bytes")
    return 0


def entries(root):
    for shard in sorted(os.listdir(root)):
        shard_path = os.path.join(root, shard)
        if len(shard) == 2 and os.path.isdir(shard_path):
            for name in sorted(os.listdir(shard_path)):
                path = os.path.join(shard_path, name)
                if os.path.isdir(path):
                    yield path


def size_of(path):
    return sum(os.path.getsize(os.path.join(base, name)) for base, _, names in os.walk(path) for name in names)


def finish(args):
    root = cache_dir()
    report = os.path.join(work_dir(), "report.log")
    lines = []
    if os.path.exists(report):
        with open(report, encoding="utf-8") as handle:
            lines = handle.read().splitlines()
    count = {kind: sum(line.startswith(kind + " ") for line in lines) for kind in ("hit", "miss", "kept", "not cached", "not kept")}
    stale = sum(line.startswith("hit ") and line.endswith(" stale") for line in lines)
    pruned = 0
    total = 0
    kept_entries = 0
    held = 0
    unknown = 0
    by_model = collections.Counter()
    if root:
        now = time.time()
        survivors = []
        for path in list(entries(root)):
            model = None
            try:
                with open(os.path.join(path, "meta.json"), encoding="utf-8") as meta:
                    model = json.load(meta)["model"]
            except (OSError, ValueError, KeyError):
                pass
            # An entry that does not name its model, or names a held-out one,
            # is removed: nothing of those models is kept.
            if model is None:
                shutil.rmtree(path)
                unknown += 1
            elif is_held_out(model):
                shutil.rmtree(path)
                held += 1
            elif now - os.stat(path).st_mtime > args.days * 86400:
                shutil.rmtree(path)
                pruned += 1
            else:
                survivors.append((os.stat(path).st_mtime, path, size_of(path)))
                by_model[model.split("-")[0]] += 1
        survivors.sort()
        total = sum(size for _, _, size in survivors)
        while survivors and total > args.max_mb * 1024 * 1024:
            _, path, size = survivors.pop(0)
            shutil.rmtree(path)
            total -= size
            pruned += 1
        kept_entries = len(survivors)
        for shard in os.listdir(root):
            path = os.path.join(root, shard)
            if os.path.isdir(path) and not os.listdir(path):
                os.rmdir(path)
            elif shard.startswith(".new-"):
                shutil.rmtree(path, ignore_errors=True)
    changed = count["kept"] > 0 or pruned > 0 or stale > 0 or held > 0 or unknown > 0
    summary = (
        f"score cache: {count['hit']} hits, {count['miss']} misses ({count['kept']} kept), "
        f"{count['not cached']} not cached; {kept_entries} entries, {total / 1048576:.1f} MiB; "
        f"{pruned} pruned, {stale} refreshed; changed={str(changed).lower()}\n"
        f"score cache entries by model prefix: {', '.join(f'{name} {n}' for name, n in sorted(by_model.items())) or 'none'}; "
        f"held out ({', '.join(HELD_OUT)}): {held} found and removed; without a model name: {unknown} removed"
    )
    print(summary)
    if args.report:
        with open(args.report, "w", encoding="utf-8") as out:
            out.write("\n".join(lines) + "\n" + summary + "\n")
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as handle:
            handle.write(f"changed={str(changed).lower()}\n")
    return 0


def main(argv):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command_name", required=True)
    for name in ("lookup", "store"):
        p = sub.add_parser(name)
        p.add_argument("--step", required=True)
        p.add_argument("--dir", required=True)
        p.add_argument("--dest", required=True)
        p.add_argument("--extra", action="append", default=[])
        if name == "store":
            p.add_argument("--status", type=int)
        else:
            p.add_argument("command", nargs="+")
    p = sub.add_parser("finish")
    p.add_argument("--days", type=int, default=21)
    p.add_argument("--max-mb", type=int, default=256)
    p.add_argument("--report")
    args = parser.parse_args(argv)
    if args.command_name == "finish":
        return finish(args)
    # Whatever goes wrong with the cache, the step runs as it always did: a
    # lookup that fails says "not cached" (2), never "miss" (1), and a store
    # that fails keeps nothing.
    failed = 2 if args.command_name == "lookup" else 0
    try:
        if args.command_name == "lookup":
            os.remove(keyfile(args))
    except OSError:
        pass
    try:
        return {"lookup": lookup, "store": store}[args.command_name](args)
    except Exception as error:
        log(f"not cached {os.path.basename(os.path.abspath(args.dir))} {args.step}: {args.command_name} failed, {type(error).__name__}: {error}")
        return failed


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
