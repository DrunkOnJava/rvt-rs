//! Partition element-record headers: ElementId + BuiltInCategory + bbox.
//!
//! Revit 2024 project partitions frame each *element* record with a
//! fixed 88-byte prologue whose first field is the record's own
//! `ElementId` and whose sixth field is the element's Revit
//! `BuiltInCategory` id (a negative, publicly documented constant).
//! The prologue is followed by the element's model-space bounding
//! box as six `f64` feet values.
//!
//! Layout observed on `2024_Core_Interior.rvt`
//! (sha256 `c805df44…`, Revit 2024, eight `Partitions/*` streams,
//! ~190 MiB inflated):
//!
//! ```text
//! +0x00  u64  ElementId of this record (declared in Global/ElemTable)
//! +0x08  u32  record flags (0x00e1 / 0x0111 / 0x0131 / 0x0141 / … )
//! +0x0c  u32  0x0000059f on every observed record of this shape
//! +0x10  u16  0
//! +0x12  i64  BuiltInCategory id, negative (OST_Columns = -2000100)
//! +0x1a  16B  0xff sentinel padding (two u64 slots, never set here)
//! +0x2a  u64  design option ElementId, 0xff×8 = main model (#319)
//! +0x32  u64  container ElementId, 0xffff_ffff_ffff_ffff = none
//! +0x3a  u64  0xff sentinel (never set on observed records)
//! +0x42  u32  placement kind: 0xffffef7f placed / 0xffff8000 symbol
//! +0x46  u32  unattributed (0x0928 / 0x0929 / 0x1929 / 0x09a8 / …)
//! +0x4a  u16  unattributed (0x0766 / 0x0e55 / 0x0788 / 0x04d6)
//! +0x4c  u32  0xffffffff on every observed record of this shape
//! +0x50  8B   bbox marker 46 01 ff ff ff ff ab 05
//! +0x58  48B  bounding box: min x/y/z, max x/y/z, feet, f64 LE
//! +0x88  u32  reference-list length n
//! +0x8c  n*8  n u64 reference slots
//! ```
//!
//! The two fields at `+0x32` and `+0x42` are what separates the
//! records Revit's own exporter emits as building elements from the
//! rest — see [`PartitionElementRecord::is_exported_instance`] and
//! `reports/element-framing/RE-21-partition-element-record-instance-rule.md`.
//!
//! The counted reference list that follows the bounding box is what
//! binds a door or window to its host wall — see
//! [`decode_reference_list`],
//! [`PartitionElementRecord::preceding_reference`] and
//! `reports/element-framing/RE-23-door-window-host-binding.md`.
//!
//! 24880 records of this shape were found on that file; 23470 of
//! them (94.3 %) carry the bbox marker at exactly `+0x50`, which is
//! why the marker offset is treated as fixed rather than searched.
//! Every `ElementId` recovered this way is declared in
//! `Global/ElemTable`, and the scan requires that join — an offset
//! whose leading `u64` is not a declared id is rejected, so a random
//! byte match cannot become an element.
//!
//! # Honesty
//!
//! - The `BuiltInCategory` ids are Autodesk's published API
//!   constants; nothing here is derived from Revit binaries.
//! - Only the fields above are claimed. The sentinel runs, the flags
//!   word, `0x059f`, `+0x46` and `+0x4a` are recorded, not interpreted.
//! - `+0x32` is named `container` because every non-sentinel value
//!   observed is an ElementId declared in `Global/ElemTable`, is
//!   lower than every member id that names it, owns a contiguous
//!   ElementId block spanning several categories at once, and has no
//!   element record of its own. What kind of container Revit calls it
//!   is not claimed.
//! - `+0x42` is named `placement_kind` because it takes exactly two
//!   values on the corpus and they partition the records into placed
//!   instances and type/symbol envelopes. The numbers are recorded,
//!   not decoded.
//! - The `+0x88` list is named a *reference list* because its slots
//!   hold ElementIds and because the slot before the record's own id
//!   is, on every door and window record of the recorded edge, the
//!   host wall Revit's own exporter voids. What the other slots mean
//!   is recorded, not claimed: the leading slot is `3` on every
//!   observed record, and the remaining slots are family / type /
//!   level ids in ascending order. The list length is `u32`-counted,
//!   not sentinel-terminated, and the count is bounded by
//!   [`REFERENCE_LIST_MAX_ENTRIES`] so a garbage word cannot make the
//!   decoder walk off.
//! - One more slot of that list is read, and only for placed
//!   instances: [`PartitionElementRecord::type_symbol_reference`],
//!   the last slot before the record's own id that is itself a
//!   `PLACEMENT_KIND_SYMBOL` record of the same `BuiltInCategory`.
//!   On the recorded edge that is `5755` on all 256 exported
//!   `OST_Columns` instances, which is the `IfcColumnType.Tag` Revit's
//!   own export writes for every one of them (#215, RE-26). What the
//!   remaining slots mean is still recorded, not claimed.
//! - A *second* counted list follows the first, framed identically.
//!   Its last slot is named [`PartitionElementRecord::owner_reference`]
//!   because on every [`OST_SKETCH_LINES`] record of the recorded edge
//!   it names the element whose sketch the line belongs to: the 3106
//!   sketch-line records on `2024_Core_Interior.rvt` name 135 owners,
//!   and 100 of them are exactly the 80 `IfcSlab` + 20
//!   `IfcShadingDevice` ElementIds the reference export tags (#31,
//!   RE-25). On other categories it is only what the byte says.
//! - Category membership alone is **not** an instance claim: a
//!   family symbol carries the same category as its instances.

