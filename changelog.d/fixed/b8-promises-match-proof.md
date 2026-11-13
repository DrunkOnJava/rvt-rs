- **The support matrix, the status pages and the viewer promise what their
  proof shows.** An audit of every capability in `docs/support-matrix.json`
  and of `README.md`, `docs/compatibility.md`, `ROADMAP.md`, `docs/status.md`
  and the viewer's support text against the tests, reports and code each names
  corrected over 70 statements, and changed no status. Among them: the viewer's
  legend gave 1.0's range as 2015 to 2027 (it is 2024 to 2027) and listed phase
  filtering as not ready (the phase field has it); the roadmap had the Python
  bindings writing glTF and plan SVG (they write IFC4 and CSV); notes still
  said railings, horizontal bracing, roofs by extrusion outside 2025, Areas,
  Floors and Rooms' storeys, connector SystemType and 2008 to 2014 element
  records were not handled, which later work does handle; and functions,
  tests and counts are named as the code and reports now have them. Four
  status questions the audit raised are on #1933.
