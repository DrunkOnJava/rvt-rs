//! The field names, class lists, tolerances and Revit-version lists the partition recovers share.

/// Field naming the IFC entity an element's "Export to IFC As" override,
/// or its type's "Export Type to IFC As", names (#212, RE-45). The decoder
/// does not act on it; [`crate::ifc::category_map::lookup_export_override`]
/// decides which values the IFC writer honours.
pub const IFC_EXPORT_AS_FIELD: &str = "m_ifc_export_as";

/// Field naming the IFC predefined type an element's or its type's export
/// parameters set (RE-45). The IFC writer uses it only when it is an
/// enumerator of the entity the element exports as.
pub const IFC_PREDEFINED_TYPE_FIELD: &str = "m_ifc_predefined_type";

/// Field carrying the Revit release that saved the file, beside
/// [`IFC_EXPORT_AS_FIELD`]: whether Revit's exporter acts on the value can
/// depend on it ([`crate::ifc::category_map::export_override_applies`],
/// RE-188).
pub const IFC_EXPORT_RELEASE_FIELD: &str = "m_ifc_export_release";

/// The family Revit names its empty curtain panel type after (#309). Revit's
/// IFC4 export leaves every panel of it out: all 24 on Snowdon Towers. The
/// name is Revit's English one, so a file saved in another language keeps
/// its empty panels. The panel type's unset material
/// ([`crate::partition_names::find_panel_type_materials`]) is only a
/// cross-check, since a panel set to "By Category" may store no material
/// either.
pub const EMPTY_PANEL_FAMILY_NAME: &str = "Empty System Panel";

/// Field marking a curtain panel that holds a basic wall (RE-64, RE-72).
pub const CURTAIN_PANEL_WALL_FIELD: &str = "m_curtain_panel_is_wall";

/// Class of a curtain panel that holds a basic wall. Revit's IFC4 export
/// writes each as an `IfcCurtainWall` with its own body, nested under the
/// curtain wall it is a panel of: all 18 on Snowdon Towers (RE-72).
pub const CURTAIN_PANEL_WALL_CLASS: &str = "CurtainPanelWall";

/// Field naming an element-record element's type, from the partition name
/// entries (RE-38).
pub const TYPE_NAME_FIELD: &str = "m_type_name";

/// Field naming the family of that type (RE-38).
pub const FAMILY_NAME_FIELD: &str = "m_family_name";

/// Field carrying the type's ElementId (RE-38).
pub const TYPE_ID_FIELD: &str = "m_type_id";

/// Give each element-record element its family and type names (RE-38).
///
/// The record's reference list is read again at its source offset. Its type
/// is the one reference with a name entry of the record's category, and
/// the family is the one such id in the type's own partition record
/// ([`crate::partition_names::resolve_type`] /
/// [`crate::partition_names::resolve_family`]). An element gets the fields
/// only when both joins are unique.
/// Field carrying the class tag an element record holds at `+0x4a` (RE-76).
pub const CLASS_TAG_FIELD: &str = "m_class_tag";

/// Field carrying the name of that class in the file's own schema (RE-76).
pub const REVIT_CLASS_FIELD: &str = "m_revit_class";

/// Fields holding the three axes of a curtain mullion or panel turned or
/// tilted off the model's axes (RE-106), from its transform: X, then Y,
/// then Z, each as model x, y and z.
pub const CURTAIN_AXES_FIELDS: [&str; 9] = [
    "m_curtain_axis_xx",
    "m_curtain_axis_xy",
    "m_curtain_axis_xz",
    "m_curtain_axis_yx",
    "m_curtain_axis_yy",
    "m_curtain_axis_yz",
    "m_curtain_axis_zx",
    "m_curtain_axis_zy",
    "m_curtain_axis_zz",
];

/// Classes whose turned or tilted instances carry [`CURTAIN_AXES_FIELDS`] (RE-106).
pub const CURTAIN_AXES_CLASSES: [&str; 2] = ["CurtainWallMullion", "CurtainWallPanel"];

/// Field holding the model axis (0 for X, 1 for Y, 2 for Z) a curtain
/// mullion on the model's axes runs along: the one its transform's Z axis
/// lies on (RE-226). A turned or tilted mullion carries
/// [`CURTAIN_AXES_FIELDS`] instead.
pub const CURTAIN_MULLION_ALONG_FIELD: &str = "m_curtain_mullion_along";

/// Set on a curtain mullion that stands on a straight curtain wall with an
/// unedited profile and whose type has its own rectangular section (RE-246):
/// Revit's export draws it as an extrusion of its box's section along its
/// length (RE-243).
pub const MULLION_PLAIN_HOST_FIELD: &str = "m_mullion_plain_host";

/// How many of its two end faces the Revit solid of a curtain mullion keeps,
/// 0, 1 or 2, where that follows from the boxes of the mullions of its curtain
/// wall (`kept_ends`, RE-268). Its `NetSurfaceArea` is its section's
/// perimeter times its length plus that many of its section's areas.
pub const MULLION_KEPT_ENDS_FIELD: &str = "m_mullion_kept_ends";

/// Fields holding a door's or window's facing: the plan direction of
/// its transform's Y axis (RE-87), which Revit's exporter sets against its
/// host wall to decide whether the door's symbol is flipped.
// Tracker: B72.
pub const OPENING_FACING_FIELDS: [&str; 2] = ["m_opening_facing_x", "m_opening_facing_y"];

/// Field carrying a family instance's original symbol (RE-167): the
/// FamilySymbol `ExporterIFCUtils.GetOriginalSymbol` returns, whose GlobalId
/// Revit's export gives the instance's type object while its `Tag` stays the
/// instance's symbol.
pub const ORIGINAL_SYMBOL_FIELD: &str = "m_original_symbol";

/// The [`ORIGINAL_SYMBOL_FIELD`] of an instance that has no original symbol
/// because it uses its own geometry (Revit's invalid ElementId, -1): its
/// `GElement` object holds `0xffffffff` there. Revit's export then gives its
/// type a sub-element GlobalId of the instance's own (RE-167): all 164 such
/// columns of Core Interior.
pub const INSTANCE_GEOMETRY: u32 = u32::MAX;

/// The schema classes of the elements an original symbol may be: a family's
/// symbol, and the mullion and panel types curtain parts take theirs from
/// (RE-166, RE-167: RE1 Architecture's mullions' original symbols are
/// `SysMullionFamSym` objects, its panels' `SysPanelFamSym`).
pub const ORIGINAL_SYMBOL_CLASSES: [&str; 3] =
    ["FamilySymbol", "SysMullionFamSym", "SysPanelFamSym"];

