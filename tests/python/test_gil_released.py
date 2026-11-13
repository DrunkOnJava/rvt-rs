"""The calls that decode a model let other Python threads run (#1884), on Core Interior.

Its export takes seconds. A call that held the GIL for them would leave a
second thread without a turn for as long: a thread that wakes every
millisecond would show one gap between two wakes as long as the call. So each
call below runs while such a thread ticks, and the longest gap between two
ticks inside the call must be a small part of the call's time. It runs where
``RVT_PROJECT_CORPUS_DIR`` holds ``2024_Core_Interior.rvt`` and skips
elsewhere, as a fast file cannot tell a held GIL from a short call.
"""

from __future__ import annotations

import os
import pathlib
import threading
import time
from typing import Callable

import pytest

import rvt  # type: ignore

_DIR = os.environ.get("RVT_PROJECT_CORPUS_DIR")
CORE = pathlib.Path(_DIR) / "2024_Core_Interior.rvt" if _DIR else None

pytestmark = pytest.mark.skipif(
    CORE is None or not CORE.is_file(),
    reason="RVT_PROJECT_CORPUS_DIR does not hold 2024_Core_Interior.rvt",
)

# The call must take this long for the gap to mean something.
LONG_ENOUGH = 1.0


def took_and_longest_gap(call: Callable[[], object]) -> tuple[float, float]:
    """Seconds ``call`` took, and the longest time a ticking thread went without running inside it."""
    stop = threading.Event()
    ticks: list[float] = []

    def tick() -> None:
        while not stop.is_set():
            ticks.append(time.perf_counter())
            time.sleep(0.001)

    ticker = threading.Thread(target=tick)
    ticker.start()
    time.sleep(0.05)
    started = time.perf_counter()
    call()
    finished = time.perf_counter()
    stop.set()
    ticker.join()
    points = [started] + [t for t in ticks if started <= t <= finished] + [finished]
    return finished - started, max(b - a for a, b in zip(points, points[1:]))


# The two calls whose result is one string: a call that returns a million
# dicts holds the GIL, by necessity, to build them, and would show that as a gap.
@pytest.mark.parametrize("method", ["write_ifc", "export_diagnostics_json"])
def test_a_decode_lets_other_threads_run(method: str) -> None:
    handle = rvt.RevitFile(str(CORE))
    took, gap = took_and_longest_gap(getattr(handle, method))
    if took < LONG_ENOUGH:
        pytest.skip(f"{method} took {took:.2f} s, too short to tell a held GIL from a short call")
    assert gap < took / 4, f"{method} ran {took:.2f} s and a thread ticking each millisecond waited {gap:.2f} s"


def test_opening_a_file_lets_other_threads_run() -> None:
    took, gap = took_and_longest_gap(lambda: rvt.RevitFile(str(CORE)))
    if took < 0.2:
        pytest.skip(f"opening took {took:.2f} s, too short to tell a held GIL from a short call")
    assert gap < took / 4, f"opening ran {took:.2f} s and a thread ticking each millisecond waited {gap:.2f} s"
