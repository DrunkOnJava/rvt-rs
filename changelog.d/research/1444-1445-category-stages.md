- **Rebar, Structural Beam Systems and Furniture Systems stay unread because
  no Revit export holds one, and the 199 elements RvtIO counts beyond rvt-rs
  on `rac_advanced` and `rac_basic` are in no frame of any category (#1444,
  #1445, RE-350).** Measure run 42097940515 (59 models with a Revit export,
  37 Autodesk samples of 2020 to 2027, every probe exit 0) finds the files
  hold them as frames, the same on all eight saves of each sample: 346 `Rebar`
  on `rst_basic` (14 on Snowdon Towers Structural), 16 Structural Beam
  Systems on `rst_basic` (15 with a body) and 1 on `rac_advanced`, 3 placed
  Furniture Systems on `rac_basic`. None of the 59 exports holds an
  `IfcReinforcingBar` or an `IfcSystemFurnitureElement`, no export holds a
  placed Furniture System, and the one export of a project with a beam
  system, the three.js IFC2X3 export of `rac_advanced`, holds neither it nor
  any of the project's 38 framing members, so no category is added. The only
  `IfcElementAssembly` in any export are 3 in each of Vitoria's two 2018
  files, Revit assemblies with no representation (RE-315), which no frame
  of any category carries. Revit's exporter has code for beam systems and
  rebar (revit-ifc, cited by class), so the absence of an export is not
  evidence that Revit never writes them. The sweep reads 1,349 curtain
  panels, 3,276 mullions and 100 doors on `rac_advanced` and 44, 144 and 16
  on `rac_basic` on every save, every placed frame written; the 192 panels,
  the door, the 4 mullions and the 2 panels RvtIO counts on its 2023 files
  are declared in `Global/ElemTable` with a record in the partition chain on
  all 16 saves of 2020 to 2027 and have no frame of any category, and the
  one Revit export of `rac_advanced` (2015 to 2020, IFC2X3) writes none of
  the 193 (Measure run 42104379244). `probe_re350_category_stages` and the
  scorer's `category census` lines are new, and the held-out, BIMcollab and
  racadv models keep only their counts.
