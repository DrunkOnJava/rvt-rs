# RE-483: before Revit 2024, whether a door or window's family form with no material is effectively shown (a bound `Visible` read from its parameter) against `<Unnamed>` in Revit's material set (run pending)

**Date:** 2026-11-15
**Issues:** #1603 (Revit 2015 to 2023: the 70 windows of the spent Revit 2018 project lack Revit's materials); follows RE-482 (no category of the host family's forms by their stored `Visible` separates `<Unnamed>` written from not), RE-481, RE-479, RE-204 to RE-206, RE-149.
**Probe and scorer:** `probe_1603_unnamed_bound_before_2024` and `tools/re/window_door_unnamed_bound_vs_ifc.py`. No `src/` change.
**Status:** pending the run. Rows registered on #1603 (comment 6893649932) before it.

## 1. The question

RE-482 read each family's forms by their stored `Visible`. On the spent Revit 2018 project that left 44 of the 45 windows whose only unset form is stored hidden with `<Unnamed>` written, and 8 of 15 with an unbound unset form stored shown; the 8 windows with a bound unset form stored shown have none. A stored `Visible` says nothing of what a form bound to a family parameter shows. Is `<Unnamed>` written where an unset form is effectively shown, the bound parameter's value read on the instance, else on its type?

## 2. What the probe and the scorer count

For each door and window the export writes, the probe prints the facts of its family's forms with no material that are not voids: how many are effectively shown, hidden or unresolved, how many are bound to a family parameter and how many of those resolve. A form's effective visibility is its stored `Visible` where it is not bound, else the value of the bound parameter on the instance, else on its type, matched by the parameter's caption as `instances_without_body` matches it. A parameter with no caption or no value is unresolved. The scorer counts, for the elements whose type map holds geometry mapped to no material and to which Revit gives materials, `<Unnamed>` written or not by the family's category, and prints the reference export's IFC schema and Revit release from its header. Counts only for held-out projects.

## 3. The registered rows

Registered on #1603 before the run (2026-11-15, comment 6893649932).

1. **Reach.** Of the bound unset forms of the Revit 2015 to 2023 doors and windows, how many resolve. Reported.
2. **The spent project.** A rule is worth proposing only if it is right on every one of the 68 windows.
3. **Other pre-2024 projects.** The same cross-tab with effective visibility; a rule that fails there is one project's behaviour and is not written.
4. **The exporter.** The reference export's schema and Revit release beside the written counts.
5. **Nothing moves.** No `src/` change.