use crate::{Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where this record shape is corpus-proven: 2024 on
/// `2024_Core_Interior.rvt` against Revit's own export, 2025 on the
/// `Drshelden/IFC-ECS` RE1 projects (MIT) against theirs (RE-32).
pub const PARTITION_ELEMENT_RECORD_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// Autodesk `BuiltInCategory.OST_Columns` — architectural columns.
pub const OST_COLUMNS: i64 = -2_000_100;
/// Autodesk `BuiltInCategory.OST_Walls`.
pub const OST_WALLS: i64 = -2_000_011;
/// Autodesk `BuiltInCategory.OST_Doors`.
pub const OST_DOORS: i64 = -2_000_023;
/// Autodesk `BuiltInCategory.OST_Windows`.
pub const OST_WINDOWS: i64 = -2_000_014;
/// Autodesk `BuiltInCategory.OST_Floors` — floor slabs.
pub const OST_FLOORS: i64 = -2_000_032;
/// Autodesk `BuiltInCategory.OST_BuildingPad` — site/building pads.
///
/// Revit's own exporter emits a building pad as `IfcSlab`, which is
/// why it joins `OST_FLOORS` in the slab recovery: on
/// `2024_Core_Interior.rvt` the single exported slab with no
/// `OST_Floors` record (`Pad:Site Pad`, ElementId 21975) carries this
/// category instead (#212, RE-22).
pub const OST_BUILDING_PAD: i64 = -2_001_263;

/// Autodesk `BuiltInCategory.OST_Rooms` — Revit room elements, which
/// Revit's own exporter emits as `IfcSpace`.
///
/// The category was not assumed: a brute scan of all 23 470 decodable
/// element records on `2024_Core_Interior.rvt` histogrammed every
/// `BuiltInCategory` present and scored each one's instance-rule
/// selection against the 116 room ElementIds the reference export
/// carries in `IfcSpaceType.Tag`. `OST_Rooms` selects **116, exactly
/// that set** — no false positive, no miss — and no other category
/// intersects it at all (#90, RE-29).
pub const OST_ROOMS: i64 = -2_000_160;

/// Autodesk `BuiltInCategory.OST_SketchLines` — the lines of a
/// sketch (floor / roof / ceiling boundary, extrusion profile).
///
/// Records of this category carry the sketch segment's own bounding
/// box, which for a plan boundary line is the segment itself, and
/// name the sketched element in
/// [`PartitionElementRecord::owner_reference`] (#31, RE-25).
pub const OST_SKETCH_LINES: i64 = -2_000_045;

// RE-33: categories whose element records export as typed products.
// Each was found the way `OST_Rooms` was: a census of every decodable
// record's category, scored against the `Tag` sets of Revit's own
// exports (see `PRODUCT_RECORD_CATEGORIES`). The names are the public
// `BuiltInCategory` enum members.

/// Autodesk `BuiltInCategory.OST_Furniture`.
pub const OST_FURNITURE: i64 = -2_000_080;
/// Autodesk `BuiltInCategory.OST_Casework`.
pub const OST_CASEWORK: i64 = -2_001_000;
/// Autodesk `BuiltInCategory.OST_PlumbingFixtures`.
pub const OST_PLUMBING_FIXTURES: i64 = -2_001_160;
/// Autodesk `BuiltInCategory.OST_SpecialityEquipment`.
pub const OST_SPECIALITY_EQUIPMENT: i64 = -2_001_350;
/// Autodesk `BuiltInCategory.OST_Ceilings`.
pub const OST_CEILINGS: i64 = -2_000_038;
/// Autodesk `BuiltInCategory.OST_CurtainWallMullions`.
pub const OST_CURTAIN_WALL_MULLIONS: i64 = -2_000_171;
/// Autodesk `BuiltInCategory.OST_CurtainWallPanels`.
pub const OST_CURTAIN_WALL_PANELS: i64 = -2_000_170;
/// Autodesk `BuiltInCategory.OST_StairsRailing` — railings.
pub const OST_STAIRS_RAILING: i64 = -2_000_126;
/// Autodesk `BuiltInCategory.OST_Cornices` — wall sweeps.
pub const OST_CORNICES: i64 = -2_000_181;
/// Autodesk `BuiltInCategory.OST_DuctCurves` — ducts.
pub const OST_DUCT_CURVES: i64 = -2_008_000;
/// Autodesk `BuiltInCategory.OST_DuctFitting`.
pub const OST_DUCT_FITTING: i64 = -2_008_010;
/// Autodesk `BuiltInCategory.OST_PipeCurves` — pipes.
pub const OST_PIPE_CURVES: i64 = -2_008_044;
/// Autodesk `BuiltInCategory.OST_PipeFitting`.
pub const OST_PIPE_FITTING: i64 = -2_008_049;

// RE-36: structural categories. Nearly every frame of these uses the
// second prologue, so their instances export through RE-34's inferred
// ElementIds; the oracle is the VIM export of the same sample
// (`reports/element-framing/RE-36-structural-categories.md`).

/// Autodesk `BuiltInCategory.OST_StructuralFraming` — beams, joists,
/// girders and purlins.
pub const OST_STRUCTURAL_FRAMING: i64 = -2_001_320;
/// Autodesk `BuiltInCategory.OST_StructuralColumns`.
pub const OST_STRUCTURAL_COLUMNS: i64 = -2_001_330;
/// Autodesk `BuiltInCategory.OST_StructuralFoundation` — isolated
/// footings, wall foundations and foundation slabs.
pub const OST_STRUCTURAL_FOUNDATION: i64 = -2_001_300;
/// Autodesk `BuiltInCategory.OST_GenericModel`.
pub const OST_GENERIC_MODEL: i64 = -2_000_151;

// RE-37: fixtures, site and circulation categories. Scored like RE-33's,
// against the `Tag` sets of Revit's own exports, with the ids of RE-35.

/// Autodesk `BuiltInCategory.OST_LightingFixtures`.
pub const OST_LIGHTING_FIXTURES: i64 = -2_001_120;
/// Autodesk `BuiltInCategory.OST_DuctTerminal` — air terminals.
pub const OST_DUCT_TERMINAL: i64 = -2_008_013;
/// Autodesk `BuiltInCategory.OST_FoodServiceEquipment`.
pub const OST_FOOD_SERVICE_EQUIPMENT: i64 = -2_001_043;
/// Autodesk `BuiltInCategory.OST_Planting`.
pub const OST_PLANTING: i64 = -2_001_360;
/// Autodesk `BuiltInCategory.OST_Parking`.
pub const OST_PARKING: i64 = -2_001_180;
/// Autodesk `BuiltInCategory.OST_Entourage`.
pub const OST_ENTOURAGE: i64 = -2_001_370;
/// Autodesk `BuiltInCategory.OST_Hardscape`.
pub const OST_HARDSCAPE: i64 = -2_001_036;
/// Autodesk `BuiltInCategory.OST_VerticalCirculation` — elevators.
pub const OST_VERTICAL_CIRCULATION: i64 = -2_001_052;
/// Autodesk `BuiltInCategory.OST_Ramps`.
pub const OST_RAMPS: i64 = -2_000_180;

// #323 (RE-39): stairs and the parts Revit's export aggregates under them.
// The stair exports as a bodiless `IfcStair` whose runs, landings and
// stringers name it in their reference lists.

/// Autodesk `BuiltInCategory.OST_Stairs`.
pub const OST_STAIRS: i64 = -2_000_120;
/// Autodesk `BuiltInCategory.OST_StairsRuns`.
pub const OST_STAIRS_RUNS: i64 = -2_000_919;
/// Autodesk `BuiltInCategory.OST_StairsLandings`.
pub const OST_STAIRS_LANDINGS: i64 = -2_000_920;
/// Autodesk `BuiltInCategory.OST_StairsStringerCarriage` — stair
/// stringers.
pub const OST_STAIRS_STRINGER_CARRIAGE: i64 = -2_000_123;

/// Autodesk `BuiltInCategory.OST_Roofs`. Exported as `IfcRoof` with the
/// record's box, the entity Revit's own export gives every Snowdon roof
/// (#323).
pub const OST_ROOFS: i64 = -2_000_035;

/// Autodesk `BuiltInCategory.OST_EdgeSlab` — slab edges. Revit's own export
/// writes them as `IfcBuildingElementProxy`; on Snowdon Towers it holds all
/// 59 slab edges outside a non-primary design option and none of the 9 in
/// one (RE-40, #319).
pub const OST_EDGE_SLAB: i64 = -2_001_392;

/// Categories whose placed element records export directly as typed IFC
/// products, with the class name each is decoded as (RE-33).
///
/// On every file where a category occurs, the RE-21 instance rule (no
/// container reference, placed) selects only ElementIds that Revit's own
/// export carries as `Tag`, with no false positive:
///
/// - `Drshelden/IFC-ECS` RE1 (Revit 2025, MIT): furniture 13, casework
///   10, plumbing fixtures 7, specialty equipment 9, ceilings 6,
///   mullions 10, panels 2, railings 1 (Architecture); ducts 13, duct
///   fittings 14, pipes 3, pipe fittings 3 (Mechanical); pipes 3, pipe
///   fittings 6 (Plumbing);
/// - Snowdon Towers Architectural (Revit 2024, local only): wall sweeps
///   36, furniture 1.
///
/// RE-36 adds structural framing, structural columns, structural
/// foundations and generic models. Nearly all of their frames use the
/// second prologue, so their ids are the enclosing records' (RE-35). On
/// Snowdon Towers Structural every one of them the VIM export of the model
/// carries has the record's category there (914 framing members, 74
/// columns, 90 foundations, 91 generic models, none under another
/// category), and 260 of the 261 generic models on the architectural
/// sample are in Revit's IFC4 export. The 261st is in a non-primary design
/// option, which the exporter now leaves out (RE-40).
///
/// RE-37 adds lighting fixtures, air terminals, food-service equipment,
/// planting, parking, entourage, hardscape, vertical circulation and
/// ramps, each exported element a `Tag` of Revit's own export with none
/// outside it: 447 lighting fixtures, 26 food-service items, 157 site
/// elements, 2 ramps and 2 elevators on Snowdon Towers; 7 air terminals on
/// RE1 Mechanical; 12 lighting fixtures on RE1 Electrical.
///
/// RE-39 adds stairs with their runs, landings and stringers, and roofs.
/// RE-40 adds slab edges, once elements in non-primary design options are
/// left out: all 59 exported on Snowdon Towers are Revit's.
pub const PRODUCT_RECORD_CATEGORIES: [(i64, &str); 32] = [
    (OST_FURNITURE, "Furniture"),
    (OST_CASEWORK, "Casework"),
    (OST_PLUMBING_FIXTURES, "PlumbingFixture"),
    (OST_SPECIALITY_EQUIPMENT, "SpecialtyEquipment"),
    (OST_CEILINGS, "Ceiling"),
    (OST_CURTAIN_WALL_MULLIONS, "CurtainWallMullion"),
    (OST_CURTAIN_WALL_PANELS, "CurtainWallPanel"),
    (OST_STAIRS_RAILING, "Railing"),
    (OST_CORNICES, "WallSweep"),
    (OST_DUCT_CURVES, "Duct"),
    (OST_DUCT_FITTING, "DuctFitting"),
    (OST_PIPE_CURVES, "Pipe"),
    (OST_PIPE_FITTING, "PipeFitting"),
    (OST_STRUCTURAL_FRAMING, "StructuralFraming"),
    (OST_STRUCTURAL_COLUMNS, "StructuralColumn"),
    (OST_STRUCTURAL_FOUNDATION, "StructuralFoundation"),
    (OST_GENERIC_MODEL, "GenericModel"),
    (OST_LIGHTING_FIXTURES, "LightingFixture"),
    (OST_DUCT_TERMINAL, "DuctTerminal"),
    (OST_FOOD_SERVICE_EQUIPMENT, "FoodServiceEquipment"),
    (OST_PLANTING, "Planting"),
    (OST_PARKING, "Parking"),
    (OST_ENTOURAGE, "Entourage"),
    (OST_HARDSCAPE, "Hardscape"),
    (OST_VERTICAL_CIRCULATION, "VerticalCirculation"),
    (OST_RAMPS, "Ramp"),
    (OST_STAIRS, "Stair"),
    (OST_STAIRS_RUNS, "StairsRun"),
    (OST_STAIRS_LANDINGS, "StairsLanding"),
    (OST_STAIRS_STRINGER_CARRIAGE, "StairsStringer"),
    (OST_ROOFS, "Roof"),
    (OST_EDGE_SLAB, "SlabEdge"),
];

/// Smallest bounding-box extent, in feet, a placed instance needs on every
/// axis to count as having a 3D body ([`PartitionElementRecord::has_volume`]).
pub const VOLUME_EPSILON_FEET: f64 = 1e-6;

/// Lower bound of the Revit `BuiltInCategory` id band.
pub const BUILTIN_CATEGORY_MIN: i64 = -2_100_000;
/// Upper bound of the Revit `BuiltInCategory` id band.
pub const BUILTIN_CATEGORY_MAX: i64 = -1_990_000;

/// Offset of the `BuiltInCategory` id from the record start.
pub const CATEGORY_OFFSET: usize = 0x12;
/// Offset of the container ElementId reference from the record start.
pub const CONTAINER_OFFSET: usize = 0x32;
/// Sentinel value of the container reference meaning "no container".
pub const CONTAINER_NONE: u64 = u64::MAX;
/// Offset of the design-option ElementId from the record start: `0xff`×8
/// for an element of the main model, otherwise the `OST_DesignOptions`
/// record the element belongs to (#319, RE-40). The same offset holds it in
/// the second prologue (RE-30).
pub const DESIGN_OPTION_OFFSET: usize = 0x2a;
/// Offset of the placement-kind word from the record start.
pub const PLACEMENT_KIND_OFFSET: usize = 0x42;
/// Placement-kind value carried by a placed element instance.
pub const PLACEMENT_KIND_INSTANCE: u32 = 0xffff_ef7f;
/// Placement-kind value carried by a family/type symbol envelope.
pub const PLACEMENT_KIND_SYMBOL: u32 = 0xffff_8000;
/// Offset of the bounding-box marker from the record start.
pub const BBOX_MARKER_OFFSET: usize = 0x50;
/// Offset of the six bounding-box doubles from the record start.
pub const BBOX_OFFSET: usize = 0x58;
/// Minimum bytes a complete record header occupies.
pub const RECORD_MIN_LEN: usize = BBOX_OFFSET + 48;
/// Offset of the counted reference list, immediately after the box.
pub const REFERENCE_LIST_OFFSET: usize = RECORD_MIN_LEN;
/// Largest reference-list length the decoder will accept.
///
/// The longest list observed on `2024_Core_Interior.rvt` is 101 slots
/// (an `OST_Walls` record); the bound is an order of magnitude above
/// that so an unrelated `u32` cannot make the scan read megabytes.
pub const REFERENCE_LIST_MAX_ENTRIES: usize = 1024;

/// Marker that precedes the bounding box on Revit 2024 files.
///
/// The marker is per release (RE-32): a `u16`, `0xFF`×4, and a `u16`
/// equal to the release's `Global/ElemTable` header constant plus 40
/// (1411 + 40 = `0x05ab` on 2024). Use [`bbox_marker`] for a given
/// release.
pub const BBOX_MARKER: [u8; 8] = [0x46, 0x01, 0xff, 0xff, 0xff, 0xff, 0xab, 0x05];

/// The bbox marker of Revit 2025 files: `0x0159`, `0xFF`×4, `0x05d3`
/// (1451 + 40), measured on the RE1 projects and on two other 2025
/// files (RE-32).
pub const BBOX_MARKER_2025: [u8; 8] = [0x59, 0x01, 0xff, 0xff, 0xff, 0xff, 0xd3, 0x05];

/// The bbox marker of `revit_version`, or `None` where the record shape
/// is not proven (fail closed).
pub fn bbox_marker(revit_version: u32) -> Option<[u8; 8]> {
    match revit_version {
        2024 => Some(BBOX_MARKER),
        2025 => Some(BBOX_MARKER_2025),
        _ => None,
    }
}

/// A decoded partition element-record header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionElementRecord {
    /// Stream the record was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the record in the concatenated inflated stream.
    pub offset: usize,
    /// The record's own Revit ElementId (cross-checked against ElemTable).
    pub element_id: u32,
    /// Unattributed flags word at `+0x08`.
    pub flags: u32,
    /// Revit `BuiltInCategory` id (negative).
    pub builtin_category: i64,
    /// Raw container reference at `+0x32`; [`CONTAINER_NONE`] when unset.
    pub container: u64,
    /// Raw placement-kind word at `+0x42`.
    pub placement_kind: u32,
    /// Model bounding box in feet: `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    pub bbox_feet: [f64; 6],
    /// Reference slot immediately before this record's own ElementId
    /// in the counted list at `+0x88` (#222, RE-23).
    ///
    /// `None` when the list does not decode, does not name the
    /// record's own id, names it in the first slot, or when the slot
    /// before it holds a value outside the `u32` ElementId range.
    ///
    /// On a door or window record this is the host wall; on other
    /// categories it is only what the byte says. Nothing here checks
    /// that the value is a wall — the caller does
    /// (`partition_schema_mvp::opening_instances_from_records`).
    pub preceding_reference: Option<u32>,
    /// Last slot of the *second* counted reference list at `+0x88`
    /// (#31, RE-25).
    ///
    /// `None` when either list does not decode, when the second list
    /// is empty, or when its last slot is outside the `u32`
    /// ElementId range.
    ///
    /// On an [`OST_SKETCH_LINES`] record this is the element whose
    /// sketch the line belongs to. Nothing here checks what kind of
    /// element that is — the caller does
    /// (`element_record_plan_profiles`).
    pub owner_reference: Option<u32>,
    /// The first counted reference list at `+0x88`, verbatim (#215,
    /// #228, RE-26).
    ///
    /// Empty when the list does not decode. [`Self::preceding_reference`]
    /// is one read of this list; [`Self::type_symbol_reference`] is
    /// another. The slots are kept as `u64` so a caller sees the list
    /// exactly as framed.
    #[serde(default)]
    pub references: Vec<u64>,
    /// `true` when the frame carries no ElementId at `+0x00` (the second
    /// prologue, RE-30) and [`Self::element_id`] is the id of the partition
    /// record the frame sits in ([`assign_second_prologue_ids`], RE-35).
    /// `false` for a frame that starts its record, with the id at `+0x00`.
    #[serde(default)]
    pub id_from_enclosing_record: bool,
    /// The design option the element belongs to, from `+0x2a`
    /// ([`DESIGN_OPTION_OFFSET`]). `None` for the main model, and for a
    /// value outside the `u32` ElementId range.
    ///
    /// Whether the option is its set's primary one is document-level
    /// knowledge: see [`crate::partition_design_options`].
    #[serde(default)]
    pub design_option: Option<u32>,
}

