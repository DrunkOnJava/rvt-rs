//! Mode-aware IFC content policy.
//!
//! [`ExportQualityMode`] is both a *validation gate* (see
//! [`super::ExportQualityMode::validate`]) and a *content policy*
//! for what the document exporter emits:
//!
//! | Mode | Content |
//! |---|---|
//! | `scaffold` | Framework + production elements; geometry when recovered. No diagnostic proxies. |
//! | `typed-no-geometry` | Mapped / typed elements only; geometry fields stripped. |
//! | `geometry` | Mapped / typed elements; attach recovered curves, loops, hosts and elevations when present. |
//! | `strict` | Same emission as `geometry`; validation then fails closed on incomplete output. |
//!
//! `HostObjAttr` and other low-confidence parent-class hits are never
//! emitted on the production path — use [`super::DiagnosticRvtDocExporter`]
//! for research proxies.

use super::category_map;
use super::entities::{self, Extrusion, Property, PropertySet, PropertyValue};
use super::from_decoded::{wall_segment_angle_radians, wall_segment_length_feet};
use super::{ExportQualityMode, MaterialInfo, Storey, UNRESOLVED_ARCWALL_THICKNESS_FEET};
use crate::elements::floor::Floor;
use crate::elements::level::Level;
use crate::elements::openings::{Door, Window};
use crate::elements::styling::Material;
use crate::elements::wall::Wall;
use crate::geometry::{
    recover_door_host, recover_floor_boundary, recover_level_elevation,
    recover_wall_location_curve_from_wall, recover_window_host,
};
use crate::walker::{DecodedElement, InstanceField};
use std::collections::{BTreeMap, HashMap};

mod wall_areas;
mod wall_tops;

/// Result of folding production walker hits into an IFC draft.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct TypedProductionAppend {
    /// Revit element id → index in `entities` (for host linking).
    pub id_to_entity: HashMap<u32, usize>,
    /// Named materials recovered from partition / typed Material rows.
    pub materials: Vec<MaterialInfo>,
    /// Production class histogram (Level / Floor / Material / …).
    pub production_class_counts: BTreeMap<String, usize>,
    /// Floors/Rooms assigned to a storey via Level ElementId bind.
    /// Stays 0 on current corpora (partition Levels lack ElementIds).
    pub level_elementid_binds: usize,
    /// Layered elements' layers, by ElementId (RE-53).
    pub element_layers: BTreeMap<u32, super::ElementLayers>,
    /// The layers of walls their data does not place, by ElementId, for
    /// their materials only (RE-88).
    pub unplaced_wall_layers: BTreeMap<u32, Vec<super::LayerBand>>,
    /// The materials each element's type draws its geometry in, by
    /// ElementId (RE-82).
    pub element_type_materials: BTreeMap<u32, Vec<String>>,
    /// Each element's type, by ElementId (RE-110).
    pub element_type_ids: BTreeMap<u32, u32>,
    /// The standard property sets Revit's export gives each type object, by
    /// the type's ElementId and the STEP name of the type entity
    /// (`IFCWALLTYPE`): its common set's `IsExternal`, `FireRating` and
    /// `LoadBearing` (RE-286), and a stair or flight type's dimensions
    /// (RE-286). A type whose elements disagree has none.
    pub type_entity_sets: BTreeMap<u32, BTreeMap<String, Vec<PropertySet>>>,
    /// Each family instance's original symbol, by ElementId (RE-167).
    pub element_original_symbols: BTreeMap<u32, u32>,
    /// Whether Revit's exporter takes each door's and window's symbol as
    /// flipped, by ElementId: read in every quality mode, as the flip
    /// is part of the type's GlobalId.
    // Tracker: B72, B74.
    pub element_flips: BTreeMap<u32, bool>,
    /// The entity indexes of the `Pset_PipeSegmentOccurrence` sets whose
    /// `InvertElevation` rvt-rs worked out (RE-157). Nothing reads them any
    /// more: the invert is written as the height above the model's zero, so
    /// no step takes a storey's elevation off it.
    pub computed_inverts: Vec<usize>,
    /// The further pieces written inside their element's body rather than
    /// as elements of their own (#823), by index in the entities
    /// ([`super::IfcModel::body_pieces`]).
    pub body_pieces: BTreeMap<usize, Vec<usize>>,
}

/// What a quality mode allows the document exporter to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportContentPolicy {
    /// Attach recovered placement / extrusion / host links when present.
    pub include_geometry: bool,
    /// Drop walker hits that have no [`category_map`] entry (no PROXY fallback).
    pub require_mapped_ifc_type: bool,
}

impl ExportContentPolicy {
    pub fn for_quality_mode(mode: ExportQualityMode) -> Self {
        match mode {
            ExportQualityMode::Scaffold => Self {
                include_geometry: true,
                require_mapped_ifc_type: false,
            },
            ExportQualityMode::TypedNoGeometry => Self {
                include_geometry: false,
                require_mapped_ifc_type: true,
            },
            ExportQualityMode::Geometry | ExportQualityMode::Strict => Self {
                include_geometry: true,
                require_mapped_ifc_type: true,
            },
        }
    }
}

/// Property-set name carried by a body recovered from a partition
/// element record's bounding box (#204, #211).
pub const ELEMENT_RECORD_PROPERTY_SET: &str = "RvtElementRecordGeometry";

/// Property naming an element-record element's family (RE-38).
pub const FAMILY_NAME_PROPERTY: &str = "FamilyName";

/// Property naming an element-record element's type (RE-38).
pub const TYPE_NAME_PROPERTY: &str = "TypeName";

/// The `(family, type)` names RE-38 attached to `decoded`, when both are.
fn family_and_type(decoded: &DecodedElement) -> Option<(String, String)> {
    let field = |key: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(v) if name == key => Some(v.clone()),
            _ => None,
        })
    };
    Some((
        field(crate::partition_schema_mvp::FAMILY_NAME_FIELD)?,
        field(crate::partition_schema_mvp::TYPE_NAME_FIELD)?,
    ))
}

/// Decoded classes whose record bounding-box `z` extent is the
/// element's own thickness rather than an envelope height (#212).
pub const SLAB_THICKNESS_CLASSES: &[&str] = &["Floor", "BuildingPad"];

/// Classes whose sketch of separate loops Revit's export writes as one
/// element with a body item per piece, where it writes a floor as one
/// element per piece (#823, #331): the BIM4LCA office's compound ceilings
/// (Measure run 39240535083).
pub const PIECES_IN_BODY_CLASSES: &[&str] = &["Ceiling"];

/// Value of the `ThicknessSource` property on a record-backed plate.
pub const RECORD_BBOX_THICKNESS_SOURCE: &str = "partition_element_record_bbox_z_extent";

/// Property carrying a recovered room number (#90, RE-29).
///
/// The STEP writer reads this back as `IfcSpace.Name`, as Revit's own
/// exporter writes it.
pub const ROOM_NUMBER_PROPERTY: &str = "RoomNumber";

/// Property carrying a room's Revit ElementId. `IfcSpace` declares no
/// `Tag`, the attribute every other element carries it in, and its `Name`
/// is the room number, so the id is written here.
pub const ROOM_ELEMENT_ID_PROPERTY: &str = "ElementId";

/// Property carrying a recovered room name (#90, RE-29).
///
/// The STEP writer reads this back to fill `IfcSpace.LongName`, the
/// slot Revit's own exporter puts the room name in.
pub const ROOM_NAME_PROPERTY: &str = "RoomName";

/// Classes that must never appear as production building elements.
pub fn is_misleading_proxy_class(class_name: &str) -> bool {
    matches!(
        class_name,
        "HostObjAttr" | "HostObject" | "Element" | "Symbol"
    )
}

/// Whether an IFC4 entity name is an `IfcDistributionElement` among those the
/// exporter writes: the only kind of element an `IfcDistributionPort` may be
/// tied to (RE-138). Equipment written as a building element proxy is not.
pub(crate) fn is_distribution_element(ifc_type: &str) -> bool {
    ifc_type.starts_with("IFCFLOW")
        || matches!(
            ifc_type,
            "IFCDUCTSEGMENT"
                | "IFCDUCTFITTING"
                | "IFCPIPESEGMENT"
                | "IFCPIPEFITTING"
                | "IFCAIRTERMINAL"
                | "IFCSANITARYTERMINAL"
                | "IFCWASTETERMINAL"
                | "IFCFIRESUPPRESSIONTERMINAL"
                | "IFCLIGHTFIXTURE"
                | "IFCELECTRICAPPLIANCE"
                | "IFCALARM"
        )
}

/// An element's place in plan, for a door's or window's flip.
// Tracker: B72.
struct PlanFrame {
    location: [f64; 2],
    /// The direction of the element's axis, where it has a body.
    axis_turn: Option<f64>,
    /// The same, from the body an aggregate whole holds until its parts
    /// are known.
    held_axis_turn: Option<f64>,
}

/// The direction of a body's axis in plan: its rotation, turned a quarter
/// where the body is deeper than it is wide, so the axis runs along its
/// longer side.
fn plan_axis_turn(rotation_radians: Option<f64>, body: &Extrusion) -> f64 {
    rotation_radians.unwrap_or(0.0)
        + if body.width_feet >= body.depth_feet {
            0.0
        } else {
            std::f64::consts::FRAC_PI_2
        }
}

/// Whether Revit's exporter takes a door's or window's symbol as flipped
/// (revit-ifc `DoorWindowInfo.CalculateDoorWindowInformation`): whether
/// the side of its host wall's axis its box centre lies on differs from the
/// side its own Y axis (`facing`) points to, a side being +Y of the wall's
/// direction. The direction's sign cancels out. A door with no host wall, or
/// no facing read, is not flipped, as Revit's exporter has it for a door
/// without a wall.
fn door_symbol_flipped(door: [f64; 2], wall: [f64; 2], wall_turn: f64, facing: [f64; 2]) -> bool {
    const EPS: f64 = 1e-9;
    let wall_y = [-wall_turn.sin(), wall_turn.cos()];
    let offset = (door[0] - wall[0]) * wall_y[0] + (door[1] - wall[1]) * wall_y[1];
    let positive_hinge_side = offset > -EPS;
    let facing_positive = wall_y[0] * facing[0] + wall_y[1] * facing[1] > -EPS;
    positive_hinge_side != facing_positive
}

/// The record properties that say what an element is rather than where it
/// is or how it is drawn, which an export without geometry keeps,
/// along with the type's text parameters (RE-77).
// Tracker: B74.
const IDENTITY_PROPERTIES: [&str; 10] = [
    crate::element_record_level_refs::LEVEL_ELEMENT_ID_PROPERTY,
    "LevelBindSource",
    ROOM_ELEMENT_ID_PROPERTY,
    ROOM_NUMBER_PROPERTY,
    ROOM_NAME_PROPERTY,
    "SourceStream",
    "RevitClass",
    FAMILY_NAME_PROPERTY,
    "FamilyNameSource",
    TYPE_NAME_PROPERTY,
];

/// `set` less every property that is not one of [`IDENTITY_PROPERTIES`] or
/// a type text parameter of `decoded`.
// Tracker: B74.
fn identity_property_set(mut set: PropertySet, decoded: &DecodedElement) -> PropertySet {
    let type_parameters: Vec<&str> = decoded
        .fields
        .iter()
        .filter_map(|(name, _)| {
            name.strip_prefix(crate::partition_schema_mvp::TYPE_PARAMETER_FIELD_PREFIX)
        })
        .collect();
    set.properties.retain(|property| {
        IDENTITY_PROPERTIES.contains(&property.name.as_str())
            || type_parameters.contains(&property.name.as_str())
    });
    set
}

/// An Area's ElementId, number, name and Level (#763), in the set a room's
/// are written in, so the writer names its `IfcSpace` as it names a room's
/// (`Name` the number and `LongName` the name, as Revit's export does) and
/// `ifc::apply_record_level_reference_storeys` puts it in its Level's storey.
fn area_property_set(decoded: &DecodedElement) -> PropertySet {
    let text = |wanted: &str| {
        decoded
            .fields
            .iter()
            .find_map(|(field, value)| match value {
                InstanceField::String(text) if field == wanted && !text.is_empty() => {
                    Some(text.clone())
                }
                _ => None,
            })
    };
    let mut properties = Vec::new();
    if let Some(id) = decoded.id {
        properties.push(Property {
            name: ROOM_ELEMENT_ID_PROPERTY.into(),
            value: PropertyValue::Integer(i64::from(id)),
        });
    }
    if let Some(number) = text(crate::partition_schema_mvp::ROOM_NUMBER_FIELD) {
        properties.push(Property {
            name: ROOM_NUMBER_PROPERTY.into(),
            value: PropertyValue::Text(number),
        });
    }
    if let Some(name) = text("m_name") {
        properties.push(Property {
            name: ROOM_NAME_PROPERTY.into(),
            value: PropertyValue::Text(name),
        });
    }
    let level = decoded
        .fields
        .iter()
        .find_map(|(field, value)| match value {
            InstanceField::ElementId { id, .. }
                if field == crate::element_record_level_refs::LEVEL_REFERENCE_FIELD =>
            {
                Some(*id)
            }
            _ => None,
        });
    if let Some(id) = level {
        properties.push(Property {
            name: crate::element_record_level_refs::LEVEL_ELEMENT_ID_PROPERTY.into(),
            value: PropertyValue::Integer(i64::from(id)),
        });
        properties.push(Property {
            name: "LevelBindSource".into(),
            value: PropertyValue::Text(
                crate::element_record_level_refs::LEVEL_REFERENCE_SOURCE.into(),
            ),
        });
    }
    PropertySet {
        name: ELEMENT_RECORD_PROPERTY_SET.into(),
        properties,
    }
}

/// Strip placement / body / host claims so an export cannot over-claim geometry.
pub fn strip_building_element_geometry(entities: &mut [entities::IfcEntity]) {
    for entity in entities.iter_mut() {
        if let entities::IfcEntity::BuildingElement {
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index,
            solid_shape,
            representation_map_index,
            ..
        } = entity
        {
            *location_feet = None;
            *rotation_radians = None;
            *extrusion = None;
            *host_element_index = None;
            *solid_shape = None;
            *representation_map_index = None;
        }
        // #847: a port's point and axis are geometry too.
        if let entities::IfcEntity::Port {
            location_feet,
            direction,
            ..
        } = entity
        {
            *location_feet = None;
            *direction = None;
        }
    }
}

