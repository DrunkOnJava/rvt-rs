//! The answers the tools return. Each tool publishes the schema of its type as
//! its `outputSchema` and returns the value as `structuredContent`, with the
//! same JSON as text. A field the model does not give is absent, never
//! guessed.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// How the file's `Global/ElemTable` stream frames its records.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ElemTableProvenance {
    /// Bytes per record (28 or 40 on every file in the corpus).
    pub record_stride: usize,
    /// Offset of the owner field within a record.
    pub owner_field_offset: usize,
    /// `explicit` for the 28- and 40-byte layouts, `implicit` for the
    /// 12-byte fallback no corpus file uses.
    pub framing: String,
    /// Width of the owner field, on the explicit layouts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner_field_bytes: Option<usize>,
}

/// What the file itself says about where its elements come from.
#[derive(Debug, Clone, Default, Serialize, JsonSchema)]
pub struct FileProvenance {
    /// The Revit release that saved the file (from `BasicFileInfo`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revit_release: Option<u32>,
    /// That release's build tag (from `BasicFileInfo`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revit_build: Option<String>,
    /// How many classes the file's embedded schema (`Formats/Latest`) lists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_class_count: Option<usize>,
    /// The record layout of `Global/ElemTable`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elem_table: Option<ElemTableProvenance>,
    /// How many `Partitions/NN` streams the file has.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition_count: Option<usize>,
    /// The release whose record layout the file takes, which can be older than the saving release.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_release: Option<u32>,
    /// How many elements the provenance table left out past its cap of 250,000 listed; their record fields are absent and `provenance_omitted` says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub omitted_entries: Option<usize>,
}

/// The native record the native readers hold for an element.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NativeRecordOut {
    /// In how many partitions the element has a native record.
    pub copies: u32,
    /// Which copy this is: `only`, `current` (the current one of several, RE-216) or `unresolved`.
    pub copy: String,
    /// The partition of that copy, such as `Partitions/46`. It can differ from `provenance.stream`, the frame the exporter selected. The record a reader decoded is in `provenance.value_sources`, where that reader notes its choice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the record's header starts in that partition inflated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// The class the record names in the file's schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
}

/// One record a value source is made of.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct SourcePartOut {
    /// The ElementId of the record's element: a sketch line, a type symbol, a run type.
    pub element_id: u32,
    /// The partition the record is in, such as `Partitions/46`.
    pub stream: String,
    /// Where the record starts in that partition inflated: its frame, or for a run type the place its id is stored.
    pub offset: usize,
}

/// The copy of an element's data a reader took a value from.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValueSourceOut {
    /// What was read: `bounded_line` (a wall's or beam's location line), `bounded_arc`, `instance_transform`, `saved_graphics`, `native_run`, `native_landing`, `selected_frame` for a body read from the frame itself (or, with a `sketch_lines` or `type_section` source beside it, for its box and placement only, the profile being that source's), or `selected_frame_box` for a body whose box and placement are read from the frame and whose profile is read from a record no reader notes yet (see `profile_source`). Also `sketch_lines` and `sketch_list` (the frames of a profile's sketch lines and the Sketch's curve list), `sketch_line_data`, `type_section` (the type symbol's frame), `stair_run_lines`, `stair_run_ends`, `stair_run_type`, `stair_run_base_cuts` and `landing_lines`.
    pub reader: String,
    /// How the copy was chosen: `selected` (the frame the exporter selected), `agreeing` (every copy holds the same value; the lowest-numbered partition's is named), `current` (the copies disagree; the current partition's), `native` (a native record by member name, its current copy) or `frame` (the frames of other elements' records, listed in `parts`).
    pub rule: String,
    /// In how many partitions the element's data named it, so how many copies there were to choose from; for a source made of `parts`, how many records it is made of.
    pub copies: u32,
    /// The partition of the copy, such as `Partitions/46`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the copy starts in that partition inflated: the element-data header whose element id follows it, for the readers of element data; the record's header, for a native record; the frame, for `selected_frame`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// For the rule `agreeing`, the numbers of the partitions that hold a copy, every one of which holds the same value; the copy named is the lowest-numbered partition's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub partitions: Vec<u32>,
    /// The records the value is made of, where it is made of several or of another element's (a profile's sketch lines, a type symbol's frame), at most 64. Such a source has no `stream` or `offset` of its own.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<SourcePartOut>,
    /// How many more records than the 64 listed in `parts` the value is made of.
    #[serde(skip_serializing_if = "is_zero")]
    pub parts_left_out: u32,
    /// True when the value is held in another partition than `provenance.stream`, the frame: the copy named is in another and, for `agreeing`, so is every copy that agrees with it.
    pub differs_from_frame: bool,
}