impl PartitionElementRecord {
    /// Plan-footprint origin, quantised to 1e-4 ft, for de-duplication.
    pub fn footprint_key(&self) -> (i64, i64, i64) {
        (
            quantise(self.bbox_feet[0]),
            quantise(self.bbox_feet[1]),
            quantise(self.bbox_feet[2]),
        )
    }

    /// Plan centre of the bounding box, in feet.
    pub fn plan_centre_feet(&self) -> (f64, f64) {
        (
            (self.bbox_feet[0] + self.bbox_feet[3]) * 0.5,
            (self.bbox_feet[1] + self.bbox_feet[4]) * 0.5,
        )
    }

    /// Bounding-box extents `(dx, dy, dz)` in feet.
    pub fn extents_feet(&self) -> (f64, f64, f64) {
        (
            self.bbox_feet[3] - self.bbox_feet[0],
            self.bbox_feet[4] - self.bbox_feet[1],
            self.bbox_feet[5] - self.bbox_feet[2],
        )
    }

    /// Whether the record's bounding box has extent on all three axes
    /// (more than [`VOLUME_EPSILON_FEET`]).
    ///
    /// A placed instance with no volume is a 2D symbol family: a floor
    /// drain drawn in plan, a wheelchair turning circle, an accessible
    /// clearance zone. Revit's IFC export leaves such an instance out,
    /// because it has no 3D body to write. On Snowdon Towers all 60 placed
    /// instances without volume (45 specialty equipment, 15 plumbing
    /// fixtures) are missing from Revit's export, and none of the 4,077
    /// instances the export does hold, nor any on `2024_Core_Interior.rvt`
    /// or the RE1 models, lacks volume (#309).
    pub fn has_volume(&self) -> bool {
        let (dx, dy, dz) = self.extents_feet();
        [dx, dy, dz]
            .iter()
            .all(|extent| extent.is_finite() && *extent > VOLUME_EPSILON_FEET)
    }

    /// True when the bbox is expressed in family-local coordinates
    /// (centred on the plan origin) rather than project coordinates.
    ///
    /// Retained as a diagnostic. It is a *proxy* for
    /// [`Self::is_type_symbol`] and a strictly weaker one: on
    /// `2024_Core_Interior.rvt` it agrees on all 17 `OST_Columns`
    /// symbols but misses the 15 `OST_Doors`, 2 `OST_Windows` and 1
    /// `OST_Walls` symbol whose envelope is centred on only one axis.
    pub fn is_family_local(&self) -> bool {
        is_family_local_bbox(&self.bbox_feet)
    }

    /// The container ElementId this record belongs to, if any.
    ///
    /// `None` when `+0x32` carries [`CONTAINER_NONE`], and also when
    /// it carries a value outside the `u32` ElementId range — an
    /// unrecognised encoding is not turned into an id.
    pub fn container_element_id(&self) -> Option<u32> {
        if self.container == CONTAINER_NONE || self.container > u64::from(u32::MAX) {
            return None;
        }
        Some(self.container as u32)
    }

    /// True when `+0x32` is set, i.e. the record is a member of a
    /// container element rather than a standalone element.
    pub fn is_container_member(&self) -> bool {
        self.container != CONTAINER_NONE
    }

    /// True when `+0x42` marks a placed element instance.
    pub fn is_placed_instance(&self) -> bool {
        self.placement_kind == PLACEMENT_KIND_INSTANCE
    }

    /// True when `+0x42` marks a family/type symbol envelope.
    pub fn is_type_symbol(&self) -> bool {
        self.placement_kind == PLACEMENT_KIND_SYMBOL
    }

    /// The instance rule (#211): a record is a standalone placed
    /// instance — the thing Revit's own exporter emits as a building
    /// element — when it carries no container reference **and** its
    /// placement kind is [`PLACEMENT_KIND_INSTANCE`].
    ///
    /// Measured on `2024_Core_Interior.rvt` against the ElementId set
    /// Revit's full IFC export tags: `OST_Walls` 360/360, `OST_Doors`
    /// 132/132, `OST_Windows` 6/6, `OST_Columns` 256/256 — exact id
    /// sets, no false positives, no misses. It is a *test*, not a
    /// heuristic: it replaces the family-local bbox proxy and the
    /// highest-id-per-footprint collapse that #204 used for columns.
    ///
    /// RE-22 extended the same measurement to `OST_Floors` and found
    /// the rule exact there too, once the reference side is read
    /// correctly: the 99 selected ElementIds are *all* exported — 79
    /// as `IfcSlab` and 20 as `IfcShadingDevice` — so there are no
    /// false positives, and the 80th exported slab (`Pad:Site Pad`,
    /// 21975) simply carries [`OST_BUILDING_PAD`] rather than
    /// `OST_Floors`. `OST_Floors` + `OST_BuildingPad` under this rule
    /// reproduce the export's 80 `IFCSLAB` and 20 `IFCSHADINGDEVICE`
    /// id sets exactly (#212).
    pub fn is_exported_instance(&self) -> bool {
        !self.is_container_member() && self.is_placed_instance()
    }

    /// The family/type symbol this instance is placed from (#215,
    /// RE-26).
    ///
    /// `symbols` is the ElementId set of the records of the **same**
    /// `BuiltInCategory` that carry [`PLACEMENT_KIND_SYMBOL`] — the
    /// type envelopes RE-21 §5 identified. The answer is the last
    /// slot of [`Self::references`] **before** the record's own
    /// ElementId that is in that set.
    ///
    /// Slots after the record's own id are ignored on purpose: RE-23
    /// §3 measured that 81 door records carry a trailing symbol slot
    /// that is not the door's own type, and every column record on
    /// the recorded edge carries `75407` there while its type is
    /// `5755`.
    ///
    /// Measured on `2024_Core_Interior.rvt`: all 256 exported
    /// `OST_Columns` instances have exactly one candidate before
    /// their own id and it is `5755` (`Column_Sqaure : 24" x 24"`),
    /// which is the `IfcColumnType.Tag` Revit's own export writes for
    /// every one of them.
    pub fn type_symbol_reference(&self, symbols: &BTreeSet<u32>) -> Option<u32> {
        let own = u64::from(self.element_id);
        let index = self.references.iter().position(|slot| *slot == own)?;
        self.references[..index]
            .iter()
            .rev()
            .filter(|slot| **slot <= u64::from(u32::MAX))
            .map(|slot| *slot as u32)
            .find(|id| symbols.contains(id))
    }
}

fn quantise(v: f64) -> i64 {
    (v * 10_000.0).round() as i64
}

/// A bbox centred on the plan origin is a family/type definition
/// envelope, not a placed instance: Revit stores symbol geometry in
/// family coordinates, so `min_x == -max_x` and `min_y == -max_y`.
pub fn is_family_local_bbox(bbox: &[f64; 6]) -> bool {
    (bbox[0] + bbox[3]).abs() < 1e-6 && (bbox[1] + bbox[4]).abs() < 1e-6
}

/// Whether this release's partition element-record shape is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    PARTITION_ELEMENT_RECORD_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
        && bbox_marker(revit_version).is_some()
}

fn read_u64(buf: &[u8], off: usize) -> Option<u64> {
    buf.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    read_u64(buf, off).map(f64::from_bits)
}

fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    buf.get(off..off.checked_add(2)?)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
}

/// Decode the counted reference list whose `u32` length prefix sits
/// at `at` (#222, RE-23).
///
/// Returns `None` when the length word is zero, exceeds
/// [`REFERENCE_LIST_MAX_ENTRIES`], or would run past the end of
/// `buf`. The slots are returned verbatim as `u64`; a slot outside
/// the `u32` ElementId range is kept rather than dropped, so callers
/// see the list exactly as framed.
pub fn decode_reference_list(buf: &[u8], at: usize) -> Option<Vec<u64>> {
    let count = buf
        .get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))? as usize;
    if count == 0 || count > REFERENCE_LIST_MAX_ENTRIES {
        return None;
    }
    let start = at.checked_add(4)?;
    let end = start.checked_add(count.checked_mul(8)?)?;
    if end > buf.len() {
        return None;
    }
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        out.push(read_u64(buf, start + index * 8)?);
    }
    Some(out)
}

/// Decode the last slot of the *second* counted reference list whose
/// first list's `u32` length prefix sits at `at` (#31, RE-25).
///
/// Both lists are framed identically — a `u32` count followed by that
/// many `u64` slots — so the second starts `4 + 8 * n1` bytes after
/// the first. Returns `None` unless both decode through
/// [`decode_reference_list`] and the second's last slot is a non-zero
/// `u32`-range ElementId.
pub fn decode_owner_reference(buf: &[u8], at: usize) -> Option<u32> {
    let first = decode_reference_list(buf, at)?;
    let second_at = at
        .checked_add(4)?
        .checked_add(first.len().checked_mul(8)?)?;
    let second = decode_reference_list(buf, second_at)?;
    let last = *second.last()?;
    if last == 0 || last > u64::from(u32::MAX) {
        return None;
    }
    Some(last as u32)
}

