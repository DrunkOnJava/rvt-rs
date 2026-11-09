- **Revit 2015 files follow the export phase (#1332, RE-372).** An element that does
  not exist in the export phase and a room whose phase is not the export phase are
  left out, as on 2016 and later, and `--phase` is accepted on 2015. On one Revit 2015
  project with an export (counts only), the 3 air terminals Revit's export leaves out
  are gone and its 241 elements stay. Design options stay 2016 and later.
