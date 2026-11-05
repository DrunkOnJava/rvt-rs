- **The viewer's Schedule panel, element info panel and scene tree show each
  element on the storey the IFC writes it in, and list only the elements it
  writes (#1422).** `build_schedule`, `element_info_panel` and
  `build_scene_graph` took an element's level from its own storey and listed
  the further pieces of an element of separate loops, which the export writes
  as items of the owner's body and not as elements. They now read the
  writer's own decision, `rvt::ifc::step_writer::placements` and `storey_of`,
  as the MCP server, the rooms schedule and the elements CSV do (#1412,
  #1417): a part of a stair, a railing or a curtain wall is on the storey of
  its whole, an element in a room on the storey of the room, an opening on the
  storey of its host, and a folded piece has no schedule row and no tree node.
  An element the export contains in the building only is in the schedule with
  an empty storey and in the tree's `Unassigned` storey. A folded piece is
  still drawn in the 3-D view, so a click can pick it; its panel is that of
  the element whose body it is part of. A door or window is nested under its
  host wall in the scene graph only where both are on the same storey, and an
  element whose host is not a node of its own is a node of its storey instead
  of being left out. The JSON the bindings return keeps its shape.
  `docs/viewer-api.md` says what the storey is.