/// The slot immediately before `element_id`'s last occurrence in
/// `entries`, when that slot is a `u32`-range ElementId.
fn slot_before(entries: &[u64], element_id: u32) -> Option<u32> {
    let target = u64::from(element_id);
    let index = entries.iter().rposition(|value| *value == target)?;
    let previous = *entries.get(index.checked_sub(1)?)?;
    if previous == 0 || previous > u64::from(u32::MAX) {
        return None;
    }
    Some(previous as u32)
}

/// Decode one Revit 2024 record at `offset`, fail-closed.
///
/// `declared_ids` is the `Global/ElemTable` id set; a record whose
/// leading `u64` is not declared there is rejected outright.
pub fn decode_at(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
) -> Option<PartitionElementRecord> {
    decode_at_with_marker(stream, buf, offset, declared_ids, &BBOX_MARKER)
}

/// [`decode_at`] for the release whose bbox marker is `marker`
/// (see [`bbox_marker`]).
pub fn decode_at_with_marker(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
    marker: &[u8; 8],
) -> Option<PartitionElementRecord> {
    if offset.checked_add(RECORD_MIN_LEN)? > buf.len() {
        return None;
    }
    let raw_id = read_u64(buf, offset)?;
    if carries_no_element_id(raw_id) {
        return None;
    }
    let element_id = raw_id as u32;
    if !declared_ids.contains(&element_id) {
        return None;
    }
    decode_frame_as(stream, buf, offset, element_id, marker)
}

/// Decode the frame at `offset` as the record of `element_id`, which the
/// caller has already established: the ElementId at `+0x00` for a
/// first-prologue frame, or its enclosing record's id
/// ([`assign_second_prologue_ids`]) for a second-prologue frame. Everything from `+0x10` on is read and
/// validated exactly as for any record.
pub fn decode_frame_as(
    stream: &str,
    buf: &[u8],
    offset: usize,
    element_id: u32,
    marker: &[u8; 8],
) -> Option<PartitionElementRecord> {
    if offset.checked_add(RECORD_MIN_LEN)? > buf.len() {
        return None;
    }
    if u16::from_le_bytes([buf[offset + 0x10], buf[offset + 0x11]]) != 0 {
        return None;
    }
    let builtin_category = read_u64(buf, offset + CATEGORY_OFFSET)? as i64;
    if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&builtin_category) {
        return None;
    }
    if buf[offset + BBOX_MARKER_OFFSET..offset + BBOX_MARKER_OFFSET + 8] != marker[..] {
        return None;
    }
    let mut bbox_feet = [0.0f64; 6];
    for (index, slot) in bbox_feet.iter_mut().enumerate() {
        let value = read_f64(buf, offset + BBOX_OFFSET + index * 8)?;
        if !value.is_finite() {
            return None;
        }
        *slot = value;
    }
    for axis in 0..3 {
        if bbox_feet[axis + 3] < bbox_feet[axis] {
            return None;
        }
    }
    let flags = read_u64(buf, offset + 0x08).map(|v| (v & 0xffff_ffff) as u32)?;
    let container = read_u64(buf, offset + CONTAINER_OFFSET)?;
    let placement_kind =
        read_u64(buf, offset + PLACEMENT_KIND_OFFSET).map(|v| (v & 0xffff_ffff) as u32)?;
    let references = offset
        .checked_add(REFERENCE_LIST_OFFSET)
        .and_then(|at| decode_reference_list(buf, at))
        .unwrap_or_default();
    let preceding_reference = slot_before(&references, element_id);
    let owner_reference = offset
        .checked_add(REFERENCE_LIST_OFFSET)
        .and_then(|at| decode_owner_reference(buf, at));
    let design_option = frame_design_option(buf, offset);
    Some(PartitionElementRecord {
        stream: stream.to_string(),
        offset,
        element_id,
        flags,
        builtin_category,
        container,
        placement_kind,
        bbox_feet,
        preceding_reference,
        owner_reference,
        references,
        id_from_enclosing_record: false,
        design_option,
    })
}

/// The design option at `+0x2a` of the frame at `offset`
/// ([`PartitionElementRecord::design_option`]).
pub fn frame_design_option(buf: &[u8], offset: usize) -> Option<u32> {
    read_u64(buf, offset.checked_add(DESIGN_OPTION_OFFSET)?)
        .filter(|value| *value != u64::MAX)
        .and_then(|value| u32::try_from(value).ok())
}

/// Whether the frame at `offset` is a placed instance under the RE-21
/// rule: no container reference and the placed-instance kind. Both fields
/// sit at the same offsets in the second prologue (RE-33).
fn is_instance_frame(buf: &[u8], offset: usize) -> bool {
    read_u64(buf, offset + CONTAINER_OFFSET) == Some(CONTAINER_NONE)
        && read_u64(buf, offset + PLACEMENT_KIND_OFFSET)
            .is_some_and(|v| (v & 0xffff_ffff) as u32 == PLACEMENT_KIND_INSTANCE)
}

/// Length of the header that opens every partition record: the `u64`
/// ElementId, the `u32` record size, the `u16` prologue constant
/// ([`record_prologue_constant`]) and a `u16` count (RE-35).
pub const PARTITION_RECORD_HEADER_LEN: usize = 16;

/// Length of the trailer that closes every partition record: an unread
/// `u64`, a `u32` flag word (`0` or `0x0100_0000`) and the record size
/// again (RE-35).
pub const PARTITION_RECORD_TRAILER_LEN: usize = 16;

/// The `u16` at `+0x0c` of every partition record header: the release
/// constant plus 28 (`0x059f` on 2024, `0x05c7` on 2025), which is 12
/// below the constant the release's [`bbox_marker`] ends with.
pub fn record_prologue_constant(marker: &[u8; 8]) -> u16 {
    u16::from_le_bytes([marker[6], marker[7]]).wrapping_sub(12)
}

/// One record of a partition's leading record chain (RE-35).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionRecordSpan {
    /// Offset of the record header.
    pub start: usize,
    /// Offset one past the record body, where its trailer starts.
    pub end: usize,
    /// The `u64` the header opens with: the ElementId the record belongs
    /// to. It is declared in `Global/ElemTable` for every project element;
    /// the caller checks.
    pub element_id: u64,
}

/// The leading chain of records in one inflated partition (RE-35).
///
/// A partition opens with one record per element, back to back: a
/// [`PARTITION_RECORD_HEADER_LEN`]-byte header, the body, and a
/// [`PARTITION_RECORD_TRAILER_LEN`]-byte trailer that repeats the size.
/// A frame with the element's ElementId at `+0x00` (the first prologue)
/// starts its record, so its header *is* the record header; a
/// second-prologue frame (RE-30) sits inside its element's record body.
///
/// The walk starts at offset 0 and stops at the first position that is
/// not a header whose trailer echoes its size. What follows (the records
/// of loaded families' own documents, with their own ElementIds) is not
/// part of the chain. Every step moves at least
/// `PARTITION_RECORD_HEADER_LEN + PARTITION_RECORD_TRAILER_LEN` bytes, so
/// the walk is linear in `buf`.
pub fn partition_record_chain(buf: &[u8], marker: &[u8; 8]) -> Vec<PartitionRecordSpan> {
    let constant = record_prologue_constant(marker);
    let mut chain = Vec::new();
    let mut at = 0usize;
    while let Some(element_id) = read_u64(buf, at) {
        let Some(size) = read_u32(buf, at + 8).map(|v| v as usize) else {
            break;
        };
        if size < PARTITION_RECORD_HEADER_LEN || read_u16(buf, at + 0x0c) != Some(constant) {
            break;
        }
        let Some(end) = at.checked_add(size) else {
            break;
        };
        let flags = end.checked_add(8).and_then(|t| read_u32(buf, t));
        let echo = end.checked_add(12).and_then(|t| read_u32(buf, t));
        if !matches!(flags, Some(0 | 0x0100_0000)) || echo.map(|v| v as usize) != Some(size) {
            break;
        }
        chain.push(PartitionRecordSpan {
            start: at,
            end,
            element_id,
        });
        at = end + PARTITION_RECORD_TRAILER_LEN;
    }
    chain
}

/// The record of `chain` whose body contains `offset`.
pub fn enclosing_record(
    chain: &[PartitionRecordSpan],
    offset: usize,
) -> Option<&PartitionRecordSpan> {
    let index = chain.partition_point(|span| span.start <= offset);
    let span = chain.get(index.checked_sub(1)?)?;
    (offset < span.end).then_some(span)
}

/// [`PartitionElementRecord::has_volume`] read straight off the frame at
/// `offset`, for frames that are never decoded.
fn frame_has_volume(buf: &[u8], offset: usize) -> bool {
    let mut bbox = [0.0f64; 6];
    for (index, slot) in bbox.iter_mut().enumerate() {
        match read_f64(buf, offset + BBOX_OFFSET + index * 8) {
            Some(value) if value.is_finite() => *slot = value,
            _ => return false,
        }
    }
    (0..3).all(|axis| bbox[axis + 3] - bbox[axis] > VOLUME_EPSILON_FEET)
}

/// ElementIds for the second-prologue frames of one partition, by frame
/// offset (RE-35).
///
/// A second-prologue frame (RE-30) carries no ElementId at `+0x00`, but it
/// sits inside its element's record in the partition's leading record
/// chain ([`partition_record_chain`]), whose header carries the ElementId.
/// A placed-instance frame (RE-21 rule) whose enclosing record's id is
/// declared in `Global/ElemTable` gets that id. Frames outside the chain
/// (loaded families' own documents) and frames whose record id is not
/// declared get none (fail closed).
///
/// On every first-prologue frame of the corpus (23,470 on
/// `2024_Core_Interior.rvt`, 2,075 on Snowdon Towers, 152 / 50 / 136 on
/// the RE1 models) the enclosing record starts at the frame and carries
/// the same id. On Snowdon Towers, the RE1 models and both `255ribeiro`
/// 2025 projects, every id this gives a recovered category is either a
/// `Tag` of Revit's own export, of the entity that category exports as,
/// or an element Revit's export leaves out (#309).
pub fn assign_second_prologue_ids(
    buf: &[u8],
    marker: &[u8; 8],
    declared_ids: &BTreeSet<u32>,
) -> BTreeMap<usize, u32> {
    let chain = partition_record_chain(buf, marker);
    let mut out = BTreeMap::new();
    for hit in memchr::memmem::find_iter(buf, marker) {
        let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
            continue;
        };
        if !is_second_prologue_instance(buf, offset) {
            continue;
        }
        let Some(span) = enclosing_record(&chain, offset) else {
            continue;
        };
        if let Ok(id) = u32::try_from(span.element_id) {
            if declared_ids.contains(&id) {
                out.insert(offset, id);
            }
        }
    }
    out
}

/// Whether a frame's `+0x00` `u64` holds no ElementId: `0`, a value above
/// `u32::MAX` (RE-30's second prologue), or `u32::MAX`, the 32-bit form of
/// Revit's invalid ElementId −1 (RE-43).
pub fn carries_no_element_id(raw: u64) -> bool {
    raw == 0 || raw >= u64::from(u32::MAX)
}