/// Bytes before the end of a family instance's `GElement` data object
/// (RE-153) where it holds its original symbol's ElementId (RE-167): at +300
/// of 320 bytes on RE1 Architecture, +330 of 350 and +392 of 412 on the MEP
/// models.
pub const ORIGINAL_SYMBOL_FROM_END: usize = 20;

/// Fields holding the plan direction of a family instance turned off the
/// model's axes (RE-87), from its transform: its X axis where its Z axis is
/// the model's, or else its first flat axis where one axis is vertical, as
/// for a fixture hosted on a ceiling or a wall (RE-90).
pub const INSTANCE_X_AXIS_FIELDS: [&str; 2] = ["m_instance_x_axis_x", "m_instance_x_axis_y"];

/// Fields holding the plan origin of a turned family instance, model feet,
/// from its transform (RE-87): the point a window's opening in a tapered
/// wall is centred on (RE-89).
pub const INSTANCE_ORIGIN_FIELDS: [&str; 2] = ["m_instance_origin_x", "m_instance_origin_y"];

/// How far off the model's axes a family instance must be turned to be
/// drawn turned (RE-87): 1e-4 radians, 0.001 ft at 10 ft. RE1 Architecture
/// holds 34 instances off them by less than 0.001 degrees, round-off rather
/// than a turn; Snowdon Towers' 628 turned ones are all 0.01 degrees or more
/// off.
pub const INSTANCE_TURN_MIN_RADIANS: f64 = 1e-4;

/// Fields holding a window's or door's opening as Revit's export cuts it
/// (RE-93, RE-94): the plan point it is centred on (the element's origin),
/// its plan direction (the element's X axis), its base elevation (the
/// origin's, plus a window type's Default Sill Height), and its width and
/// height, feet: a window type's Width and Height, a door type's Rough
/// Width and Rough Height.
pub const FILLER_OPENING_FIELDS: [&str; 7] = [
    "m_filler_opening_x",
    "m_filler_opening_y",
    "m_filler_opening_axis_x",
    "m_filler_opening_axis_y",
    "m_filler_opening_base",
    "m_filler_opening_width",
    "m_filler_opening_height",
];

/// How close a door's record box height must be to its type's Rough Height
/// for the door to take its rough opening (RE-94), feet.
pub const DOOR_ROUGH_HEIGHT_TOLERANCE_FEET: f64 = 1e-3;

/// Give each upright door whose record box is its type's Rough Height tall
/// its type's rough opening, from its origin up (RE-94,
/// [`crate::partition_type_parameters::type_door_openings`]). On Snowdon
/// Towers every door Revit cuts at its rough opening has a body that tall,
/// and every other door a body its frame's height instead.
/// Fields carrying a beam's I section from its type (RE-103): flange
/// width, overall depth, web thickness and flange thickness, feet.
pub const BEAM_I_SECTION_FIELDS: [&str; 4] = [
    "m_i_section_width",
    "m_i_section_depth",
    "m_i_section_web",
    "m_i_section_flange",
];

/// Fields carrying a hollow-section column's outside width and height, wall
/// thickness and outside corner radius, feet
/// ([`crate::partition_type_parameters::type_hollow_sections`], W6).
pub const HOLLOW_SECTION_FIELDS: [&str; 4] = [
    "m_hollow_section_width",
    "m_hollow_section_height",
    "m_hollow_section_wall",
    "m_hollow_section_radius",
];

/// Set on a column whose record holds no `CleanPairGStep`
/// ([`crate::native_column_joins::ColumnJoins::without_clean_steps`]):
/// Revit's export draws it as an extrusion, and gives it the extrusion's
/// depth as its `Qto_ColumnBaseQuantities.Length` (RE-247).
pub const COLUMN_UNJOINED_FIELD: &str = "m_column_unjoined";

/// Classes whose type's hollow section is read (W6).
pub const HOLLOW_SECTION_CLASSES: [&str; 2] = ["StructuralColumn", "Column"];

/// Fields carrying the plan direction of an I-section structural column's
/// X axis, from its transform (RE-104): its flanges run along it.
pub const COLUMN_I_AXIS_FIELDS: [&str; 2] = ["m_i_section_axis_x", "m_i_section_axis_y"];

/// Classes whose type's I section is read (RE-103, RE-104).
pub(super) const I_SECTION_CLASSES: [&str; 2] = ["StructuralFraming", "StructuralColumn"];

/// Fields carrying a door's Width and Height from its type (RE-187), feet.
pub const DOOR_TYPE_SIZE_FIELDS: [&str; 2] = ["m_door_type_width", "m_door_type_height"];

/// Fields carrying the Width and Height of the symbol that draws a door or
/// window (its `InstanceInfo.m_symbolId`): its type's, or the symbol its own
/// parameters make, which revit-ifc's `WidthCalculator`, `HeightCalculator`
/// and `AreaCalculator` read for its base quantities (#1016), feet.
pub const DRAWN_SIZE_FIELDS: [&str; 2] = ["m_drawn_symbol_width", "m_drawn_symbol_height"];

/// Fields carrying the extent along X and along Z of the outline a door's
/// or window's symbol cuts in its host (RE-215), feet.
pub const CUT_LOOP_SIZE_FIELDS: [&str; 2] = ["m_cut_loop_width", "m_cut_loop_height"];

/// Fields carrying a door's or window's record box extent along its own X
/// axis and its height, where its symbol cuts no loop (#810), feet.
pub const INSTANCE_BOX_SIZE_FIELDS: [&str; 2] = ["m_instance_box_width", "m_instance_box_height"];

/// Fields carrying the extents along X, Y and Z of the solid a door's or
/// window's symbol cuts where it cuts no loop (RE-225), feet.
pub const CUT_SOLID_SIZE_FIELDS: [&str; 3] = [
    "m_cut_solid_width",
    "m_cut_solid_depth",
    "m_cut_solid_height",
];

/// Releases a door's or window's cut loop is measured on (RE-215): 2024
/// (Core Interior) and 2025 (RE1 Architecture, the R2025 models).
pub const CUT_LOOP_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// How closely an instance's plan X axis must lie along a model axis for
/// its record box to be along its own axes (#810).
pub const INSTANCE_AXIS_TOLERANCE: f64 = 1e-6;

