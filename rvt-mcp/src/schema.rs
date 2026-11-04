//! The answers the tools return. Each tool publishes the schema of its type as
//! its `outputSchema` and returns the value as `structuredContent`, with the
//! same JSON as text. A field the model does not give is absent, never
//! guessed.

use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

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
    /// name were derived. Absent where the export has no record for it.
    pub trust_markers: BTreeMap<String, Value>,
    pub property_sets: Vec<PropertySetOut>,
    /// The element's type object and its sets, where the model has them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub element_type: Option<TypeOut>,
    /// Other entities with the same ElementId.
    pub other_entities: Vec<EntityRefOut>,
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
