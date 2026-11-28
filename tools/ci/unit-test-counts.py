#!/usr/bin/env python3
"""Hold the number of unit tests in each source file to the list in unit-test-counts.txt.

Unit tests are banned in this repository except a few for a pure helper (CONTRIBUTING.md,
"Tests"). On 2026-11-27 every one of them was read against that rule, and Griffin chose to
freeze the count there: a file that gains a unit test, or a new file that holds one, fails
this check until the change raises its number here on purpose, so the exception cannot
grow unread. A file whose count falls fails too, so the list stays exact. A raise needs
another agent's approval and the advisor's sign-off before it lands (Griffin, 2026-11-27).

A unit test is a `#[test]` attribute, or a namespaced one such as `#[tokio::test]` (at the
start of a line, whatever follows it), in a
tracked `.rs` file under a crate's `src/` folder, `src/**/tests/` included; the end-to-end
targets in each crate's top-level `tests/` folder are not counted.

Usage: python3 tools/ci/unit-test-counts.py [--write]
"""
import pathlib
import re
import subprocess
import sys

LIST = pathlib.Path(__file__).with_name("unit-test-counts.txt")
# `#[test]` and the namespaced kinds (`#[tokio::test]`, `#[wasm_bindgen_test]`) alike.
TEST = re.compile(r"^\s*#\[(?:[a-z_]+::)?[a-z_]*test\]")


def counted():
    files = subprocess.run(["git", "ls-files", "*.rs"], check=True, capture_output=True, text=True).stdout.split()
    out = {}
    for path in files:
        if "src" not in pathlib.PurePosixPath(path).parts[:-1]:
            continue
        n = sum(1 for line in pathlib.Path(path).read_text(encoding="utf-8").splitlines() if TEST.match(line))
        if n:
            out[path] = n
    return out


def listed():
    out = {}
    for line in LIST.read_text(encoding="utf-8").splitlines():
        line = line.split("#", 1)[0].strip()
        if line:
            count, path = line.split(maxsplit=1)
            out[path] = int(count)
    return out


def main():
    now = counted()
    if sys.argv[1:] == ["--write"]:
        head = "".join(l + "\n" for l in LIST.read_text(encoding="utf-8").splitlines() if l.startswith("#")) if LIST.exists() else ""
        LIST.write_text(head + "".join(f"{n:4d} {path}\n" for path, n in sorted(now.items())), encoding="utf-8")
        print(f"wrote {sum(now.values())} unit tests in {len(now)} files")
        return 0
    want = listed()
    wrong = sorted(set(now) | set(want))
    wrong = [(p, want.get(p, 0), now.get(p, 0)) for p in wrong if want.get(p, 0) != now.get(p, 0)]
    if not wrong:
        print(f"unit tests: {sum(now.values())} in {len(now)} files, as listed")
        return 0
    for path, was, is_ in wrong:
        how = "gained" if is_ > was else "lost"
        print(f"::error file={path}::{path} holds {is_} unit tests and the list says {was}: it {how} {abs(is_ - was)}")
    print("Unit tests are frozen (Griffin, 2026-11-27). A new one must be a pure helper's, tested on the ways it can fail;")
    print("if it is, raise the file's number in tools/ci/unit-test-counts.txt in the same change and say why in the PR;")
    print("a raise lands only with another agent's approval and the advisor's sign-off.")
    return 1


if __name__ == "__main__":
    sys.exit(main())
