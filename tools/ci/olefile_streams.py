#!/usr/bin/env python3
"""Compare rvt-rs's reading of a Revit file's OLE container with olefile's (#1938).

The container is the one layer every other reader stands on, and the only reader of it the tests
use is rvt-rs's own (the `cfb` crate, through `reader::RevitFile`). olefile is a second reader of
the same format, written separately and in another language. For each Revit file given, this lists
the file's streams with olefile and reads them, runs `rvt-dump --raw` on the same file, and holds
the two readings to each other:

  - the streams: the same names, as many of them as rvt-dump says it dumps, and the same stored
    size for each (rvt-dump prints `raw=<bytes>`, what `RevitFile::read_stream` returned);
  - the bytes: the stored bytes of each stream, which rvt-dump writes beside its decoded ones, are
    the bytes olefile reads (compared as the multiset of (size, sha256) of the streams, so that
    rvt-dump's own file names, which it makes unique and safe, take no part).

Storages are listed by olefile and not by rvt-rs, whose reader lists streams: they are counted and
not compared. A path with no Revit file, a file olefile cannot open, an rvt-dump that fails, and a
run that compared no stream all fail: a check that compared nothing is not a pass.

    python3 tools/ci/olefile_streams.py <rvt-dump> <file or directory>...

A directory is searched for `.rvt`, `.rfa`, `.rte` and `.rft` files. Needs olefile
(tools/ci/requirements-olefile.txt). Exit 0 when every file agrees, 1 otherwise.
"""

import argparse
import collections
import hashlib
import re
import subprocess
import sys
import tempfile
from pathlib import Path

import olefile

REVIT_SUFFIXES = {".rvt", ".rfa", ".rte", ".rft"}

# rvt-dump prints `Dumping <n> streams from <path>`, then for each stream it read `  <name, padded
# to 30>  raw=<bytes> bytes ...`, and for one over its stream limit `  <name>  (not read: ...)`.
HEADER = re.compile(r"^Dumping (?P<count>[0-9]+) streams from ")
STREAM = re.compile(r"^  (?P<name>.+?) {2,}raw=(?P<size>[0-9]+) bytes")

# How long rvt-dump may take on one file, seconds. The largest corpus file takes a few.
DUMP_TIMEOUT = 600


def rust_escape_debug(text):
    """What Rust's `str::escape_debug` makes of `text`: rvt-dump prints each stream name with it."""
    simple = {"\0": "\\0", "\t": "\\t", "\r": "\\r", "\n": "\\n", "\\": "\\\\", '"': '\\"', "'": "\\'"}
    return "".join(
        simple[ch] if ch in simple else ch if ch.isprintable() else "\\u{%x}" % ord(ch) for ch in text
    )


def revit_files(argument):
    """The Revit files `argument` names: itself, or those under it."""
    path = Path(argument)
    if path.is_file():
        return [path]
    if not path.is_dir():
        return []
    return sorted(
        file
        for file in path.rglob("*")
        if file.is_file() and file.suffix.lower() in REVIT_SUFFIXES and ".git" not in file.parts
    )


def olefile_reading(path):
    """(name -> (stored size, sha256 of its bytes), the storages olefile lists) of the file."""
    streams = {}
    with olefile.OleFileIO(str(path)) as ole:
        for entry in ole.listdir(streams=True, storages=False):
            data = ole.openstream(entry).read()
            size = ole.get_size(entry)
            if len(data) != size:
                raise ValueError(f"stream {entry!r}: the directory says {size} bytes and olefile read {len(data)}")
            streams["/".join(entry)] = (size, hashlib.sha256(data).hexdigest())
        storages = ["/".join(entry) for entry in ole.listdir(streams=False, storages=True)]
    return streams, storages


def rvt_reading(rvt_dump, path):
    """(announced stream count, name -> stored size, multiset of (size, sha256) of the raw dumps) from rvt-dump."""
    with tempfile.TemporaryDirectory() as out:
        run = subprocess.run(
            [rvt_dump, str(path), "-o", out, "--raw"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=DUMP_TIMEOUT,
        )
        if run.returncode != 0:
            raise ValueError(f"rvt-dump exited {run.returncode}: {run.stderr.strip()[:400]}")
        announced, sizes = None, {}
        for line in run.stdout.splitlines():
            header = HEADER.match(line)
            if header:
                announced = int(header["count"])
                continue
            stream = STREAM.match(line)
            if stream:
                sizes[stream["name"]] = int(stream["size"])
        raw = collections.Counter()
        for dump in Path(out).glob("*.raw"):
            data = dump.read_bytes()
            raw[(len(data), hashlib.sha256(data).hexdigest())] += 1
    return announced, sizes, raw


def differences(path, rvt_dump):
    """Every way the two readings of the file differ, and (streams, stored bytes, storages) compared."""
    try:
        theirs, storages = olefile_reading(path)
    except Exception as error:  # olefile raises its own and OSError kinds for a file it cannot read
        return [f"olefile could not read it: {type(error).__name__}: {error}"], (0, 0, 0)
    try:
        announced, ours, raw = rvt_reading(rvt_dump, path)
    except (ValueError, OSError, subprocess.TimeoutExpired) as error:
        return [f"rvt-dump failed: {error}"], (0, 0, 0)
    problems = []
    named = {rust_escape_debug(name): value for name, value in theirs.items()}
    if len(named) != len(theirs):
        problems.append("two of olefile's stream names print alike")
    if announced != len(ours):
        problems.append(f"rvt-dump announced {announced} streams and listed {len(ours)}")
    for name in sorted(set(named) - set(ours)):
        problems.append(f"olefile lists the stream {name!r} and rvt-dump does not")
    for name in sorted(set(ours) - set(named)):
        problems.append(f"rvt-dump lists the stream {name!r} and olefile does not")
    for name in sorted(set(named) & set(ours)):
        if named[name][0] != ours[name]:
            problems.append(f"stream {name!r}: olefile says {named[name][0]} bytes, rvt-dump {ours[name]}")
    wanted = collections.Counter(value for value in theirs.values())
    if raw != wanted:
        missing = wanted - raw
        extra = raw - wanted
        problems.append(
            f"the stored bytes differ: {sum(missing.values())} of olefile's streams are not among rvt-dump's"
            f" raw dumps and {sum(extra.values())} of rvt-dump's are not among olefile's"
        )
    return problems, (len(theirs), sum(size for size, _ in theirs.values()), len(storages))


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("rvt_dump", help="the rvt-dump binary")
    parser.add_argument("paths", nargs="+", help="Revit files, or directories to search for them")
    arguments = parser.parse_args()

    failed = 0
    files = streams = stored = storages = 0
    for argument in arguments.paths:
        found = revit_files(argument)
        if not found:
            print(f"FAIL {argument}: no Revit file")
            failed += 1
            continue
        for path in found:
            problems, (count, size, kinds) = differences(path, arguments.rvt_dump)
            shown = path.as_posix()
            if problems:
                failed += 1
                print(f"FAIL {shown}")
                for problem in problems:
                    print(f"       {problem}")
                continue
            files += 1
            streams += count
            stored += size
            storages += kinds
            print(f"ok   {shown}: {count} streams, {size:,} stored bytes, {kinds} storages not compared")
    print(
        f"olefile_streams: compared {files} files, {streams} streams, {stored:,} stored bytes"
        f" (names, sizes and bytes); {storages} storages listed by olefile and not compared; {failed} failed"
    )
    if streams == 0 and not failed:
        print("olefile_streams: no stream was compared, which is not a pass")
        return 1
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
