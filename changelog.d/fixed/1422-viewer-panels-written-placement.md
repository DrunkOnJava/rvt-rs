- **The viewer's Schedule panel, element info panel and scene tree show each
  element on the storey the IFC writes it in, and list only the elements it
  writes (#1422).** `build_schedule`, `element_info_panel` and
  `build_scene_graph` took an element's level from its own storey and listed
  the further pieces of an element of separate loops, which the export writes
  as items of the owner's body and not as elements. They now read the
  writer's own decision, `rvt::ifc::step_writer::placements` and `storey_of`,
  as the MCP server, the rooms schedule and the elements CSV do (#1412,
  #1417): a part of a stair (a run, landing or stringer) or of a curtain wall
  is on the storey of its whole, an element in a room on the storey of the
  room, an opening on the storey of its host, and a folded piece has no
  schedule row and no tree node. An element the export contains in the
  building only is in the schedule with an empty storey and in the tree's
  `Unassigned` storey. An element with folded pieces has no size rows in its
  panel, as its CSV row has no size cells: its body is several items, which
  one width, depth and height cannot state, and the panel does not call the
  size missing. A folded piece is still drawn in the 3-D view, so a click can
  pick it; the click selects the element whose body it is part of (the panel
  names it in the new `entity_index`), with its tree row, its GlobalId, and a
  highlight and a zoom over the element and all its pieces, and a category's
  highlight and zoom cover the pieces of the elements in it. The category
  list counts the elements the tree lists. A part of several wholes sits
  under the first, as the export has it. A door or window is nested under its
  host wall in the scene graph only where both are on the same storey, an
  element whose host is not a node of its own is a node of its storey, and so
  is one of two elements that host each other. The JSON the bindings return
  keeps its shape, with `entity_index` added to the panel.
  `docs/viewer-api.md` says what the storey is. Measured with
  `probes/examples/probe_1422_viewer_panel_levels.rs` in Measure run
  41986216576 (code head a8785bb9, `base=none`, `scorers=none`, nordic and
  r2025 on), where all 8 models pass the probe's 10 checks (the schedule's
  rows and the tree's nodes are exactly the written elements, each on the
  storey `storey_of` gives it, in level-elevation order; every sampled
  panel has that storey, names the written element in `entity_index` and,
  for an element with folded pieces, states no size and does not list
  `extents` as missing; a folded piece has the panel of its owner): on the
  BIM4LCA office as built the schedule goes from 13,021 rows to 12,974 and
  at building permit from 13,067 to 13,020, the 47 rows left out in each
  being `IfcCovering` pieces, and 599 rows of each change storey (542
  `IfcMember`, 35 `IfcStairFlight` and 22 `IfcSlab`, all parts of a whole):
  542 from one storey to another and 57 from none to a storey, and the tree
  and the panels move the same 599; the tree's `Unassigned` storey goes from
  60 elements to 3 as built and from 59 to 2 at permit. The panels of all 19
  elements with folded pieces as built and all 20 at permit have their size
  rows left out. On the Revit 2025 stair models r2025-13, r2025-14 and
  r2025-15 the 27 rows stay 27 and 3 take a storey, and on r2025-16 and
  r2025-17 the 25 rows stay 25 and 1 takes one, each from none to a storey;
  Einhoven's 37 elements move none and lose none, and neither it nor the
  Revit 2025 models has an element with folded pieces, a null result that
  does not exercise the fold path or the size rows. The IFC text `rvt-ifc`
  writes is the same on 8 of 8 of those models, in run 41986242000 (head
  a8785bb9 against main b510ff3c, `scorers=none`). These figures cover the
  Rust side: the three functions' output and the panel's `entity_index` and
  extents. The probe did not count elements that host each other, so that
  path is not known to be exercised. Not measured: the viewer's TypeScript
  (the click selecting the written element, the highlight and zoom over its
  pieces, the category counts, the part under the first whole) in a real
  browser, since no browser test exercises the fold path; the held-out
  models and Autodesk's samples; 2024 Core Interior and RE1; and the
  viewer's saved-mesh model beyond what the probe builds
  (`ExportOptions::default()`). The probe's checks derive from `placements`,
  so they prove the viewer's views follow the writer's decision and not that
  the decision is right (the MCP scorer reading the written IFC does, run
  41977947391). The diagnostics' count of folded pieces is #1424's: before it
  the Schedule panel's total read 47 below the diagnostics' on the office.