/// Accumulate production walker hits into IFC entities + storeys + materials.
pub fn append_typed_production_elements(
    decoded_iter: impl Iterator<Item = DecodedElement>,
    entities: &mut Vec<entities::IfcEntity>,
    building_storeys: &mut Vec<Storey>,
    policy: ExportContentPolicy,
) -> TypedProductionAppend {
    let mut out = TypedProductionAppend::default();
    let mut pending_hosts: Vec<(usize, u32)> = Vec::new();
    // RE-84: doors and windows whose type draws no geometry.
    let mut without_geometry: std::collections::BTreeSet<usize> = Default::default();
    let mut pending_parts: Vec<(usize, u32)> = Vec::new();
    let mut pending_bounded: Vec<(usize, Vec<u32>)> = Vec::new();
    // RE-194: each element's parameters named for a standard set's property,
    // written once the entity it is written as is final.
    let mut pending_named: Vec<(usize, Vec<(String, Property)>)> = Vec::new();
    // #331, #816: each element written in pieces, and its further pieces.
    let mut pieces_of: Vec<(usize, std::ops::Range<usize>)> = Vec::new();
    // #822: the sets holding `Reference` for each entity and predefined type
    // written, worked out once.
    let mut reference_kinds: ReferenceKinds = Default::default();
    // RE-138: the joins of ducts and pipes, by entity index and ElementId.
    let mut pending_joins: Vec<(usize, u32, crate::partition_schema_mvp::ConnectorJoins)> =
        Vec::new();
    // B54: each duct and pipe, by entity index and ElementId.
    let mut curve_ends: Vec<(usize, u32, &'static str)> = Vec::new();
    // #847: each pipe's two ends and the connector entry keys at each.
    let mut pipe_ends: BTreeMap<usize, PipeEnds> = BTreeMap::new();
    // RE-162: each MEP system's name and members, by its ElementId.
    let mut pending_systems: BTreeMap<u32, (Option<String>, Vec<usize>)> = Default::default();
    // Bodies of aggregate wholes, held back until their parts are known: a
    // whole that no part names keeps its own body.
    let mut held_bodies: BTreeMap<usize, Extrusion> = BTreeMap::new();
    // B72: each element's plan frame by entity index, and each door's and
    // window's facing and host, for the flip its type's GlobalId hashes.
    let mut frames: HashMap<usize, PlanFrame> = Default::default();
    let mut facings: BTreeMap<u32, [f64; 2]> = BTreeMap::new();
    let mut opening_hosts: Vec<(usize, u32, u32)> = Vec::new();
    // RE-267: each wall's entity index and the elements its top join and cut
    // pairs name, for the cut at the floor that covers it.
    let mut pending_tops: Vec<(usize, Vec<u32>)> = Vec::new();
    let mut level_bind = crate::level_bind::LevelStoreyBind::new();
    // RE-286: what each type object's common set holds, by type and type
    // entity; `None` once two of its elements disagree.
    let mut type_commons: BTreeMap<(u32, String), Option<TypeCommon>> = BTreeMap::new();

    for decoded in decoded_iter {
        *out.production_class_counts
            .entry(decoded.class.clone())
            .or_insert(0) += 1;

        if is_misleading_proxy_class(&decoded.class) {
            continue;
        }

        if decoded.class == "Level" {
            if let Some(storey) = storey_from_level(&decoded) {
                let storey_index = building_storeys.len();
                // Fail-closed: only record when the Level carries an ElementId.
                level_bind.record_level(decoded.id, storey_index);
                building_storeys.push(storey);
            }
            continue;
        }

        // RE-53: a wall's layers, for the glTF export to draw in colour.
        if policy.include_geometry {
            if let (Some(id), Some(mut layers)) = (
                decoded.id,
                crate::partition_schema_mvp::element_layers_from_fields(&decoded.fields),
            ) {
                // The element's own system family name, localized
                // (RE-123), where one was given; else the class's.
                layers.system_family = decoded
                    .fields
                    .iter()
                    .find_map(|(name, value)| match value {
                        InstanceField::String(family)
                            if name == crate::partition_schema_mvp::FAMILY_NAME_FIELD =>
                        {
                            Some(family.clone())
                        }
                        _ => None,
                    })
                    .or_else(|| {
                        crate::partition_schema_mvp::system_family(&decoded.class, true)
                            .map(String::from)
                    });
                out.element_layers.insert(id, layers);
            } else if let (Some(id), Some(bands)) = (
                decoded.id,
                crate::partition_schema_mvp::unplaced_wall_layers_from_fields(&decoded.fields),
            ) {
                out.unplaced_wall_layers.insert(id, bands);
            }
        }

        // RE-82: the materials the element's type draws its geometry in.
        if let Some(id) = decoded.id {
            let names: Vec<String> = decoded
                .fields
                .iter()
                .filter_map(|(name, value)| match value {
                    InstanceField::String(text)
                        if name == crate::partition_schema_mvp::TYPE_MATERIAL_FIELD =>
                    {
                        Some(text.clone())
                    }
                    _ => None,
                })
                .collect();
            if !names.is_empty() {
                out.element_type_materials.insert(id, names);
            }
            // RE-110: the element's type, which the writer types it by.
            if let Some(type_id) = decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == crate::partition_schema_mvp::TYPE_ID_FIELD =>
                {
                    Some(*id)
                }
                _ => None,
            }) {
                out.element_type_ids.insert(id, type_id);
            }
            // RE-167: the original symbol, whose GlobalId the type takes.
            if let Some(original) = decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == crate::partition_schema_mvp::ORIGINAL_SYMBOL_FIELD =>
                {
                    Some(*id)
                }
                _ => None,
            }) {
                out.element_original_symbols.insert(id, original);
            }
            // B72: a door's or window's facing, for its type's flip.
            if let Some(facing) =
                crate::partition_schema_mvp::opening_facing_from_fields(&decoded.fields)
            {
                facings.insert(id, facing);
            }
        }

        // Materials are IfcMaterial rows, never building-element proxies.
        if decoded.class == "Material" {
            if let Some(info) = material_info_from_decoded(&decoded) {
                out.materials.push(info);
            }
            continue;
        }

        // ArcWall IFC emission stays on the partition path in
        // `ifc::mod` (storey index from recovered elevations). The
        // walker still yields ArcWall `DecodedElement`s for API
        // consumers; skipping here avoids duplicate IFCWALL rows.
        if decoded.class == "ArcWall" {
            continue;
        }

        // Opening-index rows are not typed Door/Window. Keep them in
        // `iter_elements` / production_class_counts for File Status, but
        // do not flood IFC with thousands of hostless IFCOPENINGELEMENT
        // stubs — emit only once host Wall ElementIds join (still open).
        if decoded.class == "ArcWallRectOpening" {
            continue;
        }

        let mapping = category_map::lookup(&decoded.class);
        if mapping.is_none() && policy.require_mapped_ifc_type {
            continue;
        }
        // Scaffold must not invent PROXY rows for unmapped partition
        // MVP classes (e.g. stray research tags).
        let Some(mapping) = mapping else {
            continue;
        };
        // A per-element Revit "IFC Export As" override redirects the
        // entity type (#212, RE-22). Only values `category_map`
        // recognises are honoured; anything else keeps the class
        // mapping, so an unknown override can never invent a type.
        // #864: Revit's export writes a mass only where its own "Export to
        // IFC As" names an entity, as the BIM4LCA office's two VOLUME masses
        // name IfcSpace.
        if decoded.class == "Mass" && ifc_export_override(&decoded).is_none() {
            continue;
        }
        let effective = ifc_export_override(&decoded).unwrap_or(mapping);
        let ifc_type = effective.ifc_type.to_string();
        let family_space = space_by_export_value(&decoded);
        // RE-45: the element's own or its type's IFC predefined type, when
        // it is an enumerator of the entity it exports as; a family instance
        // written as a space through its value takes its own (#763).
        let predefined_type = decoded
            .fields
            .iter()
            .find_map(|(name, value)| match value {
                InstanceField::String(text)
                    if name == crate::partition_schema_mvp::IFC_PREDEFINED_TYPE_FIELD =>
                {
                    category_map::predefined_type_for(effective.ifc_type, text).or_else(|| {
                        family_space
                            .then(|| category_map::space_predefined_type_for(text))
                            .flatten()
                    })
                }
                _ => None,
            })
            .or_else(|| {
                // #323: a flight takes the shape its treads give.
                (effective.ifc_type == "IFCSTAIRFLIGHT")
                    .then(|| {
                        decoded.fields.iter().find_map(|(name, value)| match value {
                            InstanceField::Integer { value, .. }
                                if name == crate::partition_schema_mvp::STAIR_RUN_SHAPE_FIELD =>
                            {
                                crate::native_stairs::flight_predefined_type(*value)
                            }
                            _ => None,
                        })
                    })
                    .flatten()
            })
            .or_else(|| {
                // #323: a stair takes the type its runs and landings give.
                (effective.ifc_type == "IFCSTAIR")
                    .then(|| {
                        decoded.fields.iter().find_map(|(name, value)| match value {
                            InstanceField::String(text)
                                if name == crate::partition_schema_mvp::STAIR_TYPE_FIELD =>
                            {
                                crate::native_stairs::STAIR_TYPES
                                    .iter()
                                    .copied()
                                    .find(|kind| *kind == text.as_str())
                            }
                            _ => None,
                        })
                    })
                    .flatten()
            })
            .or(effective.predefined_type);

        // RE-65: a stair and its parts carry the name Revit gives them.
        let own_name = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(text)
                if name == crate::partition_schema_mvp::ELEMENT_NAME_FIELD =>
            {
                Some(text.clone())
            }
            _ => None,
        });
        // Revit names electrical equipment `Family:<Panel Name>:ElementId`
        // (RE1 Electrical: `262416-PANEL:RE-1:428352`), from an instance
        // parameter rvt-rs does not read, so it gets no `Family:Type` name.
        let named_by_type = decoded.class != "ElectricalEquipment";
        // RE-154: the type name. A door or window whose type draws no
        // geometry is written as its opening (RE-84), which takes none.
        let opening_only = matches!(decoded.class.as_str(), "Door" | "Window")
            && decoded.fields.iter().any(|(name, value)| {
                name == crate::partition_schema_mvp::TYPE_WITHOUT_GEOMETRY_FIELD
                    && matches!(value, InstanceField::Bool(true))
            });
        // A duct's system family follows its type's shape, read through the
        // native record path (B59); where it is not read, the duct's
        // Reference is still its type's name (RE-134).
        let duct_type = (decoded.class == "Duct")
            .then(|| {
                decoded.fields.iter().find_map(|(name, value)| match value {
                    InstanceField::String(text)
                        if name == crate::partition_schema_mvp::TYPE_NAME_FIELD =>
                    {
                        Some(text.clone())
                    }
                    _ => None,
                })
            })
            .flatten();
        // The Reference is the type's name however Revit names the element:
        // electrical equipment named by its panel name has one too (B29).
        // A stair, run or landing named by RE-65 still has its own type's.
        let run_type = matches!(
            decoded.class.as_str(),
            "Stair" | "StairsRun" | "StairsLanding"
        )
        .then(|| {
            decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::String(text)
                    if name == crate::partition_schema_mvp::TYPE_NAME_FIELD =>
                {
                    Some(text.clone())
                }
                _ => None,
            })
        })
        .flatten();
        let reference_type = family_and_type(&decoded)
            .filter(|_| own_name.is_none() && !opening_only)
            .map(|(_, type_name)| type_name)
            .or(duct_type)
            .or(run_type);
        let name = match (
            own_name,
            decoded.id,
            family_and_type(&decoded).filter(|_| named_by_type),
        ) {
            (Some(name), _, _) => name,
            // RE-38: `Family:Type:ElementId`, the name Revit's own export
            // gives the element.
            (None, Some(id), Some((family, type_name))) => format!("{family}:{type_name}:{id}"),
            (None, Some(id), None) => format!("{}-{}", decoded.class, id),
            (None, None, _) => format!("{}-unnamed", decoded.class),
        };
        let type_guid = decoded.id.map(|id| id.to_string());

        let mut location_feet = None;
        let mut rotation_radians = None;
        let mut extrusion = None;
        let mut property_set = None;
        let mut pending_host_id = None;
        let mut piece_bodies: Vec<Extrusion> = Vec::new();
        let mut void_bodies: Vec<VoidBody> = Vec::new();
        let mut held_body = None;
        let mut solid_shape = None;

        // Read in every mode: a door's or window's type GlobalId hangs on
        // its flip against its host wall (B72), and the record's property
        // set says what the element is (B74). A mode without geometry keeps
        // only the plan frames and that, below.
        {
            // Partition element records carry their own model bbox for
            // every category, so one envelope path serves all of them
            // (#204 columns, #211 walls / doors / windows). Checked
            // before the class arms so a schema-field decoder is never
            // asked to read element-record fields.
            let element_record_geometry = if is_partition_element_record(&decoded) {
                element_record_geometry_from_decoded(&decoded)
            } else {
                None
            };
            if let Some(RecordGeometry {
                location,
                rotation,
                body,
                solid,
                properties,
                pieces,
                void_openings,
            }) = element_record_geometry
            {
                location_feet = Some(location);
                rotation_radians = rotation;
                solid_shape = solid;
                piece_bodies = pieces;
                void_bodies = void_openings;
                // #323 / RE-46: a stair or curtain wall is carried by its
                // parts, as in Revit's export; its own record box would
                // double their volume. It is held until the parts are known.
                if crate::partition_schema_mvp::AGGREGATE_WHOLE_CLASSES
                    .contains(&decoded.class.as_str())
                {
                    held_body = Some(body);
                } else {
                    extrusion = Some(body);
                }
                property_set = Some(properties);
            } else {
                match decoded.class.as_str() {
                    "Wall" | "ArcWall" => {
                        if let Some(geom) = wall_geometry_from_decoded(&decoded) {
                            location_feet = Some(geom.location_feet);
                            rotation_radians = Some(geom.rotation_radians);
                            extrusion = Some(geom.extrusion);
                            property_set = geom.property_set;
                        }
                    }
                    "Floor" => {
                        if let Some(geom) = floor_boundary_annotation_from_decoded(&decoded) {
                            // Boundary without resolved thickness: record
                            // provenance only — do not claim an extruded body.
                            property_set = Some(geom);
                        }
                    }
                    // #763: an Area's record holds no box, so its space has
                    // no body, but it is named as a room's is.
                    "Area" => property_set = Some(area_property_set(&decoded)),
                    _ => {}
                }
            }
            // Host-wall binding is independent of which geometry
            // carrier produced the body. Before #222 this lived in the
            // `else` arm above, so a door recovered from a partition
            // element record — which always has a record bbox body —
            // never reached it and could not be voided into its wall.
            match decoded.class.as_str() {
                "Door" => {
                    let door = Door::from_decoded(&decoded);
                    if let Some(host) = recover_door_host(&door).ok() {
                        pending_host_id = host.host_element_id.as_u32();
                    }
                }
                "Window" => {
                    let window = Window::from_decoded(&decoded);
                    if let Some(host) = recover_window_host(&window).ok() {
                        pending_host_id = host.host_element_id.as_u32();
                    }
                }
                _ => {}
            }
        }

        let frame = location_feet.map(|location| PlanFrame {
            location: [location[0], location[1]],
            axis_turn: extrusion
                .as_ref()
                .map(|body| plan_axis_turn(rotation_radians, body)),
            held_axis_turn: held_body
                .as_ref()
                .map(|body| plan_axis_turn(rotation_radians, body)),
        });
        let level_slab = level_slab(
            &ifc_type,
            &decoded,
            extrusion.as_ref(),
            solid_shape.is_some(),
            property_set.as_ref(),
        );
        let opening_host = pending_host_id;
        if !policy.include_geometry {
            location_feet = None;
            rotation_radians = None;
            extrusion = None;
            property_set = property_set
                .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
                .map(|set| identity_property_set(set, &decoded));
            pending_host_id = None;
            piece_bodies.clear();
            void_bodies.clear();
            held_body = None;
            solid_shape = None;
        }

        let entity_index = entities.len();
        if let Some(id) = decoded.id {
            out.id_to_entity.insert(id, entity_index);
            if let Some(host_id) = opening_host {
                opening_hosts.push((entity_index, id, host_id));
            }
        }
        if let Some(frame) = frame {
            frames.insert(entity_index, frame);
        }
        if let Some(host_id) = pending_host_id {
            pending_hosts.push((entity_index, host_id));
            if decoded.fields.iter().any(|(name, value)| {
                name == crate::partition_schema_mvp::TYPE_WITHOUT_GEOMETRY_FIELD
                    && matches!(value, InstanceField::Bool(true))
            }) {
                without_geometry.insert(entity_index);
            }
        }
        if let Some(body) = held_body {
            held_bodies.insert(entity_index, body);
        }
        if let Some(whole) = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. }
                if name == crate::partition_schema_mvp::AGGREGATE_WHOLE_FIELD =>
            {
                Some(*id)
            }
            _ => None,
        }) {
            pending_parts.push((entity_index, whole));
        }
        if let Some(id) = decoded.id {
            let joins = crate::partition_schema_mvp::connector_joins_from_fields(&decoded.fields);
            if !joins.is_empty() {
                pending_joins.push((entity_index, id, joins));
            }
            // #828: a duct's ports are DUCT ports, a pipe's PIPE ports.
            match decoded.class.as_str() {
                "Duct" => {
                    curve_ends.push((entity_index, id, "DUCT"));
                    if let Some(ends) =
                        crate::partition_schema_mvp::duct_ends_from_fields(&decoded.fields)
                    {
                        pipe_ends.insert(
                            entity_index,
                            (
                                ends,
                                crate::partition_schema_mvp::duct_end_keys_from_fields(
                                    &decoded.fields,
                                ),
                            ),
                        );
                    }
                }
                "Pipe" => {
                    curve_ends.push((entity_index, id, "PIPE"));
                    if let Some(body) =
                        crate::partition_schema_mvp::pipe_body_from_fields(&decoded.fields)
                    {
                        pipe_ends.insert(
                            entity_index,
                            (
                                [body.start, body.end],
                                crate::partition_schema_mvp::pipe_end_keys_from_fields(
                                    &decoded.fields,
                                ),
                            ),
                        );
                    }
                }
                _ => {}
            }
        }
        // RE-162: the MEP systems the element is a member of.
        for (system, name) in crate::partition_schema_mvp::mep_systems_from_fields(&decoded.fields)
        {
            let entry = pending_systems
                .entry(system)
                .or_insert_with(|| (name, Vec::new()));
            entry.1.push(entity_index);
        }

        // Floor/Room → storey via Level ElementId only when both sides
        // carry ids that match. Partition MVP Levels are id-less today,
        // so this stays None (honest Unassigned) on available corpora.
        let storey_index = level_bind.storey_index_for(&decoded);
        if storey_index.is_some() {
            out.level_elementid_binds += 1;
        }

        let mut reference_sets = reference_type
            .map(|type_name| {
                let kinds = reference_kinds
                    .entry((ifc_type.clone(), predefined_type))
                    .or_insert_with(|| reference_set_kinds(&ifc_type, predefined_type));
                reference_property_sets(kinds, &type_name)
            })
            .unwrap_or_default();
        add_is_external(&mut reference_sets, &ifc_type, &decoded, opening_only);
        add_covering_finish(&mut reference_sets, &ifc_type, &decoded);
        add_mullion_span(&mut reference_sets, &ifc_type, &decoded);
        add_door_window_quantities(&mut reference_sets, &ifc_type, &decoded);
        add_wall_quantities(&mut reference_sets, &ifc_type, &decoded);
        add_wall_height(
            &mut reference_sets,
            &ifc_type,
            &decoded,
            extrusion.as_ref().filter(|_| solid_shape.is_none()),
        );
        add_wall_areas_and_volumes(
            &mut reference_sets,
            &ifc_type,
            &decoded,
            extrusion.as_ref().filter(|_| solid_shape.is_none()),
            property_set.as_ref(),
        );
        add_curtain_wall_length(&mut reference_sets, &ifc_type, &decoded);
        add_body_quantities(
            &mut reference_sets,
            &ifc_type,
            &decoded,
            extrusion.as_ref(),
            solid_shape.as_ref(),
        );
        add_beam_length(&mut reference_sets, &ifc_type, property_set.as_ref());
        add_beam_span(&mut reference_sets, &ifc_type);
        add_beam_slope(&mut reference_sets, &ifc_type, &decoded);
        add_beam_roll(&mut reference_sets, &ifc_type, &decoded);
        add_slab_gross_area(
            &mut reference_sets,
            &ifc_type,
            &decoded,
            extrusion.as_ref(),
            solid_shape.is_none() && piece_bodies.is_empty(),
        );
        add_ceiling_quantities(
            &mut reference_sets,
            &ifc_type,
            &decoded,
            extrusion.as_ref(),
            &piece_bodies,
            solid_shape.is_none(),
        );
        add_stair_flight_quantities(&mut reference_sets, &ifc_type, &decoded);
        add_railing_height(&mut reference_sets, &ifc_type, &decoded);
        add_railing_length(&mut reference_sets, &ifc_type, &decoded);
        add_space_height(&mut reference_sets, &ifc_type, &decoded);
        add_space_net_volume(
            &mut reference_sets,
            &ifc_type,
            &decoded,
            extrusion.as_ref(),
            solid_shape.is_none() && piece_bodies.is_empty(),
        );
        add_extend_to_structure(&mut reference_sets, &ifc_type, &decoded);
        let mut serial_sets = serial_number_property_sets(&decoded);
        serial_sets.extend(revit_parameter_property_sets(&decoded));
        let named = named_parameter_properties(&decoded);
        // RE-151: the name of the openings of a floor's tagged voids.
        // B55: the openings a wall's edited elevation profile cuts, in the
        // wall's plane relative to its placement.
        let mut profile_openings: Vec<ProfileOpening> = Vec::new();
        if let (Some(location), Some(body)) = (location_feet, extrusion.as_ref()) {
            let local_turn = if body.width_feet >= body.depth_feet {
                0.0
            } else {
                std::f64::consts::FRAC_PI_2
            };
            let turn = rotation_radians.unwrap_or(0.0) + local_turn;
            let (axis, left) = ([turn.cos(), turn.sin()], [-turn.sin(), turn.cos()]);
            let offset = |p: &[f64; 3], direction: [f64; 2]| {
                (p[0] - location[0]) * direction[0] + (p[1] - location[1]) * direction[1]
            };
            for (tag, outline) in
                crate::partition_schema_mvp::profile_openings_from_fields(&decoded.fields)
            {
                let Some(plane) = outline.first().map(|p| offset(p, left)) else {
                    continue;
                };
                let mut uv: Vec<(f64, f64)> = outline
                    .iter()
                    .map(|p| (offset(p, axis), p[2] - location[2]))
                    .collect();
                let area: f64 = uv
                    .iter()
                    .zip(uv.iter().cycle().skip(1))
                    .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
                    .sum();
                if area < 0.0 {
                    uv.reverse();
                }
                profile_openings.push((
                    tag,
                    uv,
                    local_turn,
                    plane,
                    body.width_feet.min(body.depth_feet),
                ));
            }
        }
        let opening_name = (!void_bodies.is_empty() || !profile_openings.is_empty()).then(|| {
            let type_id = decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == crate::partition_schema_mvp::TYPE_ID_FIELD =>
                {
                    Some(*id)
                }
                _ => None,
            });
            match (family_and_type(&decoded), type_id) {
                (Some((family, type_name)), Some(type_id)) => {
                    format!("{family}:{type_name}:{type_id}")
                }
                _ => format!("Opening for {name}"),
            }
        });
        let level_set = level_parameter_property_set(&decoded, &ifc_type);
        record_type_common(&mut type_commons, &ifc_type, &decoded);
        // #823: Revit's export writes a ceiling of separate loops as one
        // covering whose body holds every piece, where it splits a floor.
        let pieces_in_body =
            PIECES_IN_BODY_CLASSES.contains(&decoded.class.as_str()) && solid_shape.is_none();
        entities.push(entities::IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            predefined_type: predefined_type.map(str::to_string),
            storey_index,
            material_index: None,
            property_set,
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape,
            representation_map_index: None,
        });
        if policy.include_geometry && decoded.class == "Wall" {
            let named = crate::partition_schema_mvp::wall_top_cutters_from_fields(&decoded.fields);
            if !named.is_empty() {
                pending_tops.push((entity_index, named));
            }
        }
        // RE-270: a wall joined to a roof or floor, whose length is confirmed.
        if policy.include_geometry && decoded.class == "Wall" {
            if let Some(named) =
                crate::partition_schema_mvp::wall_bounded_from_fields(&decoded.fields)
            {
                let confirmed = decoded.fields.iter().any(|(name, value)| {
                    name == crate::partition_schema_mvp::WALL_LENGTH_FIELD
                        && matches!(value, InstanceField::Float { .. })
                });
                if confirmed {
                    pending_bounded.push((entity_index, named));
                }
            }
        }
        // #331: each further piece of a sketch of separate loops is an
        // element of its own with the element's `Tag`, named `…:2`, `…:3`,
        // as in Revit's export.
        if !piece_bodies.is_empty() {
            if let Some(entities::IfcEntity::BuildingElement { .. }) = entities.last() {
                let first = entities.last().cloned().expect("just pushed");
                let start = entities.len();
                for (index, body) in piece_bodies.into_iter().enumerate() {
                    let mut piece = first.clone();
                    if let entities::IfcEntity::BuildingElement {
                        name, extrusion, ..
                    } = &mut piece
                    {
                        *name = format!("{name}:{}", index + 2);
                        *extrusion = Some(body);
                    }
                    entities.push(piece);
                }
                if pieces_in_body {
                    out.body_pieces
                        .insert(entity_index, (start..entities.len()).collect());
                }
                pieces_of.push((entity_index, start..entities.len()));
            }
        }
        // RE-151: each tagged void of a floor's sketch is an opening voiding
        // the floor, or the piece it is in, named `Family:Type:<type id>` as
        // in Revit's export.
        if let Some(opening_name) = opening_name {
            for (piece, tag, outline_feet, depth_feet) in void_bodies {
                entities.push(entities::IfcEntity::VoidOpening {
                    host: entity_index + piece,
                    tag: tag.to_string(),
                    name: opening_name.clone(),
                    outline_feet,
                    depth_feet,
                });
            }
            for (tag, outline_feet, axis_turn_radians, plane_offset_feet, thickness_feet) in
                profile_openings
            {
                entities.push(entities::IfcEntity::WallProfileOpening {
                    host: entity_index,
                    tag: tag.to_string(),
                    name: opening_name.clone(),
                    outline_feet,
                    axis_turn_radians,
                    plane_offset_feet,
                    thickness_feet,
                });
            }
        }
        // RE-172: the Level parameters, in the `Constraints` set Revit's
        // export writes them in.
        if let Some(set) = level_set {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set,
            });
        }
        // RE-47: a stair's or flight's riser and tread dimensions, in the
        // standard set Revit's export writes them in.
        if let Some(set) = stair_property_set(&decoded) {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set,
            });
        }
        // RE-153: a room's Pset_SpaceCommon and its Floor Finish.
        for set in space_common_property_set(&decoded)
            .into_iter()
            .chain(space_covering_property_set(&decoded))
        {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set,
            });
        }
        // A pipe's or duct's length along its axis: a pipe's between its
        // ends (RE-131), a duct's the dimension of its box its section leaves
        // (RE-134).
        let segment_length = match decoded.class.as_str() {
            "Pipe" => {
                crate::partition_schema_mvp::pipe_body_from_fields(&decoded.fields).map(|pipe| {
                    (0..3)
                        .map(|axis| (pipe.end[axis] - pipe.start[axis]).powi(2))
                        .sum::<f64>()
                        .sqrt()
                })
            }
            "Duct" => decoded.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. }
                    if name == crate::partition_schema_mvp::DUCT_LENGTH_FIELD =>
                {
                    Some(*value)
                }
                _ => None,
            }),
            _ => None,
        };
        let duct_shape = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(shape)
                if name == crate::partition_schema_mvp::DUCT_SHAPE_FIELD =>
            {
                Some(shape.clone())
            }
            _ => None,
        });
        // RE-157: a pipe's invert, in model height (RE-157 §6). A pipe whose
        // parameter named for the property gives it (RE-194) has that, as it
        // is.
        let explicit_invert = named.iter().any(|(set, property)| {
            set == "Pset_PipeSegmentOccurrence" && property.name == INVERT_ELEVATION_PROPERTY
        });
        let invert = match decoded.class.as_str() {
            "Pipe" if !explicit_invert => pipe_invert_height(&decoded.fields),
            _ => None,
        };
        // #822: the length in IFC4's base quantities of the segment, where
        // Revit's IFC4 export writes it (IFC2X3's flow-segment sets held it
        // as a property), and a pipe's invert in Pset_PipeSegmentOccurrence,
        // on an element written as a pipe or duct segment only, not one its
        // Export to IFC As writes as another entity.
        let segment_sets = match effective.ifc_type {
            "IFCPIPESEGMENT" => Some((
                "Qto_PipeSegmentBaseQuantities",
                invert.map(|height| PropertySet {
                    name: "Pset_PipeSegmentOccurrence".into(),
                    properties: vec![Property {
                        name: INVERT_ELEVATION_PROPERTY.into(),
                        value: PropertyValue::LengthFeet(height),
                    }],
                }),
            )),
            "IFCDUCTSEGMENT" => Some(("Qto_DuctSegmentBaseQuantities", None)),
            _ => None,
        };
        // RE-247: a pipe's cross-section and outer surface are those of the
        // cylinder it is drawn as (RE-131), as Revit's export writes them.
        let pipe_radius = (decoded.class == "Pipe")
            .then(|| crate::partition_schema_mvp::pipe_body_from_fields(&decoded.fields))
            .flatten()
            .map(|pipe| pipe.radius_feet)
            .filter(|radius| radius.is_finite() && *radius > 0.0);
        if let Some((quantities, occurrence)) = segment_sets {
            let quantities = segment_length.map(|length| {
                let mut properties = vec![Property {
                    name: "Length".into(),
                    value: PropertyValue::PositiveLengthFeet(length),
                }];
                if let Some(radius) = pipe_radius {
                    properties.push(Property {
                        name: "GrossCrossSectionArea".into(),
                        value: PropertyValue::AreaSquareFeet(
                            std::f64::consts::PI * radius * radius,
                        ),
                    });
                    properties.push(Property {
                        name: "OuterSurfaceArea".into(),
                        value: PropertyValue::AreaSquareFeet(
                            2.0 * std::f64::consts::PI * radius * length,
                        ),
                    });
                }
                PropertySet {
                    name: quantities.into(),
                    properties,
                }
            });
            if let Some(set) = quantities {
                entities.push(entities::IfcEntity::ElementPropertySet {
                    element: entity_index,
                    set,
                });
            }
            if let Some(set) = occurrence {
                out.computed_inverts.push(entities.len());
                entities.push(entities::IfcEntity::ElementPropertySet {
                    element: entity_index,
                    set,
                });
            }
        }
        // A duct's Pset_DuctSegmentTypeCommon: its type's shape (B43), where
        // it is known and the duct is written as a duct segment.
        if let Some(shape) = duct_shape.filter(|_| effective.ifc_type == "IFCDUCTSEGMENT") {
            entities.push(entities::IfcEntity::ElementPropertySet {
                element: entity_index,
                set: PropertySet {
                    name: "Pset_DuctSegmentTypeCommon".into(),
                    properties: vec![Property {
                        name: "Shape".into(),
                        value: PropertyValue::Enumerated(vec![shape]),
                    }],
                },
            });
        }
        if level_slab {
            add_level_slab_pitch(&mut reference_sets);
        }
        // RE-154: the common property sets' Reference, the type's name;
        // RE-156: the shared parameter Serial Number.
        for set in reference_sets.into_iter().chain(serial_sets) {
            add_element_set(entities, entity_index, set);
        }
        if !named.is_empty() {
            pending_named.push((entity_index, named));
        }
    }

    // RE-267: a wall's top is cut at the underside of the floor that covers it.
    wall_tops::cut_wall_tops(entities, &pending_tops);

    // RE-270: the side areas and volumes of a wall joined to floors that
    // leave it as its record's box. It runs after the pass above, on purpose:
    // it reads each extrusion at its final height, and a wall that pass cut
    // has the underside of the floor that cut it for its top, which that
    // floor then clears.
    wall_areas::add_bounded_wall_quantities(entities, &pending_bounded);

    // #323: group each whole's parts into one aggregate.
    let mut parts_by_whole: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (part_index, whole_id) in pending_parts {
        if let Some(&whole_index) = out.id_to_entity.get(&whole_id) {
            parts_by_whole
                .entry(whole_index)
                .or_default()
                .push(part_index);
        }
    }
    // A whole no part names is not carried by parts: it keeps its body, as
    // Revit's export does for a stair family placed on its own (Snowdon
    // 1603717).
    for (index, body) in held_bodies {
        if parts_by_whole.contains_key(&index) {
            continue;
        }
        if let Some(entities::IfcEntity::BuildingElement { extrusion, .. }) =
            entities.get_mut(index)
        {
            *extrusion = Some(body);
        }
    }
    // B72: each door's and window's flip against its host wall's axis; a
    // host carried by its parts has no body of its own to give one.
    for (index, id, host_id) in opening_hosts {
        let Some(&host_index) = out.id_to_entity.get(&host_id) else {
            continue;
        };
        let (Some(opening), Some(host), Some(facing)) = (
            frames.get(&index),
            frames.get(&host_index),
            facings.get(&id),
        ) else {
            continue;
        };
        let host_turn = if parts_by_whole.contains_key(&host_index) {
            host.axis_turn
        } else {
            host.axis_turn.or(host.held_axis_turn)
        };
        if let Some(turn) = host_turn {
            out.element_flips.insert(
                id,
                door_symbol_flipped(opening.location, host.location, turn, *facing),
            );
        }
    }
    for (whole, parts) in parts_by_whole {
        entities.push(entities::IfcEntity::Aggregate { whole, parts });
    }

    // RE-138 (#528): a join is written when both elements are exported as
    // elements that take ports, distribution elements and proxies (whose
    // ports are nested in them, B83, B86), and once however many of the two
    // it was read from.
    let is_distribution = |index: usize| {
        matches!(
            entities.get(index),
            Some(entities::IfcEntity::BuildingElement { ifc_type, .. })
                if is_distribution_element(ifc_type)
        )
    };
    let takes_ports = |index: usize| {
        is_distribution(index)
            || matches!(
                entities.get(index),
                Some(entities::IfcEntity::BuildingElement { ifc_type, .. })
                    if ifc_type == "IFCBUILDINGELEMENTPROXY"
            )
    };
    let mut connections: std::collections::BTreeSet<[(usize, u32, u32); 2]> = Default::default();
    // #847: each connector's joins, read from either side's list.
    let mut joined: HashMap<(u32, u32), Vec<(u32, u32)>> = HashMap::new();
    for (_, id, joins) in &pending_joins {
        for &(index, other_id, other_index) in joins {
            joined
                .entry((*id, index))
                .or_default()
                .push((other_id, other_index));
            joined
                .entry((other_id, other_index))
                .or_default()
                .push((*id, index));
        }
    }
    for (entity, id, joins) in pending_joins {
        for (index, other_id, other_index) in joins {
            let Some(&other) = out.id_to_entity.get(&other_id) else {
                continue;
            };
            if !takes_ports(entity) || !takes_ports(other) {
                continue;
            }
            let mut ends = [(entity, id, index), (other, other_id, other_index)];
            ends.sort();
            connections.insert(ends);
        }
    }
    let curve_ends: Vec<(usize, u32, &'static str)> = curve_ends
        .into_iter()
        .filter(|(element, _, _)| is_distribution(*element))
        .collect();
    for [(a, a_id, a_index), (b, b_id, b_index)] in connections {
        entities.push(entities::IfcEntity::PortConnection {
            a,
            a_id,
            a_index,
            b,
            b_id,
            b_index,
        });
    }
    // B54: both ends of every duct and pipe, joined or not.
    for (element, id, kind) in curve_ends {
        for index in 0..2 {
            let place = pipe_ends
                .get(&element)
                .and_then(|ends| pipe_port_point(id, index, ends, &joined));
            entities.push(entities::IfcEntity::Port {
                element,
                id,
                index,
                predefined_type: Some(kind.to_string()),
                flow_direction: None,
                location_feet: place.map(|(point, _)| point),
                direction: place.map(|(_, outward)| outward),
            });
        }
    }
    for (id, (name, members)) in pending_systems {
        entities.push(entities::IfcEntity::System { id, name, members });
    }

    if policy.include_geometry {
        for (entity_index, host_id) in pending_hosts {
            if let Some(&host_index) = out.id_to_entity.get(&host_id) {
                if let Some(entities::IfcEntity::BuildingElement {
                    ifc_type,
                    predefined_type,
                    host_element_index,
                    ..
                }) = entities.get_mut(entity_index)
                {
                    *host_element_index = Some(host_index);
                    // RE-84: a door or window whose type draws no geometry
                    // is only the hole it cuts, and Revit's export writes
                    // that opening alone, with no door or window.
                    if without_geometry.contains(&entity_index) {
                        *ifc_type = "IFCOPENINGELEMENT".into();
                        *predefined_type = Some("OPENING".into());
                    }
                }
            }
        }
    }
    add_named_parameter_sets(entities, pending_named);
    add_piece_sets(entities, pieces_of);
    for ((type_id, entity), common) in type_commons {
        let sets = common.map_or_else(Vec::new, |common| common.into_sets(&entity));
        if sets.is_empty() {
            continue;
        }
        out.type_entity_sets
            .entry(type_id)
            .or_default()
            .insert(entity, sets);
    }

    out
}

fn storey_from_level(decoded: &DecodedElement) -> Option<Storey> {
    let level = Level::from_decoded(decoded);
    // Drafting / non-building levels stay out of the spatial tree.
    if level.is_building_story == Some(false) {
        return None;
    }
    let name = level
        .name
        .clone()
        .or_else(|| decoded.id.map(|id| format!("Level-{id}")))?;
    // Prefer recovered elevation; name-only partition Levels (2024
    // without ArcWall trailers) still form storeys at 0.0 — callers
    // must treat that as elevation-unresolved, not surveyed height.
    let elevation_feet = recover_level_elevation(&level)
        .ok()
        .map(|e| e.elevation_feet)
        .or(level.elevation_feet)
        .unwrap_or(0.0);
    Some(Storey {
        name,
        elevation_feet,
    })
}

fn material_info_from_decoded(decoded: &DecodedElement) -> Option<MaterialInfo> {
    let material = Material::from_decoded(decoded);
    let name = material.name?;
    if name.trim().is_empty() {
        return None;
    }
    Some(MaterialInfo {
        name,
        color_packed: material.color,
        transparency: material.transparency,
    })
}

#[allow(dead_code)] // retained for when opening→host IFC emission lands
fn opening_index_property_set(decoded: &DecodedElement) -> PropertySet {
    let mut properties = vec![
        Property {
            name: "OpeningKindResolved".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "DoorWindowDiscriminated".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "Source".into(),
            value: PropertyValue::Text("partition_rect_opening_index".into()),
        },
    ];
    for (name, value) in &decoded.fields {
        match (name.as_str(), value) {
            ("m_related_id_a", InstanceField::ElementId { id, .. }) => {
                properties.push(Property {
                    name: "RelatedIdA".into(),
                    value: PropertyValue::Integer(i64::from(*id)),
                });
            }
            ("m_related_id_b", InstanceField::ElementId { id, .. }) => {
                properties.push(Property {
                    name: "RelatedIdB".into(),
                    value: PropertyValue::Integer(i64::from(*id)),
                });
            }
            ("m_host_id", InstanceField::ElementId { id, .. }) => {
                properties.push(Property {
                    name: "HostIdCandidate".into(),
                    value: PropertyValue::Integer(i64::from(*id)),
                });
            }
            ("m_related_id_a_in_elem_table", InstanceField::Bool(b)) => {
                properties.push(Property {
                    name: "RelatedIdAInElemTable".into(),
                    value: PropertyValue::Boolean(*b),
                });
            }
            ("m_related_id_b_in_elem_table", InstanceField::Bool(b)) => {
                properties.push(Property {
                    name: "RelatedIdBInElemTable".into(),
                    value: PropertyValue::Boolean(*b),
                });
            }
            ("m_host_elem_table_confirmed", InstanceField::Bool(b)) => {
                properties.push(Property {
                    name: "HostElemTableConfirmed".into(),
                    value: PropertyValue::Boolean(*b),
                });
            }
            ("m_index", InstanceField::Integer { value, .. }) => {
                properties.push(Property {
                    name: "Index".into(),
                    value: PropertyValue::Integer(*value),
                });
            }
            _ => {}
        }
    }
    PropertySet {
        name: "RvtArcWallRectOpening".into(),
        properties,
    }
}

struct RecoveredWallGeom {
    location_feet: [f64; 3],
    rotation_radians: f64,
    extrusion: Extrusion,
    property_set: Option<PropertySet>,
}

fn wall_geometry_from_decoded(decoded: &DecodedElement) -> Option<RecoveredWallGeom> {
    let wall = Wall::from_decoded(decoded);
    let curve = recover_wall_location_curve_from_wall(&wall, decoded).ok()?;
    let (start, end) = curve.line_endpoints_xy()?;
    let length_feet = wall_segment_length_feet(start, end);
    if !length_feet.is_finite() || length_feet < 0.01 {
        return None;
    }
    // Fail closed: do not invent a 10 ft height.
    let height_feet = wall.unconnected_height_feet?;
    if !height_feet.is_finite() || height_feet <= 0.0 {
        return None;
    }
    let z = wall
        .location_start
        .map(|p| p.z)
        .or(wall.base_offset_feet)
        .unwrap_or(0.0);
    let location_feet = [(start[0] + end[0]) / 2.0, (start[1] + end[1]) / 2.0, z];
    let rotation_radians = wall_segment_angle_radians(start, end);
    let mut properties = vec![
        Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(false),
        },
        Property {
            name: "LocationCurveSource".into(),
            value: PropertyValue::Text(format!("{:?}", curve.source)),
        },
    ];
    if let Some(id) = curve.location_curve_id {
        properties.push(Property {
            name: "LocationCurveId".into(),
            value: PropertyValue::Integer(id.get()),
        });
    }
    Some(RecoveredWallGeom {
        location_feet,
        rotation_radians,
        extrusion: Extrusion {
            width_feet: length_feet,
            depth_feet: UNRESOLVED_ARCWALL_THICKNESS_FEET,
            height_feet,
            profile_override: None,
        },
        property_set: Some(PropertySet {
            name: "RvtWallGeometry".into(),
            properties,
        }),
    })
}

/// Revit's `Constraints` properties for the Level an element's association
/// record names (RE-172), under the names Revit's export uses for its class.
fn level_parameter_property_set(decoded: &DecodedElement, ifc_type: &str) -> Option<PropertySet> {
    use crate::partition_level_parameters as lp;
    let text = |wanted: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(v) if name == wanted => Some(v.clone()),
            _ => None,
        })
    };
    let (level, from_list) = match text(lp::LEVEL_PARAMETER_FIELD) {
        Some(level) => (
            level,
            decoded.fields.iter().any(|(name, value)| {
                name == crate::partition_schema_mvp::LEVEL_FROM_LIST_FIELD
                    && matches!(value, InstanceField::Bool(true))
            }),
        ),
        // The highest of several listed Levels is the Reference Level of a
        // beam or brace only (RE-175).
        None if matches!(ifc_type, "IFCBEAM" | "IFCMEMBER") => (
            text(crate::partition_schema_mvp::REFERENCE_LEVEL_FROM_LIST_FIELD)?,
            true,
        ),
        None => return None,
    };
    let names: &[&str] = match ifc_type {
        "IFCBEAM" => &["Reference Level"],
        "IFCMEMBER" if from_list => &["Reference Level"],
        "IFCWALL" | "IFCCURTAINWALL" => &["Base Constraint"],
        "IFCCOLUMN" => &["Base Level", "Level"],
        "IFCRAILING" | "IFCROOF" | "IFCSTAIR" => &["Base Level"],
        "IFCDUCTSEGMENT" | "IFCPIPESEGMENT" => &["Reference Level"],
        _ => &["Level"],
    };
    let mut properties: Vec<Property> = names
        .iter()
        .map(|name| Property {
            name: (*name).into(),
            value: PropertyValue::Label(level.clone()),
        })
        .collect();
    if let Some(top) = text(lp::TOP_LEVEL_PARAMETER_FIELD) {
        match ifc_type {
            "IFCWALL" | "IFCCURTAINWALL" => properties.push(Property {
                name: "Top Constraint".into(),
                value: PropertyValue::Label(format!("Up to level: {top}")),
            }),
            "IFCSTAIR" => properties.push(Property {
                name: "Top Level".into(),
                value: PropertyValue::Label(top),
            }),
            _ => {}
        }
    }
    Some(PropertySet {
        name: "Constraints".into(),
        properties,
    })
}