/// Prefix of the fields carrying an element's type text parameters (RE-77):
/// `m_type_parameter:Type Mark` and so on.
pub const TYPE_PARAMETER_FIELD_PREFIX: &str = "m_type_parameter:";

/// Give each element the text parameters of its type ([`TYPE_ID_FIELD`],
/// RE-38 and #322), as Revit shows a type's parameters on its instances.
/// Field carrying, once per material, the name of a material an element's
/// type draws its geometry in (RE-82), in the order the type names them.
pub const TYPE_MATERIAL_FIELD: &str = "m_type_material";

/// Field set on an element whose type has a parameter value block with no
/// geometry-material map: a type that draws no geometry (RE-84). Revit's
/// export writes such a door or window as its opening alone.
pub const TYPE_WITHOUT_GEOMETRY_FIELD: &str = "m_type_without_geometry";

/// Field naming the ElementId of the stair a run, landing or stringer
/// belongs to (#323).
pub const AGGREGATE_WHOLE_FIELD: &str = "m_aggregate_whole";

/// Classes that aggregate parts instead of carrying a body of their own.
pub const AGGREGATE_WHOLE_CLASSES: &[&str] = &["Stair", CURTAIN_WALL_CLASS];

/// Class of a wall that is a curtain wall (RE-46): one a curtain-wall
/// mullion names in its reference list.
pub const CURTAIN_WALL_CLASS: &str = "CurtainWall";

/// Classes that are parts of a curtain wall when their reference list names
/// exactly one (RE-46).
///
/// Doors are not among them: of the doors whose lists name a Snowdon
/// curtain wall, Revit's export aggregates only the panel doors, and nothing
/// read so far tells those from doors inserted in the wall.
pub const CURTAIN_WALL_PART_CLASSES: &[&str] = &["CurtainWallPanel", "CurtainWallMullion"];

/// Classes that are parts of a stair when their reference list names one.
///
/// Railings are not among them: Revit's export aggregates 65 of the 70
/// Snowdon railings whose lists name a stair, and nothing read so far tells
/// the other five apart, so railings stay standalone elements.
pub const STAIR_PART_CLASSES: &[&str] = &["StairsRun", "StairsLanding", "StairsStringer"];

/// Field holding an element's IFC `Name` where Revit's export does not name
/// it `Family:Type:ElementId`: a stair and its parts (RE-65).
pub const ELEMENT_NAME_FIELD: &str = "m_element_name";

/// Value of [`FAMILY_NAME_SOURCE_FIELD`] for a stair component's family,
/// read from its type's construction flag (RE-65).
pub const TYPE_KIND_FAMILY_SOURCE: &str = "system_family_by_type_kind";

/// Field holding a stair's or flight's number of risers (RE-47).
pub const STAIR_RISER_COUNT_FIELD: &str = "m_stair_riser_count";

/// Field holding a stair's or flight's riser height, feet (RE-47).
pub const STAIR_RISER_HEIGHT_FIELD: &str = "m_stair_riser_height";

/// Field holding a stair's or flight's tread depth, feet (RE-47).
pub const STAIR_TREAD_DEPTH_FIELD: &str = "m_stair_tread_depth";

/// Field holding a flight's number of treads: its risers, one fewer when it
/// begins and ends with a riser, one more for each end that is a tread
/// (RE-92).
pub const STAIR_TREAD_COUNT_FIELD: &str = "m_stair_tread_count";

/// Field holding how far a flight's treads overhang its risers, its run
/// type's nosing length, feet; on a stair, the one every run of it has
/// (RE-52).
pub const STAIR_NOSING_LENGTH_FIELD: &str = "m_stair_nosing_length";

/// Fields holding the unit plan direction from a wall's inside to its
/// exterior face, model axes (RE-53).
pub const WALL_EXTERIOR_FIELDS: [&str; 2] = ["m_wall_exterior_x", "m_wall_exterior_y"];

/// Field holding a wall's layers, exterior first: each a vector of width
/// (feet), shading colour (`0x00BBGGRR`, or -1 for the category's) and
/// transparency (RE-53).
pub const WALL_LAYERS_FIELD: &str = "m_wall_layers";

/// Fields holding a wall's centreline start and end in plan, model feet
/// (RE-54).
pub const WALL_AXIS_START_FIELDS: [&str; 2] = ["m_wall_axis_start_x", "m_wall_axis_start_y"];

/// See [`WALL_AXIS_START_FIELDS`].
pub const WALL_AXIS_END_FIELDS: [&str; 2] = ["m_wall_axis_end_x", "m_wall_axis_end_y"];

/// Fields holding a curved wall's location arc (RE-75), in plan: its centre
/// `x` and `y`, radius, start and end angle (radians), and the `x` and `y`
/// of its unit X and Y axes. The arc runs from
/// `centre + radius · (cos a · X + sin a · Y)` at the start angle to the end
/// angle.
pub const WALL_ARC_FIELDS: [&str; 9] = [
    "m_wall_arc_centre_x",
    "m_wall_arc_centre_y",
    "m_wall_arc_radius",
    "m_wall_arc_start_angle",
    "m_wall_arc_end_angle",
    "m_wall_arc_x_axis_x",
    "m_wall_arc_x_axis_y",
    "m_wall_arc_y_axis_x",
    "m_wall_arc_y_axis_y",
];

/// Field holding a wall's thickness, its type's layers summed (RE-54).
pub const WALL_TYPE_THICKNESS_FIELD: &str = "m_wall_type_thickness";

/// Field holding how far a tapered wall's exterior face leans from
/// vertical, radians, wider at the base (RE-86). Its line and
/// [`WALL_TYPE_THICKNESS_FIELD`] give the body at the wall's top.
pub const WALL_EXTERIOR_FACE_ANGLE_FIELD: &str = "m_wall_exterior_face_angle";

/// Fields holding how far past the start and the end of its centreline a
/// wall's body reaches at a butt join, feet, negative where it stops short
/// (RE-70), or at a T joint (RE-73). Absent at an end neither decides.
pub const WALL_JOIN_REACH_FIELDS: [&str; 2] = ["m_wall_join_start_reach", "m_wall_join_end_reach"];

/// Fields holding the one wall a wall butt-joins at its start and at its
/// end, where the join lists decide which of the two runs through (RE-70),
/// or the wall it stops against at a T joint (RE-73).
pub const WALL_JOIN_PARTNER_FIELDS: [&str; 2] = ["m_wall_join_start_wall", "m_wall_join_end_wall"];

