"""The room fallback through the Python API (#1359), on Autodesk's
``rac_basic_sample_project`` (Revit 2018).

Its 14 rooms are in Working Drawings, the second of three phases (Existing,
Working Drawings, Learning Content), and the last phase holds none. A call with
no phase writes the rooms of Working Drawings and raises a ``RuntimeWarning``
that names it; a phase named, the last included, is exact. This is the Python
twin of the Measure runs of RE-326 section 9.4 (the ``rvt-ifc`` side), and the
only place the warnings run against a real file. It runs where
``RVT_SAMPLE_PROJECTS_DIR`` holds ``2018-rac_basic_sample_project.rvt`` (the
wheel job fetches it, pinned by sha256 as ``research/autodesk-sample-projects.tsv``
pins it) and skips elsewhere.
"""

from __future__ import annotations

import os
import pathlib
import warnings

import pytest

import rvt  # type: ignore

_DIR = os.environ.get("RVT_SAMPLE_PROJECTS_DIR")
RAC = pathlib.Path(_DIR) / "2018-rac_basic_sample_project.rvt" if _DIR else None

pytestmark = pytest.mark.skipif(
    RAC is None or not RAC.is_file(),
    reason="RVT_SAMPLE_PROJECTS_DIR does not hold 2018-rac_basic_sample_project.rvt",
)

LAST = "Learning Content"
LAST_ID = 961604
WORKING_DRAWINGS = "Working Drawings"
WORKING_DRAWINGS_ID = 86961


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


def fallback_warnings(caught: list) -> list:
    return [
        w
        for w in caught
        if issubclass(w.category, RuntimeWarning)
        and "holds no room" in str(w.message)
    ]


@pytest.fixture
def rac() -> rvt.RevitFile:
    return rvt.RevitFile(str(RAC))


def test_the_default_export_writes_the_rooms_of_working_drawings_and_warns(rac):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        ifc = rac.write_ifc()
    assert spaces(ifc) == 14
    ours = fallback_warnings(caught)
    assert len(ours) == 1, [str(w.message) for w in caught]
    message = str(ours[0].message)
    assert WORKING_DRAWINGS in message
    assert "14" in message
    assert "phase=" in message


def test_the_diagnostics_name_the_phase_and_count_the_rooms(rac):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        diagnostics = rac.export_diagnostics()
    assert diagnostics["room_phase_fallback"] == {
        "phase": WORKING_DRAWINGS,
        "rooms": 14,
    }
    assert left_out_rooms(diagnostics) == []
    assert len(fallback_warnings(caught)) == 1


def test_naming_the_last_phase_is_exact(rac):
    for phase in (LAST, str(LAST_ID), LAST_ID):
        with warnings.catch_warnings(record=True) as caught:
            warnings.simplefilter("always")
            ifc = rac.write_ifc(phase=phase)
            diagnostics = rac.export_diagnostics(phase=phase)
        assert spaces(ifc) == 0, repr(phase)
        assert "room_phase_fallback" not in diagnostics, repr(phase)
        assert fallback_warnings(caught) == [], repr(phase)
        items = left_out_rooms(diagnostics)
        assert len(items) == 1, items
        assert items[0]["count"] == 14
        assert items[0]["phase"] == WORKING_DRAWINGS


def test_the_default_is_the_working_drawings_export(rac):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        default = body(rac.write_ifc())
    named = rac.write_ifc(phase=WORKING_DRAWINGS)
    assert body(named) == default
    assert body(rac.write_ifc(phase=WORKING_DRAWINGS_ID)) == default
    assert "room_phase_fallback" not in rac.export_diagnostics(phase=WORKING_DRAWINGS)


def test_a_call_with_no_phase_goes_back_to_the_fallback(rac):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        default = body(rac.write_ifc())
        assert spaces(rac.write_ifc(phase=LAST)) == 0
        assert body(rac.write_ifc()) == default
        assert spaces(rac.write_ifc(phase=LAST_ID)) == 0
        assert body(rvt.rvt_to_ifc(str(RAC))) == default


def test_the_reads_hold_the_rooms_and_warn(rac):
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        elements = rac.decoded_elements()
    assert sum(1 for e in elements if e["class_name"] == "Room") == 14
    ours = fallback_warnings(caught)
    assert len(ours) == 1, [str(w.message) for w in caught]
    assert "phase=" in str(ours[0].message)

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        counts = rac.element_counts()
    assert counts["by_class"]["Room"] == 14
    assert len(fallback_warnings(caught)) == 1


def test_the_reads_follow_the_phase_an_export_named(rac):
    with warnings.catch_warnings():
        warnings.simplefilter("ignore")
        rac.write_ifc(phase=LAST)
        assert "Room" not in rac.element_counts()["by_class"]
        rac.write_ifc()
        assert rac.element_counts()["by_class"]["Room"] == 14


def test_the_room_schedule_lists_the_rooms_and_warns(rac):
    def rows(csv: str) -> int:
        return len(csv.strip().splitlines())

    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        assert rows(rac.schedule_csv(kind="rooms")) == 15
    assert len(fallback_warnings(caught)) == 1
    with warnings.catch_warnings(record=True) as caught:
        warnings.simplefilter("always")
        assert rows(rac.schedule_csv(kind="rooms", phase=LAST)) == 1
    assert fallback_warnings(caught) == []