/// `Pset_StairCommon` for a stair and `Pset_StairFlightCommon` for a
/// flight, holding the number of risers, riser height and tread length
/// their data records (RE-47), with the measure types those standard sets
/// declare. `None` when nothing was read.
///
/// As revit-ifc's calculators give them (CLEANROOM.md source 5): a flight's
/// number of treads and nosing length are its own (RE-92, RE-52), and a
/// stair's nosing length the one its runs share; `TreadLengthAtOffset` is
/// the stair's tread depth, which a flight carries, and so is
/// `TreadLengthAtInnerSide` of a flight and of its stair, which revit-ifc
/// takes from the stair type's inside-boundary width where that is set and
/// else from the tread depth (RE-287); the flights' and stairs' of the BIM4LCA
/// office, as built and at permit, and of the R2025 models equal the depth, as
/// no type there sets one. `WalkingLineOffset` is not written: revit-ifc gives
/// it as half the run's width in feet and the export scales it as millimetres.
fn stair_property_set(decoded: &DecodedElement) -> Option<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    let (name, flight) = match decoded.class.as_str() {
        "Stair" => ("Pset_StairCommon", false),
        "StairsRun" => ("Pset_StairFlightCommon", true),
        _ => return None,
    };
    let mut properties = Vec::new();
    for (field, value) in &decoded.fields {
        if let (mvp::STAIR_TREAD_DEPTH_FIELD, InstanceField::Float { value, .. }) =
            (field.as_str(), value)
        {
            properties.push(Property {
                name: "TreadLengthAtOffset".into(),
                value: PropertyValue::PositiveLengthFeet(*value),
            });
            properties.push(Property {
                name: "TreadLengthAtInnerSide".into(),
                value: PropertyValue::PositiveLengthFeet(*value),
            });
        }
        let property = match (field.as_str(), value) {
            (mvp::STAIR_TREAD_COUNT_FIELD, InstanceField::Integer { value, .. }) if flight => {
                Property {
                    name: "NumberOfTreads".into(),
                    value: PropertyValue::CountValue(*value),
                }
            }
            (mvp::STAIR_NOSING_LENGTH_FIELD, InstanceField::Float { value, .. }) => Property {
                name: "NosingLength".into(),
                value: PropertyValue::LengthFeet(*value),
            },
            (mvp::STAIR_RISER_COUNT_FIELD, InstanceField::Integer { value, .. }) => Property {
                name: "NumberOfRiser".into(),
                value: PropertyValue::CountValue(*value),
            },
            (mvp::STAIR_RISER_HEIGHT_FIELD, InstanceField::Float { value, .. }) => Property {
                name: "RiserHeight".into(),
                value: PropertyValue::PositiveLengthFeet(*value),
            },
            (mvp::STAIR_TREAD_DEPTH_FIELD, InstanceField::Float { value, .. }) => Property {
                name: "TreadLength".into(),
                value: PropertyValue::PositiveLengthFeet(*value),
            },
            _ => continue,
        };
        properties.push(property);
    }
    (!properties.is_empty()).then(|| PropertySet {
        name: name.into(),
        properties,
    })
}

/// The sets holding `Reference` of each entity and predefined type written
/// ([`reference_set_kinds`]).
type ReferenceKinds = BTreeMap<
    (String, Option<&'static str>),
    Vec<(
        &'static str,
        super::standard_property_sets::StandardPropertyKind,
    )>,
>;

/// The property sets whose `Reference` an element written as `ifc_type`, with
/// the predefined type `predefined_type`, gets (RE-154, #822), each with the
/// type it declares `Reference` as: every IFC4 standard set that applies to
/// the entity and declares `Reference` (`IfcLabel` in the reinforcement
/// sets). Revit's IFC4 export gives the BIM4LCA office's elements exactly
/// these sets, but for those revit-ifc matches by a prefix of the entity's
/// name, such as `Pset_StairFlightCommon` on a stair. None for a space,
/// whose `Reference` is [`space_common_property_set`]'s, and only the sets
/// in [`CONFIRMED_REFERENCE_SETS`].
fn reference_set_kinds(
    ifc_type: &str,
    predefined_type: Option<&str>,
) -> Vec<(
    &'static str,
    super::standard_property_sets::StandardPropertyKind,
)> {
    use super::standard_property_sets::STANDARD_PROPERTY_SETS;
    if ifc_type == "IFCSPACE" {
        return Vec::new();
    }
    STANDARD_PROPERTY_SETS
        .iter()
        .filter(|set| set.applies_to(ifc_type, predefined_type))
        .filter(|set| CONFIRMED_REFERENCE_SETS.contains(&set.name))
        .filter_map(|set| Some((set.name, set.property_kind("Reference")?)))
        .collect()
}

/// The sets whose `Reference` a Revit export measured writes, equal to
/// rvt-rs's and never where rvt-rs leaves it out (RE-239 §7): the office's
/// and the R2025 models' IFC4 exports, and RE1's IFC2X3 exports for the duct
/// and pipe fitting and duct segment types. Reference goes in no other set
/// until a Revit export confirms it (the maintainer, 2026-10-29).
const CONFIRMED_REFERENCE_SETS: &[&str] = &[
    "Pset_AirTerminalTypeCommon",
    "Pset_BeamCommon",
    "Pset_BuildingElementProxyCommon",
    "Pset_ColumnCommon",
    "Pset_CoveringCommon",
    "Pset_CurtainWallCommon",
    "Pset_DoorCommon",
    "Pset_DuctFittingTypeCommon",
    "Pset_DuctSegmentTypeCommon",
    "Pset_ElectricApplianceTypeCommon",
    "Pset_EnvironmentalImpactIndicators",
    "Pset_FireSuppressionTerminalTypeCommon",
    "Pset_FurnitureTypeCommon",
    "Pset_LightFixtureTypeCommon",
    "Pset_MemberCommon",
    "Pset_OpeningElementCommon",
    "Pset_PipeFittingTypeCommon",
    "Pset_PipeSegmentTypeCommon",
    "Pset_PlateCommon",
    "Pset_RailingCommon",
    "Pset_RampCommon",
    "Pset_ReinforcementBarPitchOfBeam",
    "Pset_ReinforcementBarPitchOfColumn",
    "Pset_ReinforcementBarPitchOfSlab",
    "Pset_ReinforcementBarPitchOfWall",
    "Pset_RoofCommon",
    "Pset_SanitaryTerminalTypeCommon",
    "Pset_SlabCommon",
    "Pset_SpaceCommon",
    "Pset_StairCommon",
    "Pset_StairFlightCommon",
    "Pset_WallCommon",
    "Pset_WasteTerminalTypeCommon",
    "Pset_WindowCommon",
];

/// The sets `kinds` ([`reference_set_kinds`]), each holding `type_name` as
/// its `Reference`.
fn reference_property_sets(
    kinds: &[(
        &'static str,
        super::standard_property_sets::StandardPropertyKind,
    )],
    type_name: &str,
) -> Vec<PropertySet> {
    use super::standard_property_sets::StandardPropertyKind as Kind;
    kinds
        .iter()
        .filter_map(|(name, kind)| {
            let value = match kind {
                Kind::Identifier => PropertyValue::Identifier(type_name.to_string()),
                Kind::Label => PropertyValue::Label(type_name.to_string()),
                Kind::Text => PropertyValue::Text(type_name.to_string()),
                _ => return None,
            };
            Some(PropertySet {
                name: (*name).into(),
                properties: vec![Property {
                    name: "Reference".into(),
                    value,
                }],
            })
        })
        .collect()
}

/// The set Revit's own IFC export writes an element's Category, family and
/// type in, and its type object's Category and Family Name (#755).
pub const REVIT_OTHER_SET: &str = "Other";

/// The parameters Revit's own IFC export writes on an element for its
/// category, family, type and phase, under its set and property names
/// (#755): `Other` (Category, Family, Family and Type, Type, Type Id)
/// and `Phasing` (Phase Created). Each is written only from a value the
/// element carries. The type's own parameters (Family Name, Type Name) go on
/// its type object, as Revit writes them.
fn revit_parameter_property_sets(decoded: &DecodedElement) -> Vec<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    let text = |key: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::String(v) if name == key => Some(v.clone()),
            _ => None,
        })
    };
    let label = |name: &str, value: String| Property {
        name: name.into(),
        value: PropertyValue::Label(value),
    };
    let mut other = Vec::new();
    if let Some(category) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == "m_builtinCategory" => {
            crate::partition_element_records::builtin_category_name(*value)
        }
        _ => None,
    }) {
        other.push(label("Category", category.into()));
    }
    let type_name = text(mvp::TYPE_NAME_FIELD);
    if let (Some(family), Some(type_name)) = (text(mvp::FAMILY_NAME_FIELD), &type_name) {
        other.push(label("Family", family.clone()));
        other.push(label("Family and Type", format!("{family}: {type_name}")));
    }
    if let Some(type_name) = &type_name {
        other.push(label("Type", type_name.clone()));
    }
    if let Some(type_id) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. } if name == mvp::TYPE_ID_FIELD => Some(*id),
        _ => None,
    }) {
        other.push(label("Type Id", type_id.to_string()));
    }
    let mut sets = Vec::new();
    if !other.is_empty() {
        sets.push(PropertySet {
            name: REVIT_OTHER_SET.into(),
            properties: other,
        });
    }
    if let Some(phase) = text(mvp::PHASE_CREATED_FIELD) {
        sets.push(PropertySet {
            name: "Phasing".into(),
            properties: vec![label("Phase Created", phase)],
        });
    }
    sets
}

/// The set Revit's export gives an element's shared parameter `Serial
/// Number` in (RE-156): `Pset_ManufacturerOccurrence.SerialNumber`
/// (`IfcIdentifier`), where IFC4 declares it. Revit's IFC2X3 export of the
/// RE1 models also gives a building-element proxy
/// `Pset_PrecastConcreteElementGeneral.SerialNumber`, which IFC4's set does
/// not have (#822). Empty when the element has none.
fn serial_number_property_sets(decoded: &DecodedElement) -> Vec<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    let serial = decoded
        .fields
        .iter()
        .find_map(|(field, value)| match value {
            InstanceField::String(text) if field == mvp::SERIAL_NUMBER_FIELD => Some(text),
            _ => None,
        });
    let Some(serial) = serial.filter(|text| !text.is_empty()) else {
        return Vec::new();
    };
    vec![PropertySet {
        name: "Pset_ManufacturerOccurrence".into(),
        properties: vec![Property {
            name: "SerialNumber".into(),
            value: PropertyValue::Identifier(serial.clone()),
        }],
    }]
}

/// The common property set Revit's export writes `IsExternal` in on an
/// element exported as `ifc_type` (RE-193).
fn is_external_set(ifc_type: &str) -> Option<&'static str> {
    Some(match ifc_type {
        "IFCWALL" | "IFCWALLSTANDARDCASE" => "Pset_WallCommon",
        "IFCCURTAINWALL" => "Pset_CurtainWallCommon",
        "IFCDOOR" => "Pset_DoorCommon",
        "IFCWINDOW" => "Pset_WindowCommon",
        "IFCSLAB" => "Pset_SlabCommon",
        "IFCROOF" => "Pset_RoofCommon",
        "IFCCOLUMN" => "Pset_ColumnCommon",
        "IFCBEAM" => "Pset_BeamCommon",
        "IFCMEMBER" => "Pset_MemberCommon",
        "IFCPLATE" => "Pset_PlateCommon",
        "IFCCOVERING" => "Pset_CoveringCommon",
        "IFCRAILING" => "Pset_RailingCommon",
        "IFCRAMP" => "Pset_RampCommon",
        "IFCSTAIR" => "Pset_StairCommon",
        "IFCBUILDINGELEMENTPROXY" => "Pset_BuildingElementProxyCommon",
        "IFCSPACE" => "Pset_SpaceCommon",
        _ => return None,
    })
}

/// An element's `IsExternal` in its common property set `set` (RE-193): the
/// Yes/No parameter named `<set>.IsExternal` on the element, which revit-ifc's
/// property set entry reads before anything else (`PropertySetEntry
/// ("Pset_WallCommon.IsExternal", "IsExternal")` in its pset definitions,
/// CLEANROOM.md source 5), else the value Revit's exporter calculates
/// (`crate::partition_schema_mvp::IS_EXTERNAL_FIELD`). `None` where neither
/// is read.
fn is_external_value(decoded: &DecodedElement, set: &str) -> Option<bool> {
    use crate::partition_schema_mvp as mvp;
    let flag = |wanted: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Bool(flag) if name == wanted => Some(*flag),
            _ => None,
        })
    };
    flag(&format!(
        "{}{set}.{}",
        mvp::NAMED_PARAMETER_FIELD_PREFIX,
        mvp::IS_EXTERNAL_PARAMETER_NAME
    ))
    .or_else(|| flag(mvp::IS_EXTERNAL_FIELD))
}

/// Add `IsExternal` (`IfcBoolean`) to the common property set of the entity
/// an element is exported as, among `sets` ([`is_external_value`], RE-193),
/// starting the set when the element has none (no type name for its
/// `Reference`). Nothing where no value is read, for a door written as its
/// opening alone (RE-84), or for a space, whose set
/// [`space_common_property_set`] writes.
fn add_is_external(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    opening_only: bool,
) {
    if opening_only {
        return;
    }
    let Some(common) = is_external_set(ifc_type).filter(|set| *set != "Pset_SpaceCommon") else {
        return;
    };
    let Some(external) = is_external_value(decoded, common) else {
        return;
    };
    let property = Property {
        name: "IsExternal".into(),
        value: PropertyValue::Boolean(external),
    };
    match sets.iter_mut().find(|set| set.name == common) {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: common.into(),
            properties: vec![property],
        }),
    }
}

/// What the common property set of a type object holds (RE-286).
#[derive(Clone, PartialEq)]
struct TypeCommon {
    fire_rating: Option<String>,
    load_bearing: Option<bool>,
    /// A stair type of a stair whose data is read: revit-ifc's stair
    /// calculators, run on the type itself, give 0 (RE-286).
    stair_type: bool,
    /// A flight type's nosing length, feet: its run type's (RE-286).
    nosing_feet: Option<f64>,
}

/// Classes of the categories revit-ifc's `LoadBearingCalculator` (CLEANROOM.md
/// source 5) calls load bearing before it reads any parameter, which a type
/// object's common set holds as it does the element's (RE-286): the BIM4LCA
/// office's 10 structural column types and its structural framing type.
const LOAD_BEARING_TYPE_CLASSES: &[&str] = &["StructuralColumn", "StructuralFraming"];

/// The entities whose type objects Revit's IFC4 exports of the BIM4LCA office
/// (exporter 24.0.5) and of the R2025 models (25.0.2) give a common set, among
/// those rvt-rs writes a type object for (RE-286). Not a
/// ramp's, which holds no set in the office's export; a railing's, which that
/// export writes no type object for at all, though the R2025 models' write one;
/// nor a curtain member's, which is the exporter's cache (RE-269).
const TYPE_COMMON_ENTITIES: &[&str] = &[
    "IFCWALL",
    "IFCCURTAINWALL",
    "IFCDOOR",
    "IFCWINDOW",
    "IFCSLAB",
    "IFCROOF",
    "IFCCOLUMN",
    "IFCBEAM",
    "IFCCOVERING",
    "IFCSTAIR",
    "IFCSTAIRFLIGHT",
    "IFCBUILDINGELEMENTPROXY",
    "IFCSPACE",
];

impl TypeCommon {
    /// The sets of the type entity `type_entity` (`IFCWALLTYPE`): its common
    /// set, and for a stair or flight type the stair dimension sets.
    fn into_sets(self, type_entity: &str) -> Vec<PropertySet> {
        let Some(occurrence) = type_entity.strip_suffix("TYPE") else {
            return Vec::new();
        };
        let mut sets = Vec::new();
        if let Some(name) = is_external_set(occurrence) {
            let mut properties = Vec::new();
            let defines = |property: &str| {
                super::standard_property_sets::property_kind(name, property).is_some()
            };
            if let Some(rating) = self.fire_rating.filter(|_| defines("FireRating")) {
                properties.push(Property {
                    name: "FireRating".into(),
                    value: PropertyValue::Label(rating),
                });
            }
            if let Some(bearing) = self.load_bearing.filter(|_| {
                super::standard_property_sets::property_kind(name, "LoadBearing")
                    == Some(super::standard_property_sets::StandardPropertyKind::Boolean)
            }) {
                properties.push(Property {
                    name: "LoadBearing".into(),
                    value: PropertyValue::Boolean(bearing),
                });
            }
            if occurrence == "IFCSTAIR" && self.stair_type {
                properties.extend(stair_type_zeros());
            }
            if !properties.is_empty() {
                sets.push(PropertySet {
                    name: name.into(),
                    properties,
                });
            }
        }
        if let (true, Some(nosing)) = (occurrence == "IFCSTAIRFLIGHT", self.nosing_feet) {
            sets.push(PropertySet {
                name: "Pset_StairFlightCommon".into(),
                properties: vec![Property {
                    name: "NosingLength".into(),
                    value: PropertyValue::LengthFeet(nosing),
                }],
            });
        }
        sets
    }
}

/// What revit-ifc's stair calculators give a stair type object, which is no
/// stair and no flight: 0 risers, 0 treads and a nosing length of 0, in
/// `Pset_StairCommon` (RE-286), and with `--revit-set-placement` in
/// `Pset_StairFlightCommon` too ([`super::revit_set_placement`]). The BIM4LCA
/// office's three stair types of stairs with data and each R2025 stair model's
/// hold them; its fourth stair type, of a family placed as a stair, holds none.
/// Measured only: why the calculators give 0 on a type is not read.
fn stair_type_zeros() -> [Property; 3] {
    [
        Property {
            name: "NosingLength".into(),
            value: PropertyValue::LengthFeet(0.0),
        },
        Property {
            name: "NumberOfRiser".into(),
            value: PropertyValue::CountValue(0),
        },
        Property {
            name: "NumberOfTreads".into(),
            value: PropertyValue::CountValue(0),
        },
    ]
}

/// Note what the type object of `decoded`'s type holds in its common set
/// (RE-286): the type's Fire Rating (RE-77) and its `LoadBearing`
/// ([`LOAD_BEARING_TYPE_CLASSES`]), as revit-ifc runs the common set's
/// entries on the type itself (CLEANROOM.md source 5). No `IsExternal`:
/// Revit's newer exporters write none on a type object (RE-286).
/// Two elements of one type that disagree leave it none.
fn record_type_common(
    commons: &mut BTreeMap<(u32, String), Option<TypeCommon>>,
    ifc_type: &str,
    decoded: &DecodedElement,
) {
    use crate::partition_schema_mvp as mvp;
    let upper = ifc_type.to_ascii_uppercase();
    if !TYPE_COMMON_ENTITIES.contains(&upper.as_str()) {
        return;
    }
    let Some(type_id) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::ElementId { id, .. } if name == mvp::TYPE_ID_FIELD => Some(*id),
        _ => None,
    }) else {
        return;
    };
    let fire_rating = decoded
        .fields
        .iter()
        .find_map(|(field, value)| match value {
            InstanceField::String(text)
                if field.strip_prefix(mvp::TYPE_PARAMETER_FIELD_PREFIX) == Some("Fire Rating")
                    && !text.is_empty() =>
            {
                Some(text.clone())
            }
            _ => None,
        });
    let found = TypeCommon {
        fire_rating,
        load_bearing: LOAD_BEARING_TYPE_CLASSES
            .contains(&decoded.class.as_str())
            .then_some(true),
        stair_type: decoded.class == "Stair"
            && decoded
                .fields
                .iter()
                .any(|(name, _)| name == mvp::STAIR_RISER_COUNT_FIELD),
        nosing_feet: decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. }
                if decoded.class == "StairsRun" && name == mvp::STAIR_NOSING_LENGTH_FIELD =>
            {
                Some(*value)
            }
            _ => None,
        }),
    };
    commons
        .entry((type_id, format!("{upper}TYPE")))
        .and_modify(|seen| {
            if seen.as_ref() != Some(&found) {
                *seen = None;
            }
        })
        .or_insert(Some(found));
}

/// Add `ExtendToStructure` (`IfcBoolean`) to a wall's `Pset_WallCommon`
/// among `sets`: whether its top is joined to a roof or floor, as its native
/// record says (RE-228). Nothing where the record is not read.
fn add_extend_to_structure(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    if !matches!(ifc_type, "IFCWALL" | "IFCWALLSTANDARDCASE") {
        return;
    }
    let Some(joined) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Bool(flag) if name == crate::partition_schema_mvp::WALL_TOP_JOINED_FIELD => {
            Some(*flag)
        }
        _ => None,
    }) else {
        return;
    };
    let property = Property {
        name: "ExtendToStructure".into(),
        value: PropertyValue::Boolean(joined),
    };
    match sets.iter_mut().find(|set| set.name == "Pset_WallCommon") {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: "Pset_WallCommon".into(),
            properties: vec![property],
        }),
    }
}

/// Whether a slab's top is known to lie level, so that it gets `PitchAngle`
/// 0 in its `Pset_SlabCommon`: revit-ifc's `SlopeCalculator` (CLEANROOM.md
/// source 5) gives a slab with no sloped extrusion and no location curve
/// the slope of its largest upward face (RE-238). A stair landing drawn from
/// its top down its type's thickness (RE-214) is level, and so is a slab
/// drawn as an extrusion whose depth is its type's layers' total
/// thickness, as a sloped or shape-edited slab's record box is deeper than
/// its layers. Any other slab gets nothing: on the BIM4LCA office the
/// record boxes of 8 sloped floors are drawn level, and Revit gives them
/// 1.9 to 24 degrees.
fn level_slab(
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
    has_solid_shape: bool,
    property_set: Option<&PropertySet>,
) -> bool {
    const TOLERANCE_FEET: f64 = 1e-3;
    if ifc_type != "IFCSLAB" {
        return false;
    }
    let landing = property_set
        .iter()
        .flat_map(|set| &set.properties)
        .any(|property| {
            property.name == "BodySource"
                && matches!(&property.value, PropertyValue::Text(text) if text == LANDING_SLAB_BODY_SOURCE)
        });
    let layered = !has_solid_shape
        && extrusion.is_some_and(|body| {
            crate::partition_schema_mvp::element_layers_from_fields(&decoded.fields)
                .filter(|layers| layers.stacked && !layers.layers.is_empty())
                .is_some_and(|layers| {
                    let total: f64 = layers.layers.iter().map(|layer| layer.width_feet).sum();
                    (total - body.height_feet).abs() <= TOLERANCE_FEET
                })
        });
    landing || layered
}

/// Add `PitchAngle` 0 to the `Pset_SlabCommon` among `sets` of a slab
/// [`level_slab`] finds level, read before a mode without geometry strips
/// its body.
fn add_level_slab_pitch(sets: &mut Vec<PropertySet>) {
    let property = Property {
        name: "PitchAngle".into(),
        value: PropertyValue::AngleRadians(0.0),
    };
    match sets.iter_mut().find(|set| set.name == "Pset_SlabCommon") {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: "Pset_SlabCommon".into(),
            properties: vec![property],
        }),
    }
}

/// Add a stair railing's `Qto_RailingBaseQuantities.Length` to `sets`, its
/// path along its stair (RE-254).
fn add_railing_length(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCRAILING" {
        return;
    }
    let Some(length) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. }
            if name == crate::partition_schema_mvp::RAILING_LENGTH_FIELD =>
        {
            Some(*value)
        }
        _ => None,
    }) else {
        return;
    };
    sets.push(PropertySet {
        name: "Qto_RailingBaseQuantities".into(),
        properties: vec![Property {
            name: "Length".into(),
            value: PropertyValue::PositiveLengthFeet(length),
        }],
    });
}

/// Add a space's `Qto_SpaceBaseQuantities.Height` to `sets`, its room's
/// height (RE-255).
fn add_space_height(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCSPACE" {
        return;
    }
    let Some(height) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. }
            if name == crate::partition_schema_mvp::ROOM_HEIGHT_FIELD =>
        {
            Some(*value)
        }
        _ => None,
    }) else {
        return;
    };
    sets.push(PropertySet {
        name: "Qto_SpaceBaseQuantities".into(),
        properties: vec![Property {
            name: "Height".into(),
            value: PropertyValue::PositiveLengthFeet(height),
        }],
    });
}

/// Add a space's `Qto_SpaceBaseQuantities.NetVolume` to `sets` (#1016,
/// RE-296): the area of its outline, its holes taken off, times the height of
/// its box, where the room's stored solid is a prism of that outline
/// ([`crate::partition_schema_mvp::ROOM_PRISM_FIELD`]). Revit's export writes
/// the volume of the room's closed shell, which on such a room is that: on
/// every one of the 167 rooms of the BIM4LCA office as built and the 178 at
/// building permit that are prisms, and on none of the rooms whose ceiling
/// steps. `GrossVolume` is left out: Revit's export writes it only on the
/// rooms it draws as an extrusion, 88 of those 167, and nothing in the file
/// tells them from the other 79 it draws as a tessellation.
fn add_space_net_volume(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
    single: bool,
) {
    if ifc_type != "IFCSPACE" || !single {
        return;
    }
    let prism = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::ROOM_PRISM_FIELD
            && matches!(value, InstanceField::Bool(true))
    });
    let from_solid = decoded.fields.iter().any(|(name, value)| {
        name == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD
            && matches!(value, InstanceField::String(source)
                if source == crate::partition_room_boundaries::ROOM_OUTLINE_SOURCE)
    });
    let Some(body) = extrusion.filter(|_| prism && from_solid) else {
        return;
    };
    let Some(area) = profile_area(body) else {
        return;
    };
    let height = body.height_feet;
    if !(height.is_finite() && height > 0.0) {
        return;
    }
    sets.push(PropertySet {
        name: "Qto_SpaceBaseQuantities".into(),
        properties: vec![Property {
            name: "NetVolume".into(),
            value: PropertyValue::VolumeCubicFeet(area * height),
        }],
    });
}

/// Add `Height` (`IfcPositiveLengthMeasure`) to a railing's
/// `Pset_RailingCommon` among `sets`: its type's top rail height (RE-231).
fn add_railing_height(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCRAILING" {
        return;
    }
    let Some(height) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. }
            if name == crate::partition_schema_mvp::RAILING_HEIGHT_FIELD =>
        {
            Some(*value)
        }
        _ => None,
    }) else {
        return;
    };
    let property = Property {
        name: "Height".into(),
        value: PropertyValue::PositiveLengthFeet(height),
    };
    match sets.iter_mut().find(|set| set.name == "Pset_RailingCommon") {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: "Pset_RailingCommon".into(),
            properties: vec![property],
        }),
    }
}

/// Add a door's `Qto_DoorBaseQuantities` or a window's
/// `Qto_WindowBaseQuantities` to `sets`: the Width and Height of the symbol
/// that draws it, and their product as its Area. revit-ifc's
/// `WidthCalculator`, `HeightCalculator` and `AreaCalculator` (CLEANROOM.md
/// source 5) read a door's or window's Width and Height parameters, its own
/// or its type's, where no opening profile gives them (#1016); an instance
/// whose own parameters change its size is drawn by a symbol of its own,
/// which holds them (RE-215). Nothing where that symbol's size is not read.
fn add_door_window_quantities(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
) {
    use crate::partition_schema_mvp as mvp;
    let set = match ifc_type {
        "IFCDOOR" => "Qto_DoorBaseQuantities",
        "IFCWINDOW" => "Qto_WindowBaseQuantities",
        _ => return,
    };
    let Some([width, height]) = mvp::drawn_size_from_fields(&decoded.fields) else {
        return;
    };
    sets.push(PropertySet {
        name: set.into(),
        properties: vec![
            Property {
                name: "Width".into(),
                value: PropertyValue::PositiveLengthFeet(width),
            },
            Property {
                name: "Height".into(),
                value: PropertyValue::PositiveLengthFeet(height),
            },
            Property {
                name: "Area".into(),
                value: PropertyValue::AreaSquareFeet(width * height),
            },
        ],
    });
}

/// Add the base quantities a column's body gives to `sets` (#1016, RE-247):
/// its `Length`, the depth of the extrusion it is drawn as, and, where that
/// extrusion is its type's hollow section (whose family parameters are
/// Revit's profile exactly), its `CrossSectionArea` (the section's area),
/// `OuterSurfaceArea` (the length times the section's outline and hole) and
/// `GrossVolume` (the area times the length), as revit-ifc's
/// `PropertyUtil.CreateBeamColumnBaseQuantities` takes them (CLEANROOM.md
/// source 5). Its `NetVolume` is left out: Revit's export writes the volume
/// Revit computes, less than that on every one of the BIM4LCA office's
/// hollow columns. A column drawn as its record's box gets none of these, as a
/// round column is drawn as one too. All on a column joined by no
/// `CleanPairGStep`
/// ([`crate::partition_schema_mvp::COLUMN_UNJOINED_FIELD`]), which Revit's
/// export draws as that extrusion. A joined column's Length is left out:
/// Revit's export draws 18 of the BIM4LCA office's at permit as a cut mesh
/// and gives them none, and no field tells them from the others. None where
/// another solid draws it. A curtain panel's `Width`, its box's larger side
/// wherever Revit writes it, is left out too: Revit's export writes none on
/// 3 of the office's panels that no field tells apart.
fn add_body_quantities(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
    solid_shape: Option<&entities::SolidShape>,
) {
    let unjoined = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::COLUMN_UNJOINED_FIELD
            && matches!(value, InstanceField::Bool(true))
    });
    if !unjoined {
        return;
    }
    let (feet, profile) = match (ifc_type, extrusion, solid_shape) {
        ("IFCCOLUMN", Some(body), None) => (body.height_feet, Some(body)),
        ("IFCCOLUMN", _, Some(entities::SolidShape::PlacedExtrusion { depth_feet, .. })) => {
            (*depth_feet, None)
        }
        _ => return,
    };
    if !(feet.is_finite() && feet > 0.0) {
        return;
    }
    let mut properties = vec![Property {
        name: "Length".into(),
        value: PropertyValue::PositiveLengthFeet(feet),
    }];
    if let Some((area, perimeter)) = profile.and_then(hollow_section_area_perimeter) {
        properties.extend([
            Property {
                name: "CrossSectionArea".into(),
                value: PropertyValue::AreaSquareFeet(area),
            },
            Property {
                name: "OuterSurfaceArea".into(),
                value: PropertyValue::AreaSquareFeet(perimeter * feet),
            },
            Property {
                name: "GrossVolume".into(),
                value: PropertyValue::VolumeCubicFeet(area * feet),
            },
        ]);
    }
    sets.push(PropertySet {
        name: "Qto_ColumnBaseQuantities".into(),
        properties,
    });
}