/// Fields holding how far past the start and the end of its centreline
/// each of a wall's layers reaches at a layered butt join (RE-71) or T
/// joint (RE-73), exterior first, feet.
pub const WALL_JOIN_LAYER_REACH_FIELDS: [&str; 2] = [
    "m_wall_join_start_layer_reaches",
    "m_wall_join_end_layer_reaches",
];

/// Fields holding, at an angled join, how far past the start and the end of
/// its centreline each of a wall's layers reaches along its exterior-side
/// and its interior-side edge, exterior first, two values per layer, feet
/// (RE-74).
pub const WALL_JOIN_LAYER_EDGE_REACH_FIELDS: [&str; 2] = [
    "m_wall_join_start_layer_edge_reaches",
    "m_wall_join_end_layer_edge_reaches",
];

/// Fields holding whether a wall runs through that join (true) or stops at
/// it (false) (RE-70).
pub const WALL_JOIN_THROUGH_FIELDS: [&str; 2] =
    ["m_wall_join_start_through", "m_wall_join_end_through"];

/// Field holding a floor's, roof's or ceiling's layers, top first, in the
/// form of [`WALL_LAYERS_FIELD`] (RE-57).
pub const SLAB_LAYERS_FIELD: &str = "m_slab_layers";

/// Field holding the total thickness of a floor's type's layers, feet,
/// whether or not the floor's record box is that deep (RE-295).
pub const SLAB_TYPE_THICKNESS_FIELD: &str = "m_slab_type_thickness";

/// Field holding how many layers of that thickness a floor's type has
/// (RE-295).
pub const SLAB_TYPE_LAYER_COUNT_FIELD: &str = "m_slab_type_layer_count";

/// Set on a floor, ceiling or roof whose native record was read and holds no
/// step that cuts its solid, a `CutPairGStep` or a `WallCutoutGStep` (RE-295).
pub const SLAB_UNCUT_FIELD: &str = "m_slab_uncut";

/// Field holding a railing's height, its type's top rail height, feet
/// (RE-231).
pub const RAILING_HEIGHT_FIELD: &str = "m_railing_height";

/// Field holding a stair railing's `Length`, its path along its stair,
/// feet (RE-254).
pub const RAILING_LENGTH_FIELD: &str = "m_railing_length";

/// Field recording that [`FAMILY_NAME_FIELD`] is a system family's name
/// derived by [`super::system_family`], not read from the file (RE-63).
pub const FAMILY_NAME_SOURCE_FIELD: &str = "m_family_name_source";

/// Value of [`FAMILY_NAME_SOURCE_FIELD`].
pub const SYSTEM_FAMILY_SOURCE: &str = "system_family_by_category";

/// Give each wall its type's layers, exterior first, each with its
/// material's shading, and the plan direction of its exterior face: the
/// right of its location line's direction when its flip flag is set, the
/// left otherwise (RE-53, [`crate::partition_compound_structure`],
/// [`crate::partition_materials`]). A wall whose word is 1 also gets its
/// centreline and its type's thickness, from which the exporter builds its
/// body (RE-54). A wall without a type, a location line or an orientation
/// gets none of these. Membranes, which have no width, are left out.
/// The field prefix of the openings a wall's edited elevation profile cuts
///: `m_profile_opening_<k>`, the opening's outline as model-feet
/// points in the wall's vertical plane, and `m_profile_opening_<k>_tag`, the
/// ElementId it is tagged with.
// Tracker: B55.
pub const PROFILE_OPENING_FIELD: &str = "m_profile_opening";

/// How far apart two sketch points may lie and still meet, and how far off a
/// plane or a side a point may lie and still be on it, feet.
// Tracker: B55.
pub(super) const PROFILE_EPS_FEET: f64 = 1e-3;

/// Field holding a wall's Length, feet: its curve's length, where the curve
/// cut back at its joins is as long ([`crate::native_walls::trimmed_ends`],
/// RE-252), so Revit's export gives it either way; on a curtain wall, its
/// curve's length, which Revit's export writes as its
/// `Qto_CurtainWallQuantities.Length` (#1016).
pub const WALL_LENGTH_FIELD: &str = "m_wall_length";

/// Field set on a wall whose native `SWall` record names, in
/// `m_WallAttributesId`, the type rvt-rs picks for it ([`TYPE_ID_FIELD`]):
/// the wall's type as its saved file states it (RE-253). The export's
/// diagnostics list these walls for the quantity census.
pub const WALL_TYPE_CONFIRMED_FIELD: &str = "m_wall_type_confirmed";

/// Field holding the length of a wall's native curve, feet, on every wall with
/// a centreline whose native curve is the same line (RE-252): one of the two
/// lengths Revit's export may write as its `Length`. The export's diagnostics
/// list it (`wall_curve_lengths_feet`), not its IFC file, for the census to
/// compare with the other, Revit's own cut-back `Axis`, and count apart a
/// wall whose two lengths differ (the maintainer, 2026-10-26 and 2026-10-27).
pub const WALL_CURVE_LENGTH_FIELD: &str = "m_wall_curve_length";

/// How close a wall's native curve's ends must be to its centreline's to be
/// taken as the same line, feet.
pub(super) const NATIVE_WALL_LINE_TOLERANCE_FEET: f64 = 1e-4;

/// Fields holding where a stair run's sketch starts, model feet (RE-52).
pub const STAIR_RUN_ORIGIN_FIELDS: [&str; 3] = [
    "m_stair_run_origin_x",
    "m_stair_run_origin_y",
    "m_stair_run_origin_z",
];

/// Fields holding the unit plan direction a stair run climbs in (RE-52).
pub const STAIR_RUN_CLIMB_FIELDS: [&str; 2] = ["m_stair_run_climb_x", "m_stair_run_climb_y"];

/// Fields holding the unit plan direction across a stair run (RE-52).
pub const STAIR_RUN_ACROSS_FIELDS: [&str; 2] = ["m_stair_run_across_x", "m_stair_run_across_y"];

/// Field holding a stair run's width, feet (RE-52).
pub const STAIR_RUN_WIDTH_FIELD: &str = "m_stair_run_width";

/// Field holding a stair run's side view, `[along, up]` feet from its
/// sketch origin (RE-52).
pub const STAIR_RUN_PROFILE_FIELD: &str = "m_stair_run_profile";