/// Whether the frame at `offset` is a placed instance in the
/// `BuiltInCategory` band with no ElementId at `+0x00`
/// ([`carries_no_element_id`]): the frames [`assign_second_prologue_ids`]
/// resolves.
fn is_second_prologue_instance(buf: &[u8], offset: usize) -> bool {
    read_u64(buf, offset + CATEGORY_OFFSET)
        .is_some_and(|v| (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&(v as i64)))
        && read_u64(buf, offset).is_some_and(carries_no_element_id)
        && is_instance_frame(buf, offset)
}

/// Second-prologue ElementIds for a whole file, keyed by partition stream
/// and frame offset (RE-35). Empty where the release's record shape is not
/// proven. [`crate::RevitFile::second_prologue_ids`] memoises it.
pub fn compute_second_prologue_ids(rf: &mut RevitFile) -> SecondPrologueIds {
    let mut out = SecondPrologueIds::new();
    let Ok(version) = rf.basic_file_info().map(|b| b.version) else {
        return out;
    };
    let Some(marker) = bbox_marker(version).filter(|_| supports_revit_version(version)) else {
        return out;
    };
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return out,
    };
    if declared.is_empty() {
        return out;
    }
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let assigned = assign_second_prologue_ids(inflated.bytes(), &marker, &declared);
        if !assigned.is_empty() {
            out.insert(stream, assigned);
        }
    }
    out
}

/// Second-prologue ElementIds: partition stream, then frame offset.
pub type SecondPrologueIds = BTreeMap<String, BTreeMap<usize, u32>>;

/// [`find_category_records_with_marker`], also decoding the
/// second-prologue frames of `builtin_category` that
/// [`assign_second_prologue_ids`] gave an id (RE-35).
pub fn find_category_records_assigned(
    stream: &str,
    buf: &[u8],
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
    marker: &[u8; 8],
    assigned: &BTreeMap<usize, u32>,
) -> Vec<PartitionElementRecord> {
    let mut out =
        find_category_records_with_marker(stream, buf, builtin_category, declared_ids, marker);
    for (&offset, &element_id) in assigned {
        if read_u64(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) != Some(builtin_category) {
            continue;
        }
        if let Some(mut record) = decode_frame_as(stream, buf, offset, element_id, marker) {
            record.id_from_enclosing_record = true;
            out.push(record);
        }
    }
    out.sort_by_key(|record| record.offset);
    out
}

/// [`find_category_records_assigned`] for several categories in one sweep:
/// `out[i]` holds the records of `builtin_categories[i]`, exactly as a call
/// for that category alone returns them.
///
/// A call per category searches the whole partition once per category, and
/// the product sweep asks for dozens. Category ids share their top six
/// bytes (`e1 ff ff ff ff ff` for every `OST_*` id read so far), so the
/// categories are grouped by those bytes, each group costs one search, and
/// each hit's low two bytes pick its category. Every occurrence of a
/// category's 8 bytes carries its group's 6 at `+2`, and the search steps
/// one byte past each hit, so no occurrence is missed.
pub fn find_categories_records_assigned(
    stream: &str,
    buf: &[u8],
    builtin_categories: &[i64],
    declared_ids: &BTreeSet<u32>,
    marker: &[u8; 8],
    assigned: &BTreeMap<usize, u32>,
) -> Vec<Vec<PartitionElementRecord>> {
    let mut out: Vec<Vec<PartitionElementRecord>> = vec![Vec::new(); builtin_categories.len()];
    if buf.len() >= RECORD_MIN_LEN {
        let mut groups: BTreeMap<[u8; 6], Vec<(usize, [u8; 2])>> = BTreeMap::new();
        for (index, category) in builtin_categories.iter().enumerate() {
            let bytes = (*category as u64).to_le_bytes();
            let suffix: [u8; 6] = bytes[2..].try_into().expect("6 bytes");
            groups
                .entry(suffix)
                .or_default()
                .push((index, [bytes[0], bytes[1]]));
        }
        for (suffix, members) in &groups {
            let finder = memchr::memmem::Finder::new(suffix);
            let mut cursor = 2usize;
            while cursor + suffix.len() <= buf.len() {
                let Some(found) = finder.find(&buf[cursor..]) else {
                    break;
                };
                let at = cursor + found - 2;
                cursor += found + 1;
                let low = [buf[at], buf[at + 1]];
                for (index, _) in members.iter().filter(|(_, bytes)| *bytes == low) {
                    if at >= CATEGORY_OFFSET {
                        if let Some(record) = decode_at_with_marker(
                            stream,
                            buf,
                            at - CATEGORY_OFFSET,
                            declared_ids,
                            marker,
                        ) {
                            out[*index].push(record);
                        }
                    }
                }
            }
        }
    }
    for (&offset, &element_id) in assigned {
        let Some(category) = read_u64(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
            continue;
        };
        for (index, wanted) in builtin_categories.iter().enumerate() {
            if *wanted != category {
                continue;
            }
            if let Some(mut record) = decode_frame_as(stream, buf, offset, element_id, marker) {
                record.id_from_enclosing_record = true;
                out[index].push(record);
            }
        }
    }
    for records in &mut out {
        records.sort_by_key(|record| record.offset);
    }
    out
}

/// Find every element record in `buf` carrying `builtin_category`.
///
/// The scan anchors on the 8-byte little-endian encoding of the
/// category id and back-references [`CATEGORY_OFFSET`] to the record
/// start, then validates through [`decode_at`].
pub fn find_category_records(
    stream: &str,
    buf: &[u8],
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
) -> Vec<PartitionElementRecord> {
    find_category_records_with_marker(stream, buf, builtin_category, declared_ids, &BBOX_MARKER)
}

/// [`find_category_records`] for the release whose bbox marker is
/// `marker`.
pub fn find_category_records_with_marker(
    stream: &str,
    buf: &[u8],
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
    marker: &[u8; 8],
) -> Vec<PartitionElementRecord> {
    let needle = (builtin_category as u64).to_le_bytes();
    let mut out = Vec::new();
    if buf.len() < RECORD_MIN_LEN {
        return out;
    }
    let mut cursor = 0usize;
    while cursor + 8 <= buf.len() {
        let Some(found) = find_subslice(&buf[cursor..], &needle) else {
            break;
        };
        let hit = cursor + found;
        if hit >= CATEGORY_OFFSET {
            if let Some(record) =
                decode_at_with_marker(stream, buf, hit - CATEGORY_OFFSET, declared_ids, marker)
            {
                out.push(record);
            }
        }
        cursor = hit + 1;
    }
    out
}

/// First occurrence of `needle` in `haystack`.
///
/// A single-category scan runs this over every inflated `Partitions/*`
/// byte; the product sweep asks for dozens of categories at once through
/// [`find_categories_records_assigned`] instead. `memchr::memmem` is the
/// vectorised form of exactly the first-byte-then-compare loop this used
/// to spell out by hand.
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    memchr::memmem::find(haystack, needle)
}

/// The categories the exporter recovers instances of, with the class name
/// each is decoded as: the architectural core plus
/// [`PRODUCT_RECORD_CATEGORIES`].
pub const RECOVERED_CATEGORIES: [(i64, &str); 39] = [
    (OST_WALLS, "Wall"),
    (OST_DOORS, "Door"),
    (OST_WINDOWS, "Window"),
    (OST_COLUMNS, "Column"),
    (OST_FLOORS, "Floor"),
    (OST_BUILDING_PAD, "BuildingPad"),
    (OST_ROOMS, "Room"),
    (OST_FURNITURE, "Furniture"),
    (OST_CASEWORK, "Casework"),
    (OST_PLUMBING_FIXTURES, "PlumbingFixture"),
    (OST_SPECIALITY_EQUIPMENT, "SpecialtyEquipment"),
    (OST_CEILINGS, "Ceiling"),
    (OST_CURTAIN_WALL_MULLIONS, "CurtainWallMullion"),
    (OST_CURTAIN_WALL_PANELS, "CurtainWallPanel"),
    (OST_STAIRS_RAILING, "Railing"),
    (OST_CORNICES, "WallSweep"),
    (OST_DUCT_CURVES, "Duct"),
    (OST_DUCT_FITTING, "DuctFitting"),
    (OST_PIPE_CURVES, "Pipe"),
    (OST_PIPE_FITTING, "PipeFitting"),
    (OST_STRUCTURAL_FRAMING, "StructuralFraming"),
    (OST_STRUCTURAL_COLUMNS, "StructuralColumn"),
    (OST_STRUCTURAL_FOUNDATION, "StructuralFoundation"),
    (OST_GENERIC_MODEL, "GenericModel"),
    (OST_LIGHTING_FIXTURES, "LightingFixture"),
    (OST_DUCT_TERMINAL, "DuctTerminal"),
    (OST_FOOD_SERVICE_EQUIPMENT, "FoodServiceEquipment"),
    (OST_PLANTING, "Planting"),
    (OST_PARKING, "Parking"),
    (OST_ENTOURAGE, "Entourage"),
    (OST_HARDSCAPE, "Hardscape"),
    (OST_VERTICAL_CIRCULATION, "VerticalCirculation"),
    (OST_RAMPS, "Ramp"),
    (OST_STAIRS, "Stair"),
    (OST_STAIRS_RUNS, "StairsRun"),
    (OST_STAIRS_LANDINGS, "StairsLanding"),
    (OST_STAIRS_STRINGER_CARRIAGE, "StairsStringer"),
    (OST_ROOFS, "Roof"),
    (OST_EDGE_SLAB, "SlabEdge"),
];

/// Placed-instance frames in `buf`, per category, that carry the bbox
/// marker at `+0x50` and one of `categories` at `+0x12` but no ElementId
/// at `+0x00` (a `u64` of 0 or above `u32::MAX`).
///
/// This is the second prologue RE-30 measured on Autodesk's Snowdon
/// Towers samples: shaped like an element record from the category to
/// the bounding box, with no attributable ElementId, so [`decode_at`]
/// rejects it. Counting them is what lets an export say how much of a
/// file it could not attribute instead of looking complete. None on
/// `2024_Core_Interior.rvt`.
///
/// Only frames that pass the RE-21 instance rule count: no container
/// reference at [`CONTAINER_OFFSET`] and the placed-instance kind at
/// [`PLACEMENT_KIND_OFFSET`]. The second prologue keeps both fields where
/// the first has them (RE-33), and with the rule the count reproduces
/// the number of elements Revit's own export holds that rvt-rs does not:
/// on Snowdon Towers 1,425 mullions, 118 columns, 131 railings, 68
/// ceilings and 344 furniture and casework, all exactly; on the RE1 MEP
/// models 60 pipes, 48 pipe fittings, 18 duct fittings and 15 duct and
/// pipe segments, all exactly. Without it, container members and type
/// symbols counted too (6,019 frames on Snowdon against 4,886).
pub fn count_unattributed_frames(buf: &[u8], categories: &[i64]) -> BTreeMap<i64, usize> {
    count_unattributed_frames_with_marker(buf, categories, &BBOX_MARKER)
}

/// [`count_unattributed_frames`] for the release whose bbox marker is
/// `marker`.
pub fn count_unattributed_frames_with_marker(
    buf: &[u8],
    categories: &[i64],
    marker: &[u8; 8],
) -> BTreeMap<i64, usize> {
    count_unattributed_frames_excluding(buf, categories, marker, &BTreeMap::new())
}