/// Add a beam's `Qto_BeamBaseQuantities.Length` to `sets` (#1016): the
/// length of the location line it is drawn along (RE-49, its record set's
/// `AxisLength`), where its body is that line's box
/// ([`BEAM_AXIS_BODY_SOURCE`]), whose ends lie in the beam's record box:
/// Revit's export writes it as its body's extrusion length, the line's on
/// each of the BIM4LCA office's 8 beams. Its areas and volumes are left out:
/// that box is not the beam's section.
fn add_beam_length(sets: &mut Vec<PropertySet>, ifc_type: &str, record: Option<&PropertySet>) {
    if ifc_type != "IFCBEAM" {
        return;
    }
    let Some(record) = record else {
        return;
    };
    let along_axis = record.properties.iter().any(|property| {
        property.name == "BodySource"
            && matches!(&property.value, PropertyValue::Text(text) if text == BEAM_AXIS_BODY_SOURCE)
    });
    let length = record
        .properties
        .iter()
        .find_map(|property| match &property.value {
            PropertyValue::LengthFeet(feet) if property.name == "AxisLength" => Some(*feet),
            _ => None,
        });
    let (true, Some(length)) = (along_axis, length) else {
        return;
    };
    if !(length.is_finite() && length > 0.0) {
        return;
    }
    sets.push(PropertySet {
        name: "Qto_BeamBaseQuantities".into(),
        properties: vec![Property {
            name: "Length".into(),
            value: PropertyValue::PositiveLengthFeet(length),
        }],
    });
}

/// Add a beam's `Pset_BeamCommon.Span` to `sets` (W7): revit-ifc's
/// `SpanCalculator` (CLEANROOM.md source 5) takes it, where no parameter
/// gives it, from the length of the extrusion the beam is exported as, the
/// length `Qto_BeamBaseQuantities.Length` holds ([`add_beam_length`]), which
/// is the line the beam is drawn along (RE-49). Revit's export writes the
/// two equal on each of the BIM4LCA office's 8 beams, as built and at
/// permit. Written only where `sets` holds that `Length`, so a beam Revit
/// draws another way, whose export has no extrusion length, gets none.
fn add_beam_span(sets: &mut Vec<PropertySet>, ifc_type: &str) {
    if ifc_type != "IFCBEAM" {
        return;
    }
    let Some(length) = beam_extrusion_length(sets) else {
        return;
    };
    add_beam_common(
        sets,
        Property {
            name: "Span".into(),
            value: PropertyValue::PositiveLengthFeet(length),
        },
    );
}

/// The `Length` of the `Qto_BeamBaseQuantities` among `sets` ([`add_beam_length`]),
/// the length of the extrusion Revit's export draws the beam as.
fn beam_extrusion_length(sets: &[PropertySet]) -> Option<f64> {
    sets.iter()
        .find(|set| set.name == "Qto_BeamBaseQuantities")
        .and_then(|set| {
            set.properties.iter().find_map(|property| {
                match (property.name.as_str(), &property.value) {
                    ("Length", PropertyValue::PositiveLengthFeet(feet)) => Some(*feet),
                    _ => None,
                }
            })
        })
}

/// Add `property` to the `Pset_BeamCommon` among `sets`, or a new one.
fn add_beam_common(sets: &mut Vec<PropertySet>, property: Property) {
    match sets.iter_mut().find(|set| set.name == "Pset_BeamCommon") {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: "Pset_BeamCommon".into(),
            properties: vec![property],
        }),
    }
}

/// Add a level beam's `Pset_BeamCommon.Slope` 0 to `sets` (W7): revit-ifc's
/// `SlopeCalculator` (CLEANROOM.md source 5) takes the slope of a beam it
/// exports as an extrusion, where no parameter named `Slope` gives it, from
/// the extrusion's own slope where there is one, and otherwise as the angle
/// between its location line and the horizontal, 0 for a line in the plan.
/// Only a beam written with the extrusion length of [`add_beam_length`]
/// whose location line does not leave the horizontal (its direction's
/// vertical share below [`crate::partition_beam_axes::HORIZONTAL_EPS`]) gets
/// it: a sloped beam's value comes from the extrusion's slope, which is not
/// measured, so it gets none. Revit's export writes 0 on each of the BIM4LCA
/// office's 8 beams, as built and at permit, all level.
fn add_beam_slope(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCBEAM" || beam_extrusion_length(sets).is_none() {
        return;
    }
    let Some((start, end)) = crate::partition_schema_mvp::beam_axis_from_fields(&decoded.fields)
    else {
        return;
    };
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if !(length.is_finite() && length > 0.0)
        || (delta[2] / length).abs() > crate::partition_beam_axes::HORIZONTAL_EPS
    {
        return;
    }
    add_beam_common(
        sets,
        Property {
            name: "Slope".into(),
            value: PropertyValue::AngleRadians(0.0),
        },
    );
}

/// Add an unrotated beam's `Pset_BeamCommon.Roll` 0 to `sets` (W7, RE-276):
/// revit-ifc's `RollCalculator` (CLEANROOM.md source 5) takes the roll from
/// the beam's Cross-Section Rotation parameter, in radians, whatever its
/// value. rvt-rs reads that rotation from the beam's native record
/// ([`crate::partition_schema_mvp::BEAM_ROTATION_FIELD`]) and writes the roll
/// of a beam whose rotation is exactly 0, as it is on each of the BIM4LCA
/// office's 8 beams, as built and at permit, on beams written with the
/// extrusion length of [`add_beam_length`]. A rotated beam gets none: no
/// measured beam is rotated, so its roll is not measured.
fn add_beam_roll(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCBEAM" || beam_extrusion_length(sets).is_none() {
        return;
    }
    let unrotated = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::BEAM_ROTATION_FIELD
            && matches!(value, InstanceField::Float { value, .. } if *value == 0.0)
    });
    if !unrotated {
        return;
    }
    add_beam_common(
        sets,
        Property {
            name: "Roll".into(),
            value: PropertyValue::AngleRadians(0.0),
        },
    );
}

/// Add a floor's `Qto_SlabBaseQuantities.GrossArea` to `sets` (#1016,
/// RE-249): the area of the profile its body extrudes, its outline less its
/// holes, as Revit's export writes it. Only on a floor drawn as one extrusion
/// (`single`): a sketch of separate loops is several elements sharing its
/// `Tag`, and a stair landing's slab gets none from Revit's export. Where the
/// floor's type has one layer and the extrusion is as deep (RE-238), also,
/// as revit-ifc's `DepthCalculator` and `GrossVolumeCalculator` take them
/// from that extrusion (CLEANROOM.md source 5), its `Depth`, the extrusion's
/// length, and its `GrossVolume`, the area times that length. Its
/// `Perimeter` is left out: Revit's export writes it in feet, unscaled, in
/// a file whose length unit is millimetres (RE-235 §5).
///
/// Where the floor's type's layers are read and its record holds no step that
/// cuts it ([`crate::partition_schema_mvp::SLAB_UNCUT_FIELD`]), also its
/// `NetVolume` (RE-295, #1016), that area times the layers' total thickness:
/// the volume of the floor's solid, which Revit's export writes, is that on
/// every floor of the BIM4LCA office as built and at building permit that
/// nothing cuts, flat or sloped, whatever its layers. Its `Width` is left out
/// (RE-294): Revit 2024's exporter writes a compound floor's type thickness
/// there and a later exporter (Revit 2026) writes none on one, so the
/// exporters disagree; on a floor of one layer Revit writes its profile's
/// extent along the extrusion's Y axis, which no field of the floor's record
/// gives.
fn add_slab_gross_area(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
    single: bool,
) {
    if ifc_type != "IFCSLAB" || decoded.class != "Floor" || !single {
        return;
    }
    let Some(body) = extrusion else {
        return;
    };
    let Some(area) = profile_area(body) else {
        return;
    };
    let mut properties = vec![Property {
        name: "GrossArea".into(),
        value: PropertyValue::AreaSquareFeet(area),
    }];
    const TOLERANCE_FEET: f64 = 1e-3;
    let depth = body.height_feet;
    let layers = crate::partition_schema_mvp::element_layers_from_fields(&decoded.fields)
        .filter(|layers| layers.stacked && !layers.layers.is_empty());
    let layered = layers.as_ref().is_some_and(|layers| {
        layers.layers.len() == 1 && (layers.layers[0].width_feet - depth).abs() <= TOLERANCE_FEET
    });
    let float = |wanted: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let uncut = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::SLAB_UNCUT_FIELD
            && matches!(value, InstanceField::Bool(true))
    });
    if layered && depth.is_finite() && depth > 0.0 {
        properties.push(Property {
            name: "Depth".into(),
            value: PropertyValue::PositiveLengthFeet(depth),
        });
        properties.push(Property {
            name: "GrossVolume".into(),
            value: PropertyValue::VolumeCubicFeet(area * depth),
        });
    }
    if let Some(thickness) = float(crate::partition_schema_mvp::SLAB_TYPE_THICKNESS_FIELD)
        .filter(|thickness| thickness.is_finite() && *thickness > 0.0)
    {
        if uncut {
            properties.push(Property {
                name: "NetVolume".into(),
                value: PropertyValue::VolumeCubicFeet(area * thickness),
            });
        }
    }
    sets.push(PropertySet {
        name: "Qto_SlabBaseQuantities".into(),
        properties,
    });
}

/// The area of the profile `body` extrudes, its rectangle or its outline less
/// its holes; `None` for another profile or an area that is not positive.
fn profile_area(body: &Extrusion) -> Option<f64> {
    use crate::ifc::from_decoded::polygon_area_sqft;
    let area = match &body.profile_override {
        None => body.width_feet * body.depth_feet,
        Some(entities::ProfileDef::Rectangle {
            width_feet,
            depth_feet,
        }) => width_feet * depth_feet,
        Some(entities::ProfileDef::ArbitraryClosed { points }) => polygon_area_sqft(points),
        Some(entities::ProfileDef::ArbitraryWithVoids { points, voids }) => {
            polygon_area_sqft(points) - voids.iter().map(|v| polygon_area_sqft(v)).sum::<f64>()
        }
        Some(_) => return None,
    };
    (area.is_finite() && area > 0.0).then_some(area)
}

/// The area and the length of the outline and hole of a hollow section
/// `body` extrudes ([`hollow_section_profile`]), its arcs counted exactly;
/// `None` for another profile.
fn hollow_section_area_perimeter(body: &Extrusion) -> Option<(f64, f64)> {
    let Some(entities::ProfileDef::IndexedPolyCurveWithVoids { outer, voids }) =
        &body.profile_override
    else {
        return None;
    };
    let (mut area, mut perimeter) = curve_area_length(outer)?;
    area = area.abs();
    for void in voids {
        let (hole, length) = curve_area_length(void)?;
        area -= hole.abs();
        perimeter += length;
    }
    (area.is_finite() && area > 0.0 && perimeter.is_finite()).then_some((area, perimeter))
}

/// The signed area (positive counter-clockwise) and the length of a closed
/// curve of lines and three-point arcs, by Green's theorem, so an arc counts
/// as itself and not its chord.
fn curve_area_length(curve: &entities::IndexedCurve) -> Option<(f64, f64)> {
    use std::f64::consts::TAU;
    let point = |i: usize| curve.points.get(i).copied();
    let (mut area, mut length) = (0.0, 0.0);
    for segment in &curve.segments {
        match *segment {
            entities::CurveSegment::Line([a, b]) => {
                let ((x0, y0), (x1, y1)) = (point(a)?, point(b)?);
                area += (x0 * y1 - x1 * y0) / 2.0;
                length += (x1 - x0).hypot(y1 - y0);
            }
            entities::CurveSegment::Arc([a, m, b]) => {
                let (p0, p1, p2) = (point(a)?, point(m)?, point(b)?);
                let d = 2.0 * (p0.0 * (p1.1 - p2.1) + p1.0 * (p2.1 - p0.1) + p2.0 * (p0.1 - p1.1));
                if d.abs() < 1e-12 {
                    return None;
                }
                let sq = |p: (f64, f64)| p.0 * p.0 + p.1 * p.1;
                let cx =
                    (sq(p0) * (p1.1 - p2.1) + sq(p1) * (p2.1 - p0.1) + sq(p2) * (p0.1 - p1.1)) / d;
                let cy =
                    (sq(p0) * (p2.0 - p1.0) + sq(p1) * (p0.0 - p2.0) + sq(p2) * (p1.0 - p0.0)) / d;
                let r = (p0.0 - cx).hypot(p0.1 - cy);
                let angle = |p: (f64, f64)| (p.1 - cy).atan2(p.0 - cx);
                let (a0, am, a2) = (angle(p0), angle(p1), angle(p2));
                let ccw = (a2 - a0).rem_euclid(TAU);
                let sweep = if (am - a0).rem_euclid(TAU) < ccw {
                    ccw
                } else {
                    ccw - TAU
                };
                let a1 = a0 + sweep;
                area += (r * r * sweep + cx * r * (a1.sin() - a0.sin())
                    - cy * r * (a1.cos() - a0.cos()))
                    / 2.0;
                length += r * sweep.abs();
            }
        }
    }
    Some((area, length))
}

/// Add a ceiling's `Qto_CoveringBaseQuantities` to `sets` (#1016, RE-250,
/// RE-297): on a ceiling drawn as one extrusion of its sketch's outline whose
/// record holds no step that cuts it
/// ([`crate::partition_schema_mvp::SLAB_UNCUT_FIELD`]), where the extrusion is
/// as deep as its type's layers' total thickness (as RE-238 tells a level
/// slab), its `GrossArea`, the area of that profile, its holes taken off.
/// Revit's export writes a cut ceiling's area less the cut: the sketch's area
/// is 0.7 to 1.1 percent larger than Revit's on the 2 of the BIM4LCA office's
/// ceilings as built and the 9 at permit whose records hold a `CutPairGStep`,
/// and on no other. On a ceiling of several pieces Revit's export gives the
/// area of one of them, which no field of the record says. Its `Width` is left
/// out (RE-294): Revit 2024's exporter writes the extrusion's depth there and
/// a later one (Revit 2025 or 2026) writes none on a covering, so the exporters
/// disagree.
fn add_ceiling_quantities(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
    pieces: &[Extrusion],
    extruded: bool,
) {
    const TOLERANCE_FEET: f64 = 1e-3;
    if ifc_type != "IFCCOVERING" || decoded.class != "Ceiling" || !extruded {
        return;
    }
    let Some(body) = extrusion else {
        return;
    };
    let mut properties = Vec::new();
    let from_sketch = decoded.fields.iter().any(|(name, value)| {
        name == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD
            && matches!(value, InstanceField::String(source)
                if source == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE)
    });
    let uncut = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::SLAB_UNCUT_FIELD
            && matches!(value, InstanceField::Bool(true))
    });
    // A ceiling whose layers fill its body's depth is level (RE-238); a
    // sloped one's profile is only its plan projection, so it gets neither.
    let layered = crate::partition_schema_mvp::element_layers_from_fields(&decoded.fields)
        .filter(|layers| layers.stacked && !layers.layers.is_empty())
        .is_some_and(|layers| {
            let total: f64 = layers.layers.iter().map(|layer| layer.width_feet).sum();
            (total - body.height_feet).abs() <= TOLERANCE_FEET
        });
    if layered && pieces.is_empty() && from_sketch && uncut {
        if let Some(area) = profile_area(body) {
            properties.push(Property {
                name: "GrossArea".into(),
                value: PropertyValue::AreaSquareFeet(area),
            });
        }
    }
    if properties.is_empty() {
        return;
    }
    sets.push(PropertySet {
        name: "Qto_CoveringBaseQuantities".into(),
        properties,
    });
}

/// Add a stair run's `Qto_StairFlightBaseQuantities` to `sets` (#1016,
/// RE-247): its `Length`, the distance from its first riser line to its last
/// ([`crate::partition_schema_mvp::STAIR_RUN_LENGTH_FIELD`]), and its
/// `NetVolume`, its side views' areas times the run's width, the solids
/// [`STAIR_RUN_BODY_SOURCE`] draws (one, or a riserless run's treads). None
/// for a run not drawn from its sketch.
fn add_stair_flight_quantities(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
) {
    if ifc_type != "IFCSTAIRFLIGHT" || decoded.class != "StairsRun" {
        return;
    }
    let Some(run) = crate::partition_schema_mvp::stair_run_body_from_fields(&decoded.fields)
        .filter(|run| run.profile.len() >= 3)
    else {
        return;
    };
    let length = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. }
            if name == crate::partition_schema_mvp::STAIR_RUN_LENGTH_FIELD =>
        {
            Some(*value)
        }
        _ => None,
    });
    let area: f64 = std::iter::once(&run.profile)
        .chain(&run.pieces)
        .map(|ring| {
            ring.iter()
                .zip(ring.iter().cycle().skip(1))
                .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
                .sum::<f64>()
                .abs()
                / 2.0
        })
        .sum();
    let volume = area * run.width_feet;
    if !(volume.is_finite() && volume > 0.0) {
        return;
    }
    let mut properties: Vec<Property> = length
        .filter(|length| length.is_finite() && *length > 0.0)
        .map(|length| Property {
            name: "Length".into(),
            value: PropertyValue::PositiveLengthFeet(length),
        })
        .into_iter()
        .collect();
    properties.push(Property {
        name: "NetVolume".into(),
        value: PropertyValue::VolumeCubicFeet(volume),
    });
    sets.push(PropertySet {
        name: "Qto_StairFlightBaseQuantities".into(),
        properties,
    });
}

/// Add a wall's `Qto_WallBaseQuantities` `Width` to `sets`: its type's
/// thickness, the wall's Width parameter revit-ifc's `WidthCalculator`
/// (CLEANROOM.md source 5) reads (#1016), also on a wall the partition
/// reader does not draw from its centreline, from its type's layers. Its `Length` too where the partition
/// reader gives one ([`crate::partition_schema_mvp::WALL_LENGTH_FIELD`]):
/// Revit's export writes either the wall's curve's length or the length of
/// that curve cut back at its joins, by how it draws the wall, so a Length is
/// written only where the two are the same (Griffin, 2026-10-26, RE-252).
/// Nothing where the thickness is not read.
/// Add a curtain wall's `Qto_CurtainWallQuantities.Length` to `sets`: its
/// curve's length ([`crate::partition_schema_mvp::WALL_LENGTH_FIELD`]), which
/// revit-ifc's `LengthCalculator` takes from the wall's location curve
/// (CLEANROOM.md source 5): on all 218 of the BIM4LCA office's straight
/// curtain walls, as built and at permit (#1016). Nothing on a curved one.
fn add_curtain_wall_length(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    use crate::partition_schema_mvp as mvp;
    if ifc_type != "IFCCURTAINWALL" || decoded.class != mvp::CURTAIN_WALL_CLASS {
        return;
    }
    let Some(length) = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == mvp::WALL_LENGTH_FIELD => Some(*value),
        _ => None,
    }) else {
        return;
    };
    sets.push(PropertySet {
        name: "Qto_CurtainWallQuantities".into(),
        properties: vec![Property {
            name: "Length".into(),
            value: PropertyValue::PositiveLengthFeet(length),
        }],
    });
}

fn add_wall_quantities(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    use crate::partition_schema_mvp as mvp;
    if !matches!(ifc_type, "IFCWALL" | "IFCWALLSTANDARDCASE") {
        return;
    }
    let thickness = mvp::wall_axis_from_fields(&decoded.fields)
        .map(|axis| axis.thickness_feet)
        .or_else(|| mvp::wall_arc_from_fields(&decoded.fields).map(|arc| arc.thickness_feet))
        .or_else(|| mvp::wall_layers_width_from_fields(&decoded.fields));
    let Some(thickness) = thickness.filter(|t| t.is_finite() && *t > 0.0) else {
        return;
    };
    let length = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == mvp::WALL_LENGTH_FIELD => Some(*value),
        _ => None,
    });
    let mut properties: Vec<Property> = length
        .filter(|l| l.is_finite() && *l > 0.0)
        .map(|length| Property {
            name: "Length".into(),
            value: PropertyValue::PositiveLengthFeet(length),
        })
        .into_iter()
        .collect();
    properties.push(Property {
        name: "Width".into(),
        value: PropertyValue::PositiveLengthFeet(thickness),
    });
    sets.push(PropertySet {
        name: "Qto_WallBaseQuantities".into(),
        properties,
    });
}

/// Add a wall's `Qto_WallBaseQuantities.Height` to `sets`: the depth of
/// the extrusion it is drawn as, where its native record names nothing
/// joined to or cutting it ([`crate::partition_schema_mvp::WALL_UNCUT_FIELD`]).
/// revit-ifc's `WallExporter` (CLEANROOM.md source 5) writes the vertical
/// span of Revit's body, and on those walls the record box rvt-rs extrudes
/// is that tall: on all 81 of the BIM4LCA office's as built, 70 at building
/// permit and 93 of puzzbobb's R2025 walls, with Revit writing a `Height` on
/// each (RE-266). Nothing on any other wall, whose body may be cut (#1067).
fn add_wall_height(
    sets: &mut Vec<PropertySet>,
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
) {
    if !matches!(ifc_type, "IFCWALL" | "IFCWALLSTANDARDCASE") {
        return;
    }
    let uncut = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::WALL_UNCUT_FIELD
            && matches!(value, InstanceField::Bool(true))
    });
    let Some(height) = extrusion
        .map(|body| body.height_feet)
        .filter(|height| uncut && height.is_finite() && *height > 0.0)
    else {
        return;
    };
    let property = Property {
        name: "Height".into(),
        value: PropertyValue::PositiveLengthFeet(height),
    };
    match sets
        .iter_mut()
        .find(|set| set.name == "Qto_WallBaseQuantities")
    {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: "Qto_WallBaseQuantities".into(),
            properties: vec![property],
        }),
    }
}

/// Add a wall's `Qto_WallBaseQuantities` `GrossSideArea`, `NetSideArea`,
/// `GrossVolume` and `NetVolume` to `sets`, where its native record holds no
/// step that cuts, embeds or sweeps it
/// ([`crate::partition_schema_mvp::WALL_WHOLE_FIELD`]) and no roof or floor
/// joined to it, and its body is its record box cut back at its joins
/// (`partition_element_record_join_trimmed`), a plain rectangle.
/// revit-ifc's `PropertyUtil.CreateWallBaseQuantities` (CLEANROOM.md source 5)
/// takes the side areas from the largest vertical face of the wall's solid,
/// `NetVolume` from the solid's volume and `GrossVolume` as the wall type's
/// width times the side area. On a rectangular prism those are the longer
/// side's area, the rectangle's area times the height, and the width times
/// that side's area: Revit's on all 97 of the BIM4LCA office's such walls
/// (49 as built, 48 at building permit) and 91 of puzzbobb's R2025 walls, and
/// Revit's export writes all four on each (RE-270). Nothing on any other wall,
/// whose solid may be cut, staircased or open (#1046).
fn add_wall_areas_and_volumes(
    sets: &mut [PropertySet],
    ifc_type: &str,
    decoded: &DecodedElement,
    extrusion: Option<&Extrusion>,
    property_set: Option<&PropertySet>,
) {
    if !matches!(ifc_type, "IFCWALL" | "IFCWALLSTANDARDCASE") {
        return;
    }
    let flagged = |wanted: &str| {
        decoded
            .fields
            .iter()
            .any(|(name, value)| name == wanted && matches!(value, InstanceField::Bool(true)))
    };
    let boxed = property_set.is_some_and(|set| {
        set.properties.iter().any(|property| {
            property.name == "BodySource"
                && matches!(&property.value, PropertyValue::Text(source)
                    if source == crate::element_record_wall_joins::WALL_BODY_JOIN_TRIMMED)
        })
    });
    if !(flagged(crate::partition_schema_mvp::WALL_UNCUT_FIELD)
        && flagged(crate::partition_schema_mvp::WALL_WHOLE_FIELD)
        && boxed)
    {
        return;
    }
    let Some(body) = extrusion.filter(|body| body.profile_override.is_none()) else {
        return;
    };
    let Some(set) = sets
        .iter_mut()
        .find(|set| set.name == "Qto_WallBaseQuantities")
    else {
        return;
    };
    let Some(width) = set
        .properties
        .iter()
        .find_map(|property| match &property.value {
            PropertyValue::PositiveLengthFeet(width) if property.name == "Width" => Some(*width),
            _ => None,
        })
    else {
        return;
    };
    let Some(quantities) = prism_quantities(body, width) else {
        return;
    };
    set.properties.extend(quantities);
}

/// A rectangular wall's `GrossSideArea`, `NetSideArea`, `GrossVolume` and
/// `NetVolume` (RE-270): the longer side's area twice, the type's `width`
/// times it, and the rectangle's area times the height. `None` where a size
/// is not a positive number.
fn prism_quantities(body: &Extrusion, width: f64) -> Option<[Property; 4]> {
    let sizes = [body.width_feet, body.depth_feet, body.height_feet, width];
    if !sizes.iter().all(|size| size.is_finite() && *size > 0.0) {
        return None;
    }
    let side = body.width_feet.max(body.depth_feet) * body.height_feet;
    let volume = body.width_feet * body.depth_feet * body.height_feet;
    Some([
        Property {
            name: "GrossSideArea".into(),
            value: PropertyValue::AreaSquareFeet(side),
        },
        Property {
            name: "NetSideArea".into(),
            value: PropertyValue::AreaSquareFeet(side),
        },
        Property {
            name: "GrossVolume".into(),
            value: PropertyValue::VolumeCubicFeet(width * side),
        },
        Property {
            name: "NetVolume".into(),
            value: PropertyValue::VolumeCubicFeet(volume),
        },
    ])
}

/// Add `Span` (`IfcPositiveLengthMeasure`) to a curtain mullion's
/// `Pset_MemberCommon` among `sets`: its length, which revit-ifc's
/// `SpanCalculator` (CLEANROOM.md source 5) takes from the depth of the
/// mullion's body extrusion where no parameter gives it. A mullion runs along
/// its own Z axis (RE-226): on the model's axes, its record box's extent
/// along the model axis that Z lies on; turned or tilted, the extent along Z
/// of the box along its own axes (RE-106). Nothing where neither is read.
/// The same length is its `Qto_MemberBaseQuantities.Length`, which Revit's
/// export gives every one of the BIM4LCA office's 10,003 mullions equal to
/// their `Span` (#1016).
fn add_mullion_span(sets: &mut Vec<PropertySet>, ifc_type: &str, decoded: &DecodedElement) {
    use crate::partition_schema_mvp as mvp;
    if ifc_type != "IFCMEMBER" || decoded.class != "CurtainWallMullion" {
        return;
    }
    let float = |wanted: &str| {
        decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let (Some(width), Some(depth), Some(height)) = (
        float("m_bboxWidth"),
        float("m_bboxDepth"),
        float("m_bboxHeight"),
    ) else {
        return;
    };
    let size = [width, depth, height];
    let along = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == mvp::CURTAIN_MULLION_ALONG_FIELD => {
            usize::try_from(*value).ok()
        }
        _ => None,
    });
    // The box's two sides across the mullion, then its length along it.
    let sides = match (mvp::curtain_axes_from_fields(&decoded.fields), along) {
        (Some(axes), _) => oriented_box(axes, size).map(|(_, half)| half.map(|h| 2.0 * h)),
        (None, Some(axis)) if axis < 3 => {
            let across: Vec<f64> = (0..3).filter(|i| *i != axis).map(|i| size[i]).collect();
            Some([across[0], across[1], size[axis]])
        }
        _ => None,
    };
    let Some(span) = sides
        .map(|sides| sides[2])
        .filter(|span| span.is_finite() && *span > 0.0)
    else {
        return;
    };
    let property = Property {
        name: "Span".into(),
        value: PropertyValue::PositiveLengthFeet(span),
    };
    match sets.iter_mut().find(|set| set.name == "Pset_MemberCommon") {
        Some(set) => set.properties.push(property),
        None => sets.push(PropertySet {
            name: "Pset_MemberCommon".into(),
            properties: vec![property],
        }),
    }
    let mut quantities = vec![Property {
        name: "Length".into(),
        value: PropertyValue::PositiveLengthFeet(span),
    }];
    if let Some(volume) = mullion_box_volume(decoded, sides, span) {
        quantities.push(Property {
            name: "NetVolume".into(),
            value: PropertyValue::VolumeCubicFeet(volume),
        });
    }
    if let Some(area) = mullion_net_surface_area(decoded, sides, span) {
        quantities.push(Property {
            name: "NetSurfaceArea".into(),
            value: PropertyValue::AreaSquareFeet(area),
        });
    }
    sets.push(PropertySet {
        name: "Qto_MemberBaseQuantities".into(),
        properties: quantities,
    });
}

