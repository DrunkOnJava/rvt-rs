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
    /// The ElementId of the record's element, which need not be the element the source is of.
    pub element_id: u32,
    /// The partition the record is in, such as `Partitions/46`.
    pub stream: String,
    /// Where the record starts in that partition inflated; what starts there depends on the reader.
    pub offset: usize,
}

/// The copy of an element's data a reader took a value from. The readers, the rules and what each source's `parts` are, are listed in `docs/export-diagnostics.md`, "Value sources".
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValueSourceOut {
    /// The name of the reader that took the value, such as the reader of a wall's location line or of a type's parameters. Each reader and the body it belongs to is listed in `docs/export-diagnostics.md`, "Value sources".
    pub reader: String,
    /// How the copy was chosen. The rules are listed in `docs/export-diagnostics.md`, "Value sources"; for a source made of `parts` it is how each record's copy was chosen.
    pub rule: String,
    /// In how many partitions the element's data named it, so how many copies there were to choose from; for a source made of several `parts`, how many records it is made of; for one record, in how many partitions that record was held.
    pub copies: u32,
    /// The partition of the copy, such as `Partitions/46`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<String>,
    /// Where the copy starts in that partition inflated: the element-data header whose element id follows it, for the readers of element data; the record's header, for a native record; the frame, for a body read from the frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
    /// Where every copy holds the same value, the numbers of the partitions that hold a copy; the copy named is the lowest-numbered partition's.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[schemars(default)]
    pub partitions: Vec<u32>,
    /// The records the value is made of, where it is made of several or of another element's, at most 64. Such a source has no `stream` or `offset` of its own.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[schemars(default)]
    pub parts: Vec<SourcePartOut>,
    /// How many more records than the 64 listed in `parts` the value is made of.
    #[serde(skip_serializing_if = "is_zero")]
    #[schemars(default)]
    pub parts_left_out: u32,
    /// True when the value is held in another partition than `provenance.stream`, the frame: the copy named is in another and, where every copy holds the same value, so is every copy; for a source made of `parts`, none of them is in the frame's partition (`docs/export-diagnostics.md` says where a profile's records count as held there).
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
    /// `family_body`, and for an Area `area_plan_topology`,
    /// `area_boundary`, `area_boundary_joined` or `none`.
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
    #[schemars(default)]
    pub value_sources: Vec<ValueSourceOut>,
}

/// One element in a list.
#[derive(Debug, Serialize, JsonSchema)]
pub struct ElementRow {
    /// The Revit ElementId.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// The `Name` the IFC export writes. An `IfcSpace`'s is its room number,
    /// or `<room name>:<ElementId>` for a room with a name and no number.
    /// Absent only for a room with neither whose name is exactly the
    /// exporter's `<Revit class>-<ElementId>`, made where no source named
    /// it: that placeholder is not served as a name. The placeholder names
    /// of other elements (a wall nothing named is `Wall-<ElementId>`) are
    /// still served.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// An `IfcSpace`'s `LongName`, its room name. Absent for any other
    /// element and for a room whose name was not read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
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
    /// How many the reason counts. Mostly elements; for
    /// `element_record_frame_stopped` it is frames of record markers in every
    /// category (several frames can belong to one element), not lost elements.
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
    /// By IFC entity: the elements the model holds that the export does not
    /// write as elements, each a further piece of an element of separate
    /// loops, written as an item of the body of the element that owns it.
    /// They are not in `total`, `by_ifc_type` or the lists, nor in the
    /// diagnostics' `exported` counts, which hold them apart in
    /// `exported.body_pieces_folded`. Always present, empty where the export
    /// folds none.
    pub folded_into_bodies: BTreeMap<String, usize>,
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
    /// The type object's `Tag`: the type's Revit ElementId, or the ElementId
    /// of the room an `IfcSpaceType` is made for.
    pub element_id: String,
    /// The type object's IFC entity, such as `IFCWALLTYPE`.
    pub ifc_type: String,
    /// The type object's `Name`.
    pub name: String,
    /// The sets the export writes on the type object, `Other` and
    /// `Identity Data` among them.
    pub property_sets: Vec<PropertySetOut>,
}

/// A storey as one element sits on it.
#[derive(Debug, Serialize, JsonSchema)]
pub struct LevelRefOut {
    pub index: usize,
    pub name: String,
    pub elevation_feet: f64,
}

/// Another entity the export writes as an element for the same ElementId (a
/// slab's further pieces written as elements of their own, a curtain wall's
/// parts); a piece folded into another element's body is none.
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
    /// The `Name` the IFC export writes, as in `list_elements`: absent only
    /// for a room whose name is exactly the exporter's `<class>-<ElementId>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// An `IfcSpace`'s `LongName`, its room name, as in `list_elements`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
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
    /// The type object the export writes for the element, with its sets;
    /// absent where the export writes none for it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<TypeOut>,
    /// Other entities the export writes as elements with the same ElementId:
    /// not a piece folded into the body of the element, which is no entity of
    /// its own.
    pub other_entities: Vec<EntityRefOut>,
}

/// One fail-closed decision about an element (#1326).
#[derive(Debug, Serialize, JsonSchema)]
pub struct DecisionOut {
    /// The step that made it: `element_skipped`, `body_not_drawn`, `body_left_as_box`, `opening_not_cut`, `outline_not_read` or `storey_not_bound`.
    pub stage: String,
    /// The reason code: the `reason` of the `skipped` item in `get_diagnostics`, or `body_is_record_box`, `space_outline_is_record_box`, `area_body_refused`, `slab_shape_refused`, `wall_base_cut_refused` or `wall_end_extension_refused`.
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
    /// The `Name` the export writes for the element, as in `list_elements`:
    /// absent where the element is not exported, and for a room whose name is
    /// exactly the exporter's `<class>-<ElementId>` placeholder.
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
    /// or, for an Area, `area_plan_topology`, `area_boundary` or
    /// `area_boundary_joined`). Absent for
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
    /// Building elements the IFC export writes (the diagnostics'
    /// `exported.building_elements`), for `export_gltf` too: the glb also
    /// draws each piece of an element of separate loops that the IFC folds
    /// into the body of another element (`elements.folded_into_bodies`), which
    /// is not counted here or in `entity_counts`, and `notes` says how many.
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