/// Field carrying the side views of a run's further solids, after the first
/// in [`STAIR_RUN_PROFILE_FIELD`]: a riserless run's treads (#357).
pub const STAIR_RUN_PIECES_FIELD: &str = "m_stair_run_pieces";

/// Field holding the distance along a stair run's climb from its first
/// riser line to its last, feet: its flight's `Length` as Revit's export
/// gives it, with or without a riser at its top (#1016, RE-247).
pub const STAIR_RUN_LENGTH_FIELD: &str = "m_stair_run_length";

/// Fields holding a spiral run's centre, model feet (#357).
pub const SPIRAL_RUN_CENTRE_FIELDS: [&str; 2] = ["m_spiral_run_centre_x", "m_spiral_run_centre_y"];

/// Fields holding a spiral run's inner and outer radius, feet (#357).
pub const SPIRAL_RUN_RADII_FIELDS: [&str; 2] =
    ["m_spiral_run_inner_radius", "m_spiral_run_outer_radius"];

/// Field holding a spiral run's treads, each `[bottom, top, down-run angle,
/// up-run angle at the outer radius, up-run angle at the inner radius]`, feet
/// and radians about the centre (#357).
pub const SPIRAL_RUN_TREADS_FIELD: &str = "m_spiral_run_treads";

/// Bottom, feet, of a stair landing's body (#824): its type's thickness
/// below its top. The landing's own fields (`m_locationZ`, `m_bboxHeight`)
/// keep its record box, which spans its stair's storey.
pub const LANDING_BODY_BOTTOM_FIELD: &str = "m_landing_body_bottom";

/// Top, feet, of a stair landing's body (#824).
pub const LANDING_BODY_TOP_FIELD: &str = "m_landing_body_top";

/// [`crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD`] of a
/// landing outline read from the lines in its own element data (#824).
pub const LANDING_OUTLINE_SOURCE: &str = "partition_landing_lines";

/// Field holding a tread landing's outline (#824): each curve `[0, start x,
/// start y, end x, end y]` for a line or `[1, start, middle, end]` for an
/// arc through its middle point, model feet, in loop order.
pub const LANDING_TREAD_OUTLINE_FIELD: &str = "m_landing_tread_outline";

/// Field holding a tread landing's underside, feet (#824).
pub const LANDING_TREAD_BOTTOM_FIELD: &str = "m_landing_tread_bottom";

/// Field holding a tread landing's top, feet (#824).
pub const LANDING_TREAD_TOP_FIELD: &str = "m_landing_tread_top";

/// How closely a landing's lines must lie at its top and meet each other,
/// feet. Its outline and slab must lie in its record box to
/// [`SKETCH_BOX_TOLERANCE_FEET`], as a sketched outline must.
pub const LANDING_LINE_TOLERANCE_FEET: f64 = 1e-6;

/// Field carrying the `m_runType` a stair run's treads all give
/// ([`crate::native_stairs::native_run_shapes`], #323), which names its
/// flight's predefined type
/// ([`crate::native_stairs::flight_predefined_type`]).
pub const STAIR_RUN_SHAPE_FIELD: &str = "m_stair_run_shape";

/// Field naming the `IfcStairTypeEnum` value a stair's runs and landings
/// give it ([`crate::native_stairs::stair_predefined_type`], #323).
pub const STAIR_TYPE_FIELD: &str = "m_stair_type";

/// Field holding a beam's Cross-Section Rotation, radians: the `m_rotation`
/// of the `FamInstCurveDriver` in its native record (W7, RE-276).
pub const BEAM_ROTATION_FIELD: &str = "m_beam_cross_section_rotation";

/// Fields holding the first end of a beam's location line, model feet
/// (RE-49).
pub const BEAM_AXIS_START_FIELDS: [&str; 3] = [
    "m_beam_axis_start_x",
    "m_beam_axis_start_y",
    "m_beam_axis_start_z",
];

/// Fields holding the second end of a beam's location line, model feet
/// (RE-49).
pub const BEAM_AXIS_END_FIELDS: [&str; 3] = [
    "m_beam_axis_end_x",
    "m_beam_axis_end_y",
    "m_beam_axis_end_z",
];

/// Fields holding the first end of a pipe's centreline, model feet (RE-131).
pub const PIPE_AXIS_START_FIELDS: [&str; 3] = [
    "m_pipe_axis_start_x",
    "m_pipe_axis_start_y",
    "m_pipe_axis_start_z",
];

/// Fields holding the second end of a pipe's centreline, model feet (RE-131).
pub const PIPE_AXIS_END_FIELDS: [&str; 3] = [
    "m_pipe_axis_end_x",
    "m_pipe_axis_end_y",
    "m_pipe_axis_end_z",
];

/// Field holding a pipe's outside radius, feet (RE-131).
pub const PIPE_RADIUS_FIELD: &str = "m_pipe_radius";

/// Fields holding the keys of the connector entries at a pipe's first and
/// second end (#847), each a vector of `[element, connector index]` pairs.
pub const PIPE_END_KEYS_FIELDS: [&str; 2] = ["m_pipe_start_keys", "m_pipe_end_keys"];

/// Fields holding a duct's two ends, model feet: the centres of the faces of
/// its record box across its length (#847, RE-134).
pub const DUCT_ENDS_FIELDS: [[&str; 3]; 2] = [
    ["m_duct_start_x", "m_duct_start_y", "m_duct_start_z"],
    ["m_duct_end_x", "m_duct_end_y", "m_duct_end_z"],
];

/// Fields holding the keys of the connector entries at a duct's two ends
/// (#847), as [`PIPE_END_KEYS_FIELDS`] for a pipe.
pub const DUCT_END_KEYS_FIELDS: [&str; 2] = ["m_duct_start_keys", "m_duct_end_keys"];

/// Fields holding a duct's or pipe's `m_dWidthOrDiameter` and `m_dHeight`,
/// feet (RE-134).
pub const CURVE_SIZE_FIELDS: [&str; 2] = ["m_curve_width", "m_curve_height"];

/// Field holding a duct's length along its axis, feet: the dimension of its
/// record box its width and height leave (RE-134).
pub const DUCT_LENGTH_FIELD: &str = "m_duct_length";

/// Field holding a pipe's inner diameter, feet, from its curve object
/// (RE-157).
pub const PIPE_INNER_DIAMETER_FIELD: &str = "m_pipe_inner_diameter";