/// A curtain mullion's `NetVolume` (#1016, RE-243): its box's two sides across
/// it times its length, where Revit's export draws it as an extrusion of
/// that section, a mullion whose type has its own rectangular section
/// (RE-246) on a straight curtain wall with an unedited profile
/// ([`crate::partition_schema_mvp::MULLION_PLAIN_HOST_FIELD`]). revit-ifc's
/// `NetVolumeCalculator` (CLEANROOM.md source 5) takes the volume of the
/// solid it writes.
fn mullion_box_volume(decoded: &DecodedElement, sides: Option<[f64; 3]>, span: f64) -> Option<f64> {
    use crate::partition_schema_mvp as mvp;
    let plain = decoded.fields.iter().any(|(name, value)| {
        name == mvp::MULLION_PLAIN_HOST_FIELD && matches!(value, InstanceField::Bool(true))
    });
    let [a, b, _] = sides?;
    (plain && a.is_finite() && b.is_finite() && a > 0.0 && b > 0.0).then_some(a * b * span)
}

/// A curtain mullion's `NetSurfaceArea` (#1016, RE-268): the perimeter of its
/// box's section times its length plus its section's area for each end face
/// its Revit solid keeps, where Revit's export draws it as an extrusion of
/// that section (RE-243) and the boxes of its curtain wall's mullions say
/// which ends are kept ([`crate::partition_schema_mvp::MULLION_KEPT_ENDS_FIELD`]).
/// revit-ifc's `NetSurfaceAreaCalculator` (CLEANROOM.md source 5) sums the
/// faces of the solid it writes.
fn mullion_net_surface_area(
    decoded: &DecodedElement,
    sides: Option<[f64; 3]>,
    span: f64,
) -> Option<f64> {
    use crate::partition_schema_mvp as mvp;
    let plain = decoded.fields.iter().any(|(name, value)| {
        name == mvp::MULLION_PLAIN_HOST_FIELD && matches!(value, InstanceField::Bool(true))
    });
    let kept = decoded
        .fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if name == mvp::MULLION_KEPT_ENDS_FIELD => {
                u8::try_from(*value).ok().filter(|ends| *ends <= 2)
            }
            _ => None,
        })?;
    let [a, b, _] = sides?;
    if !(plain && a.is_finite() && b.is_finite() && a > 0.0 && b > 0.0) {
        return None;
    }
    Some(2.0 * (a + b) * span + f64::from(kept) * a * b)
}

/// Add `Finish` (`IfcText`) to a covering's `Pset_CoveringCommon` among
/// `sets`: its type's finish layers' materials.
// Tracker: B41.
fn add_covering_finish(sets: &mut [PropertySet], ifc_type: &str, decoded: &DecodedElement) {
    if ifc_type != "IFCCOVERING" {
        return;
    }
    let finish = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text)
            if name == crate::partition_schema_mvp::COVERING_FINISH_FIELD =>
        {
            Some(text.clone())
        }
        _ => None,
    });
    let Some(finish) = finish else {
        return;
    };
    if let Some(set) = sets
        .iter_mut()
        .find(|set| set.name == "Pset_CoveringCommon")
    {
        set.properties.push(Property {
            name: "Finish".into(),
            value: PropertyValue::Text(finish),
        });
    }
}

/// The property Revit's exporter fills from a type's Fire Rating (RE-77).
const FIRE_RATING_PROPERTY: &str = "FireRating";

/// The values of the parameters named `<property set>.<property>` an element
/// carries (RE-194), by set: the properties [`super::standard_property_sets`]
/// lists, each as the IFC4 type it declares. `IsExternal` is
/// [`add_is_external`]'s. Then its type's Fire Rating (RE-77) as the
/// `FireRating` of each standard set declaring one that the element's own
/// parameters leave without it: Revit's exporter reads the parameter named
/// for the property first and the type's Fire Rating after it, as its
/// export of the BIM4LCA office shows (26 walls with no such parameter carry
/// their type's, and 4 whose parameter differs from their type's carry the
/// parameter's).
fn named_parameter_properties(decoded: &DecodedElement) -> Vec<(String, Property)> {
    use super::standard_property_sets::{self as sps, StandardPropertyKind as Kind};
    use crate::partition_schema_mvp as mvp;
    let mut properties = Vec::new();
    for (field, value) in &decoded.fields {
        let Some((set, property)) = field
            .strip_prefix(mvp::NAMED_PARAMETER_FIELD_PREFIX)
            .and_then(|name| name.split_once('.'))
        else {
            continue;
        };
        if property == mvp::IS_EXTERNAL_PARAMETER_NAME {
            continue;
        }
        let value = match (sps::property_kind(set, property), value) {
            (Some(Kind::Boolean), InstanceField::Bool(flag)) => PropertyValue::Boolean(*flag),
            (Some(Kind::Identifier), InstanceField::String(text)) => {
                PropertyValue::Identifier(text.clone())
            }
            (Some(Kind::Label), InstanceField::String(text)) => PropertyValue::Label(text.clone()),
            (Some(Kind::Text), InstanceField::String(text)) => PropertyValue::Text(text.clone()),
            (Some(Kind::Count), InstanceField::Integer { value, .. }) => {
                PropertyValue::CountValue(*value)
            }
            (Some(Kind::Integer), InstanceField::Integer { value, .. }) => {
                PropertyValue::Integer(*value)
            }
            (Some(Kind::Length), InstanceField::Float { value, .. }) => {
                PropertyValue::LengthFeet(*value)
            }
            (Some(Kind::PositiveLength), InstanceField::Float { value, .. }) => {
                PropertyValue::PositiveLengthFeet(*value)
            }
            (Some(Kind::ThermalTransmittance), InstanceField::Float { value, .. }) => {
                PropertyValue::ThermalTransmittance(*value)
            }
            _ => continue,
        };
        properties.push((
            set.to_string(),
            Property {
                name: property.into(),
                value,
            },
        ));
    }
    let type_fire_rating = decoded
        .fields
        .iter()
        .find_map(|(field, value)| match value {
            InstanceField::String(text)
                if field.strip_prefix(mvp::TYPE_PARAMETER_FIELD_PREFIX) == Some("Fire Rating")
                    && !text.is_empty() =>
            {
                Some(text.clone())
            }
            _ => None,
        });
    if let Some(rating) = type_fire_rating {
        for set in sps::STANDARD_PROPERTY_SETS {
            let named = properties
                .iter()
                .any(|(held, property)| held == set.name && property.name == FIRE_RATING_PROPERTY);
            if named || sps::property_kind(set.name, FIRE_RATING_PROPERTY) != Some(Kind::Label) {
                continue;
            }
            properties.push((
                set.name.to_string(),
                Property {
                    name: FIRE_RATING_PROPERTY.into(),
                    value: PropertyValue::Label(rating.clone()),
                },
            ));
        }
    }
    properties
}

/// Put `property` in `set`, replacing a property of the same name there.
fn set_property(set: &mut PropertySet, property: Property) {
    match set
        .properties
        .iter_mut()
        .find(|existing| existing.name == property.name)
    {
        Some(existing) => *existing = property,
        None => set.properties.push(property),
    }
}

/// Give the element at `element` among `entities` the set `set`: merged into
/// the set of the same name already given to it, each property replacing one
/// of the same name there, else as a set of its own.
fn add_element_set(entities: &mut Vec<entities::IfcEntity>, element: usize, set: PropertySet) {
    let held = entities
        .iter_mut()
        .skip(element)
        .find_map(|entity| match entity {
            entities::IfcEntity::ElementPropertySet {
                element: owner,
                set: held,
            } if *owner == element && held.name == set.name => Some(held),
            _ => None,
        });
    let Some(held) = held else {
        entities.push(entities::IfcEntity::ElementPropertySet { element, set });
        return;
    };
    for property in set.properties {
        set_property(held, property);
    }
}

/// Write each element's parameters named for a standard set's property
/// (RE-194, [`named_parameter_properties`]) as Revit's exporter does: in
/// each set that applies to the entity the element is finally written as,
/// so a door or window written as its opening alone (RE-84) gets none, and
/// in its set of the same name where it has one, each value replacing
/// rvt-rs's own.
fn add_named_parameter_sets(
    entities: &mut Vec<entities::IfcEntity>,
    pending: Vec<(usize, Vec<(String, Property)>)>,
) {
    use super::standard_property_sets as sps;
    let mut held: BTreeMap<(usize, String), usize> = BTreeMap::new();
    for (index, entity) in entities.iter().enumerate() {
        if let entities::IfcEntity::ElementPropertySet { element, set } = entity {
            held.entry((*element, set.name.clone())).or_insert(index);
        }
    }
    for (element, properties) in pending {
        let Some(entities::IfcEntity::BuildingElement {
            ifc_type,
            predefined_type,
            ..
        }) = entities.get(element)
        else {
            continue;
        };
        let (ifc_type, predefined_type) = (ifc_type.clone(), predefined_type.clone());
        for (set, property) in properties {
            if !sps::applies_to(&set, &ifc_type, predefined_type.as_deref()) {
                continue;
            }
            match held.get(&(element, set.clone())) {
                Some(&index) => {
                    if let Some(entities::IfcEntity::ElementPropertySet { set: held_set, .. }) =
                        entities.get_mut(index)
                    {
                        set_property(held_set, property);
                    }
                }
                None => {
                    held.insert((element, set.clone()), entities.len());
                    entities.push(entities::IfcEntity::ElementPropertySet {
                        element,
                        set: PropertySet {
                            name: set,
                            properties: vec![property],
                        },
                    });
                }
            }
        }
    }
}

/// Give each further piece of an element written in pieces (#331) every
/// further property set of the element, once they are final (#816): Revit's
/// export gives each piece all of them (Snowdon Towers' architectural model:
/// 19 elements in pieces, each `:2` piece with `Pset_SlabCommon` and the
/// rest).
fn add_piece_sets(
    entities: &mut Vec<entities::IfcEntity>,
    pieces_of: Vec<(usize, std::ops::Range<usize>)>,
) {
    for (element, pieces) in pieces_of {
        let sets: Vec<PropertySet> = entities
            .iter()
            .filter_map(|entity| match entity {
                entities::IfcEntity::ElementPropertySet {
                    element: owner,
                    set,
                } if *owner == element => Some(set.clone()),
                _ => None,
            })
            .collect();
        for piece in pieces {
            for set in &sets {
                entities.push(entities::IfcEntity::ElementPropertySet {
                    element: piece,
                    set: set.clone(),
                });
            }
        }
    }
}

/// A pipe's two ends, model feet, and the connector entry keys at each
/// (#847).
type PipeEnds = (
    [[f64; 3]; 2],
    [Vec<crate::partition_pipe_axes::EntryKey>; 2],
);

/// The end of pipe `id` that is its connector `index` (#847), and the unit
/// vector out of the pipe there: the one end whose connector entry keys name
/// that connector, either the pipe itself with `index` or the element and
/// connector `joined` says the pipe's `index` is joined to (RE-138). On the
/// RE1 models every end with a port in Revit's export is named this way
/// (Measure run 39370306743), and a port with no flow direction points out
/// of its pipe (19 of 19). `None` where no end, or both, name it.
fn pipe_port_point(
    id: u32,
    index: u32,
    (points, keys): &PipeEnds,
    joined: &HashMap<(u32, u32), Vec<(u32, u32)>>,
) -> Option<([f64; 3], [f64; 3])> {
    let own = u64::from(id);
    let partners = joined.get(&(id, index));
    let names = |keys: &[crate::partition_pipe_axes::EntryKey]| {
        keys.iter().any(|&(element, key_index)| {
            if element == own {
                key_index == index
            } else {
                partners.is_some_and(|partners| {
                    partners.iter().any(|&(other, other_index)| {
                        u64::from(other) == element && other_index == key_index
                    })
                })
            }
        })
    };
    let (end, other) = match [names(&keys[0]), names(&keys[1])] {
        [true, false] => (points[0], points[1]),
        [false, true] => (points[1], points[0]),
        _ => return None,
    };
    let out = [end[0] - other[0], end[1] - other[1], end[2] - other[2]];
    let length = (out[0] * out[0] + out[1] * out[1] + out[2] * out[2]).sqrt();
    (length > 0.0).then(|| (end, [out[0] / length, out[1] / length, out[2] / length]))
}

/// The property of `Pset_PipeSegmentOccurrence` holding a pipe's invert.
const INVERT_ELEVATION_PROPERTY: &str = "InvertElevation";
/// How far a pipe's two ends may differ, feet, along an axis it is taken to
/// lie across.
const PIPE_AXIS_TOLERANCE_FEET: f64 = 1e-6;

/// A pipe's invert in model height, feet (RE-157): the lower end of a
/// vertical pipe, or a horizontal pipe's axis less half its inner diameter.
/// Revit's export gives it as the height above the model's zero, not above
/// the pipe's storey: the BIM4LCA office's 12 vertical pipes, on a storey 55.5
/// m up, carry 55.32 m (RE-157 §6). `None` for a sloped pipe, or a horizontal
/// one whose inner diameter was not read: the diameters are read on the
/// release RE-157 measured the invert on (2025), and a 2024 pipe's axis alone
/// gives an invert Revit's export does not. A vertical pipe's needs no
/// diameter, so a 2024 vertical pipe has one (#869).
fn pipe_invert_height(fields: &[(String, InstanceField)]) -> Option<f64> {
    use crate::partition_schema_mvp as mvp;
    let pipe = mvp::pipe_body_from_fields(fields)?;
    let (start, end) = (pipe.start, pipe.end);
    let near = |a: f64, b: f64| (a - b).abs() <= PIPE_AXIS_TOLERANCE_FEET;
    let level = near(start[2], end[2]);
    if !level && near(start[0], end[0]) && near(start[1], end[1]) {
        return Some(start[2].min(end[2]));
    }
    if !level {
        return None;
    }
    let inner = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == mvp::PIPE_INNER_DIAMETER_FIELD => {
            Some(*value)
        }
        _ => None,
    })?;
    Some(start[2] - inner / 2.0)
}

/// The entities a room's space contains in Revit's export: on RE1
/// Architecture its furniture, sanitary terminals and equipment written as
/// proxies, and no wall, door, slab, covering or curtain wall. On the
/// BIM4LCA office (#792) Revit's export also puts its waste, fire
/// suppression and air terminals and electric appliances in their rooms'
/// spaces.
// Tracker: B63.
const ROOM_CONTENT_TYPES: [&str; 7] = [
    "IFCFURNITURE",
    "IFCSANITARYTERMINAL",
    "IFCBUILDINGELEMENTPROXY",
    "IFCWASTETERMINAL",
    "IFCFIRESUPPRESSIONTERMINAL",
    "IFCAIRTERMINAL",
    "IFCELECTRICAPPLIANCE",
];

/// A closed plan loop, model feet.
type PlanLoop = Vec<(f64, f64)>;

/// A space's plan outline and voids in model coordinates: its body's profile
/// moved to its placement.
fn space_plan_outline(
    location: &[f64; 3],
    rotation: f64,
    extrusion: &Extrusion,
) -> Option<(PlanLoop, Vec<PlanLoop>)> {
    let (sin, cos) = rotation.sin_cos();
    let place = |ring: &[(f64, f64)]| -> PlanLoop {
        ring.iter()
            .map(|(x, y)| {
                (
                    location[0] + x * cos - y * sin,
                    location[1] + x * sin + y * cos,
                )
            })
            .collect()
    };
    match &extrusion.profile_override {
        Some(entities::ProfileDef::ArbitraryClosed { points }) => Some((place(points), Vec::new())),
        Some(entities::ProfileDef::ArbitraryWithVoids { points, voids }) => Some((
            place(points),
            voids.iter().map(|ring| place(ring)).collect(),
        )),
        None => {
            let (w, d) = (extrusion.width_feet / 2.0, extrusion.depth_feet / 2.0);
            Some((place(&[(-w, -d), (w, -d), (w, d), (-w, d)]), Vec::new()))
        }
        Some(_) => None,
    }
}

/// Contain each room's furniture, fixtures and equipment in its space, as
/// Revit's export does: a family instance written as one of
/// [`ROOM_CONTENT_TYPES`], hosted by nothing, whose location lies in the plan
/// outline of a space on its storey, or, where it lies in none, whose plan box
/// overlaps exactly one, as a cabinet recessed into a room's wall does (RE1
/// Architecture's 375441, 0.08 ft outside its room). Everything else stays in
/// its storey.
// Tracker: B63.
pub(super) fn contain_in_spaces(entities: &mut Vec<entities::IfcEntity>) {
    use crate::element_record_plan_profiles::inside;
    let is_family_instance = |set: &Option<PropertySet>| {
        set.as_ref().is_some_and(|set| {
            set.properties.iter().any(|p| {
                p.name == "RevitClass"
                    && matches!(&p.value, PropertyValue::Text(class) if class == "FamilyInstance")
            })
        })
    };
    let mut spaces = Vec::new();
    for (index, entity) in entities.iter().enumerate() {
        let entities::IfcEntity::BuildingElement {
            ifc_type,
            storey_index: Some(storey),
            location_feet: Some(location),
            rotation_radians,
            extrusion: Some(extrusion),
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        // #763: a family instance written as a space holds nothing; the
        // furniture around it stays in its room.
        if ifc_type != "IFCSPACE" || is_family_instance(property_set) {
            continue;
        }
        if let Some((outer, voids)) =
            space_plan_outline(location, rotation_radians.unwrap_or(0.0), extrusion)
        {
            spaces.push((index, *storey, outer, voids));
        }
    }
    let mut contents: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, entity) in entities.iter().enumerate() {
        let entities::IfcEntity::BuildingElement {
            ifc_type,
            storey_index: Some(storey),
            location_feet: Some(location),
            rotation_radians,
            extrusion,
            host_element_index: None,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        if !ROOM_CONTENT_TYPES.contains(&ifc_type.as_str()) || !is_family_instance(property_set) {
            continue;
        }
        let holds = |outer: &PlanLoop, voids: &[PlanLoop], point: (f64, f64)| {
            inside(outer, point) && !voids.iter().any(|ring| inside(ring, point))
        };
        let point = (location[0], location[1]);
        let on_storey = || {
            spaces
                .iter()
                .filter(|(_, space_storey, ..)| space_storey == storey)
        };
        let mut space = on_storey()
            .find(|(_, _, outer, voids)| holds(outer, voids, point))
            .map(|(space, ..)| *space);
        if space.is_none() {
            let plan_box = extrusion
                .as_ref()
                .filter(|body| body.profile_override.is_none())
                .and_then(|body| {
                    space_plan_outline(location, rotation_radians.unwrap_or(0.0), body)
                })
                .map(|(outline, _)| outline);
            if let Some(plan_box) = plan_box {
                let overlapping: Vec<usize> = on_storey()
                    .filter(|(_, _, outer, voids)| {
                        plan_box.iter().any(|&corner| holds(outer, voids, corner))
                            || outer.iter().any(|&vertex| inside(&plan_box, vertex))
                    })
                    .map(|(space, ..)| *space)
                    .collect();
                if let [only] = overlapping[..] {
                    space = Some(only);
                }
            }
        }
        if let Some(space) = space {
            contents.entry(space).or_default().push(index);
        }
    }
    for (space, elements) in contents {
        entities.push(entities::IfcEntity::SpaceContainment { space, elements });
    }
}

/// A room's or an Area's `Pset_SpaceCommon` (RE-153, #763), as Revit's
/// export writes it: `Reference` (`IfcIdentifier`) its name and number, or
/// its name alone where it has no number (RE-196), and `IsExternal`
/// (RE-193). `None` when none was read. A room's Floor Finish is in
/// [`space_covering_property_set`].
fn space_common_property_set(decoded: &DecodedElement) -> Option<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    let family_space = space_by_export_value(decoded);
    if decoded.class != "Room" && decoded.class != "Area" && !family_space {
        return None;
    }
    let text = |wanted: &str| {
        decoded
            .fields
            .iter()
            .find_map(|(field, value)| match value {
                InstanceField::String(text) if field == wanted && !text.is_empty() => {
                    Some(text.clone())
                }
                _ => None,
            })
    };
    let mut properties = Vec::new();
    if family_space {
        // #763: a family instance written as a space is referred to by its
        // type's name, as Revit's export writes it.
        if let Some((_, type_name)) = family_and_type(decoded) {
            properties.push(Property {
                name: "Reference".into(),
                value: PropertyValue::Identifier(type_name),
            });
        }
    } else if let Some(name) = text("m_name") {
        // A room with no number is referred to by its name alone (RE-196).
        let reference = match text(mvp::ROOM_NUMBER_FIELD) {
            Some(number) => format!("{name} {number}"),
            None => name,
        };
        properties.push(Property {
            name: "Reference".into(),
            value: PropertyValue::Identifier(reference),
        });
    }
    if let Some(external) = is_external_value(decoded, "Pset_SpaceCommon") {
        properties.push(Property {
            name: "IsExternal".into(),
            value: PropertyValue::Boolean(external),
        });
    }
    (!properties.is_empty()).then(|| PropertySet {
        name: "Pset_SpaceCommon".into(),
        properties,
    })
}

/// A room's `Pset_SpaceCoveringRequirements`: `FloorCovering` (`IfcLabel`)
/// its Floor Finish (RE-153). IFC4 puts a space's coverings in this set;
/// IFC2X3's `Pset_SpaceCommon`, where Revit's IFC2X3 export of RE1
/// Architecture writes the same value, has them (#35). `None` when no
/// finish was read.
fn space_covering_property_set(decoded: &DecodedElement) -> Option<PropertySet> {
    use crate::partition_schema_mvp as mvp;
    if decoded.class != "Room" {
        return None;
    }
    let finish = decoded
        .fields
        .iter()
        .find_map(|(field, value)| match value {
            InstanceField::String(text)
                if field == mvp::ROOM_FLOOR_FINISH_FIELD && !text.is_empty() =>
            {
                Some(text.clone())
            }
            _ => None,
        })?;
    Some(PropertySet {
        name: "Pset_SpaceCoveringRequirements".into(),
        properties: vec![Property {
            name: "FloorCovering".into(),
            value: PropertyValue::Label(finish),
        }],
    })
}

/// The IFC entity mapping a per-element "IFC Export As" override
/// names, when the decoded element carries one,
/// [`category_map::lookup_export_override`] recognises the value, and
/// Revit's exporter of the file's release acts on it for this element
/// ([`category_map::export_override_applies`], RE-188).
fn ifc_export_override(decoded: &DecodedElement) -> Option<&'static category_map::Mapping> {
    let release = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. }
            if name == crate::partition_schema_mvp::IFC_EXPORT_RELEASE_FIELD =>
        {
            u32::try_from(*value).ok()
        }
        _ => None,
    });
    if !category_map::export_override_applies(&decoded.class, release) {
        return None;
    }
    decoded
        .fields
        .iter()
        .find_map(|(name, value)| match (name.as_str(), value) {
            (crate::partition_schema_mvp::IFC_EXPORT_AS_FIELD, InstanceField::String(text)) => {
                category_map::lookup_export_override(text)
            }
            _ => None,
        })
        // #763: a room, area or space is a space already; it keeps its own
        // mapping and predefined type.
        .filter(|target| {
            target.ifc_type != "IFCSPACE"
                || category_map::lookup(&decoded.class).is_none_or(|own| own.ifc_type != "IFCSPACE")
        })
}

/// Whether the element is written as `IfcSpace` through its own "Export to
/// IFC As" value (#763), a family instance rather than a room.
fn space_by_export_value(decoded: &DecodedElement) -> bool {
    ifc_export_override(decoded).is_some_and(|target| target.ifc_type == "IFCSPACE")
}

/// The "Export to IFC As" value of an element the export writes as another
/// entity than the value names (#792): a value no target lists, or one
/// Revit's exporter does not act on for this element (RE-188). `None` for
/// an element the export does not write, or writes as its value says.
pub(crate) fn unapplied_export_value(decoded: &DecodedElement) -> Option<&str> {
    if is_misleading_proxy_class(&decoded.class)
        || matches!(
            decoded.class.as_str(),
            "Material" | "ArcWall" | "ArcWallRectOpening"
        )
    {
        return None;
    }
    let mapping = category_map::lookup(&decoded.class)?;
    let value = decoded
        .fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::String(text)
                if name == crate::partition_schema_mvp::IFC_EXPORT_AS_FIELD =>
            {
                Some(text.as_str())
            }
            _ => None,
        })?;
    let written = ifc_export_override(decoded).unwrap_or(mapping).ifc_type;
    let entity = value.split('.').next().unwrap_or(value);
    (!written.eq_ignore_ascii_case(entity)).then_some(value)
}

/// True when this element came from a partition element record.
fn is_partition_element_record(decoded: &DecodedElement) -> bool {
    decoded.fields.iter().any(|(name, value)| {
        matches!(
            (name.as_str(), value),
            ("m_source", InstanceField::String(source))
                if source == "partition_element_record"
        )
    })
}

/// Placement + body from a partition element record (#204 columns,
/// #211 walls / doors / windows).
///
/// The record carries the element's model bounding box, so the
/// placement is its plan centre at the box base and the body is the
/// box itself — an envelope, not a recovered family profile or a
/// wall location curve. The property set says so rather than letting
/// a consumer read the rectangle as a modelled section.
/// What an element record gives an element: its placement, its body and
/// the properties saying where they came from.
struct RecordGeometry {
    location: [f64; 3],
    rotation: Option<f64>,
    body: Extrusion,
    /// A body the extrusion cannot express (a sloped beam, RE-49). `body`
    /// stays the record box for consumers that read only extrusions.
    solid: Option<entities::SolidShape>,
    properties: PropertySet,
    pieces: Vec<Extrusion>,
    /// Each tagged void of a floor's sketch (RE-151): the piece it is in, its
    /// tag, its outline in the body's frame and the floor's thickness.
    void_openings: Vec<VoidBody>,
}

/// A tagged void of a floor's sketch (RE-151): the piece it is in, its tag,
/// its outline in the body's frame, and the floor's thickness, feet.
type VoidBody = (usize, u32, Vec<(f64, f64)>, f64);

/// A wall's elevation-profile opening: its tag, its outline along and
/// up the wall, the wall's axis turn in its placement, the sketch plane's
/// offset to the axis's left, and the wall's thickness.
// Tracker: B55.
type ProfileOpening = (u32, Vec<(f64, f64)>, f64, f64, f64);

/// `BodyHeightSource` of a column drawn as its body after geometry joins: the
/// box at id-48 of its geometry node (RE-178).
pub const COLUMN_CUT_BODY_HEIGHT_SOURCE: &str = "partition_geometry_node_after_joins";

/// `BodyHeightSource` of a monolithic stair landing drawn as its slab: from
/// its top (`StairsLanding.m_baseElevation`) down its landing type's
/// thickness (`StairsLandingType.m_thickness`, #824).
pub const LANDING_SLAB_BODY_HEIGHT_SOURCE: &str = "native_landing_top_and_type_thickness";

/// `BodyHeightSource` of a wall whose top is cut at the underside of the
/// floor, roof or footing its top join or a cut pair names, where that
/// element covers the whole of the wall's plan (#1067, RE-267).
pub const WALL_TOP_COVER_BODY_HEIGHT_SOURCE: &str = "wall_top_cut_at_covering_floor";

/// `BodySource` of a body that is the element record's bounding box, where
/// no carrier refines it (#409).
pub const RECORD_BBOX_BODY_SOURCE: &str = "partition_element_record_bbox";

/// `BodySource` of a family instance turned off the model's axes, drawn as
/// the rectangle at its angle whose box is its record box (RE-87).
pub const INSTANCE_TURNED_BODY_SOURCE: &str = "partition_family_instance_turned_box";

/// `BodySource` of a curtain mullion or panel turned or tilted off the
/// model's axes, drawn as the box along its own axes (RE-106).
pub const INSTANCE_ORIENTED_BODY_SOURCE: &str = "partition_family_instance_oriented_box";

/// Below this, `|det|` of the axes' absolute components leaves the box along
/// them undetermined by the record box (RE-106).
const ORIENTED_BOX_MIN_DET: f64 = 0.1;

/// The half-extents along `axes` (X, Y and Z, each in model coordinates) of
/// the box whose axis-aligned box is `size` (RE-106): the box centred on the
/// record box's centre whose extent along each model axis, `sum |a_i| e_i`,
/// is the record box's. `None` where the axes leave it undetermined or a
/// half-extent would not be positive.
pub(crate) fn oriented_box(
    axes: [[f64; 3]; 3],
    size: [f64; 3],
) -> Option<([[f64; 3]; 3], [f64; 3])> {
    let m = |row: usize, col: usize| axes[col][row].abs();
    let det = m(0, 0) * (m(1, 1) * m(2, 2) - m(1, 2) * m(2, 1))
        - m(0, 1) * (m(1, 0) * m(2, 2) - m(1, 2) * m(2, 0))
        + m(0, 2) * (m(1, 0) * m(2, 1) - m(1, 1) * m(2, 0));
    if !det.is_finite() || det.abs() < ORIENTED_BOX_MIN_DET {
        return None;
    }
    let h = size.map(|s| s / 2.0);
    // Cramer's rule on sum_i |axis_i[row]| e_i = h[row].
    let solve = |col: usize| {
        let pick = |row: usize, c: usize| if c == col { h[row] } else { m(row, c) };
        (pick(0, 0) * (pick(1, 1) * pick(2, 2) - pick(1, 2) * pick(2, 1))
            - pick(0, 1) * (pick(1, 0) * pick(2, 2) - pick(1, 2) * pick(2, 0))
            + pick(0, 2) * (pick(1, 0) * pick(2, 1) - pick(1, 1) * pick(2, 0)))
            / det
    };
    let half = [solve(0), solve(1), solve(2)];
    half.iter()
        .all(|e| e.is_finite() && *e > 0.0)
        .then_some((axes, half))
}

/// The box along `axes` with half-extents `half`, centred on the record
/// box's centre, `height / 2` above the element's placement: its X by Y
/// section extruded along its Z.
fn oriented_box_solid(axes: [[f64; 3]; 3], half: [f64; 3], height: f64) -> entities::SolidShape {
    let [x, _, z] = axes;
    let centre = [0.0, 0.0, height / 2.0];
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::Rectangle {
            width_feet: 2.0 * half[0],
            depth_feet: 2.0 * half[1],
        },
        origin_feet: [
            centre[0] - half[2] * z[0],
            centre[1] - half[2] * z[1],
            centre[2] - half[2] * z[2],
        ],
        axis: z,
        ref_direction: x,
        depth_feet: 2.0 * half[2],
    }
}

