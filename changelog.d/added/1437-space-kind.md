- **Every `IfcSpace` says what it is: `RvtElementRecordGeometry.SpaceKind` is
  `room`, `area`, `mep_space` or `family_instance` (#1437).** An importer
  could not tell an Area from a room by asking what it is: the only marker in
  a default export was `SpaceOutlineSource`, which says where an outline came
  from, and a filter on its values stops catching Areas when a release reads
  them by another rule (the plan topology of #1428 gives them values of their
  own). `SpaceKind` is taken from the Revit class of the record the space was
  read from (a room, an Area, a Revit Space (MEP), or any other element whose
  own "Export to IFC As" names `IfcSpace`, #763, as `family_instance`), so it
  holds whatever the outline source is, a body-less Area included, and in an
  export with no geometry too. `SpaceOutlineSource` is unchanged and keeps
  meaning only where an outline came from, which is what decides whether to
  trust a space's area; read `SpaceKind` to tell the kinds apart and never
  filter on the outline source's values (`docs/rooms.md` now leads with which
  property answers which question). No `mep_space` is written today: rvt-rs
  reads no Revit Space from a file's partition records, which says what it
  writes and not what the files hold. The same value is `space_kind` on each
  room of the MCP server's `list_rooms` and in a last column, `space_kind`, of
  the room schedule (`rvt-schedule --schedule rooms`, Python's `schedule_csv`,
  the viewer's room schedule); the MCP summary gains `space_kinds`, and its
  `room_count`, which counts every space, is described as that (the tool
  contract in `docs/mcp-tools.*` moves with it). A space whose property set is
  not `RvtElementRecordGeometry` (in principle a wall or floor read through the
  schema walker) is written without the property, never with a guess, and
  counted: the diagnostics gain `exported.space_kinds` and
  `exported.spaces_without_kind`, and `rvt-ifc` says so in its summary when it
  is not zero. `RvtElementRecordGeometry` is rvt-rs's own set, so the property
  is in no `Pset_` count, no `decoded.parameter_value_count` and no
  property-set census. The library builder `ifc::build_ifc_model` writes the
  property sets its caller gives it and adds no kind. A new check,
  `tools/re/space_kinds_vs_ifc.py`, runs in Measure on every model and sample
  and in the IfcOpenShell job of CI. It fails on a space with no kind, with an
  outline source it does not know, or with a kind that disagrees with its
  outline source, and its run total fails on a model whose result saw fewer
  spaces than its export counted or whose geometry export lost the outline
  source; CI shows it failing on copies of Core Interior's export with the
  property renamed, every room called an area, and every room's outline
  source made an Area's, and runs it on an export with no geometry. On
  Measure run 42019512273 (head 02072245) the 111 models and Autodesk
  samples exported held 3,493 `IfcSpace`s, every one with a `SpaceKind`:
  3,187 rooms, 214 Areas and 92 family instances, and no `mep_space`. The
  BIM4LCA office holds 308 at both stages (234 rooms, 30 Areas, 44 family
  instances). The check passes on all 111: no `area` outside the Area outline
  sources and no other kind inside them, on the 55 models and samples that hold
  spaces and among them the 15 held-out and spent ones (counts only).