/// [`count_unattributed_frames_with_marker`], leaving out the frames at the
/// offsets in `assigned` ([`assign_second_prologue_ids`], RE-35).
///
/// Only frames inside the partition's leading record chain
/// ([`partition_record_chain`]) count: the records after it belong to
/// loaded families' own documents, whose frames are not model elements.
pub fn count_unattributed_frames_excluding(
    buf: &[u8],
    categories: &[i64],
    marker: &[u8; 8],
    assigned: &BTreeMap<usize, u32>,
) -> BTreeMap<i64, usize> {
    let mut counts = BTreeMap::new();
    let chain = partition_record_chain(buf, marker);
    for hit in memchr::memmem::find_iter(buf, marker) {
        let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
            continue;
        };
        if assigned.contains_key(&offset) || enclosing_record(&chain, offset).is_none() {
            continue;
        }
        let Some(category) = read_u64(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
            continue;
        };
        if !categories.contains(&category) {
            continue;
        }
        if !read_u64(buf, offset).is_some_and(carries_no_element_id) {
            continue;
        }
        // A frame with no volume is a 2D symbol Revit's export leaves out
        // (#309), so it is not a missing element either.
        if is_instance_frame(buf, offset) && frame_has_volume(buf, offset) {
            *counts.entry(category).or_insert(0) += 1;
        }
    }
    counts
}

/// [`count_unattributed_frames`] over every `Partitions/*` stream for the
/// [`RECOVERED_CATEGORIES`], keyed by class name. Empty on releases where
/// the record shape is not proven.
pub fn scan_unattributed_frames(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<BTreeMap<String, usize>> {
    let mut by_class = BTreeMap::new();
    let Some(marker) = bbox_marker(revit_version).filter(|_| supports_revit_version(revit_version))
    else {
        return Ok(by_class);
    };
    let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
    let assigned_all = rf.second_prologue_ids();
    let none = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        // A frame RE-35 gave an id to is exported, not missing.
        let assigned = assigned_all.get(&stream).unwrap_or(&none);
        for (category, count) in
            count_unattributed_frames_excluding(inflated.bytes(), &categories, &marker, assigned)
        {
            if let Some((_, class)) = RECOVERED_CATEGORIES.iter().find(|(c, _)| *c == category) {
                *by_class.entry((*class).to_string()).or_insert(0) += count;
            }
        }
    }
    Ok(by_class)
}

/// Placed instances of the [`RECOVERED_CATEGORIES`] whose bounding box has
/// no volume ([`PartitionElementRecord::has_volume`]), one per ElementId,
/// keyed by class name. The export leaves them out, as Revit's does
/// (#309); this counts them so the diagnostics can say so.
pub fn scan_volumeless_instances(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<BTreeMap<String, usize>> {
    let mut by_class = BTreeMap::new();
    if !supports_revit_version(revit_version) {
        return Ok(by_class);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(by_class),
    };
    let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
    let records = scan_category_records_multi(rf, revit_version, &categories, &declared)?;
    // An element in a non-primary design option is left out for that reason
    // instead, and counted by
    // `partition_design_options::scan_non_primary_option_instances`.
    let options = rf.design_options();
    // An element framed more than once counts once, and only when no frame
    // of it has volume.
    let mut with_volume: BTreeSet<u32> = BTreeSet::new();
    let mut without: BTreeMap<u32, i64> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| r.is_exported_instance() && !options.excludes(r))
    {
        if record.has_volume() {
            with_volume.insert(record.element_id);
        } else {
            without.insert(record.element_id, record.builtin_category);
        }
    }
    for (id, category) in without {
        if with_volume.contains(&id) {
            continue;
        }
        if let Some((_, class)) = RECOVERED_CATEGORIES.iter().find(|(c, _)| *c == category) {
            *by_class.entry((*class).to_string()).or_insert(0) += 1;
        }
    }
    Ok(by_class)
}

/// Scan every `Partitions/*` stream for records in `builtin_category`.
///
/// Returns an empty vector for unsupported releases (fail closed).
pub fn scan_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
) -> Result<Vec<PartitionElementRecord>> {
    scan_category_records_multi(rf, revit_version, &[builtin_category], declared_ids)
}