/// Properties holding a turned family instance's plan origin from its
/// transform, model feet (RE-87, RE-89).
pub const INSTANCE_ORIGIN_PROPERTIES: [&str; 2] = ["InstanceOriginX", "InstanceOriginY"];

/// Properties holding a window's opening from its type and transform
/// (RE-93, `crate::partition_schema_mvp::FillerOpening`): the plan point
/// it is centred on and the direction of its width, model feet and a unit
/// vector, then its base elevation, width and height, feet.
pub const FILLER_OPENING_PROPERTIES: [&str; 7] = [
    "OpeningCentreX",
    "OpeningCentreY",
    "OpeningAxisX",
    "OpeningAxisY",
    "OpeningBaseElevation",
    "OpeningWidth",
    "OpeningHeight",
];

/// Properties holding a door's Width and Height from its type (RE-187,
/// `crate::partition_schema_mvp::DOOR_TYPE_SIZE_FIELDS`), feet.
pub const DOOR_TYPE_SIZE_PROPERTIES: [&str; 2] = ["TypeWidth", "TypeHeight"];

/// Properties holding the extent along X and along Z of the outline a
/// door's or window's symbol cuts in its host (RE-215,
/// `crate::partition_schema_mvp::CUT_LOOP_SIZE_FIELDS`), feet.
pub const CUT_LOOP_SIZE_PROPERTIES: [&str; 2] = ["CutWidth", "CutHeight"];

/// Properties holding the extents along X, Y and Z of the solid a door's or
/// window's symbol cuts where it cuts no loop (RE-225,
/// `crate::partition_schema_mvp::CUT_SOLID_SIZE_FIELDS`), feet.
pub const CUT_SOLID_SIZE_PROPERTIES: [&str; 3] =
    ["CutSolidWidth", "CutSolidDepth", "CutSolidHeight"];

/// Properties holding a door's or window's record box extent along its own
/// X axis and its height, where its symbol cuts no loop (#810,
/// `crate::partition_schema_mvp::INSTANCE_BOX_SIZE_FIELDS`), feet.
pub const INSTANCE_BOX_SIZE_PROPERTIES: [&str; 2] = ["BoxWidth", "BoxHeight"];

/// Classes whose body another rule draws, which a family instance's turn
/// leaves alone: columns (their type's section), beams (their line), and
/// curtain-wall panels and mullions (their grid).
const TURNED_BOX_EXCLUDED_CLASSES: &[&str] = &[
    "Column",
    "StructuralColumn",
    "StructuralFraming",
    "CurtainWallPanel",
    "CurtainWallMullion",
];

/// The rectangle at the plan angle of `x_axis` whose axis-aligned box is
/// `width` × `depth` (RE-87): `(angle, along X, along Y)`. A rectangle
/// `w` × `d` turned by the angle has a box `w|cos| + d|sin|` wide and
/// `w|sin| + d|cos|` deep, which gives `w` and `d` back unless the angle is
/// near 45 degrees. `None` there, or where either side would not be
/// positive.
fn turned_box(x_axis: [f64; 2], width: f64, depth: f64) -> Option<(f64, f64, f64)> {
    let length = x_axis[0].hypot(x_axis[1]);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let (c, s) = ((x_axis[0] / length).abs(), (x_axis[1] / length).abs());
    let det = c * c - s * s;
    if det.abs() < 0.1 {
        return None;
    }
    let along = (width * c - depth * s) / det;
    let across = (depth * c - width * s) / det;
    (along > 1e-6 && across > 1e-6).then(|| (x_axis[1].atan2(x_axis[0]), along, across))
}

/// `BodySource` of a beam whose body runs along its location line (RE-49).
pub const BEAM_AXIS_BODY_SOURCE: &str = "partition_beam_axis";

/// A sloped beam's body: its section swept along its centreline, with the
/// section's depth kept in the line's vertical plane. `location` is the
/// element's placement, which the directrix is relative to.
fn beam_swept_solid(
    beam: &crate::partition_beam_axes::BeamBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let (a, b) = beam.centreline();
    let local = |p: [f64; 3]| [p[0] - location[0], p[1] - location[1], p[2] - location[2]];
    entities::SolidShape::SweptPath {
        // The fixed reference sets the profile's X axis, so X is the depth.
        profile: entities::ProfileDef::Rectangle {
            width_feet: beam.depth_feet,
            depth_feet: beam.width_feet,
        },
        directrix_points_feet: vec![local(a), local(b)],
        fixed_reference: [0.0, 0.0, 1.0],
    }
}

/// `BodySource` of a pipe drawn as the cylinder its connector entries and
/// record box make (RE-131).
pub const PIPE_AXIS_BODY_SOURCE: &str = "partition_pipe_axis";

/// A pipe's body (RE-131): its circle extruded from one end to the other.
/// `location` is the element's placement, which the extrusion's own placement
/// is relative to.
fn pipe_cylinder_solid(
    pipe: &crate::partition_pipe_axes::PipeBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let length = pipe.length_feet();
    let axis = [
        (pipe.end[0] - pipe.start[0]) / length,
        (pipe.end[1] - pipe.start[1]) / length,
        (pipe.end[2] - pipe.start[2]) / length,
    ];
    // The circle has no orientation, so the profile's X axis is any direction
    // square to the axis: the axis crossed with the model axis it is least
    // along.
    let least = (0..3)
        .min_by(|&a, &b| axis[a].abs().total_cmp(&axis[b].abs()))
        .unwrap_or(0);
    let mut across = [0.0; 3];
    across[least] = 1.0;
    let mut x = [
        axis[1] * across[2] - axis[2] * across[1],
        axis[2] * across[0] - axis[0] * across[2],
        axis[0] * across[1] - axis[1] * across[0],
    ];
    let norm = (x[0] * x[0] + x[1] * x[1] + x[2] * x[2]).sqrt().max(1e-12);
    for value in &mut x {
        *value /= norm;
    }
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::Circle {
            radius_feet: pipe.radius_feet,
        },
        origin_feet: [
            pipe.start[0] - location[0],
            pipe.start[1] - location[1],
            pipe.start[2] - location[2],
        ],
        axis,
        ref_direction: x,
        depth_feet: length,
    }
}

/// `BodySource` of a structural column drawn as its type's I section
/// (RE-104).
pub const COLUMN_I_SECTION_BODY_SOURCE: &str = "family_type_i_section";

/// `BodySource` of a column drawn as its type's square hollow section,
/// rounded corners and all, from its family's parameters (W6).
pub const COLUMN_HOLLOW_SECTION_BODY_SOURCE: &str = "family_type_hollow_section";

/// How closely a hollow-section column's record box must equal its
/// section's outside to be drawn as it, feet.
pub const HOLLOW_SECTION_BOX_TOLERANCE_FEET: f64 = 1e-3;

/// A square hollow section's profile, centred: the outside and the inside,
/// each four lines and four quarter arcs about the corners' common centres,
/// as Revit's export writes one (`IfcIndexedPolyCurve`s, W6).
fn hollow_section_profile(
    section: &crate::partition_type_parameters::HollowSection,
) -> entities::ProfileDef {
    use entities::{CurveSegment, IndexedCurve};
    let (x, y) = (section.width_feet / 2.0, section.height_feet / 2.0);
    let ring = |half_x: f64, half_y: f64, radius: f64, clockwise: bool| {
        let (cx, cy) = (half_x - radius, half_y - radius);
        let diagonal = radius * std::f64::consts::FRAC_1_SQRT_2;
        // Counter-clockwise from the bottom edge's left end.
        let mut points = vec![
            (-cx, -half_y),
            (cx, -half_y),
            (cx + diagonal, -cy - diagonal),
            (half_x, -cy),
            (half_x, cy),
            (cx + diagonal, cy + diagonal),
            (cx, half_y),
            (-cx, half_y),
            (-cx - diagonal, cy + diagonal),
            (-half_x, cy),
            (-half_x, -cy),
            (-cx - diagonal, -cy - diagonal),
        ];
        if clockwise {
            points.reverse();
        }
        let segments = if clockwise {
            // The same curves, reversed: point i is the counter-clockwise
            // ring's point 11 - i.
            vec![
                CurveSegment::Line([1, 2]),
                CurveSegment::Arc([2, 3, 4]),
                CurveSegment::Line([4, 5]),
                CurveSegment::Arc([5, 6, 7]),
                CurveSegment::Line([7, 8]),
                CurveSegment::Arc([8, 9, 10]),
                CurveSegment::Line([10, 11]),
                CurveSegment::Arc([11, 0, 1]),
            ]
        } else {
            vec![
                CurveSegment::Line([0, 1]),
                CurveSegment::Arc([1, 2, 3]),
                CurveSegment::Line([3, 4]),
                CurveSegment::Arc([4, 5, 6]),
                CurveSegment::Line([6, 7]),
                CurveSegment::Arc([7, 8, 9]),
                CurveSegment::Line([9, 10]),
                CurveSegment::Arc([10, 11, 0]),
            ]
        };
        IndexedCurve { points, segments }
    };
    let t = section.wall_feet;
    entities::ProfileDef::IndexedPolyCurveWithVoids {
        outer: ring(x, y, section.outer_radius_feet, false),
        voids: vec![ring(x - t, y - t, section.outer_radius_feet - t, true)],
    }
}

/// How closely an I-section column's section, turned to its X axis, must
/// fill its record box in plan (RE-104), feet.
pub const COLUMN_SECTION_TOLERANCE_FEET: f64 = 0.01;

/// An upright structural column's I section and the plan angle of its X
/// axis, which its flanges run along (RE-104): only where the section turned
/// by that angle is `box_width` × `box_depth` in plan, as its record box is.
fn column_i_section(
    fields: &[(String, InstanceField)],
    box_width: f64,
    box_depth: f64,
) -> Option<(f64, crate::partition_type_parameters::ISection)> {
    let section = crate::partition_schema_mvp::beam_i_section_from_fields(fields)?;
    let [ax, ay] = crate::partition_schema_mvp::COLUMN_I_AXIS_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    let (ax, ay) = (ax?, ay?);
    let length = ax.hypot(ay);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let (c, s) = ((ax / length).abs(), (ay / length).abs());
    let (w, d) = (section.width_feet, section.depth_feet);
    let fits = (w * c + d * s - box_width).abs() <= COLUMN_SECTION_TOLERANCE_FEET
        && (w * s + d * c - box_depth).abs() <= COLUMN_SECTION_TOLERANCE_FEET;
    fits.then(|| (ay.atan2(ax), section))
}

/// An I-section beam's body (RE-103): its type's I extruded along its
/// centreline from one end, the profile's X axis level across the line, so
/// the flanges run across and the web stands upright in the line's vertical
/// plane. `location` is the element's placement. An extrusion fixes the
/// profile's axes by its placement; a fixed-reference sweep would leave
/// them to how a reader projects the reference.
fn beam_i_solid(
    beam: &crate::partition_beam_axes::BeamBody,
    section: &crate::partition_type_parameters::ISection,
    location: [f64; 3],
) -> entities::SolidShape {
    let (a, _) = beam.centreline();
    let d = beam.direction;
    let plan = d[0].hypot(d[1]).max(1e-12);
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::IShape {
            overall_width_feet: section.width_feet,
            overall_depth_feet: section.depth_feet,
            web_thickness_feet: section.web_feet,
            flange_thickness_feet: section.flange_feet,
        },
        origin_feet: [a[0] - location[0], a[1] - location[1], a[2] - location[2]],
        axis: d,
        ref_direction: [-d[1] / plan, d[0] / plan, 0.0],
        depth_feet: beam.length_feet,
    }
}

/// `BodySource` of a stair run drawn as its treads and risers (RE-52).
pub const STAIR_RUN_BODY_SOURCE: &str = "partition_stair_run_sketch";
/// `BodySource` of a spiral stair run drawn as its treads, from its native
/// records (#357).
pub const SPIRAL_RUN_BODY_SOURCE: &str = "native_spiral_run_treads";

/// `BodySource` of a landing drawn as the one tread it is: its outline's
/// curve loop, arcs and all, one run tread thick (#824).
pub const LANDING_TREAD_BODY_SOURCE: &str = "native_landing_tread";

/// `BodySource` of a monolithic stair landing drawn as its slab, its outline
/// from its top down its type's thickness (#824).
pub const LANDING_SLAB_BODY_SOURCE: &str = "partition_landing_slab";

/// `BodySource` of a shed roof drawn along its slope (RE-56).
pub const ROOF_SLOPE_BODY_SOURCE: &str = "partition_roof_slope";

/// `BodySource` of a roof by extrusion drawn as its section (RE-211).
pub const ROOF_EXTRUSION_BODY_SOURCE: &str = "partition_roof_extrusion";

/// A roof by extrusion's body (RE-211): its section
/// ([`crate::partition_schema_mvp::RoofExtrusion`]) run along its axis from
/// `start` to `end`, as a closed faceted brep in the element's frame at
/// `origin`. The two caps are the section, which rises along `u` on its top
/// and its underside alike, cut into triangles between the two; the sides
/// join the caps along the section's outline. Every triangle faces out.
fn roof_extrusion_solid(
    extrusion: &crate::partition_schema_mvp::RoofExtrusion,
    origin: [f64; 3],
    (start, end): (f64, f64),
) -> Option<entities::SolidShape> {
    let (top, bottom) = (&extrusion.top, &extrusion.bottom);
    if top.len() < 2 || bottom.len() < 2 || !start.is_finite() || !end.is_finite() || end <= start {
        return None;
    }
    let axis = extrusion.axis;
    let u_axis = 1 - axis;
    let place = |(u, v): (f64, f64), along: f64| {
        let mut point = [0.0; 3];
        point[axis] = along - origin[axis];
        point[u_axis] = u - origin[u_axis];
        point[2] = v - origin[2];
        point
    };
    // The outline: the top along `u`, then the underside back.
    let outline: Vec<(f64, f64)> = top.iter().chain(bottom.iter().rev()).copied().collect();
    let n = outline.len();
    let mut vertices: Vec<[f64; 3]> = outline.iter().map(|&p| place(p, start)).collect();
    vertices.extend(outline.iter().map(|&p| place(p, end)));
    // Cap triangles: the outline cut into ears, since the section may be
    // concave, kept only where they cover its area exactly.
    let points: Vec<[f64; 2]> = outline.iter().map(|&(u, v)| [u, v]).collect();
    let no_holes: [u32; 0] = [];
    let mut flat: Vec<u32> = Vec::new();
    earcut::Earcut::new().earcut(points.iter().copied(), &no_holes, &mut flat);
    if flat.len() != 3 * (n - 2) || flat.iter().any(|&k| k as usize >= n) {
        return None;
    }
    let twice_area = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    // Relative to the outline's first point, so large model coordinates do
    // not cancel.
    let origin = points[0];
    let section = (0..n)
        .map(|k| {
            let (a, b) = (points[k], points[(k + 1) % n]);
            let (a, b) = (
                [a[0] - origin[0], a[1] - origin[1]],
                [b[0] - origin[0], b[1] - origin[1]],
            );
            a[0] * b[1] - b[0] * a[1]
        })
        .sum::<f64>()
        .abs();
    let covered: f64 = flat
        .chunks_exact(3)
        .map(|t| {
            twice_area(
                points[t[0] as usize],
                points[t[1] as usize],
                points[t[2] as usize],
            )
            .abs()
        })
        .sum();
    if !(section.is_finite() && section > 0.0)
        || (covered - section).abs() > 1e-9 * section.max(1.0)
    {
        return None;
    }
    let cap: Vec<[usize; 3]> = flat
        .chunks_exact(3)
        .map(|t| [t[0] as usize, t[1] as usize, t[2] as usize])
        .collect();
    let mut triangles: Vec<[usize; 3]> = Vec::new();
    // The outward side of a triangle: the caps face away from each other,
    // a side away from the section (its outline runs clockwise across the
    // plane, top first, so an edge's outward normal is on its left).
    let mut outward: Vec<[f64; 3]> = Vec::new();
    let mut axis_vector = [0.0; 3];
    axis_vector[axis] = 1.0;
    for &[a, b, c] in &cap {
        triangles.push([a, b, c]);
        outward.push(axis_vector.map(|v| -v));
        triangles.push([a + n, b + n, c + n]);
        outward.push(axis_vector);
    }
    for k in 0..n {
        let next = (k + 1) % n;
        let (du, dv) = (
            outline[next].0 - outline[k].0,
            outline[next].1 - outline[k].1,
        );
        let mut normal = [0.0; 3];
        normal[u_axis] = -dv;
        normal[2] = du;
        triangles.push([k, next, next + n]);
        outward.push(normal);
        triangles.push([k, next + n, k + n]);
        outward.push(normal);
    }
    let triangles = triangles
        .into_iter()
        .zip(outward)
        .map(|([a, b, c], out)| {
            let (pa, pb, pc) = (vertices[a], vertices[b], vertices[c]);
            let (e1, e2) = (
                [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]],
                [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]],
            );
            let normal = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            let facing = normal[0] * out[0] + normal[1] * out[1] + normal[2] * out[2];
            let [a, b, c] = if facing < 0.0 { [a, c, b] } else { [a, b, c] };
            Some(entities::BrepTriangle(
                u32::try_from(a).ok()?,
                u32::try_from(b).ok()?,
                u32::try_from(c).ok()?,
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(entities::SolidShape::FacetedBrep {
        vertices_feet: vertices,
        triangles,
    })
}

/// How closely a shed roof's rise and thickness must reproduce its record
/// box's height for its slope to be drawn, feet.
const ROOF_SLOPE_CLOSURE_FEET: f64 = 1e-4;

/// A shed roof's body (RE-56): its plan outline, rising from the edge that
/// defines its slope at that slope, with its type's thickness measured
/// square to the slope and upright sides. `outer` and `inner` are the
/// outline relative to the element's placement at `(x, y)` and the record
/// box's base, and `height` is the box's height.
///
/// `None` unless the outline lies on one side of the edge and the rise
/// across it plus the thickness's vertical extent reproduces `height`
/// within [`ROOF_SLOPE_CLOSURE_FEET`], so the body is exactly as tall as
/// the one Revit recorded.
fn roof_slope_solid(
    slope: crate::partition_schema_mvp::RoofSlope,
    [x, y]: [f64; 2],
    outer: &[(f64, f64)],
    inner: &[Vec<(f64, f64)>],
    height: f64,
) -> Option<entities::SolidShape> {
    let [sx, sy] = slope.edge_start;
    let [ex, ey] = slope.edge_end;
    let length = (ex - sx).hypot(ey - sy);
    let angle = slope.angle_radians;
    if !(length > 1e-9 && angle.is_finite() && angle > 0.0 && angle < std::f64::consts::FRAC_PI_2) {
        return None;
    }
    let (ux, uy) = ((ex - sx) / length, (ey - sy) / length);
    // Distance from the edge, local coordinates, positive into the roof.
    let offset = |(px, py): (f64, f64)| (px + x - sx) * -uy + (py + y - sy) * ux;
    let (low, high) = outer
        .iter()
        .map(|&p| offset(p))
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), d| {
            (lo.min(d), hi.max(d))
        });
    let (sign, far) = if high.abs() >= low.abs() {
        (1.0, high)
    } else {
        (-1.0, -low)
    };
    let near = if sign > 0.0 { low } else { -high };
    if near < -1e-6 {
        return None;
    }
    let rise = angle.tan();
    let vertical = slope.thickness_feet / angle.cos();
    if (far * rise + vertical - height).abs() > ROOF_SLOPE_CLOSURE_FEET {
        return None;
    }
    let mesh = super::body_geometry::sloped_slab_mesh(
        outer,
        inner,
        |p| sign * offset(p) * rise,
        vertical,
    )?;
    Some(entities::SolidShape::FacetedBrep {
        vertices_feet: mesh.vertices,
        triangles: mesh
            .triangles
            .into_iter()
            .map(|[a, b, c]| entities::BrepTriangle(a, b, c))
            .collect(),
    })
}

/// Whether `p` lies inside the ring `ring` (even-odd rule).
fn point_in_ring((px, py): (f64, f64), ring: &[(f64, f64)]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let ((ax, ay), (bx, by)) = (ring[i], ring[(i + 1) % ring.len()]);
        if (ay > py) != (by > py) && px < ax + (py - ay) * (bx - ax) / (by - ay) {
            inside = !inside;
        }
    }
    inside
}

/// A gable or hip roof's body (RE-199): each defining edge's plane rises
/// from it into the roof at its angle (raised by the edge's plate offset).
/// A plane counts within its edge's straight-skeleton wedge: on the edge's
/// side, between the lines where it meets its neighbouring defining edges'
/// planes, which converge at a hip and part at a valley. The roof's top is
/// the lowest counting plane at each point, so hips and valleys both come
/// out right over convex and L-shaped footprints. The planes' tops meet the
/// outline at one eave height, `T / cos θ` of the shallowest edge; each
/// plane's underside lies its own `T / cos θ` below its top, never below
/// the record box's base.
///
/// With `rafter_cut`, a two-cut rafter's fascia depth `F`, the eave height
/// is `F` instead, and each plane is cut level at the base raised by its
/// plate offset: `F = 0` (the default) sets the plain roof down by the
/// thickness's vertical extent, `F = T / cos θ` is the plain roof. The
/// plumb and square two-cuts coincide at `F = 0`.
///
/// Revit's export of the roofs of #356's `R2025_Models.zip` has this shape:
/// sheds, a gable, hips at 30° and at 15° and 35°, two-cuts with plate
/// offsets, and L-shaped hips with a raised edge and a hole. `None` unless the
/// highest point of the top reproduces `height` within
/// [`ROOF_SLOPE_CLOSURE_FEET`].
fn roof_planes_solid(
    slopes: &[crate::partition_schema_mvp::RoofSlope],
    [x, y]: [f64; 2],
    outer: &[(f64, f64)],
    inner: &[Vec<(f64, f64)>],
    height: f64,
    rafter_cut: Option<f64>,
) -> Option<entities::SolidShape> {
    if outer.len() < 3 {
        return None;
    }
    let mut planes: Vec<(super::body_geometry::PlanPlane, f64)> = Vec::new();
    let mut domains: Vec<Vec<super::body_geometry::PlanPlane>> = Vec::new();
    // Each edge's ends and a point just inside its middle, local feet.
    let mut ends: Vec<[(f64, f64); 2]> = Vec::new();
    let mut probes: Vec<(f64, f64)> = Vec::new();
    for slope in slopes {
        let [sx, sy] = slope.edge_start;
        let [ex, ey] = slope.edge_end;
        let length = (ex - sx).hypot(ey - sy);
        let angle = slope.angle_radians;
        if !(length > 1e-9
            && angle.is_finite()
            && angle > 0.0
            && angle < std::f64::consts::FRAC_PI_2
            && slope.thickness_feet > 0.0)
        {
            return None;
        }
        let (ux, uy) = ((ex - sx) / length, (ey - sy) / length);
        // Distance from the edge in local coordinates: -uy·px + ux·py + c.
        let c = (x - sx) * -uy + (y - sy) * ux;
        // The plane rises into the roof: the side of the edge that a point
        // just inside its midpoint lies on. Over a non-convex footprint the
        // outline crosses the edge's line, and the plane's domain ends there.
        let mid = ((sx + ex) / 2.0 - x, (sy + ey) / 2.0 - y);
        let probe = (mid.0 - uy * 1e-3, mid.1 + ux * 1e-3);
        let sign = if point_in_ring(probe, outer) {
            1.0
        } else {
            -1.0
        };
        let rise = sign * angle.tan();
        domains.push(vec![[-uy * sign, ux * sign, c * sign]]);
        ends.push([(sx - x, sy - y), (ex - x, ey - y)]);
        probes.push((mid.0 - uy * sign * 1e-3, mid.1 + ux * sign * 1e-3));
        planes.push((
            [-uy * rise, ux * rise, c * rise + slope.plate_offset_feet],
            slope.thickness_feet / angle.cos(),
        ));
    }
    // The wedge of each edge: at each end it shares with another defining
    // edge, the side of the line where their planes meet that holds the
    // edge's own middle.
    let at = |[a, b, c]: super::body_geometry::PlanPlane, (px, py): (f64, f64)| a * px + b * py + c;
    for i in 0..planes.len() {
        for j in 0..planes.len() {
            let shared = i != j
                && ends[i].iter().any(|&(px, py)| {
                    ends[j]
                        .iter()
                        .any(|&(qx, qy)| (px - qx).abs() < 1e-6 && (py - qy).abs() < 1e-6)
                });
            if !shared {
                continue;
            }
            let (pi, pj) = (planes[i].0, planes[j].0);
            let mut line = [pi[0] - pj[0], pi[1] - pj[1], pi[2] - pj[2]];
            if line[0].hypot(line[1]) < 1e-12 {
                continue;
            }
            if at(line, probes[i]) < 0.0 {
                line = line.map(|v| -v);
            }
            domains[i].push(line);
        }
    }
    let eave = if let Some(fascia) = rafter_cut {
        fascia
    } else {
        planes
            .iter()
            .map(|(_, vertical)| *vertical)
            .fold(f64::INFINITY, f64::min)
    };
    let tops: Vec<_> = planes
        .iter()
        .map(|([a, b, c], _)| [*a, *b, c + eave])
        .collect();
    // A two-cut plane is cut level at its own eave's top, the base raised by
    // its plate offset; otherwise at the base.
    let undersides: Vec<_> = planes
        .iter()
        .zip(slopes)
        .map(|(([a, b, c], vertical), slope)| {
            let floor = if rafter_cut.is_some() {
                slope.plate_offset_feet
            } else {
                0.0
            };
            ([*a, *b, c + eave - vertical], floor)
        })
        .collect();
    let mesh =
        super::body_geometry::envelope_slab_mesh(outer, inner, &tops, &undersides, &domains)?;
    let highest = mesh
        .vertices
        .iter()
        .map(|v| v[2])
        .fold(f64::NEG_INFINITY, f64::max);
    if (highest - height).abs() > ROOF_SLOPE_CLOSURE_FEET {
        return None;
    }
    Some(entities::SolidShape::FacetedBrep {
        vertices_feet: mesh.vertices,
        triangles: mesh
            .triangles
            .into_iter()
            .map(|[a, b, c]| entities::BrepTriangle(a, b, c))
            .collect(),
    })
}

/// `BodySource` of a wall drawn its type's thickness either side of its
/// centreline (RE-54).
pub const WALL_CENTRELINE_BODY_SOURCE: &str = "partition_wall_centreline";

/// `BodySource` of a tapered wall drawn as its cross-section along its
/// centreline (RE-86).
pub const WALL_TAPERED_BODY_SOURCE: &str = "partition_wall_tapered_section";

/// `BodySource` of a wall drawn from its centreline whose layers end where
/// its layered butt joins put them (RE-71).
pub const WALL_LAYER_JOIN_BODY_SOURCE: &str = "partition_wall_centreline_layer_joins";

/// `BodySource` of a curved wall drawn as the ring sector its arc makes
/// (RE-75).
pub const WALL_ARC_BODY_SOURCE: &str = "partition_wall_arc";

/// How far a curved wall's drawn outline may stray inside its arcs, feet.
pub const WALL_ARC_TOLERANCE_FEET: f64 = 0.0005;

/// A curved wall's plan outline: the ring sector between its arc's radius
/// less and plus half its thickness, from its start angle to its end angle,
/// each arc drawn as chords that stray at most [`WALL_ARC_TOLERANCE_FEET`]
/// inside it. `None` for a degenerate arc or one thicker than its radius.
fn wall_arc_outline(arc: &crate::partition_schema_mvp::WallArc) -> Option<Vec<(f64, f64)>> {
    let half = arc.thickness_feet * 0.5;
    let sweep = arc.angles[1] - arc.angles[0];
    let outer = arc.radius + half;
    if !(sweep.is_finite() && sweep > 0.0 && sweep <= std::f64::consts::TAU)
        || !(half.is_finite() && half > 0.0 && arc.radius > half)
    {
        return None;
    }
    let step = 2.0
        * (1.0 - WALL_ARC_TOLERANCE_FEET / outer)
            .clamp(-1.0, 1.0)
            .acos();
    let count = ((sweep / step).ceil() as usize).clamp(2, 512);
    let angle = |index: usize| arc.angles[0] + sweep * index as f64 / count as f64;
    let mut points: Vec<(f64, f64)> = (0..=count)
        .map(|index| {
            let [x, y] = arc.point(angle(index), half);
            (x, y)
        })
        .collect();
    points.extend((0..=count).rev().map(|index| {
        let [x, y] = arc.point(angle(index), -half);
        (x, y)
    }));
    Some(points)
}

/// `ThicknessSource` of a wall whose thickness is its type's layers summed
/// (RE-54).
pub const WALL_TYPE_THICKNESS_SOURCE: &str = "wall_type_compound_structure";

/// How far outside a wall's record box its centreline's midpoint may lie,
/// beyond the wall's own thickness, before the line is taken to belong to
/// something else.
const WALL_AXIS_BOX_TOLERANCE_FEET: f64 = 1e-6;

/// How closely a rectangle along an angled wall's line must reproduce its
/// record box for the box to set its length.
const WALL_BOX_CLOSURE_FEET: f64 = 1e-4;

/// A wall's body from its centreline (RE-54): a rectangle its type's
/// thickness across, centred on the line.
#[derive(Debug, Clone, Copy, PartialEq)]
struct WallCentrelineBody {
    centre: [f64; 2],
    /// Plan rotation of the rectangle's width axis; `None` keeps the model
    /// axes.
    rotation: Option<f64>,
    width_feet: f64,
    depth_feet: f64,
    thickness_feet: f64,
    /// How far past its line's start and end the body reaches at a butt
    /// join its join lists decide (RE-70) or a T joint (RE-73).
    join_reach_feet: [Option<f64>; 2],
}

