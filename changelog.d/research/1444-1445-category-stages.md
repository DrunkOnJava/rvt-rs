- **Rebar and Structural Beam Systems stay unread because no Revit export
  shows them, Furniture Systems are `IfcSystemFurnitureElement` in Revit's
  export, and the 199 elements RvtIO counts beyond rvt-rs on `rac_advanced`
  and `rac_basic` are family instances with no stored geometry that no Revit
  export of the project writes (#1444, #1445, RE-350).** Measure run
  42097940515 (59 models with a Revit export, 37 Autodesk samples of 2020 to
  2027, every probe exit 0) finds the files hold the three categories as
  frames, the same on all eight saves of each sample: 346 `Rebar` on
  `rst_basic` (14 on Snowdon Towers Structural), 16 Structural Beam Systems
  on `rst_basic` (15 with a body) and 1 on `rac_advanced`, 3 placed Furniture
  Systems on `rac_basic`. None of the exports measured holds an
  `IfcReinforcingBar`, and the one export of a project with a beam system,
  the three.js IFC2X3 export of `rac_advanced`, holds neither it nor any of
  the project's 38 framing instances, 23 of which the system's record lists,
  which is when revit-ifc's exporter writes no assembly (cited by class), so
  neither is added. Revit's public IFC4 export of the 2018 `rac_basic` (RE-326)
  writes its 3 Furniture Systems as `IfcSystemFurnitureElement` with no
  `PredefinedType`, and they are the sweep's 3 placed ones, `Tag` for `Tag`,
  none written that the export lacks: the category now meets the rule and is
  left for a change of its own, since `category_map` maps it to
  `IfcFurniture`. The only `IfcElementAssembly` in a measured export are 3 in
  each of Vitoria's two 2018 files, Revit assemblies with no representation
  (RE-315) that no frame carries. The sweep reads 1,349 curtain panels, 3,276
  mullions and 100 doors on `rac_advanced` and 44, 144 and 16 on `rac_basic`
  on every save, every placed frame written. The 192 panels, the door, the 4
  mullions and the 2 panels RvtIO counts on its 2023 files are
  `FamilyInstance` records in the main model that every save of 2018 to 2027
  holds with no frame of any category (Measure runs 42104379244 and
  42108271073), and the public IFC4 export of the 2018 `rac_basic` holds none
  of the 6, the IFC2X3 export of `rac_advanced` none of the 193; their family
  types are not read from the file. `probe_re350_category_stages` and the
  scorer's `category census` lines are new, and the held-out, BIMcollab and
  racadv models keep only their counts.
