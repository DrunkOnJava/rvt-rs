- **Hand-off values no longer reach the IFC.** Nineteen properties of
  `RvtElementRecordGeometry` were values one stage of the export wrote so a
  later stage could read them back, not facts about the element: the opening
  a window's or door's type and transform give (`OpeningCentreX`,
  `OpeningCentreY`, `OpeningAxisX`, `OpeningAxisY`, `OpeningBaseElevation`,
  `OpeningWidth`, `OpeningHeight`), a turned instance's origin
  (`InstanceOriginX`, `InstanceOriginY`), and the sizes its type and symbol
  give (`TypeWidth`, `TypeHeight`, `CutWidth`, `CutHeight`, `CutSolidWidth`,
  `CutSolidDepth`, `CutSolidHeight`, `BoxWidth`, `BoxHeight`,
  `SymbolWireWidth`). They stay inside the exporter, which works the openings
  and each door's and window's `OverallWidth` and `OverallHeight` out of
  them as before. The trust markers (`BodySource`, `ProfileResolved`,
  `SpaceOutlineSource` and the rest), the identity properties and the
  measured facts stay. Upgrading: nothing user-facing changes value,
  `OverallWidth`, `OverallHeight` and every opening are what they were, and
  only those 19 properties and their `IFCPROPERTYSINGLEVALUE` lines are gone
  from a door's or window's set. An importer that read a door's or window's
  size from one of them reads `OverallWidth` and `OverallHeight` instead; an
  opening's centre, axis and base elevation are in its `IfcOpeningElement`'s
  placement and extrusion. The `*_PROPERTIES` constants of
  `rvt::ifc::export_content` stay, as the names of the hand-offs.
  `rvt::ifc::IfcModel` gains a public field, `filler_sizes` (each door's and
  window's size, serialised when it is not empty), which breaks a caller that
  builds an `IfcModel` by hand.