/// How closely a duct's width and height must match two of its box's
/// dimensions, feet.
pub const CURVE_BOX_TOLERANCE_FEET: f64 = 2e-3;

/// Field carrying the Function of an element's type: `FUNCTION_PARAM` (0
/// interior, 1 exterior), RE-158, or a wall type's native `m_function`, the
/// same values followed by foundation, retaining, soffit and core shaft
/// (RE-185).
pub const TYPE_FUNCTION_FIELD: &str = "m_type_function";

/// Releases whose walls' native type Function is measured (RE-185, RE-193).
pub const WALL_TYPE_FUNCTION_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// Prefix of a field carrying the Yes/No value of a project or shared
/// parameter on an element, named by the parameter's name (RE-193).
pub const NAMED_PARAMETER_FIELD_PREFIX: &str = "m_named_parameter:";

/// Prefix of a field carrying the same value on the element's type.
pub const TYPE_NAMED_PARAMETER_FIELD_PREFIX: &str = "m_type_named_parameter:";

/// The name of the parameter Revit's exporter reads `IsExternal` from (RE-193).
pub const IS_EXTERNAL_PARAMETER_NAME: &str = "IsExternal";

/// Field carrying the `IsExternal` Revit's exporter calculates for an element
/// (RE-193), which a parameter named for the property set it is written in
/// takes precedence over ([`crate::ifc::export_content`]).
pub const IS_EXTERNAL_FIELD: &str = "m_is_external";

/// Classes whose `IsExternal` Revit's exporter takes from their type's
/// Function, so one whose Function is not read gets none: a value calculated
/// without it would be a guess (fail closed). A window's host does not give
/// it: on the R2025 models the host wall's value is Revit's on 6 of 18.
pub(super) const IS_EXTERNAL_FUNCTION_CLASSES: &[&str] = &[
    "Wall",
    CURTAIN_WALL_CLASS,
    CURTAIN_PANEL_WALL_CLASS,
    "Door",
    "Window",
    "Floor",
    "BuildingPad",
];

/// Classes whose `IsExternal` is not established: on the BIM4LCA office
/// Revit's export writes most curtain mullions and panels as not external
/// where their curtain wall is, and some as external (RE-193).
pub(super) const IS_EXTERNAL_UNKNOWN_CLASSES: &[&str] = &["CurtainWallMullion", "CurtainWallPanel"];

/// Releases where the calculation's last step, not external, is measured
/// (RE-193). The parameters and a type's Function are read on every release
/// from 2024, as RE-158 read a door's and a floor's.
pub const IS_EXTERNAL_LAST_STEP_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// Give each element of `groups` the values of the project or shared
/// parameters named `<property set>.<property>` for a property of a standard
/// set ([`crate::ifc::standard_property_sets`]) that its own data object holds
/// (RE-194), and, for each such property the element holds no
/// `<property set>.<property>` parameter for, of one named for the property
/// alone, as revit-ifc's `PropertySetEntryMap` looks the qualified name up
/// before the bare one (CLEANROOM.md source 5; RE-227: the BIM4LCA office's
/// `FireExit`, which Revit's export writes as `Pset_DoorCommon.FireExit`),
/// each as a field [`NAMED_PARAMETER_FIELD_PREFIX`]`<set>.<property>`: a
/// Yes/No's 0 or 1 as a boolean, an integer, a text of one or more
/// characters, a positive length in feet, or a length or thermal
/// transmittance as stored. The latest partition's copy decides, so a text
/// emptied there gets nothing, though an older copy holds one: on the
/// BIM4LCA office at building permit 74 columns' `FireRating`. `IsExternal`
/// is [`super::attach_is_external`]'s. Of the models measured, only the BIM4LCA
/// office defines such parameters.
/// `<set>.<property>` parameters left out where the file holds a value,
/// because Revit's export writes none (the maintainer, 2026-10-29: no
/// property value Revit's export omits). `Pset_DoorWindowGlazingType.IsWired`:
/// every one of the BIM4LCA office's 1,337 elements carrying it holds 0, and
/// its export writes it on none (RE-194 §4).
pub(super) const REVIT_OMITTED_PARAMETERS: &[&str] = &["Pset_DoorWindowGlazingType.IsWired"];

/// Field holding whether a wall's top is joined to a roof or floor, from its
/// native record ([`crate::native_walls::native_walls_and_top_joins`],
/// RE-228): Revit's `Pset_WallCommon.ExtendToStructure`.
pub const WALL_TOP_JOINED_FIELD: &str = "m_wall_top_joined";

/// Field set on a wall whose native record holds no `JoinToRoofGStep`,
/// `CutPairGStep` or `WallCutoutGStep`
/// ([`crate::native_walls::native_walls_and_top_joins`], RE-266): its body
/// is as tall as Revit's, and Revit's export writes that as its `Height`.
pub const WALL_UNCUT_FIELD: &str = "m_wall_uncut";

/// Field holding, as a vector of ElementIds, the elements a wall's top join
/// (`JoinToRoofGStep` with `m_baseOrTop` 1) and its `CutPairGStep`s name
/// ([`crate::native_walls::WallRecords::top_cutters`], RE-267): the floors,
/// roofs and footings the export may cut its top at.
pub const WALL_TOP_CUTTERS_FIELD: &str = "m_wall_top_cutters";

/// Field set on a wall whose native record holds no step that cuts, embeds or
/// sweeps its solid ([`crate::native_walls::WallRecords::whole`], RE-270).
pub const WALL_WHOLE_FIELD: &str = "m_wall_whole";

/// Field set, as a vector of ElementIds, on a wall whose native record holds
/// no step but the harmless ones, a roof or floor join and a clean join: the
/// elements its roof and floor joins name, none where it has none
/// ([`crate::native_walls::WallRecords::bounded`], RE-270).
pub const WALL_BOUNDED_FIELD: &str = "m_wall_bounded";

/// Field carrying the name of the phase an element was created in
/// ([`crate::partition_phases`], #755).
pub const PHASE_CREATED_FIELD: &str = "m_phase_created";

/// Field carrying an element's value of the shared parameter `Serial Number`
/// (RE-156).
pub const SERIAL_NUMBER_FIELD: &str = "m_serial_number";

/// Name of the shared parameter whose value Revit's export writes as
/// `SerialNumber` (RE-156).
pub const SERIAL_NUMBER_PARAMETER_NAME: &str = "Serial Number";

