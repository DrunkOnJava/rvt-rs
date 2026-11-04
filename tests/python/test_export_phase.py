"""The export phase through the Python API (#1339), on RE1 Architecture.

RE1 Architecture (Revit 2025, Drshelden/IFC-ECS, MIT) has two phases, Existing
(ElementId 12589) and New Construction (86961, the last), and its 11 rooms sit
in the second. This is the Python twin of
``ifc_phase_option_chooses_the_export_phase_of_a_real_file`` in
tests/cli_real_files.rs: the last phase, named or by ElementId, is the default
export; Existing leaves the 11 rooms out and counts them; a phase the file
lacks is an error that lists the file's phases. It runs where
``RVT_PROJECT_CORPUS_DIR`` holds ``RE1-Architecture.rvt`` and skips elsewhere.
"""

from __future__ import annotations

import json
import os
import pathlib

import pytest

import rvt  # type: ignore

_DIR = os.environ.get("RVT_PROJECT_CORPUS_DIR")
RE1 = pathlib.Path(_DIR) / "RE1-Architecture.rvt" if _DIR else None

pytestmark = pytest.mark.skipif(
    RE1 is None or not RE1.is_file(),
    reason="RVT_PROJECT_CORPUS_DIR does not hold RE1-Architecture.rvt",
)

LAST = "New Construction"
LAST_ID = 86961
FIRST = "Existing"
FIRST_ID = 12589


def body(ifc: str) -> str:
    """The STEP text without the two places the wall clock is written."""
    return "\n".join(
        line
        for line in ifc.splitlines()
        if "FILE_NAME(" not in line and "IFCOWNERHISTORY(" not in line
    )


def spaces(ifc: str) -> int:
    return ifc.count("=IFCSPACE(")


def left_out_rooms(diagnostics: dict) -> list[dict]:
    return [
        item
        for item in diagnostics["skipped"]
        if item["reason"] == "room_not_in_export_phase"
    ]


@pytest.fixture
def re1() -> rvt.RevitFile:
    return rvt.RevitFile(str(RE1))


def test_the_default_export_is_the_last_phase_with_every_room(re1):
    ifc = re1.write_ifc()
    assert spaces(ifc) == 11
    assert left_out_rooms(re1.export_diagnostics()) == []


def test_the_last_phase_by_name_or_by_id_is_the_default_export(re1):
    default = body(re1.write_ifc())
    for phase in (LAST, str(LAST_ID), LAST_ID):
        assert body(re1.write_ifc(phase=phase)) == default, repr(phase)
    assert body(rvt.rvt_to_ifc(str(RE1), phase=LAST_ID)) == default


def test_the_first_phase_leaves_out_the_rooms_that_sit_in_the_second(re1):
    for phase in (FIRST, FIRST_ID):
        ifc = re1.write_ifc(phase=phase)
        assert spaces(ifc) == 0, repr(phase)
        items = left_out_rooms(re1.export_diagnostics(phase=phase))
        assert len(items) == 1, items
        assert items[0]["count"] == 11
        assert items[0]["classes"]["Room"] == 11
        assert items[0]["phase"] == LAST
    one_shot = json.loads(rvt.rvt_to_ifc_diagnostics(str(RE1), phase=FIRST))
    assert left_out_rooms(one_shot)[0]["count"] == 11
    assert spaces(rvt.rvt_to_ifc(str(RE1), phase=FIRST)) == 0
    assert left_out_rooms(json.loads(re1.export_diagnostics_json(phase=FIRST)))


def test_one_handle_exports_the_phases_in_turn(re1):
    fresh_default = body(rvt.rvt_to_ifc(str(RE1)))
    fresh_first = body(rvt.rvt_to_ifc(str(RE1), phase=FIRST))
    assert body(re1.write_ifc()) == fresh_default
    assert body(re1.write_ifc(phase=FIRST)) == fresh_first
    # No phase after one was named is the last again, not the handle's
    # earlier choice.
    assert body(re1.write_ifc()) == fresh_default
    assert body(re1.write_ifc(phase=FIRST_ID)) == fresh_first
    assert body(re1.write_ifc(phase=LAST)) == fresh_default


def test_the_room_schedule_follows_the_phase(re1):
    def rows(csv: str) -> int:
        return len(csv.strip().splitlines())

    assert rows(re1.schedule_csv(kind="rooms")) == 12
    assert rows(re1.schedule_csv(kind="rooms", phase=FIRST)) == 1
    assert rows(re1.schedule_csv(kind="rooms")) == 12


def test_a_phase_the_file_lacks_is_an_error_that_lists_its_phases(re1):
    with pytest.raises(ValueError) as error:
        re1.write_ifc(phase="No such phase")
    assert FIRST in str(error.value) and LAST in str(error.value)
    with pytest.raises(ValueError):
        re1.export_diagnostics(phase=12345)
    with pytest.raises(ValueError):
        rvt.rvt_to_ifc(str(RE1), phase="No such phase")
    # The handle still exports after the refusal.
    assert spaces(re1.write_ifc()) == 11


def test_a_phase_is_a_name_an_id_or_none(re1):
    with pytest.raises(TypeError):
        re1.write_ifc(phase=True)
    with pytest.raises(TypeError):
        re1.write_ifc(phase=1.5)
    assert spaces(re1.write_ifc(phase=None)) == 11
