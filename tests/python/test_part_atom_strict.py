"""Integration tests of `RevitFile.part_atom_json_strict` (#502) on real
files.

`part_atom_json()` returns `None` when the `PartAtom` stream is missing or
does not parse, so a caller cannot tell the two apart. The strict variant
returns the same JSON when the stream reads, and raises `ValueError` naming
the cause when it does not.

Uses the tier-1 fixture `corpus/tier1/architectural-2024`, which has a
`PartAtom` stream, and a copy of it whose `PartAtom` directory entry is
renamed, so the stream is absent.
"""
from __future__ import annotations

import json
import pathlib

import pytest

import rvt  # type: ignore

REPO = pathlib.Path(__file__).resolve().parents[2]
TIER1 = REPO / "corpus" / "tier1" / "architectural-2024" / "architectural-2024.rvt"


def test_strict_part_atom_is_the_lenient_one_when_the_stream_reads():
    f = rvt.RevitFile(str(TIER1))
    lenient = f.part_atom_json()
    assert lenient is not None, "the tier-1 fixture has a PartAtom stream"
    assert json.loads(f.part_atom_json_strict()) == json.loads(lenient)


def test_strict_part_atom_names_a_missing_stream(tmp_path):
    data = bytearray(TIER1.read_bytes())
    name = "PartAtom".encode("utf-16-le")
    at = data.find(name)
    assert at >= 0 and data.find(name, at + 1) < 0, "one PartAtom directory entry"
    # Rename the entry's last character: PartAtom -> PartAtoN.
    data[at + len(name) - 2 : at + len(name)] = "N".encode("utf-16-le")
    copy = tmp_path / "no-part-atom.rvt"
    copy.write_bytes(bytes(data))
    f = rvt.RevitFile(str(copy))
    assert f.part_atom_json() is None
    with pytest.raises(ValueError, match="Stream not found: PartAtom"):
        f.part_atom_json_strict()