/// The body of a wall with centreline `axis` whose record box is centred on
/// `(x, y)` with plan extents `width` × `depth`.
///
/// Across the wall, the body is the type's thickness centred on the line.
/// Along it:
/// - an axis-parallel wall keeps the box's extent, which its joins trimmed
///   (RE-26). A box thinner than the type is a wall Revit cut, and keeps
///   the box.
/// - a wall at an angle is the rectangle the box closes on, when one of the
///   type's thickness along the line fits the box exactly and is centred on
///   the line; otherwise it runs from one end of its line to the other,
///   unless the box is more than twice the wall's thickness (at least a
///   foot) wider than that rectangle's.
/// - an end at a butt join its join lists decide reaches half the other
///   wall's thickness past the line's end, or stops that far short of it
///   (RE-70), for either kind of wall; an end at a T joint stops that far
///   short (RE-73).
///
/// `None` when the line is degenerate, its midpoint lies outside the box,
/// or the box is thinner than the type or too wide.
fn wall_centreline_body(
    axis: crate::partition_schema_mvp::WallAxis,
    [x, y]: [f64; 2],
    width: f64,
    depth: f64,
) -> Option<WallCentrelineBody> {
    let [sx, sy] = axis.start;
    let [ex, ey] = axis.end;
    let thickness = axis.thickness_feet;
    let length = (ex - sx).hypot(ey - sy);
    if !(length.is_finite() && length > 1e-9 && thickness.is_finite() && thickness > 0.0) {
        return None;
    }
    let (mx, my) = ((sx + ex) / 2.0, (sy + ey) / 2.0);
    let slack = thickness + WALL_AXIS_BOX_TOLERANCE_FEET;
    if (mx - x).abs() > width / 2.0 + slack || (my - y).abs() > depth / 2.0 + slack {
        return None;
    }
    let (ux, uy) = ((ex - sx) / length, (ey - sy) / length);
    let thinner = |across: f64| across < thickness - WALL_AXIS_BOX_TOLERANCE_FEET;
    // RE-70: an end its join lists decide sits that far past the line's
    // end, whatever the box says; the other end keeps its own.
    let joined = |centre: [f64; 2], run: f64| {
        let middle = (centre[0] - sx) * ux + (centre[1] - sy) * uy;
        let [start, end] = axis.join_reach_feet;
        let low = start.map_or(middle - run / 2.0, |reach| -reach);
        let high = end.map_or(middle + run / 2.0, |reach| length + reach);
        if high - low <= WALL_AXIS_BOX_TOLERANCE_FEET {
            return (centre, run);
        }
        let at = (low + high) / 2.0;
        ([sx + ux * at, sy + uy * at], high - low)
    };
    if uy.abs() <= 1e-9 {
        let (centre, run) = joined([x, sy], width);
        return (!thinner(depth)).then_some(WallCentrelineBody {
            centre,
            rotation: None,
            width_feet: run,
            depth_feet: thickness,
            thickness_feet: thickness,
            join_reach_feet: axis.join_reach_feet,
        });
    }
    if ux.abs() <= 1e-9 {
        let (centre, run) = joined([sx, y], depth);
        return (!thinner(width)).then_some(WallCentrelineBody {
            centre,
            rotation: None,
            width_feet: thickness,
            depth_feet: run,
            thickness_feet: thickness,
            join_reach_feet: axis.join_reach_feet,
        });
    }
    // The box of a rectangle `run` long and `thickness` across along
    // (ux, uy) is `run·|ux| + thickness·|uy|` wide and
    // `run·|uy| + thickness·|ux|` deep.
    let run_from_width = (width - thickness * uy.abs()) / ux.abs();
    let run_from_depth = (depth - thickness * ux.abs()) / uy.abs();
    let off_line = (x - sx) * -uy + (y - sy) * ux;
    let (centre, run) = if (run_from_width - run_from_depth).abs() <= WALL_BOX_CLOSURE_FEET
        && off_line.abs() <= WALL_BOX_CLOSURE_FEET
        && run_from_width > 0.0
    {
        ([x, y], run_from_width)
    } else {
        // A box far wider than the line's own rectangle holds a body that
        // is not that rectangle: a leaning or profiled wall.
        let limit = 2.0 * thickness.max(1.0);
        let excess_width = width - (length * ux.abs() + thickness * uy.abs());
        let excess_depth = depth - (length * uy.abs() + thickness * ux.abs());
        if excess_width > limit || excess_depth > limit {
            return None;
        }
        ([mx, my], length)
    };
    let (centre, run) = joined(centre, run);
    Some(WallCentrelineBody {
        centre,
        rotation: Some(uy.atan2(ux)),
        width_feet: run,
        depth_feet: thickness,
        thickness_feet: thickness,
        join_reach_feet: axis.join_reach_feet,
    })
}

/// A tapered wall's plan footprint as a [`WallCentrelineBody`] (RE-86): the
/// rectangle between its interior face and its exterior face at the base,
/// `height` × tan(angle) further out than at the top. It is checked against
/// the record box as a vertical wall of that width would be, on the line
/// moved half that much towards the exterior. Its `thickness_feet` stays
/// the type's, at the top.
fn tapered_wall_body(
    axis: crate::partition_schema_mvp::WallAxis,
    taper: crate::partition_schema_mvp::WallTaper,
    centre: [f64; 2],
    width: f64,
    depth: f64,
    height: f64,
) -> Option<WallCentrelineBody> {
    let spread = height * taper.angle_radians.tan();
    if !(spread.is_finite() && spread > 0.0) {
        return None;
    }
    let [ex, ey] = taper.exterior.map(|value| value * spread / 2.0);
    let shifted = crate::partition_schema_mvp::WallAxis {
        start: [axis.start[0] + ex, axis.start[1] + ey],
        end: [axis.end[0] + ex, axis.end[1] + ey],
        thickness_feet: axis.thickness_feet + spread,
        ..axis
    };
    wall_centreline_body(shifted, centre, width, depth).map(|body| WallCentrelineBody {
        thickness_feet: axis.thickness_feet,
        ..body
    })
}

/// A tapered wall's solid (RE-86): its cross-section, the type's thickness
/// about the line at the top and wider towards the exterior at the base,
/// extruded along the line through `body`'s run. It is placed in the frame
/// of the line's direction, centred on `body`.
fn tapered_wall_solid(
    body: &WallCentrelineBody,
    axis: &crate::partition_schema_mvp::WallAxis,
    taper: crate::partition_schema_mvp::WallTaper,
    height: f64,
) -> Option<entities::SolidShape> {
    let [sx, sy] = axis.start;
    let length = (axis.end[0] - sx).hypot(axis.end[1] - sy);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let along = [(axis.end[0] - sx) / length, (axis.end[1] - sy) / length];
    // The body's run: the rectangle's width, unless an axis-parallel body
    // runs along the model's Y axis.
    let run = if body.rotation.is_none() && along[1].abs() > 1e-9 {
        body.depth_feet
    } else {
        body.width_feet
    };
    let left = [-along[1], along[0]];
    let side = if taper.exterior[0] * left[0] + taper.exterior[1] * left[1] >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let half = axis.thickness_feet / 2.0;
    let spread = height * taper.angle_radians.tan();
    let inner = -side * (half + spread / 2.0);
    let mut points = vec![
        (inner, 0.0),
        (side * (half + spread / 2.0), 0.0),
        (side * (half - spread / 2.0), height),
        (inner, height),
    ];
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
        .sum();
    if area < 0.0 {
        points.reverse();
    }
    (run > 0.0 && height > 0.0).then(|| entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::ArbitraryClosed { points },
        origin_feet: [-run / 2.0, 0.0, 0.0],
        axis: [1.0, 0.0, 0.0],
        ref_direction: [0.0, 1.0, 0.0],
        depth_feet: run,
    })
}

/// A wall body's plan outline where its layered butt joins end each layer
/// on its own line (RE-71), in the body's local frame: a staircase of one
/// band per layer across the wall. An end no layered join decides keeps
/// `body`'s end for every layer.
///
/// `None` when the layers do not add up to the body's thickness, the
/// exterior is not across the line, or a layer would have no length.
fn layered_wall_profile(
    body: &WallCentrelineBody,
    axis: &crate::partition_schema_mvp::WallAxis,
    ends: &crate::partition_schema_mvp::WallLayerEnds,
) -> Option<Vec<(f64, f64)>> {
    let [sx, sy] = axis.start;
    let length = (axis.end[0] - sx).hypot(axis.end[1] - sy);
    if !(length.is_finite() && length > 1e-9) {
        return None;
    }
    let along = [(axis.end[0] - sx) / length, (axis.end[1] - sy) / length];
    let across = ends.exterior;
    let total: f64 = ends.widths.iter().sum();
    if (along[0] * across[0] + along[1] * across[1]).abs() > 1e-6
        || (across[0].hypot(across[1]) - 1.0).abs() > 1e-6
        || (total - body.thickness_feet).abs() > WALL_BOX_CLOSURE_FEET
    {
        return None;
    }
    let run = if body.rotation.is_some() || along[1].abs() <= 1e-9 {
        body.width_feet
    } else {
        body.depth_feet
    };
    let middle = (body.centre[0] - sx) * along[0] + (body.centre[1] - sy) * along[1];
    let [start, end] = &ends.reach_feet;
    let mut top = total / 2.0;
    let mut bands = Vec::with_capacity(ends.widths.len());
    for (index, width) in ends.widths.iter().enumerate() {
        // Each layer ends along both of its edges, square or slanted (RE-74).
        let low = start.as_ref().map_or([middle - run / 2.0; 2], |reach| {
            [-reach[index][0], -reach[index][1]]
        });
        let high = end.as_ref().map_or([middle + run / 2.0; 2], |reach| {
            [length + reach[index][0], length + reach[index][1]]
        });
        if high[0] - low[0] <= WALL_AXIS_BOX_TOLERANCE_FEET
            || high[1] - low[1] <= WALL_AXIS_BOX_TOLERANCE_FEET
        {
            return None;
        }
        bands.push((low, high, top, top - width));
        top -= width;
    }
    // Along the exterior face to the end, down the end's steps, back along
    // the interior face and up the start's steps.
    let mut outline: Vec<(f64, f64)> = Vec::with_capacity(bands.len() * 4);
    for (low, high, upper, lower) in &bands {
        if outline.is_empty() {
            outline.push((low[0], *upper));
        }
        outline.push((high[0], *upper));
        outline.push((high[1], *lower));
    }
    for (low, _, upper, lower) in bands.iter().rev() {
        outline.push((low[1], *lower));
        outline.push((low[0], *upper));
    }
    outline.pop();
    outline.dedup_by(|a, b| (a.0 - b.0).abs() <= 1e-12 && (a.1 - b.1).abs() <= 1e-12);
    // Where neighbouring layers end together, their shared end is one edge.
    let corners: Vec<(f64, f64)> = (0..outline.len())
        .filter(|&index| {
            let previous = outline[(index + outline.len() - 1) % outline.len()];
            let point = outline[index];
            let next = outline[(index + 1) % outline.len()];
            ((point.0 - previous.0) * (next.1 - point.1)
                - (point.1 - previous.1) * (next.0 - point.0))
                .abs()
                > 1e-12
        })
        .map(|index| outline[index])
        .collect();
    let outline = corners;
    let (cos, sin) = body
        .rotation
        .map_or((1.0, 0.0), |angle| (angle.cos(), angle.sin()));
    let mut points: Vec<(f64, f64)> = outline
        .into_iter()
        .map(|(t, a)| {
            let x = sx + along[0] * t + across[0] * a - body.centre[0];
            let y = sy + along[1] * t + across[1] * a - body.centre[1];
            (x * cos + y * sin, -x * sin + y * cos)
        })
        .collect();
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
        .sum();
    if area < 0.0 {
        points.reverse();
    }
    Some(points)
}

/// A stair run's treads and risers: its side view extruded across the run.
/// `location` is the element's placement, which the solid's own placement
/// is relative to. That placement's Z axis runs across the run and its X
/// axis is up, so the profile's Y axis is the one crossed with the other:
/// the climb direction or its reverse, by which side of the run the sketch
/// starts on.
fn stair_run_solid(
    run: &crate::partition_schema_mvp::StairRunBody,
    location: [f64; 3],
) -> entities::SolidShape {
    if !run.pieces.is_empty() {
        return stair_run_pieces_solid(run, location);
    }
    let y_axis = [run.across[1], -run.across[0]];
    let along = if y_axis[0] * run.climb[0] + y_axis[1] * run.climb[1] >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let mut points: Vec<(f64, f64)> = run.profile.iter().map(|&[u, z]| (z, along * u)).collect();
    let area: f64 = points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
        .sum();
    if area < 0.0 {
        points.reverse();
    }
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::ArbitraryClosed { points },
        origin_feet: [
            run.origin[0] - location[0],
            run.origin[1] - location[1],
            run.origin[2] - location[2],
        ],
        axis: [run.across[0], run.across[1], 0.0],
        ref_direction: [0.0, 0.0, 1.0],
        depth_feet: run.width_feet,
    }
}

