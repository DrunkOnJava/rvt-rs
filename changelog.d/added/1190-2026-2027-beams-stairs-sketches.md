- **Revit 2026 and 2027 projects draw beams along their axes, stair runs
  and landings as their steps and treads, and floors, ceilings and slabs
  from their sketch lines, with shaft voids cut from floors and ceilings
  (#1190, RE-341).** Beams are drawn as on 2024 (on 2025 they are not), runs
  from their native records as on 2025 with the stair property sets and
  quantities read from them, landings as on 2024 (on 2025 they are not), and
  the sketch lines' exact ends, curves and Sketch curve lists are read as on
  2024 and 2025. A roof there is read as before, without its slope. **No
  beam, landing, stair run, sketch outline or shaft void of a 2026 or 2027
  file is scored against a Revit export**; each is held to the same Autodesk
  project's earlier save instead (`tools/re/bodies_across_saves.py`, which
  Measure runs and fails on), and that cannot catch an error the earlier save
  shares. Against Revit's own export only one stair flight is scored, on a
  held-out 2026 model and a held-out 2027 one (property values equal 209 to
  224 of 352 and of 354, quantities 21 to 23 of 322 and 17 to 19 of 367;
  Measure run 41988011289, head 1d5bbf03). Of 111 exports in run
  42004827803 (head f9035152), 102 are unchanged and 9 change, all 2026 or
  2027. In run 42010304828 (head e56d31b5) the check passes on `rst_basic`
  (354 beams, 1 stair run, 5 outlines) and `rac_basic` (2 stair runs, 12
  outlines) and fails on one `rac_advanced` ceiling it required; the head
  after it is not yet read.