/// Where one element's answer comes from. Every element in an answer carries
/// one. Fields the model does not have are absent.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ElementProvenance {
    /// The Revit ElementId, which the IFC export writes as the `Tag`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_id: Option<String>,
    /// The IFC GlobalId the export writes: Revit's own where the file yields
    /// it, otherwise one derived from the document's identity and the
    /// ElementId.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_id: Option<String>,
    /// The IFC entity the export writes, such as `IFCWALL`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// The Revit class the element's record names, such as `Wall`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// How the element's body was derived, such as
    /// `partition_element_record_bbox` (only its record's box) or
    /// `partition_wall_centreline`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_source: Option<String>,
    /// Where the plan profile of the body came from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_source: Option<String>,
    /// For a room or space, where its outline came from: `record_box`
    /// (the area of its box, not of the room), `stored_solid`,
    /// `stored_solid_faces`, `stored_solid_approximate`, `saved_mesh`,
    /// `family_body`, and for an Area `area_boundary`,
    /// `area_boundary_joined` or `none`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub space_outline_source: Option<String>,
    /// For an Area with no body, why it has none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_body_refusal: Option<String>,
    /// The kind of record the element was read from, such as `partition_element_record_first_prologue`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carrier: Option<String>,
    /// The stream the record is in: `Partitions/NN` or `Global/Latest`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the record or frame starts in that stream inflated: the frame the exporter selected (on Revit 2024 and later the greatest vertical extent, then the higher-numbered partition and later offset; on 2008 to 2023 the newest). A body reader may take another copy; `value_sources` names it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Where the ElementId is stored: the partition record header it is the first bytes of.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_offset: Option<usize>,
    /// For a frame inside another partition record, the ElementId of that record.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enclosing_record_id: Option<String>,
    /// The class tag the record names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_tag: Option<u32>,
    /// The class that tag resolves to in the file's own schema.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    /// The element's native record, where the export's native passes indexed one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_record: Option<NativeRecordOut>,
    /// True when the record fields are absent because the provenance table lists at most 250,000 elements and this one is past them (`file.omitted_entries`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_omitted: Option<bool>,
    /// The copy each reader took a value of the element's body from, where the reader notes it and the body is drawn from that reader (`body_source`): `get_element` and `explain_element` list them, `list_elements` does not. Absent for a body no reader notes yet.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub value_sources: Vec<ValueSourceOut>,
}

/// One element in a list.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ElementRow {
    /// The Revit ElementId.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    /// The name of the storey the element is on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    /// The IFC4 `PredefinedType` the export writes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predefined_type: Option<String>,
    pub provenance: ElementProvenance,
}

/// One storey of the model.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LevelOut {
    /// Position in the model's list of storeys.
    pub index: usize,
    pub name: String,
    pub elevation_feet: f64,
    pub elevation_metres: f64,
    /// The building elements the export puts on this storey.
    pub element_count: usize,
}

/// A stream a resource limit kept from being read whole.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct UnreadOut {
    /// The stream, such as `Partitions/189`.
    pub name: String,
    pub stored_bytes: u64,
    /// `stream_too_large`, `inflated_member_too_large` or
    /// `inflated_total_too_large`.
    pub kind: String,
    /// That limit in bytes.
    pub limit: u64,
    /// The same, in words.
    pub description: String,
}

/// Something the export left out or approximated, with how many.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SkippedOut {
    pub reason: String,
    pub count: usize,
}