/// Field carrying a pipe fitting's nominal diameter, feet (RE-165).
pub const FITTING_NOMINAL_DIAMETER_FIELD: &str = "m_fitting_nominal_diameter";

/// Prefix of the field naming an MEP system an element is a member of: the
/// system's ElementId follows, and the field holds its name, empty where it
/// has none (RE-162).
pub const MEP_SYSTEM_FIELD_PREFIX: &str = "m_mep_system_";

/// Field carrying a duct's shape as IFC names it (`RECTANGULAR`, `ROUND`,
/// `FLATOVAL`), from its type.
// Tracker: B43.
pub const DUCT_SHAPE_FIELD: &str = "m_duct_shape";

/// The classes of the family instances whose connectors are read for joins:
/// the MEP categories, whose families carry connectors.
pub(super) const JOINED_CLASSES: [&str; 18] = [
    "DuctFitting",
    "PipeFitting",
    "DuctTerminal",
    "DuctAccessory",
    "PipeAccessory",
    "PlumbingFixture",
    "Sprinkler",
    "MechanicalEquipment",
    "ElectricalEquipment",
    "ElectricalFixture",
    "LightingFixture",
    "LightingDevice",
    "FireAlarmDevice",
    "DataDevice",
    "CableTrayFitting",
    "ConduitFitting",
    "SpecialtyEquipment",
    "FoodServiceEquipment",
];

/// Where a curtain grid's element data names its curtain wall, as a `u64`
/// at both offsets past the data's ElementId (RE-72).
pub const CURTAIN_GRID_WALL_OFFSETS: [usize; 2] = [85, 478];

/// How far past a curtain wall's record box a part's box may reach and
/// still lie inside it (RE-62): 0.05 ft places every measured part, and
/// 0.01 ft misses mullions whose profile reaches past the wall's box.
pub(super) const CURTAIN_PART_BOX_TOLERANCE_FEET: f64 = 0.05;

/// Field carrying a room's Floor Finish (RE-153).
pub const ROOM_FLOOR_FINISH_FIELD: &str = "m_room_floor_finish";

/// Releases a room's `Qto_SpaceBaseQuantities.Height` is read on (RE-255).
pub const ROOM_HEIGHT_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026];

/// Field holding a room's height, feet (RE-255).
pub const ROOM_HEIGHT_FIELD: &str = "m_room_height";

/// Field carrying a recovered room number.
pub const ROOM_NUMBER_FIELD: &str = "m_number";

/// Field recording where the room number / name came from.
pub const ROOM_PARAMETER_SOURCE_FIELD: &str = "m_room_parameter_source";

/// Field carrying the recovered host ElementId of a door or window.
///
/// The name is the one [`crate::elements::openings::Door`] /
/// [`crate::elements::openings::Window`] already read from
/// schema-field decodes, so the recovered value flows through the
/// existing `recover_door_host` / `recover_window_host` path without
/// a second host concept.
pub const OPENING_HOST_FIELD: &str = "m_hostId";

/// Field recording where [`OPENING_HOST_FIELD`] came from.
pub const OPENING_HOST_PROVENANCE_FIELD: &str = "m_host_provenance";

/// Value of [`OPENING_HOST_PROVENANCE_FIELD`] for the RE-23 carrier.
pub const OPENING_HOST_PROVENANCE: &str = "partition_element_record_reference_list";

/// Field carrying the exported walls a door's or window's reference list
/// names, nearest to its own ElementId first (#439). The
/// host is chosen from them once curtain walls are known
/// ([`super::bind_opening_hosts`]); the field is then removed.
pub(super) const OPENING_HOST_CANDIDATES_FIELD: &str = "m_hostCandidateIds";

/// Each Revit 2023 family instance's type ElementId, type name and family
/// name (RE-109), by ElementId. Its type is the one id in its reference
/// list, wherever it sits, that is a symbol record of its own category with
/// a 2023 name entry ([`crate::partition_names::name_entries_2023`]). Its
/// family is the one id the type's own record references that has a name
/// entry and no element record. An instance whose type or family is not
/// unique gets nothing.
/// Categories whose types are system-family types (RE-111): walls, floors,
/// roofs and ceilings.
pub(super) const SYSTEM_TYPE_CATEGORIES_2023: [i64; 4] = [
    crate::partition_element_records::OST_WALLS,
    crate::partition_element_records::OST_FLOORS,
    -2_000_035,
    -2_000_038,
];

/// How closely an outline read from its sketch lines' own curves must span
/// its element's record box (RE-96), feet.
pub const SKETCH_BOX_TOLERANCE_FEET: f64 = 1e-3;

/// Classes whose plan outline is their sketch's: roofs (RE-50) and, since
/// RE-98, ceilings.
pub const SKETCHED_PRODUCT_CLASSES: &[&str] = &["Roof", "Ceiling"];

/// Cross product (square feet) below which a line's end counts as on
/// another line when testing whether two sketch lines cross (RE-99).
pub const SHAFT_CROSSING_EPS: f64 = 1e-6;

/// How far an element may reach past a shaft's height and still be cut by it
/// (RE-99), feet.
pub const SHAFT_HEIGHT_TOLERANCE_FEET: f64 = 1e-3;

/// Fields holding the plan start and end of the (first) roof edge that
/// defines the roof's slope, model feet (RE-56). Further edges of a gable or
/// hip use [`super::roof_slope_edge_fields`] (RE-199).
pub const ROOF_SLOPE_EDGE_FIELDS: [&str; 4] = [
    "m_roof_slope_edge_start_x",
    "m_roof_slope_edge_start_y",
    "m_roof_slope_edge_end_x",
    "m_roof_slope_edge_end_y",
];

/// Field holding that edge's slope angle from horizontal, radians (RE-56).
pub const ROOF_SLOPE_ANGLE_FIELD: &str = "m_roof_slope_angle";

/// Field holding a roof by extrusion's top, `(u, z)` points along the plane
/// its sketch is in, `u` rising, model feet (RE-211).
pub const ROOF_EXTRUSION_TOP_FIELD: &str = "m_roof_extrusion_top";

/// Field holding a roof by extrusion's underside, as
/// [`ROOF_EXTRUSION_TOP_FIELD`] (RE-211).
pub const ROOF_EXTRUSION_BOTTOM_FIELD: &str = "m_roof_extrusion_bottom";

/// Field holding the model axis a roof by extrusion runs along, 0 for x and
/// 1 for y; `u` is the other horizontal axis (RE-211).
pub const ROOF_EXTRUSION_AXIS_FIELD: &str = "m_roof_extrusion_axis";