/// A spiral run's treads (#357), as Revit's export writes them: one
/// extrusion per tread, up its thickness from its bottom, of a profile of
/// two arcs and two lines about the run's centre, as an `IfcIndexedPolyCurve`
/// so a reader draws the arcs as it draws Revit's.
/// A tread landing's body (#824): its outline, lines and three-point arcs,
/// as one `IfcIndexedPolyCurve` extruded up from its underside to its top.
fn landing_tread_solid(
    tread: &crate::partition_schema_mvp::LandingTreadBody,
    location: [f64; 3],
) -> entities::SolidShape {
    use crate::native_stairs::LandingCurve;
    let mut curves = tread.outline.clone();
    // Counter-clockwise, as IFC wants a profile's outer curve.
    let area: f64 = curves
        .iter()
        .map(|curve| {
            let (p, q) = (curve.start(), curve.end());
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    if area < 0.0 {
        curves.reverse();
        for curve in &mut curves {
            *curve = match *curve {
                LandingCurve::Line(a, b) => LandingCurve::Line(b, a),
                LandingCurve::Arc(a, m, b) => LandingCurve::Arc(b, m, a),
            };
        }
    }
    let origin = curves.first().map_or([0.0, 0.0], |curve| curve.start());
    let local = |p: [f64; 2]| (p[0] - origin[0], p[1] - origin[1]);
    let mut points = Vec::new();
    let mut starts = Vec::new();
    for curve in &curves {
        starts.push(points.len());
        points.push(local(curve.start()));
        if let LandingCurve::Arc(_, m, _) = curve {
            points.push(local(*m));
        }
    }
    let segments = curves
        .iter()
        .enumerate()
        .map(|(i, curve)| {
            let start = starts[i];
            let end = starts.get(i + 1).copied().unwrap_or(0);
            match curve {
                LandingCurve::Line(..) => entities::CurveSegment::Line([start, end]),
                LandingCurve::Arc(..) => entities::CurveSegment::Arc([start, start + 1, end]),
            }
        })
        .collect();
    entities::SolidShape::PlacedExtrusion {
        profile: entities::ProfileDef::IndexedPolyCurve { points, segments },
        origin_feet: [
            origin[0] - location[0],
            origin[1] - location[1],
            tread.bottom_feet - location[2],
        ],
        axis: [0.0, 0.0, 1.0],
        ref_direction: [1.0, 0.0, 0.0],
        depth_feet: tread.top_feet - tread.bottom_feet,
    }
}

fn spiral_run_solid(
    run: &crate::partition_schema_mvp::SpiralRunBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let [r0, r1] = run.radii_feet;
    let at = |radius: f64, angle: f64| (radius * angle.cos(), radius * angle.sin());
    let items = run
        .treads
        .iter()
        .map(|tread| {
            let [outer, inner] = tread.up_angles;
            let d = tread.down_angle;
            let mut points = vec![
                at(r1, d),
                at(r1, (d + outer) / 2.0),
                at(r1, outer),
                at(r0, inner),
                at(r0, (inner + d) / 2.0),
                at(r0, d),
            ];
            // Counter-clockwise, as IFC wants a profile's outer curve.
            let area: f64 = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
                .sum();
            if area < 0.0 {
                points.reverse();
            }
            entities::SolidShape::PlacedExtrusion {
                profile: entities::ProfileDef::IndexedPolyCurve {
                    points,
                    segments: vec![
                        entities::CurveSegment::Arc([0, 1, 2]),
                        entities::CurveSegment::Line([2, 3]),
                        entities::CurveSegment::Arc([3, 4, 5]),
                        entities::CurveSegment::Line([5, 0]),
                    ],
                },
                origin_feet: [
                    run.centre[0] - location[0],
                    run.centre[1] - location[1],
                    tread.bottom_feet - location[2],
                ],
                axis: [0.0, 0.0, 1.0],
                ref_direction: [1.0, 0.0, 0.0],
                depth_feet: tread.top_feet - tread.bottom_feet,
            }
        })
        .collect();
    entities::SolidShape::Compound(items)
}

/// A run of several solids (a riserless run's treads, #357) as one
/// triangulated face set: each side view extruded across the run, a closed
/// prism, its faces turned outward.
fn stair_run_pieces_solid(
    run: &crate::partition_schema_mvp::StairRunBody,
    location: [f64; 3],
) -> entities::SolidShape {
    let point = |u: f64, up: f64, a: f64| {
        [
            run.origin[0] + run.climb[0] * u + run.across[0] * a - location[0],
            run.origin[1] + run.climb[1] * u + run.across[1] * a - location[1],
            run.origin[2] + up - location[2],
        ]
    };
    let mut vertices_feet: Vec<[f64; 3]> = Vec::new();
    let mut triangles: Vec<entities::BrepTriangle> = Vec::new();
    for ring in std::iter::once(&run.profile).chain(run.pieces.iter()) {
        let n = ring.len();
        let base = vertices_feet.len() as u32;
        for &[u, up] in ring {
            vertices_feet.push(point(u, up, 0.0));
        }
        for &[u, up] in ring {
            vertices_feet.push(point(u, up, run.width_feet));
        }
        let near = |i: usize| base + i as u32;
        let far = |i: usize| base + (n + i) as u32;
        let mut faces: Vec<entities::BrepTriangle> = Vec::new();
        for i in 1..n - 1 {
            faces.push(entities::BrepTriangle(near(0), near(i + 1), near(i)));
            faces.push(entities::BrepTriangle(far(0), far(i), far(i + 1)));
        }
        for i in 0..n {
            let j = (i + 1) % n;
            faces.push(entities::BrepTriangle(near(i), near(j), far(j)));
            faces.push(entities::BrepTriangle(near(i), far(j), far(i)));
        }
        // Turn the prism's faces outward: its signed volume is positive.
        let volume: f64 = faces
            .iter()
            .map(|t| {
                let [a, b, c] = [t.0, t.1, t.2].map(|k| vertices_feet[k as usize]);
                (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0
            })
            .sum();
        if volume < 0.0 {
            for t in &mut faces {
                std::mem::swap(&mut t.1, &mut t.2);
            }
        }
        triangles.extend(faces);
    }
    entities::SolidShape::TriangulatedFaceSet {
        vertices_feet,
        triangles,
    }
}

fn element_record_geometry_from_decoded(decoded: &DecodedElement) -> Option<RecordGeometry> {
    let class = decoded.class.as_str();
    let mut width = None;
    let mut depth = None;
    let mut height = None;
    let mut x = None;
    let mut y = None;
    let mut z = None;
    let mut source_stream = None;
    let mut revit_class = None;
    let mut type_parameters: Vec<(String, String)> = Vec::new();
    let mut wall_body_source = None;
    let mut wall_thickness = None;
    let mut wall_trim_start = None;
    let mut wall_trim_end = None;
    let mut column_body_source = None;
    let mut beam_body_source = None;
    let mut column_cut_walls = None;
    let mut column_body_z = (None, None);
    let mut landing_slab = false;
    let mut type_symbol_id = None;
    let mut type_profile = (None, None);
    let mut level_id = None;
    let mut level_bind_source: Option<String> = None;
    let mut room_name = None;
    let mut room_number = None;
    let mut family_name = None;
    let mut family_name_source: Option<String> = None;
    let mut type_name = None;
    for (name, value) in &decoded.fields {
        match (name.as_str(), value) {
            ("m_name", InstanceField::String(v)) => room_name = Some(v.clone()),
            (crate::partition_schema_mvp::FAMILY_NAME_FIELD, InstanceField::String(v)) => {
                family_name = Some(v.clone());
            }
            (crate::partition_schema_mvp::FAMILY_NAME_SOURCE_FIELD, InstanceField::String(v)) => {
                family_name_source = Some(v.clone());
            }
            (crate::partition_schema_mvp::TYPE_NAME_FIELD, InstanceField::String(v)) => {
                type_name = Some(v.clone());
            }
            (crate::partition_schema_mvp::ROOM_NUMBER_FIELD, InstanceField::String(v)) => {
                room_number = Some(v.clone());
            }
            (
                crate::element_record_wall_joins::WALL_BODY_SOURCE_FIELD,
                InstanceField::String(v),
            ) => {
                wall_body_source = Some(v.clone());
            }
            (
                crate::element_record_wall_joins::WALL_THICKNESS_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                wall_thickness = Some(*value);
            }
            (
                crate::element_record_wall_joins::WALL_TRIM_START_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                wall_trim_start = Some(*value);
            }
            (
                crate::element_record_wall_joins::WALL_TRIM_END_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                wall_trim_end = Some(*value);
            }
            (
                crate::element_record_column_cuts::COLUMN_BODY_SOURCE_FIELD,
                InstanceField::String(v),
            ) => {
                column_body_source = Some(v.clone());
            }
            (crate::element_record_beam_cuts::BEAM_BODY_SOURCE_FIELD, InstanceField::String(v)) => {
                beam_body_source = Some(v.clone());
            }
            (
                crate::element_record_column_cuts::COLUMN_CUT_WALL_COUNT_FIELD,
                InstanceField::Integer { value, .. },
            ) => {
                column_cut_walls = Some(*value);
            }
            (
                crate::partition_schema_mvp::TYPE_SYMBOL_FIELD,
                InstanceField::ElementId { id, .. },
            ) => type_symbol_id = Some(*id),
            (
                crate::element_record_level_refs::LEVEL_REFERENCE_FIELD,
                InstanceField::ElementId { id, .. },
            ) => level_id = Some(*id),
            (
                crate::element_record_level_refs::LEVEL_BIND_SOURCE_FIELD,
                InstanceField::String(source),
            ) => level_bind_source = Some(source.clone()),
            (
                crate::partition_schema_mvp::TYPE_PROFILE_WIDTH_FIELD,
                InstanceField::Float { value, .. },
            ) => type_profile.0 = Some(*value),
            (
                crate::partition_schema_mvp::TYPE_PROFILE_DEPTH_FIELD,
                InstanceField::Float { value, .. },
            ) => type_profile.1 = Some(*value),
            ("m_bboxWidth", InstanceField::Float { value, .. }) => width = Some(*value),
            ("m_bboxDepth", InstanceField::Float { value, .. }) => depth = Some(*value),
            ("m_bboxHeight", InstanceField::Float { value, .. }) => height = Some(*value),
            (
                crate::partition_schema_mvp::COLUMN_BODY_BOTTOM_FIELD,
                InstanceField::Float { value, .. },
            ) => column_body_z.0 = Some(*value),
            (
                crate::partition_schema_mvp::COLUMN_BODY_TOP_FIELD,
                InstanceField::Float { value, .. },
            ) => column_body_z.1 = Some(*value),
            // A landing's slab is placed the same way, between its own
            // bottom and top (#824).
            (
                crate::partition_schema_mvp::LANDING_BODY_BOTTOM_FIELD,
                InstanceField::Float { value, .. },
            ) => {
                column_body_z.0 = Some(*value);
                landing_slab = true;
            }
            (
                crate::partition_schema_mvp::LANDING_BODY_TOP_FIELD,
                InstanceField::Float { value, .. },
            ) => column_body_z.1 = Some(*value),
            ("m_locationX", InstanceField::Float { value, .. }) => x = Some(*value),
            ("m_locationY", InstanceField::Float { value, .. }) => y = Some(*value),
            ("m_locationZ", InstanceField::Float { value, .. }) => z = Some(*value),
            (crate::partition_schema_mvp::REVIT_CLASS_FIELD, InstanceField::String(value)) => {
                revit_class = Some(value.clone());
            }
            (name, InstanceField::String(value))
                if name.starts_with(crate::partition_schema_mvp::TYPE_PARAMETER_FIELD_PREFIX) =>
            {
                type_parameters.push((
                    name[crate::partition_schema_mvp::TYPE_PARAMETER_FIELD_PREFIX.len()..]
                        .to_string(),
                    value.clone(),
                ));
            }
            ("m_source_stream", InstanceField::String(value)) => {
                source_stream = Some(value.clone());
            }
            _ => {}
        }
    }
    let (x, y, z) = (x?, y?, z?);
    let (width, depth, height) = (width?, depth?, height?);
    if !(width.is_finite() && depth.is_finite() && height.is_finite()) {
        return None;
    }
    // Fail closed rather than emit a degenerate solid.
    if width <= 0.0 || depth <= 0.0 || height <= 0.0 {
        return None;
    }
    // RE-178: a column that a joined slab or roof cuts is drawn as its cut
    // body. Its placement stays at its own base, so storeys do not move, and
    // `BoundingBoxHeight` keeps its own extent; a body that starts higher is
    // placed `body_offset` above the base.
    let record_height = height;
    let (body_offset, height) = match column_body_z {
        (Some(bottom), Some(top)) if bottom.is_finite() && top > bottom => {
            (bottom - z, top - bottom)
        }
        _ => (0.0, height),
    };
    // RE-49: a beam whose location line the record box supports runs along
    // that line instead of filling its box.
    let beam = if class == "StructuralFraming" {
        crate::partition_schema_mvp::beam_axis_from_fields(&decoded.fields).and_then(
            |(start, end)| {
                let bbox = [
                    x - width / 2.0,
                    y - depth / 2.0,
                    z,
                    x + width / 2.0,
                    y + depth / 2.0,
                    z + height,
                ];
                crate::partition_beam_axes::beam_body(bbox, start, end)
            },
        )
    } else {
        None
    };
    // RE-131: a pipe is the cylinder its connector entries and its record
    // box make, drawn as one; a pipe they do not make one for keeps its box.
    let pipe = if class == "Pipe" {
        crate::partition_schema_mvp::pipe_body_from_fields(&decoded.fields)
    } else {
        None
    };
    // RE-103: a beam whose type is an I section, drawn as it where its body
    // along its line holds the section.
    let i_beam = beam.and_then(|beam| {
        let section = crate::partition_schema_mvp::beam_i_section_from_fields(&decoded.fields)?;
        let (start, end) = crate::partition_schema_mvp::beam_axis_from_fields(&decoded.fields)?;
        let bbox = [
            x - width / 2.0,
            y - depth / 2.0,
            z,
            x + width / 2.0,
            y + depth / 2.0,
            z + height,
        ];
        crate::partition_beam_axes::i_section_body(beam, &section, bbox, start, end)
            .map(|body| (body, section))
    });
    // RE-104: an upright structural column whose type is an I section,
    // turned to its X axis where that fills its record box.
    let i_column = if class == "StructuralColumn" {
        column_i_section(&decoded.fields, width, depth)
    } else {
        None
    };
    // W6: a column whose type is a square hollow section is drawn as it,
    // where its record box is the section's outside.
    let hollow_column = if crate::partition_schema_mvp::HOLLOW_SECTION_CLASSES.contains(&class)
        && i_column.is_none()
    {
        crate::partition_schema_mvp::hollow_section_from_fields(&decoded.fields).filter(|section| {
            let near = |a: f64, b: f64| (a - b).abs() <= HOLLOW_SECTION_BOX_TOLERANCE_FEET;
            near(section.width_feet, section.height_feet)
                && near(width, section.width_feet)
                && near(depth, section.height_feet)
        })
    } else {
        None
    };
    // RE-52: a straight run with separate treads and risers is drawn as
    // them.
    let stair_run = if class == "StairsRun" {
        crate::partition_schema_mvp::stair_run_body_from_fields(&decoded.fields)
    } else {
        None
    };
    // #357: a spiral run is drawn as its treads.
    let spiral_run = if class == "StairsRun" && stair_run.is_none() {
        crate::partition_schema_mvp::spiral_run_body_from_fields(&decoded.fields)
    } else {
        None
    };
    // #824: a landing that is one tread is drawn as it.
    let landing_tread = if class == "StairsLanding" {
        crate::partition_schema_mvp::landing_tread_body_from_fields(&decoded.fields)
    } else {
        None
    };
    // RE-54: a wall whose centreline and type thickness are known is that
    // thickness either side of the line. It replaces the box only where the
    // two differ, so a wall the box already drew right is unchanged.
    let wall_axis = if class == "Wall" {
        crate::partition_schema_mvp::wall_axis_from_fields(&decoded.fields)
    } else {
        None
    };
    // RE-86: a tapered wall is its type's thickness about its line at its
    // top and wider towards its exterior below. Without its exterior side
    // it is not drawn from its line at all.
    let wall_taper = wall_axis
        .and_then(|_| crate::partition_schema_mvp::wall_taper_from_fields(&decoded.fields));
    let wall_axis = wall_axis.filter(|_| {
        wall_taper.is_some() || !crate::partition_schema_mvp::wall_is_tapered(&decoded.fields)
    });
    // RE-71: a wall whose layered butt joins end each layer on its own line
    // is the staircase they make.
    let layer_ends = wall_axis
        .filter(|_| wall_taper.is_none())
        .and_then(|_| crate::partition_schema_mvp::wall_layer_ends_from_fields(&decoded.fields));
    let wall_centreline = wall_axis
        .and_then(|axis| match wall_taper {
            Some(taper) => tapered_wall_body(axis, taper, [x, y], width, depth, height),
            None => wall_centreline_body(axis, [x, y], width, depth),
        })
        .filter(|body| {
            layer_ends.is_some()
                || wall_taper.is_some()
                || body.rotation.is_some()
                || (body.centre[0] - x).abs() > 1e-9
                || (body.centre[1] - y).abs() > 1e-9
                || (body.width_feet - width).abs() > 1e-9
                || (body.depth_feet - depth).abs() > 1e-9
        });
    let wall_profile = match (wall_centreline, wall_axis, layer_ends.as_ref()) {
        (Some(body), Some(axis), Some(ends)) => layered_wall_profile(&body, &axis, ends),
        _ => None,
    };
    // The sketched plan profile, when the element's `OST_SketchLines`
    // records closed one (#31, RE-25). It is recovered in project
    // plan coordinates and the body is placed at the record's plan
    // centre, so the profile is expressed relative to that centre.
    let profile = crate::element_record_plan_profiles::plan_profile_from_fields(&decoded.fields);
    let relative = |outer: &[(f64, f64)], inner: &[Vec<(f64, f64)>]| {
        let points: Vec<(f64, f64)> = outer.iter().map(|(px, py)| (px - x, py - y)).collect();
        let voids: Vec<Vec<(f64, f64)>> = inner
            .iter()
            .map(|ring| ring.iter().map(|(px, py)| (px - x, py - y)).collect())
            .collect();
        if voids.is_empty() {
            entities::ProfileDef::ArbitraryClosed { points }
        } else {
            entities::ProfileDef::ArbitraryWithVoids { points, voids }
        }
    };
    // RE-75: a curved wall is the ring sector its arc and its type's
    // thickness make.
    let wall_arc = if class == "Wall" {
        crate::partition_schema_mvp::wall_arc_from_fields(&decoded.fields)
            .as_ref()
            .and_then(wall_arc_outline)
    } else {
        None
    };
    let profile_override = profile
        .as_ref()
        .map(|profile| relative(&profile.outer_xy, &profile.inner_xy))
        .or_else(|| wall_arc.as_ref().map(|outline| relative(outline, &[])));
    // RE-87: a family instance turned off the model's axes is the rectangle
    // at its angle that its record box bounds.
    // RE-106: a curtain mullion or panel turned or tilted off the model's
    // axes, as the box along its own axes whose world box is its record box.
    let oriented = crate::partition_schema_mvp::curtain_axes_from_fields(&decoded.fields)
        .and_then(|axes| oriented_box(axes, [width, depth, height]));
    let turned = if TURNED_BOX_EXCLUDED_CLASSES.contains(&class)
        || profile_override.is_some()
        || stair_run.is_some()
        || spiral_run.is_some()
        || landing_tread.is_some()
        || beam.is_some()
        || matches!(type_profile, (Some(_), Some(_)))
    {
        None
    } else {
        crate::partition_schema_mvp::instance_x_axis_from_fields(&decoded.fields)
            .and_then(|axis| turned_box(axis, width, depth))
    };
    // RE-56: a shed roof whose slope reproduces its record box rises along
    // it. RE-199: a gable or hip roof, its planes' lowest envelope, and
    // either cut level at its base when that is what its box records.
    let roof_slope = if class == "Roof" {
        profile.as_ref().and_then(|profile| {
            let slopes = crate::partition_schema_mvp::roof_slopes_from_fields(&decoded.fields);
            let fascia =
                crate::partition_schema_mvp::roof_fascia_depth_from_fields(&decoded.fields)
                    .unwrap_or(0.0);
            let outer: Vec<(f64, f64)> = profile
                .outer_xy
                .iter()
                .map(|(px, py)| (px - x, py - y))
                .collect();
            let inner: Vec<Vec<(f64, f64)>> = profile
                .inner_xy
                .iter()
                .map(|ring| ring.iter().map(|(px, py)| (px - x, py - y)).collect())
                .collect();
            use crate::partition_roof_slopes::RafterCut;
            let plain = || match slopes.as_slice() {
                [] => None,
                [slope] => roof_slope_solid(*slope, [x, y], &outer, &inner, height),
                _ => roof_planes_solid(&slopes, [x, y], &outer, &inner, height, None),
            };
            let two_cut = || {
                (!slopes.is_empty())
                    .then(|| {
                        roof_planes_solid(&slopes, [x, y], &outer, &inner, height, Some(fascia))
                    })
                    .flatten()
            };
            // RE-199: the roof's own rafter cut where it is read; elsewhere
            // the plain body, else the two-cut one its record box singles out.
            match crate::partition_schema_mvp::roof_rafter_cut_from_fields(&decoded.fields) {
                Some(RafterCut::Plumb) => plain(),
                Some(RafterCut::TwoCutPlumb) => two_cut(),
                // A square two-cut with a fascia is not measured.
                Some(RafterCut::TwoCutSquare) => (fascia == 0.0).then(two_cut).flatten(),
                None => plain().or_else(two_cut),
            }
        })
    } else {
        None
    };
    // RE-211: a roof by extrusion, its section run across its record box.
    let roof_extrusion = if class == "Roof" && roof_slope.is_none() {
        crate::partition_schema_mvp::roof_extrusion_from_fields(&decoded.fields).and_then(
            |extrusion| {
                let (centre, size) = if extrusion.axis == 0 {
                    (x, width)
                } else {
                    (y, depth)
                };
                roof_extrusion_solid(
                    &extrusion,
                    [x, y, z],
                    (centre - size / 2.0, centre + size / 2.0),
                )
            },
        )
    } else {
        None
    };
    // #331: the further pieces of a sketch of separate loops, each its own
    // body at the element's placement, as Revit exports one element per
    // piece.
    let piece_bodies: Vec<Extrusion> = profile
        .as_ref()
        .map(|profile| {
            profile
                .pieces
                .iter()
                .map(|piece| Extrusion {
                    width_feet: width,
                    depth_feet: depth,
                    height_feet: height,
                    profile_override: Some(relative(&piece.outer_xy, &piece.inner_xy)),
                })
                .collect()
        })
        .unwrap_or_default();
    // RE-151: a floor's tagged void is also an opening voiding it, as Revit's
    // export writes it, the void's outline through the floor's thickness.
    let void_openings: Vec<VoidBody> = profile
        .as_ref()
        .filter(|_| class == "Floor")
        .map(|profile| {
            profile
                .void_openings
                .iter()
                .map(|void| {
                    let outline = void.outline_xy.iter().map(|(px, py)| (px - x, py - y));
                    (void.piece, void.tag, outline.collect(), height)
                })
                .collect()
        })
        .unwrap_or_default();
    // The family/type symbol's section, when the instance joined to
    // one and the section agrees with the instance envelope (#215,
    // RE-26). The rectangle it gives is the same shape the envelope
    // gave; what changes is that the type is now the authority for it.
    let type_section = match type_profile {
        (Some(w), Some(d)) if w > 0.0 && d > 0.0 => Some((w, d)),
        _ => None,
    };
    // A column that its joined walls cut emits the **cut** rectangle,
    // not the family section: the section is still the type's, and is
    // still reported below, but it is no longer the shape of the
    // solid (#239, RE-29 §5).
    let (width, depth) = if column_body_source.is_some() {
        (width, depth)
    } else {
        type_section.unwrap_or((width, depth))
    };
    let mut properties = vec![
        // `BodySource` stays the record box unless a wall resolved its
        // joins: the placement, the plan envelope and the extrusion
        // depth are otherwise all still read from the box. What the
        // sketch lines replace is the plan *profile*, which
        // `ProfileResolved` / `ProfileSource` report separately.
        Property {
            name: "BodySource".into(),
            value: PropertyValue::Text(
                wall_profile
                    .as_ref()
                    .map(|_| WALL_LAYER_JOIN_BODY_SOURCE.into())
                    .or_else(|| {
                        wall_centreline.map(|_| {
                            if wall_taper.is_some() {
                                WALL_TAPERED_BODY_SOURCE.into()
                            } else {
                                WALL_CENTRELINE_BODY_SOURCE.into()
                            }
                        })
                    })
                    .or_else(|| wall_arc.as_ref().map(|_| WALL_ARC_BODY_SOURCE.into()))
                    .or_else(|| wall_body_source.clone())
                    .or_else(|| column_body_source.clone())
                    .or_else(|| {
                        (landing_slab && profile.is_some()).then(|| LANDING_SLAB_BODY_SOURCE.into())
                    })
                    .or_else(|| beam_body_source.clone())
                    .or_else(|| beam.map(|_| BEAM_AXIS_BODY_SOURCE.into()))
                    .or_else(|| pipe.map(|_| PIPE_AXIS_BODY_SOURCE.into()))
                    .or_else(|| stair_run.as_ref().map(|_| STAIR_RUN_BODY_SOURCE.into()))
                    .or_else(|| spiral_run.as_ref().map(|_| SPIRAL_RUN_BODY_SOURCE.into()))
                    .or_else(|| {
                        landing_tread
                            .as_ref()
                            .map(|_| LANDING_TREAD_BODY_SOURCE.into())
                    })
                    .or_else(|| roof_slope.as_ref().map(|_| ROOF_SLOPE_BODY_SOURCE.into()))
                    .or_else(|| {
                        roof_extrusion
                            .as_ref()
                            .map(|_| ROOF_EXTRUSION_BODY_SOURCE.into())
                    })
                    .or_else(|| i_column.map(|_| COLUMN_I_SECTION_BODY_SOURCE.into()))
                    .or_else(|| {
                        hollow_column
                            .as_ref()
                            .map(|_| COLUMN_HOLLOW_SECTION_BODY_SOURCE.into())
                    })
                    .or_else(|| turned.map(|_| INSTANCE_TURNED_BODY_SOURCE.into()))
                    .or_else(|| oriented.map(|_| INSTANCE_ORIENTED_BODY_SOURCE.into()))
                    .unwrap_or_else(|| RECORD_BBOX_BODY_SOURCE.into()),
            ),
        },
        Property {
            name: "ProfileResolved".into(),
            value: PropertyValue::Boolean(
                profile.is_some()
                    || type_section.is_some()
                    || beam.is_some()
                    || pipe.is_some()
                    || stair_run.is_some()
                    || spiral_run.is_some()
                    || landing_tread.is_some()
                    || wall_centreline.is_some()
                    || wall_arc.is_some()
                    || i_column.is_some()
                    || hollow_column.is_some(),
            ),
        },
        Property {
            name: "LevelBindResolved".into(),
            value: PropertyValue::Boolean(level_id.is_some()),
        },
        Property {
            name: "BoundingBoxHeight".into(),
            value: PropertyValue::LengthFeet(record_height),
        },
    ];
    if column_body_z.0.is_some() && (height - record_height).abs() > 1e-9 {
        let source = if landing_slab {
            LANDING_SLAB_BODY_HEIGHT_SOURCE
        } else {
            COLUMN_CUT_BODY_HEIGHT_SOURCE
        };
        properties.push(Property {
            name: "BodyHeightSource".into(),
            value: PropertyValue::Text(source.into()),
        });
    }
    // #219 / RE-27: the host Level the record's counted reference list
    // names. `ifc::apply_record_level_reference_storeys` reads it back
    // off the emitted element, so the two sides share one carrier.
    if let Some(id) = level_id {
        properties.push(Property {
            name: crate::element_record_level_refs::LEVEL_ELEMENT_ID_PROPERTY.into(),
            value: PropertyValue::Integer(i64::from(id)),
        });
        properties.push(Property {
            name: "LevelBindSource".into(),
            value: PropertyValue::Text(level_bind_source.unwrap_or_else(|| {
                crate::element_record_level_refs::LEVEL_REFERENCE_SOURCE.into()
            })),
        });
    }
    if let Some(profile) = profile.as_ref() {
        properties.push(Property {
            name: "ProfileSource".into(),
            value: PropertyValue::Text(
                decoded
                    .fields
                    .iter()
                    .find_map(|(name, value)| {
                        match value {
                        InstanceField::String(source)
                            if name
                                == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD =>
                        {
                            Some(source.clone())
                        }
                        _ => None,
                    }
                    })
                    .unwrap_or_else(|| {
                        crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE.into()
                    }),
            ),
        });
        properties.push(Property {
            name: "ProfileVertexCount".into(),
            value: PropertyValue::Integer(profile.vertex_count() as i64),
        });
        properties.push(Property {
            name: "ProfileVoidCount".into(),
            value: PropertyValue::Integer(profile.inner_xy.len() as i64),
        });
        if !profile.pieces.is_empty() {
            properties.push(Property {
                name: "ProfilePieceCount".into(),
                value: PropertyValue::Integer(1 + profile.pieces.len() as i64),
            });
        }
    }
    if let (Some((section_width, section_depth)), Some(symbol)) = (type_section, type_symbol_id) {
        properties.push(Property {
            name: "ProfileSource".into(),
            value: PropertyValue::Text(
                column_body_source
                    .clone()
                    .unwrap_or_else(|| crate::partition_schema_mvp::TYPE_PROFILE_SOURCE.into()),
            ),
        });
        properties.push(Property {
            name: "TypeSymbolElementId".into(),
            value: PropertyValue::Integer(i64::from(symbol)),
        });
        properties.push(Property {
            name: "TypeSectionWidth".into(),
            value: PropertyValue::LengthFeet(section_width),
        });
        properties.push(Property {
            name: "TypeSectionDepth".into(),
            value: PropertyValue::LengthFeet(section_depth),
        });
    }
    // RE-89: a turned instance reports its origin, which a window's opening
    // in a tapered wall is centred on.
    if turned.is_some() {
        if let Some(origin) =
            crate::partition_schema_mvp::instance_origin_from_fields(&decoded.fields)
        {
            for (name, value) in INSTANCE_ORIGIN_PROPERTIES.iter().zip(origin) {
                properties.push(Property {
                    name: (*name).into(),
                    value: PropertyValue::LengthFeet(value),
                });
            }
        }
    }
    // RE-93: a window reports the opening its type and transform give it.
    if let Some(opening) = crate::partition_schema_mvp::filler_opening_from_fields(&decoded.fields)
    {
        let values = [
            PropertyValue::LengthFeet(opening.centre[0]),
            PropertyValue::LengthFeet(opening.centre[1]),
            PropertyValue::Real(opening.axis[0]),
            PropertyValue::Real(opening.axis[1]),
            PropertyValue::LengthFeet(opening.base_feet),
            PropertyValue::PositiveLengthFeet(opening.width_feet),
            PropertyValue::PositiveLengthFeet(opening.height_feet),
        ];
        for (name, value) in FILLER_OPENING_PROPERTIES.iter().zip(values) {
            properties.push(Property {
                name: (*name).into(),
                value,
            });
        }
    }
    // RE-165 (B44): a pipe fitting reports its nominal size, the one diameter
    // its connectors give. Revit's IFC2X3 export writes it in
    // Pset_PipeFittingTypeCommon, which in IFC4 has no NominalDiameter, and
    // its IFC4 Reference View export writes none of the port sets that hold
    // one (#822).
    let fitting_diameter = decoded.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. }
            if name == crate::partition_schema_mvp::FITTING_NOMINAL_DIAMETER_FIELD =>
        {
            Some(*value)
        }
        _ => None,
    });
    if let Some(diameter) = fitting_diameter {
        properties.push(Property {
            name: "NominalDiameter".into(),
            value: PropertyValue::PositiveLengthFeet(diameter),
        });
    }
    // RE-187: a door reports its type's Width and Height.
    if let Some(size) = crate::partition_schema_mvp::door_type_size_from_fields(&decoded.fields) {
        for (name, feet) in DOOR_TYPE_SIZE_PROPERTIES
            .iter()
            .zip([size.width_feet, size.height_feet])
        {
            properties.push(Property {
                name: (*name).into(),
                value: PropertyValue::PositiveLengthFeet(feet),
            });
        }
    }
    // RE-215: a door or window reports the extents of the outline its
    // symbol cuts in its host.
    if let Some(size) = crate::partition_schema_mvp::cut_loop_size_from_fields(&decoded.fields) {
        for (name, feet) in CUT_LOOP_SIZE_PROPERTIES.iter().zip(size) {
            properties.push(Property {
                name: (*name).into(),
                value: PropertyValue::PositiveLengthFeet(feet),
            });
        }
    }
    // RE-225: one whose symbol cuts a solid and no loop, the solid's extents.
    if let Some(size) = crate::partition_schema_mvp::cut_solid_size_from_fields(&decoded.fields) {
        for (name, feet) in CUT_SOLID_SIZE_PROPERTIES.iter().zip(size) {
            properties.push(Property {
                name: (*name).into(),
                value: PropertyValue::PositiveLengthFeet(feet),
            });
        }
    }
    // #810: one whose symbol cuts none, its record box along its own X axis
    // and its height.
    if let Some(size) = crate::partition_schema_mvp::instance_box_size_from_fields(&decoded.fields)
    {
        for (name, feet) in INSTANCE_BOX_SIZE_PROPERTIES.iter().zip(size) {
            properties.push(Property {
                name: (*name).into(),
                value: PropertyValue::PositiveLengthFeet(feet),
            });
        }
    }
    // A wall drawn from its centreline reports its type's thickness
    // (RE-54); one whose joins resolved reports the trim it took and, unless
    // its centreline redrew it, the thickness it read off the box's thin
    // axis (RE-26).
    let thickness = wall_centreline
        .map(|body| (body.thickness_feet, WALL_TYPE_THICKNESS_SOURCE))
        .or_else(|| {
            wall_thickness.map(|t| (t, crate::element_record_wall_joins::WALL_THICKNESS_SOURCE))
        });
    if let Some((thickness, source)) = thickness.filter(|_| {
        wall_centreline.is_some() || (wall_trim_start.is_some() && wall_trim_end.is_some())
    }) {
        properties.push(Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(true),
        });
        properties.push(Property {
            name: "Thickness".into(),
            value: PropertyValue::LengthFeet(thickness),
        });
        properties.push(Property {
            name: "ThicknessSource".into(),
            value: PropertyValue::Text(source.into()),
        });
    }
    // RE-70, RE-73: an end a join decides names the wall it butt-joins and
    // whether it runs through, and, where the body was drawn to it, how
    // far past the line it reaches (negative where it stops short).
    let reaches = wall_centreline.map_or([None, None], |body| body.join_reach_feet);
    for (slot, end) in ["Start", "End"].into_iter().enumerate() {
        let partner = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. }
                if name == crate::partition_schema_mvp::WALL_JOIN_PARTNER_FIELDS[slot] =>
            {
                Some(*id)
            }
            _ => None,
        });
        let through = decoded.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Bool(through)
                if name == crate::partition_schema_mvp::WALL_JOIN_THROUGH_FIELDS[slot] =>
            {
                Some(*through)
            }
            _ => None,
        });
        if let (Some(partner), Some(through)) = (partner, through) {
            properties.push(Property {
                name: format!("Join{end}WallElementId"),
                value: PropertyValue::Integer(i64::from(partner)),
            });
            properties.push(Property {
                name: format!("Join{end}RunsThrough"),
                value: PropertyValue::Boolean(through),
            });
        }
        if let Some(reach) = reaches[slot] {
            properties.push(Property {
                name: format!("JoinReach{end}"),
                value: PropertyValue::LengthFeet(reach),
            });
        }
    }
    if let (Some(_), Some(start), Some(end)) = (wall_thickness, wall_trim_start, wall_trim_end) {
        properties.push(Property {
            name: "JoinTrimStart".into(),
            value: PropertyValue::LengthFeet(start),
        });
        properties.push(Property {
            name: "JoinTrimEnd".into(),
            value: PropertyValue::LengthFeet(end),
        });
    }
    // A room's number and name, when its own parameter block carried
    // them (#90, RE-29, RE-117). The writer puts the number in
    // `IfcSpace.Name` and the name in `IfcSpace.LongName`, as Revit's own
    // exporter does; the ElementId, which an `IfcSpace` has no `Tag` for,
    // is its own property.
    if class == "Room" {
        if let Some(id) = decoded.id {
            properties.push(Property {
                name: ROOM_ELEMENT_ID_PROPERTY.into(),
                value: PropertyValue::Integer(i64::from(id)),
            });
        }
    }
    if let Some(number) = room_number {
        properties.push(Property {
            name: ROOM_NUMBER_PROPERTY.into(),
            value: PropertyValue::Text(number),
        });
    }
    if let Some(name) = room_name {
        properties.push(Property {
            name: ROOM_NAME_PROPERTY.into(),
            value: PropertyValue::Text(name),
        });
    }
    // A column whose joined walls cut it reports how many took part
    // (#239, RE-29 section 5). The section above is the family's; this
    // is what is left of the prism after the walls are subtracted.
    if let Some(walls) = column_cut_walls {
        properties.push(Property {
            name: "JoinCutWallCount".into(),
            value: PropertyValue::Integer(walls),
        });
    }
    // RE-49: the beam's length along its location line and the section the
    // record box leaves across it.
    if let Some(beam) = beam {
        for (name, value) in [
            ("AxisLength", beam.length_feet),
            ("SectionWidth", beam.width_feet),
            ("SectionDepth", beam.depth_feet),
        ] {
            properties.push(Property {
                name: name.into(),
                value: PropertyValue::LengthFeet(value),
            });
        }
    }
    if let Some(section) = i_beam
        .map(|(_, section)| section)
        .or(i_column.map(|(_, section)| section))
    {
        properties.push(Property {
            name: "SectionShape".into(),
            value: PropertyValue::Text("I".into()),
        });
        for (name, value) in [
            ("SectionWebThickness", section.web_feet),
            ("SectionFlangeThickness", section.flange_feet),
        ] {
            properties.push(Property {
                name: name.into(),
                value: PropertyValue::LengthFeet(value),
            });
        }
    }
    if let Some(stream) = source_stream {
        properties.push(Property {
            name: "SourceStream".into(),
            value: PropertyValue::Text(stream),
        });
    }
    // RE-76: the class the element's record names in the file's own schema.
    if let Some(class) = revit_class {
        properties.push(Property {
            name: "RevitClass".into(),
            value: PropertyValue::Text(class),
        });
    }
    // RE-77: the type's text parameters, under the names Revit shows.
    for (name, value) in type_parameters {
        properties.push(Property {
            name,
            value: PropertyValue::Text(value),
        });
    }
    // RE-38: the family and type the partition name entries give this
    // element, the two halves of the `Family:Type` Revit's export uses. A
    // system-family type (#322) has a name but no family in the file, so it
    // writes only `TypeName`.
    if let (Some(family), Some(_)) = (&family_name, &type_name) {
        properties.push(Property {
            name: FAMILY_NAME_PROPERTY.into(),
            value: PropertyValue::Text(family.clone()),
        });
        // RE-63: a system family's name is derived, not read.
        if let Some(source) = family_name_source {
            properties.push(Property {
                name: "FamilyNameSource".into(),
                value: PropertyValue::Text(source),
            });
        }
    }
    if let Some(type_name) = type_name {
        properties.push(Property {
            name: TYPE_NAME_PROPERTY.into(),
            value: PropertyValue::Text(type_name),
        });
    }
    // For a plate the recorded vertical extent *is* the element's
    // thickness, which closes the `floor_slab_extrusion_thickness`
    // gap the plan-loop path had to leave open (#31, #212). Measured
    // against the reference export's `IfcExtrudedAreaSolid.Depth`:
    // 79 of 80 slabs agree exactly, and the 80th
    // (`Floor:Basement Slab`, 22756) is exported as two stacked
    // solids of 0.3333 ft and 1.1667 ft whose depths sum to the
    // recorded 1.5 ft. It is still the box extent, not a modelled
    // layer set. The plan profile is a separate question, answered
    // above: the sketch boundary when the element's
    // `OST_SketchLines` records close one (#31, RE-25), the box
    // rectangle when they do not.
    if SLAB_THICKNESS_CLASSES.contains(&class) {
        properties.push(Property {
            name: "ThicknessResolved".into(),
            value: PropertyValue::Boolean(true),
        });
        properties.push(Property {
            name: "Thickness".into(),
            value: PropertyValue::LengthFeet(height),
        });
        properties.push(Property {
            name: "ThicknessSource".into(),
            value: PropertyValue::Text(RECORD_BBOX_THICKNESS_SOURCE.into()),
        });
    }
    let record_body = Extrusion {
        width_feet: width,
        depth_feet: depth,
        height_feet: height,
        profile_override,
    };
    let tapered = match (wall_centreline, wall_axis, wall_taper) {
        (Some(body), Some(axis), Some(taper)) => tapered_wall_solid(&body, &axis, taper, height),
        _ => None,
    };
    let (location, rotation, body, solid) = match (beam, wall_centreline) {
        (_, Some(wall)) if tapered.is_some() => (
            [wall.centre[0], wall.centre[1], z],
            wall_axis.map(|axis| (axis.end[1] - axis.start[1]).atan2(axis.end[0] - axis.start[0])),
            Extrusion::rectangle(wall.width_feet, wall.depth_feet, height),
            tapered,
        ),
        (_, Some(wall)) => (
            [wall.centre[0], wall.centre[1], z],
            wall.rotation,
            Extrusion {
                profile_override: wall_profile
                    .clone()
                    .map(|points| entities::ProfileDef::ArbitraryClosed { points }),
                ..Extrusion::rectangle(wall.width_feet, wall.depth_feet, height)
            },
            None,
        ),
        (beam, None) => {
            let (rotation, body, solid) = match beam {
                // A level beam is its section's plan rectangle along the line,
                // extruded through its depth from the record box's base.
                Some(_) if i_beam.is_some() => (
                    None,
                    record_body,
                    i_beam.map(|(body, section)| beam_i_solid(&body, &section, [x, y, z])),
                ),
                Some(beam) if beam.is_horizontal() => (
                    Some(beam.plan_angle_radians()),
                    Extrusion::rectangle(beam.length_feet, beam.width_feet, beam.depth_feet),
                    None,
                ),
                Some(beam) => (None, record_body, Some(beam_swept_solid(&beam, [x, y, z]))),
                None if pipe.is_some() => (
                    None,
                    record_body,
                    pipe.map(|body| pipe_cylinder_solid(&body, [x, y, z])),
                ),
                None if oriented.is_some() => (
                    None,
                    record_body,
                    oriented.map(|(axes, half)| oriented_box_solid(axes, half, height)),
                ),
                None if hollow_column.is_some() => {
                    let section = hollow_column.as_ref().expect("checked above");
                    (
                        None,
                        Extrusion {
                            profile_override: Some(hollow_section_profile(section)),
                            ..Extrusion::rectangle(section.width_feet, section.height_feet, height)
                        },
                        None,
                    )
                }
                None if i_column.is_some() => {
                    let (angle, section) = i_column.expect("checked above");
                    (
                        Some(angle),
                        Extrusion {
                            profile_override: Some(entities::ProfileDef::IShape {
                                overall_width_feet: section.width_feet,
                                overall_depth_feet: section.depth_feet,
                                web_thickness_feet: section.web_feet,
                                flange_thickness_feet: section.flange_feet,
                            }),
                            ..Extrusion::rectangle(section.width_feet, section.depth_feet, height)
                        },
                        None,
                    )
                }
                None => match (turned, roof_slope.or(roof_extrusion)) {
                    (Some((angle, along, across)), None) => (
                        Some(angle),
                        Extrusion::rectangle(along, across, height),
                        None,
                    ),
                    (_, roof_slope) => {
                        let solid = stair_run
                            .as_ref()
                            .map(|run| stair_run_solid(run, [x, y, z]))
                            .or_else(|| {
                                spiral_run
                                    .as_ref()
                                    .map(|run| spiral_run_solid(run, [x, y, z]))
                            })
                            .or_else(|| {
                                landing_tread
                                    .as_ref()
                                    .map(|tread| landing_tread_solid(tread, [x, y, z]))
                            })
                            .or(roof_slope);
                        (None, record_body, solid)
                    }
                },
            };
            ([x, y, z], rotation, body, solid)
        }
    };
    let solid = match solid {
        None if body_offset.abs() > 1e-9 => Some(entities::SolidShape::PlacedExtrusion {
            profile: body
                .profile_override
                .clone()
                .unwrap_or(entities::ProfileDef::Rectangle {
                    width_feet: body.width_feet,
                    depth_feet: body.depth_feet,
                }),
            origin_feet: [0.0, 0.0, body_offset],
            axis: [0.0, 0.0, 1.0],
            ref_direction: [1.0, 0.0, 0.0],
            depth_feet: body.height_feet,
        }),
        solid => solid,
    };
    Some(RecordGeometry {
        location,
        rotation,
        body,
        solid,
        properties: PropertySet {
            name: ELEMENT_RECORD_PROPERTY_SET.into(),
            properties,
        },
        pieces: piece_bodies,
        void_openings,
    })
}

/// Document a recovered floor boundary without inventing slab thickness.
fn floor_boundary_annotation_from_decoded(decoded: &DecodedElement) -> Option<PropertySet> {
    let _floor = Floor::from_decoded(decoded);
    let boundary = recover_floor_boundary(decoded).ok()?;
    if boundary.vertices_xy.len() < 3 {
        return None;
    }
    Some(PropertySet {
        name: "RvtFloorGeometry".into(),
        properties: vec![
            Property {
                name: "BoundaryVertexCount".into(),
                value: PropertyValue::Integer(boundary.vertices_xy.len() as i64),
            },
            Property {
                name: "BoundaryClosed".into(),
                value: PropertyValue::Boolean(boundary.closed),
            },
            Property {
                name: "BoundarySource".into(),
                value: PropertyValue::Text(format!("{:?}", boundary.source)),
            },
            Property {
                name: "ThicknessResolved".into(),
                value: PropertyValue::Boolean(false),
            },
        ],
    })
}
