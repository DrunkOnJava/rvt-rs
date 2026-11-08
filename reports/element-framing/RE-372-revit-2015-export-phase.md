# RE-372: The export phase on Revit 2015

**Date:** 2026-11-08
**Issues:** #1332, #879; follows RE-184, RE-315, RE-326 and RE-368 section 5.
**Status:** positive, on one Revit 2015 project with an export (held out of the corpus, counts only).

## 1. The change

`PHASE_FILTER_RELEASES` (2015 to 2023) gates the export phase rules, which were on 2016 to 2023 through `EXPORT_FILTER_RELEASES`. Design options stay on 2016 to 2023: their layout is not read on 2015, and Area records keep `applies_export_filters` for the same reason. `RevitFile::set_export_phase` and `rvt-ifc --phase` accept a Revit 2015 file.

## 2. The question

BIMcollab's Revit 2015 structural example is the one Revit 2015 export with phases. RE-368 section 5 found that the 3 air terminals #879 counts as written by rvt-rs and left out by Revit's export are demolished before the file's last phase. Do those 3 go, and nothing else of its 241 elements?

## 3. Measured

Measure run 42519395233, head `f3d8f702`, main's side `d51c6aa5` (`-f base=main -f nordic=true -f r2025=true -f samples=true -f holdout=true -f bimcollab=true -f racadv=true`, every scorer). The `same-project gates` check passes on `f3d8f702`. The landing head differs from that commit by this report, `docs/export-diagnostics.md` and the changelog fragment, and no code.

- **BIMcollab's structural example, against Revit 2015's own export:** elements and spaces paired 244 to 241, and `IfcAirTerminal` written beyond the export 3 to 0: the 3 of #879 go and no other class changes. Revit's export holds 241. Elements on no storey 19 to 16, which are the 16 footings of #879, still open. It is the only model of the run whose scorer output moves.
- **Autodesk's six Revit 2015 samples:** `ifc.sha256` is the same on both sides. On `rac_basic` the rule now applies and the room fallback of #1359 writes the same 14 rooms: `room_phase_fallback` is absent on main's side and names 14 rooms on the head, as on the 2016, 2017 and 2018 saves. The rooms therefore stay, by the rule the later saves follow.
- **Every other model:** no scorer output and no schema census moves.

## 4. What this does not claim

- The rule on a second Revit 2015 project with an export: this is one.
- A 2015 room whose phase is not the export phase and which no fallback brings in: none of the measured files has one with an export to check it.
- `--phase` on a Revit 2015 file against a Revit export: no run names a phase on 2015.
- The rule before Revit 2015.