/// How closely a roof by extrusion's section must reproduce its record box,
/// feet (RE-211).
pub const ROOF_EXTRUSION_BOX_TOLERANCE_FEET: f64 = 1e-3;

/// Chords a sketch's B-spline is drawn with in a roof by extrusion's
/// section (RE-211), shared among its knot spans by their lengths.
pub const ROOF_EXTRUSION_SPLINE_SEGMENTS: usize = 256;

/// The fewest chords any knot span of a sketch's B-spline is drawn with, so
/// a short span is never skipped (RE-211).
pub const ROOF_EXTRUSION_MIN_SPAN_SEGMENTS: usize = 4;

/// The share of a sketch spline's span its second and last but one points
/// lie in from its ends (RE-211).
pub const ROOF_EXTRUSION_END_SHARE: f64 = 1e-4;

/// How far a sketch spline may stray from the chords it is drawn with in a
/// roof by extrusion's section, feet (RE-211).
pub const ROOF_EXTRUSION_CHORD_FEET: f64 = 1e-4;

/// The most points a sketch's B-spline is drawn with in a roof by
/// extrusion's section; a curve that needs more is left out (RE-211).
pub const ROOF_EXTRUSION_MAX_SPLINE_POINTS: usize = 4096;

/// How far apart two sketch curves' ends may be and still join in a roof by
/// extrusion's profile, feet (RE-211).
pub const ROOF_EXTRUSION_JOIN_FEET: f64 = 1e-4;

/// Field holding the roof's type thickness, its layers summed (RE-56).
pub const ROOF_TYPE_THICKNESS_FIELD: &str = "m_roof_type_thickness";

/// Field holding how far the first defining edge's plane is raised at the
/// edge, feet, written only when not 0 (RE-199); further edges add
/// `_<index>` ([`super::roof_slope_plate_offset_field`]).
pub const ROOF_SLOPE_PLATE_OFFSET_FIELD: &str = "m_roof_slope_plate_offset";

/// Field holding a roof's rafter cut, a [`crate::partition_roof_slopes::RafterCut`]
/// name, where it is read (RE-199).
pub const ROOF_RAFTER_CUT_FIELD: &str = "m_roof_rafter_cut";

/// Field holding a roof's fascia depth, feet, where it is read (RE-199).
pub const ROOF_FASCIA_DEPTH_FIELD: &str = "m_roof_fascia_depth";

/// Classes whose type's layers stack from the top down (RE-57).
pub const STACKED_LAYER_CLASSES: &[&str] = &["Floor", "BuildingPad", "Roof", "Ceiling"];

/// How closely a slab's type layers must add up to its record box's height
/// for them to be its layers, feet.
pub const SLAB_LAYER_HEIGHT_TOLERANCE_FEET: f64 = 1e-4;

/// Field holding a ceiling's finish, the names of its type's finish layers'
/// materials each followed by `;`.
// Tracker: B41.
pub const COVERING_FINISH_FIELD: &str = "m_covering_finish";

/// Field carrying the ElementId of the family/type symbol a placed
/// instance was recovered from (#215, RE-26).
pub const TYPE_SYMBOL_FIELD: &str = "m_type_symbol_id";

/// Field carrying the type symbol's plan width, in feet.
pub const TYPE_PROFILE_WIDTH_FIELD: &str = "m_type_profile_width";

/// Field carrying the type symbol's plan depth, in feet.
pub const TYPE_PROFILE_DEPTH_FIELD: &str = "m_type_profile_depth";

/// Field recording where the section in the two fields above came from.
pub const TYPE_PROFILE_SOURCE_FIELD: &str = "m_type_profile_source";

/// Value of [`TYPE_PROFILE_SOURCE_FIELD`] for the RE-26 carrier.
pub const TYPE_PROFILE_SOURCE: &str = "family_type_symbol_envelope";

/// How far the type section may sit from the instance envelope, in feet.
///
/// The join is accepted only when the symbol's plan extents match the
/// instance's — a symbol that does not is a symbol this decoder has
/// misread, or an instance that is scaled or rotated, and either way
/// the box it would emit is not the one the record states. On
/// `2024_Core_Interior.rvt` all 256 columns pass with a worst
/// disagreement of 8.0e-15 ft.
pub const TYPE_PROFILE_EPS_FEET: f64 = 1e-6;

/// Field naming the Level of an element whose record names it in an id list
/// (RE-174), rather than in the association record.
pub const LEVEL_FROM_LIST_FIELD: &str = "m_level_parameter_from_list";

/// Field naming the highest of the several Levels an element's record names
/// in its id lists: a beam's or brace's Reference Level (RE-175).
pub const REFERENCE_LEVEL_FROM_LIST_FIELD: &str = "m_reference_level_from_list";

/// Field recording where a column's bottom came from, when not its record.
pub const COLUMN_Z_SOURCE_FIELD: &str = "m_column_z_source";

/// Value of [`COLUMN_Z_SOURCE_FIELD`] for a bottom read from the geometry node.
pub const COLUMN_Z_GEOMETRY_NODE: &str = "partition_geometry_node";

/// Bottom, feet, of a column's body after its geometry joins (RE-178). The
/// column's own fields (`m_locationZ`, `m_bboxHeight`) keep its extent.
pub const COLUMN_BODY_BOTTOM_FIELD: &str = "m_column_body_bottom";

/// Top, feet, of a column's body after its geometry joins (RE-178).
pub const COLUMN_BODY_TOP_FIELD: &str = "m_column_body_top";

/// Give a record-backed element `level` as its host Level, recording how.
/// Value of the level-bind source field for a beam bound by its top
/// (`resolve_framing_top_levels`).
pub const FRAMING_TOP_LEVEL_SOURCE: &str = "structural_framing_top_level";

/// Classes Revit's export places by elevation when their record names no
/// Level of their own (RE-68): on Snowdon Towers every unplaced slab edge,
/// light fixture, wall sweep, generic model, hardscape element and stair is
/// on the Level [`crate::element_record_level_refs::elevation_band_level`]
/// gives, while plumbing fixtures follow the Level object they name instead.
pub const ELEVATION_PLACED_CLASSES: &[&str] = &[
    "SlabEdge",
    "LightingFixture",
    "WallSweep",
    "GenericModel",
    "Hardscape",
    "Stair",
];
