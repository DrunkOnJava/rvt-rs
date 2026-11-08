# RE-372: The export phase on Revit 2015

**Date:** 2026-11-08
**Issues:** #1332, #879; follows RE-184, RE-315, RE-326 and RE-368 section 5.
**Status:** in progress. The rule is applied on this branch and not yet measured; this report gets its figures from the Measure run on the head that lands.

## 1. The change

`PHASE_FILTER_RELEASES` (2015 to 2023) gates the export phase rules, which were on 2016 to 2023 through `EXPORT_FILTER_RELEASES`. Design options stay on 2016 to 2023: their layout is not read on 2015.

## 2. What is expected, to be measured

BIMcollab's Revit 2015 structural example is the one Revit 2015 export with phases. RE-368 section 5 found that the 3 air terminals #879 counts as written by rvt-rs and left out by Revit's export are demolished before the file's last phase. The measurement asks whether those 3 go, and nothing else of its 241 elements does.