/// Scan every `Partitions/*` stream once for records in **any** of
/// `builtin_categories`.
///
/// Same result as calling [`scan_category_records`] per category and
/// concatenating, but each stream is inflated once instead of once
/// per category — inflation dominates the cost on a 190 MiB project
/// file, and the slab path needs three categories at a time
/// (`OST_Floors`, `OST_BuildingPad`, `OST_SketchLines`).
///
/// Records are returned grouped by category in the order given, and
/// by `(stream, offset)` within a category.
pub fn scan_category_records_multi(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_categories: &[i64],
    declared_ids: &BTreeSet<u32>,
) -> Result<Vec<PartitionElementRecord>> {
    let Some(marker) = bbox_marker(revit_version).filter(|_| supports_revit_version(revit_version))
    else {
        return Ok(Vec::new());
    };
    if declared_ids.is_empty() {
        return Ok(Vec::new());
    }
    let streams = rf.partition_stream_names();
    let assigned_all = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut per_category: Vec<Vec<PartitionElementRecord>> =
        vec![Vec::new(); builtin_categories.len()];
    for stream in streams {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let assigned = assigned_all.get(&stream).unwrap_or(&none);
        for (index, records) in find_categories_records_assigned(
            &stream,
            inflated.bytes(),
            builtin_categories,
            declared_ids,
            &marker,
            assigned,
        )
        .into_iter()
        .enumerate()
        {
            per_category[index].extend(records);
        }
    }
    Ok(per_category.into_iter().flatten().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_record(element_id: u32, category: i64, bbox: [f64; 6]) -> Vec<u8> {
        let mut buf = vec![0xffu8; RECORD_MIN_LEN];
        buf[0..8].copy_from_slice(&u64::from(element_id).to_le_bytes());
        buf[8..12].copy_from_slice(&0x0141u32.to_le_bytes());
        buf[12..16].copy_from_slice(&0x059fu32.to_le_bytes());
        buf[16..18].copy_from_slice(&0u16.to_le_bytes());
        buf[CATEGORY_OFFSET..CATEGORY_OFFSET + 8].copy_from_slice(&(category as u64).to_le_bytes());
        buf[PLACEMENT_KIND_OFFSET..PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&PLACEMENT_KIND_INSTANCE.to_le_bytes());
        buf[BBOX_MARKER_OFFSET..BBOX_MARKER_OFFSET + 8].copy_from_slice(&BBOX_MARKER);
        for (index, value) in bbox.iter().enumerate() {
            let at = BBOX_OFFSET + index * 8;
            buf[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        buf
    }

    fn declared(ids: &[u32]) -> BTreeSet<u32> {
        ids.iter().copied().collect()
    }

    /// RE-32: the marker is per release; a 2025 record decodes with the
    /// 2025 marker and not with 2024's, and releases without a measured
    /// marker stay unsupported.
    #[test]
    fn bbox_marker_is_chosen_per_release() {
        assert_eq!(bbox_marker(2024), Some(BBOX_MARKER));
        assert_eq!(bbox_marker(2025), Some(BBOX_MARKER_2025));
        for unproven in [2023, 2026, 2027] {
            assert_eq!(bbox_marker(unproven), None);
            assert!(!supports_revit_version(unproven));
        }
        // Both markers end in the release's ElemTable header constant + 40.
        assert_eq!(
            u16::from_le_bytes([BBOX_MARKER[6], BBOX_MARKER[7]]),
            1411 + 40
        );
        assert_eq!(
            u16::from_le_bytes([BBOX_MARKER_2025[6], BBOX_MARKER_2025[7]]),
            1451 + 40
        );

        let mut buf = synth_record(415_431, OST_WALLS, [0.0, 0.0, 0.0, 0.4, 26.2, 11.5]);
        buf[BBOX_MARKER_OFFSET..BBOX_MARKER_OFFSET + 8].copy_from_slice(&BBOX_MARKER_2025);
        let ids = declared(&[415_431]);
        let record = decode_at_with_marker("Partitions/68", &buf, 0, &ids, &BBOX_MARKER_2025)
            .expect("2025 record decodes with the 2025 marker");
        assert_eq!(record.element_id, 415_431);
        assert!(decode_at("Partitions/68", &buf, 0, &ids).is_none());
        assert_eq!(
            find_category_records_with_marker(
                "Partitions/68",
                &buf,
                OST_WALLS,
                &ids,
                &BBOX_MARKER_2025
            )
            .len(),
            1
        );
    }

    /// RE-30: a frame with the category and bbox marker in place but no
    /// ElementId at `+0x00` is counted under its category; an attributed
    /// record and a category outside the set are not.
    #[test]
    fn unattributed_frames_are_counted_per_category() {
        let bbox = [0.0, 0.0, 0.0, 1.0, 18.9, 24.7];
        let attributed = synth_record(22805, OST_WALLS, bbox);
        let mut all_ff = synth_record(1, OST_WALLS, bbox);
        all_ff[0..8].fill(0xff);
        let mut high_word = synth_record(1, OST_DOORS, bbox);
        high_word[0..8].copy_from_slice(&0x0000_0006_ffff_ffff_u64.to_le_bytes());
        let mut outside = synth_record(1, OST_SKETCH_LINES, bbox);
        outside[0..8].fill(0xff);
        // RE-33: a contained frame or a type symbol is not a missing
        // element, so neither counts.
        let mut contained = synth_record(1, OST_WALLS, bbox);
        contained[0..8].fill(0xff);
        contained[CONTAINER_OFFSET..CONTAINER_OFFSET + 8].copy_from_slice(&20274u64.to_le_bytes());
        let mut symbol = synth_record(1, OST_DOORS, bbox);
        symbol[0..8].fill(0xff);
        symbol[PLACEMENT_KIND_OFFSET..PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&PLACEMENT_KIND_SYMBOL.to_le_bytes());
        // Each sits in a record of an undeclared id, so none is attributed.
        let first = as_record(attributed);
        let buf = [
            first.clone(),
            nested(9001, &all_ff),
            nested(9002, &high_word),
            nested(9003, &outside),
            nested(9004, &contained),
            nested(9005, &symbol),
        ]
        .concat();

        let counts = count_unattributed_frames(&buf, &[OST_WALLS, OST_DOORS]);
        assert_eq!(counts, BTreeMap::from([(OST_WALLS, 1), (OST_DOORS, 1)]));
        assert!(count_unattributed_frames(&first, &[OST_WALLS]).is_empty());
    }

    /// Append the counted reference list a real record carries after
    /// its bounding box.
    fn with_reference_list(mut buf: Vec<u8>, entries: &[u64]) -> Vec<u8> {
        buf.truncate(REFERENCE_LIST_OFFSET);
        buf.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for entry in entries {
            buf.extend_from_slice(&entry.to_le_bytes());
        }
        buf
    }

    #[test]
    fn decodes_a_well_formed_column_record() {
        let bbox = [135.5, 79.0, 0.0, 137.5, 81.0, 30.333];
        let buf = synth_record(22805, OST_COLUMNS, bbox);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.element_id, 22805);
        assert_eq!(record.builtin_category, OST_COLUMNS);
        assert_eq!(record.flags, 0x0141);
        let (dx, dy, _) = record.extents_feet();
        assert!((dx - 2.0).abs() < 1e-9);
        assert!((dy - 2.0).abs() < 1e-9);
        assert!(!record.is_family_local());
    }

    #[test]
    fn rejects_ids_absent_from_elem_table() {
        let buf = synth_record(22805, OST_COLUMNS, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[9999])).is_none());
    }

    #[test]
    fn rejects_missing_bbox_marker() {
        let mut buf = synth_record(22805, OST_COLUMNS, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        buf[BBOX_MARKER_OFFSET] = 0x00;
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[22805])).is_none());
    }

    #[test]
    fn rejects_inverted_bounding_box() {
        let buf = synth_record(22805, OST_COLUMNS, [10.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[22805])).is_none());
    }

    #[test]
    fn family_local_bbox_is_recognised() {
        let buf = synth_record(5755, OST_COLUMNS, [-1.0, -1.0, 0.0, 1.0, 1.0, 9.0]);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[5755])).expect("decodes");
        assert!(record.is_family_local());
    }

    #[test]
    fn placed_standalone_record_is_an_exported_instance() {
        let buf = synth_record(22805, OST_COLUMNS, [135.5, 79.0, 0.0, 137.5, 81.0, 30.3]);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.container, CONTAINER_NONE);
        assert_eq!(record.placement_kind, PLACEMENT_KIND_INSTANCE);
        assert!(record.container_element_id().is_none());
        assert!(!record.is_container_member());
        assert!(record.is_placed_instance());
        assert!(!record.is_type_symbol());
        assert!(record.is_exported_instance());
    }

    #[test]
    fn container_member_is_not_an_exported_instance() {
        let mut buf = synth_record(16347, OST_COLUMNS, [23.0, 109.0, 76.0, 25.0, 111.0, 91.0]);
        buf[CONTAINER_OFFSET..CONTAINER_OFFSET + 8].copy_from_slice(&16_229u64.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[16347])).expect("decodes");
        assert_eq!(record.container_element_id(), Some(16_229));
        assert!(record.is_container_member());
        assert!(record.is_placed_instance());
        assert!(!record.is_exported_instance());
    }

    #[test]
    fn type_symbol_is_not_an_exported_instance() {
        // A door symbol envelope: centred on X only, so the
        // family-local bbox proxy misses it and `+0x42` does not.
        let mut buf = synth_record(17331, OST_DOORS, [-1.749, -0.332, 0.0, 1.749, 3.251, 8.249]);
        buf[PLACEMENT_KIND_OFFSET..PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&PLACEMENT_KIND_SYMBOL.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[17331])).expect("decodes");
        assert!(!record.is_family_local(), "proxy misses this symbol");
        assert!(record.is_type_symbol());
        assert!(!record.is_placed_instance());
        assert!(!record.is_exported_instance());
    }

    #[test]
    fn out_of_range_container_reference_is_not_an_element_id() {
        let mut buf = synth_record(22805, OST_COLUMNS, [1.0, 2.0, 0.0, 3.0, 4.0, 5.0]);
        buf[CONTAINER_OFFSET..CONTAINER_OFFSET + 8]
            .copy_from_slice(&0x0001_0000_0000u64.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert!(record.is_container_member());
        assert_eq!(record.container_element_id(), None);
        assert!(!record.is_exported_instance());
    }

    #[test]
    fn scan_finds_records_at_any_offset() {
        let bbox = [1.0, 2.0, 3.0, 3.0, 4.0, 13.0];
        let mut buf = vec![0u8; 37];
        buf.extend(synth_record(4242, OST_COLUMNS, bbox));
        buf.extend(vec![0u8; 11]);
        let found = find_category_records("Partitions/46", &buf, OST_COLUMNS, &declared(&[4242]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].element_id, 4242);
        assert_eq!(found[0].offset, 37);
    }

    #[test]
    fn unsupported_release_yields_nothing() {
        assert!(!supports_revit_version(2023));
        assert!(supports_revit_version(2024));
    }

    #[test]
    fn reference_list_decodes_its_counted_slots() {
        let buf = with_reference_list(
            synth_record(25947, OST_DOORS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 20274, 23756, 24301, 25488, 25947],
        );
        let entries = decode_reference_list(&buf, REFERENCE_LIST_OFFSET).expect("decodes");
        assert_eq!(entries, vec![3, 20274, 23756, 24301, 25488, 25947]);
    }

    #[test]
    fn reference_list_rejects_a_length_that_runs_past_the_buffer() {
        let mut buf = with_reference_list(
            synth_record(25947, OST_DOORS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 25488, 25947],
        );
        buf[REFERENCE_LIST_OFFSET..REFERENCE_LIST_OFFSET + 4].copy_from_slice(&64u32.to_le_bytes());
        assert!(decode_reference_list(&buf, REFERENCE_LIST_OFFSET).is_none());
    }

    #[test]
    fn reference_list_rejects_an_unbounded_length() {
        let mut buf = with_reference_list(
            synth_record(25947, OST_DOORS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 25488, 25947],
        );
        buf[REFERENCE_LIST_OFFSET..REFERENCE_LIST_OFFSET + 4]
            .copy_from_slice(&(REFERENCE_LIST_MAX_ENTRIES as u32 + 1).to_le_bytes());
        assert!(decode_reference_list(&buf, REFERENCE_LIST_OFFSET).is_none());
    }

    #[test]
    fn preceding_reference_is_the_slot_before_the_records_own_id() {
        // ElementId 25947 (`OST_Doors`) framed with the six-slot list
        // Partitions/46 carries for it: the slot before its own id is
        // 25488, the wall Revit's export voids for this door.
        let buf = with_reference_list(
            synth_record(25947, OST_DOORS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 20274, 23756, 24301, 25488, 25947],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[25947])).expect("decodes");
        assert_eq!(record.preceding_reference, Some(25_488));
    }

    #[test]
    fn preceding_reference_ignores_slots_after_the_records_own_id() {
        // 81 of the 273 door/window records on the recorded edge carry
        // a trailing type-symbol slot after their own id, so "the last
        // two slots are (host, self)" is not the rule.
        let buf = with_reference_list(
            synth_record(20810, OST_DOORS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 17333, 20308, 20798, 20810, 132_750],
        );
        let record = decode_at("Partitions/59", &buf, 0, &declared(&[20810])).expect("decodes");
        assert_eq!(record.preceding_reference, Some(20_798));
    }

    #[test]
    fn preceding_reference_is_absent_without_a_self_slot() {
        let buf = with_reference_list(
            synth_record(22805, OST_COLUMNS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 1435, 17337],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.preceding_reference, None);
    }

    #[test]
    fn preceding_reference_is_absent_when_the_record_leads_its_list() {
        let buf = with_reference_list(
            synth_record(22805, OST_COLUMNS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[22805, 1435],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.preceding_reference, None);
    }

    #[test]
    fn preceding_reference_rejects_a_slot_outside_the_element_id_range() {
        let buf = with_reference_list(
            synth_record(22805, OST_COLUMNS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, u64::from(u32::MAX) + 1, 22805],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.preceding_reference, None);
    }

    #[test]
    fn owner_reference_is_the_last_slot_of_the_second_list() {
        // An `OST_SketchLines` record as `Partitions/46` frames it:
        // a five-slot sibling list, then a two-slot list whose last
        // entry is the sketched element (slab 20311).
        let mut buf = with_reference_list(
            synth_record(
                20310,
                OST_SKETCH_LINES,
                [20.0, 25.0, 46.0, 20.0, 114.0, 46.0],
            ),
            &[20309, 20310, 20312, 20313, 20315],
        );
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(&20_308u64.to_le_bytes());
        buf.extend_from_slice(&20_311u64.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[20310])).expect("decodes");
        assert_eq!(record.builtin_category, OST_SKETCH_LINES);
        assert_eq!(record.owner_reference, Some(20_311));
    }

    #[test]
    fn owner_reference_is_absent_without_a_second_list() {
        let buf = with_reference_list(
            synth_record(
                20310,
                OST_SKETCH_LINES,
                [20.0, 25.0, 46.0, 20.0, 114.0, 46.0],
            ),
            &[20309, 20310],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[20310])).expect("decodes");
        assert_eq!(record.owner_reference, None);
    }

    #[test]
    fn owner_reference_rejects_a_slot_outside_the_element_id_range() {
        let mut buf = with_reference_list(
            synth_record(
                20310,
                OST_SKETCH_LINES,
                [20.0, 25.0, 46.0, 20.0, 114.0, 46.0],
            ),
            &[20309, 20310],
        );
        buf.extend_from_slice(&1u32.to_le_bytes());
        buf.extend_from_slice(&(u64::from(u32::MAX) + 1).to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[20310])).expect("decodes");
        assert_eq!(record.owner_reference, None);
    }

    #[test]
    fn type_symbol_reference_is_the_symbol_slot_before_the_records_own_id() {
        // ElementId 20375 (`OST_Columns`) framed with the nine-slot
        // list Partitions/46 carries for it. Two of the slots are
        // column type symbols — 5755 before its own id and 75407
        // after — and only the first is the column's own type.
        let buf = with_reference_list(
            synth_record(20375, OST_COLUMNS, [23.0, 109.0, 76.0, 25.0, 111.0, 90.333]),
            &[3, 4258, 4546, 5755, 20307, 20308, 20375, 20843, 75407],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[20375])).expect("decodes");
        let symbols: BTreeSet<u32> = [5755u32, 75407].into_iter().collect();
        assert_eq!(record.type_symbol_reference(&symbols), Some(5755));
    }

    #[test]
    fn type_symbol_reference_is_absent_without_a_symbol_slot() {
        let buf = with_reference_list(
            synth_record(20375, OST_COLUMNS, [23.0, 109.0, 76.0, 25.0, 111.0, 90.333]),
            &[3, 4258, 20375, 75407],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[20375])).expect("decodes");
        let symbols: BTreeSet<u32> = [5755u32, 75407].into_iter().collect();
        assert_eq!(record.type_symbol_reference(&symbols), None);
    }

    #[test]
    fn type_symbol_reference_is_absent_without_a_self_slot() {
        let buf = with_reference_list(
            synth_record(20375, OST_COLUMNS, [23.0, 109.0, 76.0, 25.0, 111.0, 90.333]),
            &[3, 5755, 20307],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[20375])).expect("decodes");
        let symbols: BTreeSet<u32> = [5755u32].into_iter().collect();
        assert_eq!(record.type_symbol_reference(&symbols), None);
    }

    #[test]
    fn reference_list_is_kept_verbatim_on_the_record() {
        let buf = with_reference_list(
            synth_record(25947, OST_DOORS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]),
            &[3, 20274, 23756, 24301, 25488, 25947],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[25947])).expect("decodes");
        assert_eq!(
            record.references,
            vec![3, 20274, 23756, 24301, 25488, 25947]
        );
    }

    #[test]
    fn a_record_without_a_reference_list_still_decodes() {
        let buf = synth_record(22805, OST_COLUMNS, [1.0, 2.0, 0.0, 3.0, 4.0, 8.0]);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.preceding_reference, None);
        assert_eq!(record.owner_reference, None);
        assert!(record.references.is_empty());
    }

    #[test]
    fn product_categories_are_distinct_mapped_and_counted() {
        let mut seen = BTreeSet::new();
        for (category, class) in PRODUCT_RECORD_CATEGORIES {
            assert!(seen.insert(category), "{category} listed twice");
            assert!(
                (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&category),
                "{category} outside the BuiltInCategory band"
            );
            assert!(
                crate::ifc::category_map::lookup(class).is_some(),
                "{class} has no IFC mapping"
            );
            assert!(
                RECOVERED_CATEGORIES.contains(&(category, class)),
                "{class} frames without an ElementId would go uncounted"
            );
        }
    }

    #[test]
    fn a_product_category_record_decodes_like_any_other() {
        let buf = synth_record(738550, OST_FURNITURE, [1.0, 2.0, 0.0, 3.0, 4.0, 2.5]);
        let record = decode_at("Partitions/12", &buf, 0, &declared(&[738550])).expect("decodes");
        assert_eq!(record.builtin_category, OST_FURNITURE);
        assert_eq!(record.element_id, 738550);
    }

    /// A second-prologue frame: the first-prologue synthetic record with
    /// `+0x00` cleared to the `0x1_ffffffff` Snowdon frames carry.
    fn second_prologue(category: i64, references: &[u64]) -> Vec<u8> {
        let bbox = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let mut buf = with_reference_list(synth_record(1, category, bbox), references);
        buf[0..8].copy_from_slice(&0x0000_0001_ffff_ffff_u64.to_le_bytes());
        buf
    }

    fn first_prologue(element_id: u32, category: i64) -> Vec<u8> {
        let bbox = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        with_reference_list(
            synth_record(element_id, category, bbox),
            &[3, u64::from(element_id)],
        )
    }

    /// `frame` as a whole partition record: a first-prologue frame is its
    /// own record header, so only the size at `+0x08` and the trailer are
    /// added (RE-35).
    fn as_record(mut frame: Vec<u8>) -> Vec<u8> {
        let size = frame.len() as u32;
        frame[8..12].copy_from_slice(&size.to_le_bytes());
        trailer(&mut frame, size, 0);
        frame
    }

    /// `frame` nested in the record of `element_id`, behind a header and a
    /// few bytes of body the way a second-prologue frame sits (RE-35).
    fn nested(element_id: u64, frame: &[u8]) -> Vec<u8> {
        let mut record = element_id.to_le_bytes().to_vec();
        let size = (PARTITION_RECORD_HEADER_LEN + 6 + frame.len()) as u32;
        record.extend_from_slice(&size.to_le_bytes());
        record.extend_from_slice(&0x059fu16.to_le_bytes());
        record.extend_from_slice(&1u16.to_le_bytes());
        record.extend_from_slice(&[0u8; 6]);
        record.extend_from_slice(frame);
        trailer(&mut record, size, 0x0100_0000);
        record
    }

    fn trailer(record: &mut Vec<u8>, size: u32, flags: u32) {
        record.extend_from_slice(&0x0bcd_u64.to_le_bytes());
        record.extend_from_slice(&flags.to_le_bytes());
        record.extend_from_slice(&size.to_le_bytes());
    }

    /// Offset of the frame inside a [`nested`] record starting at `start`.
    fn nested_frame_at(start: usize) -> usize {
        start + PARTITION_RECORD_HEADER_LEN + 6
    }

    #[test]
    fn the_prologue_constant_sits_twelve_below_the_marker_constant() {
        assert_eq!(record_prologue_constant(&BBOX_MARKER), 0x059f);
        assert_eq!(record_prologue_constant(&BBOX_MARKER_2025), 0x05c7);
    }

    /// RE-35: the chain walks header to trailer from offset 0 and stops at
    /// the first position that is not a record; a record after the break
    /// (a loaded family's document) is not part of it.
    #[test]
    fn the_leading_record_chain_stops_at_the_first_non_record() {
        let first = as_record(first_prologue(100, OST_WALLS));
        let second = nested(150, &second_prologue(OST_WALLS, &[3, 90]));
        let chained = [first.clone(), second.clone()].concat();
        let family = nested(160, &second_prologue(OST_WALLS, &[3, 90]));
        let buf = [chained.clone(), vec![0x42; 40], family].concat();
        let chain = partition_record_chain(&buf, &BBOX_MARKER);
        let ids: Vec<(usize, u64)> = chain.iter().map(|r| (r.start, r.element_id)).collect();
        assert_eq!(ids, vec![(0, 100), (first.len(), 150)]);
        assert_eq!(chain[1].end, chained.len() - 16);

        // A trailer that does not echo the size ends the chain there.
        let mut broken = first.clone();
        let echo = broken.len() - 4;
        broken[echo..].copy_from_slice(&7u32.to_le_bytes());
        assert!(partition_record_chain(&broken, &BBOX_MARKER).is_empty());
        // So does a flag word other than 0 or 0x0100_0000.
        let mut flagged = first;
        let flags = flagged.len() - 8;
        flagged[flags..flags + 4].copy_from_slice(&2u32.to_le_bytes());
        assert!(partition_record_chain(&flagged, &BBOX_MARKER).is_empty());
        // And the 2025 constant is not the 2024 one.
        assert!(partition_record_chain(&chained, &BBOX_MARKER_2025).is_empty());
    }

    /// RE-35: a second-prologue frame's ElementId is its enclosing record's,
    /// whatever its reference list names. A declared type id in the list
    /// (the Projeto1 door named its type 49480) does not matter, a record
    /// with an undeclared id gives none, and a family record after the
    /// chain gives none even when its id is declared.
    #[test]
    fn a_second_prologue_frame_takes_its_enclosing_record_id() {
        let declared = declared(&[100, 150, 160, 49480]);
        let first = as_record(first_prologue(100, OST_WALLS));
        let door = nested(150, &second_prologue(OST_DOORS, &[3, 49480, 150, 160]));
        let undeclared = nested(170, &second_prologue(OST_DOORS, &[3, 150]));
        let chained = [first.clone(), door.clone(), undeclared].concat();
        let family = nested(160, &second_prologue(OST_DOORS, &[3, 160]));
        let buf = [chained, vec![0x42; 40], family].concat();
        let assigned = assign_second_prologue_ids(&buf, &BBOX_MARKER, &declared);
        assert_eq!(
            assigned.into_iter().collect::<Vec<_>>(),
            vec![(nested_frame_at(first.len()), 150)]
        );
    }

    /// RE-43: a frame whose `+0x00` holds `u32::MAX`, the 32-bit form of
    /// the invalid ElementId −1, carries no id either, and takes its
    /// record's. A declared id below it still starts its own record.
    #[test]
    fn a_frame_with_the_invalid_id_takes_its_enclosing_record_id() {
        assert!(carries_no_element_id(0));
        assert!(carries_no_element_id(u64::from(u32::MAX)));
        assert!(carries_no_element_id(0x0000_0001_ffff_ffff));
        assert!(!carries_no_element_id(1_644_693));
        let declared = declared(&[100, 1_644_693]);
        let mut run = second_prologue(OST_STAIRS_RUNS, &[3, 1_644_692, 1_644_693]);
        run[0..8].copy_from_slice(&u64::from(u32::MAX).to_le_bytes());
        let first = as_record(first_prologue(100, OST_STAIRS_RUNS));
        let buf = [first.clone(), nested(1_644_693, &run)].concat();
        let assigned = assign_second_prologue_ids(&buf, &BBOX_MARKER, &declared);
        assert_eq!(
            assigned.into_iter().collect::<Vec<_>>(),
            vec![(nested_frame_at(first.len()), 1_644_693)]
        );
    }

    #[test]
    fn a_second_prologue_frame_that_is_not_a_placed_instance_gets_no_id() {
        let declared = declared(&[100, 150]);
        let mut symbol = second_prologue(OST_DOORS, &[3, 150]);
        symbol[PLACEMENT_KIND_OFFSET..PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&PLACEMENT_KIND_SYMBOL.to_le_bytes());
        let buf = [
            as_record(first_prologue(100, OST_DOORS)),
            nested(150, &symbol),
        ]
        .concat();
        assert!(assign_second_prologue_ids(&buf, &BBOX_MARKER, &declared).is_empty());
    }

    #[test]
    fn an_assigned_frame_decodes_under_its_category_and_is_not_counted_missing() {
        let declared = declared(&[100, 150, 200]);
        let buf = [
            as_record(first_prologue(100, OST_WALLS)),
            nested(150, &second_prologue(OST_WALLS, &[3, 90])),
            as_record(first_prologue(200, OST_WALLS)),
        ]
        .concat();
        let assigned = assign_second_prologue_ids(&buf, &BBOX_MARKER, &declared);
        let records = find_category_records_assigned(
            "Partitions/68",
            &buf,
            OST_WALLS,
            &declared,
            &BBOX_MARKER,
            &assigned,
        );
        let ids: Vec<(u32, bool)> = records
            .iter()
            .map(|r| (r.element_id, r.id_from_enclosing_record))
            .collect();
        assert_eq!(ids, vec![(100, false), (150, true), (200, false)]);
        assert!(
            count_unattributed_frames_excluding(&buf, &[OST_WALLS], &BBOX_MARKER, &assigned)
                .is_empty()
        );
        assert_eq!(
            count_unattributed_frames(&buf, &[OST_WALLS]),
            BTreeMap::from([(OST_WALLS, 1)])
        );
    }

    /// A family document's frames, after the chain, are not model elements
    /// and are not counted missing either.
    #[test]
    fn frames_after_the_record_chain_are_not_counted_missing() {
        let chain = as_record(first_prologue(100, OST_WALLS));
        let family = nested(7, &second_prologue(OST_WALLS, &[3, 7]));
        let buf = [chain, vec![0x42; 40], family].concat();
        assert!(count_unattributed_frames(&buf, &[OST_WALLS]).is_empty());
    }

    /// #309: a placed instance whose box is flat on any axis has no 3D
    /// body and is neither exported nor counted as missing.
    #[test]
    fn a_placed_instance_without_volume_is_left_out_and_not_counted_missing() {
        let flat = [0.0, 0.0, 0.0, 5.0, 5.0, 0.0];
        let solid = [0.0, 0.0, 0.0, 5.0, 5.0, 3.0];
        let declared = declared(&[100, 200]);
        let buf = [
            synth_record(100, OST_SPECIALITY_EQUIPMENT, flat),
            synth_record(200, OST_SPECIALITY_EQUIPMENT, solid),
        ]
        .concat();
        let records =
            find_category_records("Partitions/68", &buf, OST_SPECIALITY_EQUIPMENT, &declared);
        let volumes: Vec<(u32, bool)> = records
            .iter()
            .map(|r| (r.element_id, r.has_volume()))
            .collect();
        assert_eq!(volumes, vec![(100, false), (200, true)]);

        let mut hidden_flat = synth_record(1, OST_SPECIALITY_EQUIPMENT, flat);
        hidden_flat[0..8].fill(0xff);
        let mut hidden_solid = synth_record(1, OST_SPECIALITY_EQUIPMENT, solid);
        hidden_solid[0..8].fill(0xff);
        let buf = [nested(300, &hidden_flat), nested(301, &hidden_solid)].concat();
        assert_eq!(
            count_unattributed_frames(&buf, &[OST_SPECIALITY_EQUIPMENT]),
            BTreeMap::from([(OST_SPECIALITY_EQUIPMENT, 1)])
        );
    }
}