/// What the export read, approximated and left out. `get_diagnostics` has
/// the whole report.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DiagnosticsSummary {
    /// `scaffold`, or how far past it the model goes.
    pub confidence_level: String,
    pub confidence_score: f32,
    /// True when a resource limit kept part of the file from being read, so
    /// the answers miss what those streams hold.
    pub partial: bool,
    pub unread_streams: Vec<UnreadOut>,
    pub skipped: Vec<SkippedOut>,
    pub warning_count: usize,
    pub unsupported_feature_count: usize,
    /// Bodies that are only their element record's bounding box: placed and
    /// sized, shape not decoded.
    pub bounding_box_bodies: usize,
    /// Spaces whose area is the area of their record's box.
    pub bounding_box_spaces: usize,
    /// Element records found but not exported for want of an ElementId.
    pub unexported_element_records: usize,
    /// Steps of the export that were skipped or fell back, in words.
    pub notes: Vec<String>,
}

/// How many building elements the model holds.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ElementCounts {
    pub total: usize,
    /// By IFC entity, such as `IFCWALL`.
    pub by_ifc_type: BTreeMap<String, usize>,
    /// By the Revit class the record names, such as `Wall`.
    pub by_revit_class: BTreeMap<String, usize>,
}

/// What `open_model` and `get_summary` return.
#[derive(Debug, Serialize, JsonSchema)]
pub struct SummaryOut {
    /// Opaque handle for the other tools. It lives while the server runs and
    /// while the model is among the most recently used ones it keeps.
    pub model_id: String,
    /// `rvt`, `rfa`, `rte`, `rft` or `unknown`, from the file name extension.
    pub file_kind: String,
    pub size_bytes: u64,
    /// The Revit release that saved the file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revit_release: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// How the file's release is read, and that row's status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_support: Option<Value>,
    pub file: FileProvenance,
    pub elements: ElementCounts,
    pub levels: Vec<LevelOut>,
    pub room_count: usize,
    pub diagnostics: DiagnosticsSummary,
    /// What the answers do not cover.
    pub not_read: String,
}

/// A page of `list_elements`.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ListElementsOut {
    pub model_id: String,
    /// How many elements match the filters.
    pub total: usize,
    pub offset: usize,
    pub limit: usize,
    pub returned: usize,
    pub elements: Vec<ElementRow>,
}

/// One property or quantity, as the IFC export writes it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PropertyOut {
    pub name: String,
    /// For a property, its IfcValue type, such as `IFCLENGTHMEASURE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ifc_type: Option<String>,
    /// For a quantity, its IFC4 quantity type, such as `IFCQUANTITYLENGTH`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    /// The value in the units the IFC is written in: metres, square metres,
    /// cubic metres, kilograms, radians.
    pub value: Value,
}

/// A property set or a quantity set of an element or its type.
#[derive(Debug, Serialize, JsonSchema)]
pub struct PropertySetOut {
    pub name: String,
    /// `properties`, or `quantities` for a `Qto_` set.
    pub kind: String,
    pub properties: Vec<PropertyOut>,
}

/// The type object an element is typed by.
#[derive(Debug, Serialize, JsonSchema)]
pub struct TypeOut {
    /// The type's Revit ElementId.
    pub element_id: String,
    pub property_sets: Vec<PropertySetOut>,
}

/// A storey as one element sits on it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LevelRefOut {
    pub index: usize,
    pub name: String,
    pub elevation_feet: f64,
}

/// Another entity the export writes for the same ElementId (a slab's further
/// pieces, a curtain wall's parts).
#[derive(Debug, Serialize, JsonSchema)]
pub struct EntityRefOut {
    pub ifc_type: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global_id: Option<String>,
}

/// What `get_element` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ElementDetail {
    pub model_id: String,
    /// The Revit ElementId.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<LevelRefOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub predefined_type: Option<String>,
    /// Where the element is placed, in the model's feet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location_feet: Option<[f64; 3]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation_degrees: Option<f64>,
    /// The ElementId of the wall that hosts a door or window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_id: Option<String>,
    /// `extrusion`, `solid`, `mapped` or `none`: the kind of body the export
    /// writes for the element.
    pub body: String,
    pub provenance: ElementProvenance,
    /// The file the element comes from.
    pub file: FileProvenance,
    /// Every `...Source`, `...Resolved` and `...Refusal` property of the
    /// element's record: how its body, profile, thickness, level and family
    /// name were derived. Empty (`{}`) where the export has no record for it.
    pub trust_markers: BTreeMap<String, Value>,
    pub property_sets: Vec<PropertySetOut>,
    /// The element's type object and its sets, where the model has them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<TypeOut>,
    /// Other entities with the same ElementId.
    pub other_entities: Vec<EntityRefOut>,
}

/// One fail-closed decision about an element (#1326).
#[derive(Debug, Serialize, JsonSchema)]
pub struct DecisionOut {
    /// The step that made it: `element_skipped`, `body_not_drawn`, `body_left_as_box`, `opening_not_cut`, `outline_not_read` or `storey_not_bound`.
    pub stage: String,
    /// The reason code: the `reason` of the `skipped` item in `get_diagnostics`, or `body_is_record_box`, `space_outline_is_record_box` or `area_body_refused`.
    pub reason: String,
    /// The class the diagnostics count it under: a Revit class, or an IFC entity such as `IFCDOOR`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// The stream its record was read from, where known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// The byte offset of the record in that stream inflated, where known. For the element-record reasons it is the element's last frame in scan order (partition number, then offset), which can differ from the frame `get_element`'s provenance names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// What tells the reason apart: a phase's name, why an Area has no body, the `BodySource` of a box body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

// What `explain_element` returns: the element, in the export or not, with
// every decision recorded for it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ExplainElementOut {
    pub model_id: String,
    /// The Revit ElementId asked about.
    pub id: String,
    /// Whether the export holds the element. A left-out element has decisions and no provenance.
    pub exported: bool,
    /// The element's name in the export, where it is exported.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Where the element's record was read from, where the export holds it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance: Option<ElementProvenance>,
    /// Every `...Source`, `...Resolved` and `...Refusal` property of the element's record. Empty (`{}`) where the export has no record for it.
    pub trust_markers: BTreeMap<String, Value>,
    /// Every decision recorded for the element, by reason.
    pub decisions: Vec<DecisionOut>,
    /// The reasons whose log was full: a decision of one of these about this element may be missing.
    pub capped_reasons: Vec<String>,
    /// The file the element comes from.
    pub file: FileProvenance,
}

/// One room.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RoomOut {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_elevation_feet: Option<f64>,
    /// The plan area of the room's outline, where the outline is one read
    /// exactly (`space_outline_source` is `stored_solid`, `stored_solid_faces`
    /// or, for an Area, `area_boundary` or `area_boundary_joined`). Absent for
    /// `record_box`, where the area of a box is not the area of a room, and for
    /// `stored_solid_approximate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_square_feet: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area_square_metres: Option<f64>,
    pub provenance: ElementProvenance,
}

/// What `list_rooms` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct RoomsOut {
    pub model_id: String,
    pub rooms: Vec<RoomOut>,
}

/// What `list_levels` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LevelsOut {
    pub model_id: String,
    pub levels: Vec<LevelOut>,
}

/// What `get_diagnostics` returns.
#[derive(Debug, Serialize, JsonSchema)]
pub struct DiagnosticsOut {
    pub model_id: String,
    /// The export diagnostics, as `rvt-ifc --diagnostics` writes them
    /// (`docs/schemas/export-diagnostics.schema.json`).
    pub diagnostics: Value,
    pub unread_streams: Vec<UnreadOut>,
    pub notes: Vec<String>,
}

/// The file an export wrote.
#[derive(Debug, Serialize, JsonSchema)]
pub struct OutputOut {
    /// Absolute path of the file.
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

/// What `export_ifc` and `export_gltf` return, with a `resource_link` to the
/// file beside it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ExportOut {
    pub model_id: String,
    /// `ifc` or `glb`.
    pub format: String,
    pub output: OutputOut,
    /// Building elements the export wrote.
    pub building_elements: usize,
    pub building_elements_with_geometry: usize,
    /// By IFC entity.
    pub entity_counts: BTreeMap<String, usize>,
    /// True when part of the file was not read and the export was written
    /// anyway (`allow_partial`).
    pub partial: bool,
    pub unread_streams: Vec<UnreadOut>,
    pub notes: Vec<String>,
    /// The export diagnostics (`ifc` only), as for `get_diagnostics`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Value>,
}
