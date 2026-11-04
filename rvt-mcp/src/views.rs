//! The answers, read from the decoded model.
//!
//! Everything here reads the [`IfcModel`] the IFC exporter builds, and the
//! GlobalIds its writer assigns, so an answer is what the written IFC holds.
//! A value the model does not have is left out, never filled in.

use std::collections::BTreeMap;

use rvt::ifc::ELEMENT_RECORD_PROPERTY_SET;
use rvt::ifc::IfcModel;
use rvt::ifc::element_provenance::ValueSource;
use rvt::ifc::entities::{IfcEntity, Property, PropertySet, PropertyValue};
use rvt::ifc::export_content::SPACE_OUTLINE_SOURCE_PROPERTY;
use rvt::ifc::schedule_csv::room_rows;
use rvt::reader::UnreadStream;
use serde_json::Value;

use crate::schema::{
    DecisionOut, DiagnosticsOut, DiagnosticsSummary, ElementCounts, ElementDetail,
    ElementProvenance, ElementRow, EntityRefOut, ExplainElementOut, LevelOut, LevelRefOut,
    LevelsOut, ListElementsOut, NativeRecordOut, PropertyOut, PropertySetOut, RoomOut, RoomsOut,
    SkippedOut, SourcePartOut, SummaryOut, TypeOut, UnreadOut, ValueSourceOut,
};
use crate::store::{Computed, Entry};

const METRES_PER_FOOT: f64 = 0.3048;
const SQUARE_METRES_PER_SQUARE_FOOT: f64 = 0.09290304;
const CUBIC_METRES_PER_CUBIC_FOOT: f64 = 0.028316846592;
const KILOGRAMS_PER_POUND: f64 = 0.45359237;

/// What the answers do not cover. Every summary carries it.
pub const NOT_READ: &str = "These answers cover what rvt-rs decodes. Where provenance.body_source is partition_element_record_bbox, an element's body is only its record's bounding box; family geometry is not read; element parameters are read on Revit 2023 to 2027 only; a file whose release is rated experimental is measured less (release_support). diagnostics.skipped and get_diagnostics list what the export left out and why, and docs/status.md in the rvt-rs repository states what is read per release and category.";

fn text_property(set: Option<&PropertySet>, name: &str) -> Option<String> {
    set?.properties
        .iter()
        .find(|property| property.name == name)
        .and_then(|property| match &property.value {
            PropertyValue::Text(text)
            | PropertyValue::Label(text)
            | PropertyValue::Identifier(text) => Some(text.clone()),
            _ => None,
        })
}

fn non_empty(text: &str) -> Option<String> {
    (!text.is_empty()).then(|| text.to_string())
}

fn storey_name(model: &IfcModel, storey_index: Option<usize>) -> Option<String> {
    storey_index
        .and_then(|index| model.building_storeys.get(index))
        .map(|storey| storey.name.clone())
}

/// A value source as the answers write it.
fn value_source_out(source: &ValueSource) -> ValueSourceOut {
    ValueSourceOut {
        reader: source.reader.clone(),
        rule: source.rule.as_str().to_string(),
        copies: source.copies,
        stream: source.stream.clone(),
        offset: source.offset,
        partitions: source.partitions.clone(),
        parts: source
            .parts
            .iter()
            .map(|part| SourcePartOut {
                element_id: part.element_id,
                stream: part.stream.clone(),
                offset: part.offset,
            })
            .collect(),
        parts_left_out: source.parts_left_out,
        differs_from_frame: source.differs_from_frame,
    }
}

/// The provenance of the building element at `index` of the model's entities.
/// `sources` says whether the answer lists the element's value sources, which
/// a list of elements does not.
fn provenance(computed: &Computed, index: usize, sources: bool) -> Option<ElementProvenance> {
    let IfcEntity::BuildingElement {
        ifc_type,
        type_guid,
        property_set,
        ..
    } = computed.model.entities.get(index)?
    else {
        return None;
    };
    let set = property_set.as_ref();
    let id = type_guid.as_deref().and_then(|tag| tag.parse::<u32>().ok());
    let table = computed.element_provenance.as_ref();
    let source = id.and_then(|id| table?.get(id));
    let omitted = id.is_some_and(|id| table.is_some_and(|table| table.is_omitted(id)));
    Some(ElementProvenance {
        element_id: type_guid.clone(),
        global_id: computed.global_ids.get(index).cloned().flatten(),
        category: Some(ifc_type.clone()),
        class: text_property(set, "RevitClass"),
        body_source: text_property(set, "BodySource"),
        profile_source: text_property(set, "ProfileSource"),
        space_outline_source: text_property(set, SPACE_OUTLINE_SOURCE_PROPERTY),
        area_body_refusal: text_property(set, "AreaBodyRefusal"),
        carrier: source.map(|entry| entry.carrier.as_str().to_string()),
        stream: source.and_then(|entry| entry.stream.clone()),
        offset: source.and_then(|entry| entry.offset),
        record_offset: source.and_then(|entry| entry.record_offset),
        enclosing_record_id: source
            .and_then(|entry| entry.enclosing_record_id)
            .map(|id| id.to_string()),
        class_tag: source.and_then(|entry| entry.class_tag).map(u32::from),
        class_name: source.and_then(|entry| entry.class_name.clone()),
        native_record: source
            .and_then(|entry| entry.native.as_ref())
            .map(|native| NativeRecordOut {
                copies: native.copies,
                copy: native.copy.as_str().to_string(),
                stream: native.stream.clone(),
                offset: native.offset,
                class_name: native.class_name.clone(),
            }),
        provenance_omitted: omitted.then_some(true),
        value_sources: source
            .filter(|_| sources)
            .map(|entry| entry.value_sources.iter().map(value_source_out).collect())
            .unwrap_or_default(),
    })
}

fn element_row(computed: &Computed, index: usize) -> Option<ElementRow> {
    let IfcEntity::BuildingElement {
        name,
        type_guid,
        predefined_type,
        storey_index,
        ..
    } = computed.model.entities.get(index)?
    else {
        return None;
    };
    Some(ElementRow {
        id: type_guid.clone(),
        name: name.clone(),
        level: storey_name(&computed.model, *storey_index),
        predefined_type: predefined_type.clone(),
        provenance: provenance(computed, index, false)?,
    })
}

pub fn unread_out(unread: &[UnreadStream]) -> Vec<UnreadOut> {
    unread
        .iter()
        .map(|stream| UnreadOut {
            name: stream.name.clone(),
            stored_bytes: stream.stored_bytes,
            kind: stream.kind.as_str().to_string(),
            limit: stream.limit,
            description: stream.to_string(),
        })
        .collect()
}

fn diagnostics_summary(computed: &Computed) -> DiagnosticsSummary {
    let diagnostics = &computed.diagnostics;
    DiagnosticsSummary {
        confidence_level: diagnostics.confidence.level.clone(),
        confidence_score: diagnostics.confidence.score,
        partial: !computed.unread.is_empty(),
        unread_streams: unread_out(&computed.unread),
        skipped: diagnostics
            .skipped
            .iter()
            .map(|item| SkippedOut {
                reason: item.reason.clone(),
                count: item.count,
            })
            .collect(),
        warning_count: diagnostics.warnings.len(),
        unsupported_feature_count: diagnostics.unsupported_features.len(),
        bounding_box_bodies: diagnostics.exported.bounding_box_bodies,
        bounding_box_spaces: diagnostics.exported.bounding_box_spaces,
        unexported_element_records: diagnostics.confidence.unexported_element_records,
        notes: computed.notes.clone(),
    }
}

pub fn levels(computed: &Computed) -> Vec<LevelOut> {
    let model = &computed.model;
    let mut counts = vec![0usize; model.building_storeys.len()];
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            storey_index: Some(index),
            ..
        } = entity
            && let Some(count) = counts.get_mut(*index)
        {
            *count += 1;
        }
    }
    model
        .building_storeys
        .iter()
        .zip(counts)
        .enumerate()
        .map(|(index, (storey, element_count))| LevelOut {
            index,
            name: storey.name.clone(),
            elevation_feet: storey.elevation_feet,
            elevation_metres: storey.elevation_feet * METRES_PER_FOOT,
            element_count,
        })
        .collect()
}

pub fn levels_out(entry: &Entry, computed: &Computed) -> LevelsOut {
    LevelsOut {
        model_id: entry.id.clone(),
        levels: levels(computed),
    }
}

pub fn summary(entry: &Entry, computed: &Computed) -> SummaryOut {
    let mut total = 0usize;
    let mut room_count = 0usize;
    let mut by_ifc_type: BTreeMap<String, usize> = BTreeMap::new();
    let mut by_revit_class: BTreeMap<String, usize> = BTreeMap::new();
    for entity in &computed.model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        total += 1;
        *by_ifc_type.entry(ifc_type.clone()).or_insert(0) += 1;
        if let Some(class) = text_property(property_set.as_ref(), "RevitClass") {
            *by_revit_class.entry(class).or_insert(0) += 1;
        }
        if ifc_type == "IFCSPACE" {
            room_count += 1;
        }
    }
    SummaryOut {
        model_id: entry.id.clone(),
        file_kind: entry.identity.file_kind().to_string(),
        size_bytes: entry.identity.size,
        revit_release: computed.diagnostics.input.revit_version,
        project_name: computed.model.project_name.clone(),
        release_support: computed
            .diagnostics
            .input
            .release_support
            .as_ref()
            .and_then(|support| serde_json::to_value(support).ok()),
        file: computed.file.clone(),
        elements: ElementCounts {
            total,
            by_ifc_type,
            by_revit_class,
        },
        levels: levels(computed),
        room_count,
        diagnostics: diagnostics_summary(computed),
        not_read: NOT_READ.to_string(),
    }
}

pub fn diagnostics_out(entry: &Entry, computed: &Computed) -> DiagnosticsOut {
    DiagnosticsOut {
        model_id: entry.id.clone(),
        diagnostics: serde_json::to_value(&computed.diagnostics).unwrap_or(Value::Null),
        unread_streams: unread_out(&computed.unread),
        notes: computed.notes.clone(),
    }
}

/// The filters and page of `list_elements`.
pub struct ElementFilter<'a> {
    pub category: Option<&'a str>,
    pub class: Option<&'a str>,
    pub level: Option<&'a str>,
    pub offset: usize,
    pub limit: usize,
}

/// Whether the IFC entity `ifc_type` (`IFCWALL`) is the one `wanted` names,
/// with or without the `Ifc` prefix, in any case.
fn category_matches(ifc_type: &str, wanted: &str) -> bool {
    let actual = ifc_type.to_ascii_uppercase();
    let wanted = wanted.trim().to_ascii_uppercase();
    actual == wanted || actual == format!("IFC{wanted}")
}

pub fn list_elements(
    entry: &Entry,
    computed: &Computed,
    filter: &ElementFilter<'_>,
) -> ListElementsOut {
    let model = &computed.model;
    let mut total = 0usize;
    let mut elements = Vec::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            storey_index,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        if let Some(wanted) = filter.category
            && !category_matches(ifc_type, wanted)
        {
            continue;
        }
        if let Some(wanted) = filter.class {
            let class = text_property(property_set.as_ref(), "RevitClass");
            if !class.is_some_and(|class| class.eq_ignore_ascii_case(wanted.trim())) {
                continue;
            }
        }
        if let Some(wanted) = filter.level {
            let level = storey_name(model, *storey_index);
            if !level.is_some_and(|level| level.eq_ignore_ascii_case(wanted.trim())) {
                continue;
            }
        }
        if total >= filter.offset
            && elements.len() < filter.limit
            && let Some(row) = element_row(computed, index)
        {
            elements.push(row);
        }
        total += 1;
    }
    ListElementsOut {
        model_id: entry.id.clone(),
        total,
        offset: filter.offset,
        limit: filter.limit,
        returned: elements.len(),
        elements,
    }
}

/// The value of a property in the units the IFC is written in.
fn natural_value(value: &PropertyValue) -> Value {
    match value {
        PropertyValue::Text(text)
        | PropertyValue::Label(text)
        | PropertyValue::Identifier(text) => Value::from(text.as_str()),
        PropertyValue::Integer(number) | PropertyValue::CountValue(number) => Value::from(*number),
        PropertyValue::Real(number)
        | PropertyValue::AngleRadians(number)
        | PropertyValue::TimeSeconds(number)
        | PropertyValue::ThermalTransmittance(number)
        | PropertyValue::VolumetricFlowRate(number)
        | PropertyValue::Pressure(number)
        | PropertyValue::Power(number)
        | PropertyValue::ElectricVoltage(number)
        | PropertyValue::ElectricCurrent(number) => Value::from(*number),
        PropertyValue::Boolean(flag) => Value::from(*flag),
        PropertyValue::Logical(flag) => flag.map_or(Value::Null, Value::from),
        PropertyValue::LengthFeet(feet) | PropertyValue::PositiveLengthFeet(feet) => {
            Value::from(feet * METRES_PER_FOOT)
        }
        PropertyValue::AreaSquareFeet(area) => Value::from(area * SQUARE_METRES_PER_SQUARE_FOOT),
        PropertyValue::VolumeCubicFeet(volume) => Value::from(volume * CUBIC_METRES_PER_CUBIC_FOOT),
        PropertyValue::MassPounds(pounds) => Value::from(pounds * KILOGRAMS_PER_POUND),
        PropertyValue::List(items) => Value::Array(items.iter().map(natural_value).collect()),
        PropertyValue::Enumerated(labels) => Value::from(labels.clone()),
        _ => Value::Null,
    }
}

/// The IfcValue type the writer gives a property, such as `IFCLENGTHMEASURE`.
fn ifc_value_type(value: &PropertyValue) -> Option<String> {
    let step = value.to_step();
    let name = step.split('(').next().unwrap_or_default();
    (!name.is_empty()).then(|| name.to_string())
}

fn property_out(property: &Property, quantities: bool) -> Option<PropertyOut> {
    if quantities {
        let (kind, written) = property.value.quantity()?;
        let value = written
            .parse::<f64>()
            .map_or_else(|_| Value::from(written.as_str()), Value::from);
        return Some(PropertyOut {
            name: property.name.clone(),
            ifc_type: None,
            quantity: Some(kind.to_string()),
            value,
        });
    }
    Some(PropertyOut {
        name: property.name.clone(),
        ifc_type: ifc_value_type(&property.value),
        quantity: None,
        value: natural_value(&property.value),
    })
}

/// A set as the writer writes it: a `Qto_` set holds its quantities only, and
/// is left out when it has none.
fn property_set_out(set: &PropertySet) -> Option<PropertySetOut> {
    let quantities = set.name.starts_with("Qto_");
    let properties: Vec<PropertyOut> = set
        .properties
        .iter()
        .filter_map(|property| property_out(property, quantities))
        .collect();
    if quantities && properties.is_empty() {
        return None;
    }
    let kind = if quantities {
        "quantities"
    } else {
        "properties"
    };
    Some(PropertySetOut {
        name: set.name.clone(),
        kind: kind.to_string(),
        properties,
    })
}

/// The type object of the element with Revit ElementId `id`, with its sets.
fn type_out(model: &IfcModel, id: &str) -> Option<TypeOut> {
    let tag = id.parse::<u32>().ok()?;
    let type_id = model.element_type_ids.get(&tag)?;
    let mut property_sets: Vec<PropertySetOut> = Vec::new();
    if let Some(sets) = model.type_property_sets.get(type_id) {
        property_sets.extend(sets.iter().filter_map(property_set_out));
    }
    if let Some(by_entity) = model.type_entity_sets.get(type_id) {
        for sets in by_entity.values() {
            property_sets.extend(sets.iter().filter_map(property_set_out));
        }
    }
    Some(TypeOut {
        element_id: type_id.to_string(),
        property_sets,
    })
}

/// The entities named by `id`: those whose Revit ElementId it is, otherwise
/// those whose IFC GlobalId it is.
fn find_entities(computed: &Computed, id: &str) -> Vec<usize> {
    let by_element_id: Vec<usize> = computed
        .model
        .entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| match entity {
            IfcEntity::BuildingElement { type_guid, .. } if type_guid.as_deref() == Some(id) => {
                Some(index)
            }
            _ => None,
        })
        .collect();
    if !by_element_id.is_empty() {
        return by_element_id;
    }
    computed
        .global_ids
        .iter()
        .enumerate()
        .filter_map(|(index, global_id)| (global_id.as_deref() == Some(id)).then_some(index))
        .collect()
}

/// Whether a property of an element's record says how its body, profile,
/// thickness, level or names were derived.
fn is_marker(name: &str) -> bool {
    name.ends_with("Source") || name.ends_with("Resolved") || name.ends_with("Refusal")
}

/// The property sets of the entity at `index`: its own and the ones written
/// beside it.
fn element_property_sets(
    model: &IfcModel,
    index: usize,
    property_set: &Option<PropertySet>,
) -> Vec<PropertySetOut> {
    let mut property_sets: Vec<PropertySetOut> = Vec::new();
    if let Some(set) = property_set {
        property_sets.extend(property_set_out(set));
    }
    for entity in &model.entities {
        if let IfcEntity::ElementPropertySet { element, set } = entity
            && *element == index
        {
            property_sets.extend(property_set_out(set));
        }
    }
    property_sets
}

/// The `...Source`, `...Resolved` and `...Refusal` properties of the
/// element's record.
fn trust_markers_of(property_sets: &[PropertySetOut]) -> BTreeMap<String, Value> {
    property_sets
        .iter()
        .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
        .flat_map(|set| set.properties.iter())
        .filter(|property| is_marker(&property.name))
        .map(|property| (property.name.clone(), property.value.clone()))
        .collect()
}

/// An element, in the export or not, with every decision recorded for it
/// (#1326). `id` is a Revit ElementId, which a left-out element has, or the
/// GlobalId of an exported one.
pub fn explain_element(
    entry: &Entry,
    computed: &Computed,
    id: &str,
) -> Result<ExplainElementOut, String> {
    let model = &computed.model;
    let id = id.trim();
    let found = find_entities(computed, id)
        .first()
        .copied()
        .and_then(|index| match model.entities.get(index)? {
            IfcEntity::BuildingElement {
                name,
                type_guid,
                property_set,
                ..
            } => Some((index, name, type_guid, property_set)),
            _ => None,
        });
    let element_id: Option<u32> = match found {
        Some((_, _, type_guid, _)) => type_guid.as_deref().and_then(|tag| tag.parse().ok()),
        None => id.parse().ok(),
    };
    let log = computed.export_decisions.as_ref();
    let decisions: Vec<DecisionOut> = match (log, element_id) {
        (Some(log), Some(element_id)) => log
            .for_element(element_id)
            .map(|decision| DecisionOut {
                stage: decision.stage.as_str().to_string(),
                reason: decision.reason.clone(),
                class: decision.class.clone(),
                stream: decision.stream.clone(),
                offset: decision.offset,
                detail: decision.detail.clone(),
            })
            .collect(),
        _ => Vec::new(),
    };
    let capped_reasons: Vec<String> = log
        .map(|log| log.capped_reasons().map(str::to_string).collect())
        .unwrap_or_default();
    if found.is_none() && decisions.is_empty() {
        let capped = if capped_reasons.is_empty() {
            String::new()
        } else {
            format!(
                " The log is full for {}, so a decision about it may be missing.",
                capped_reasons.join(", ")
            )
        };
        return Err(match element_id {
            Some(element_id) => format!(
                "ElementId {element_id} is not in the export and no decision is recorded for it.{capped} Use list_elements to find exported ids; a frame with no ElementId has no entry."
            ),
            None => format!(
                "no element has Revit ElementId or IFC GlobalId {id:?} in this model. Use list_elements to find ids."
            ),
        });
    }
    let (name, provenance_out, trust_markers) = match found {
        Some((index, name, _, property_set)) => {
            let property_sets = element_property_sets(model, index, property_set);
            (
                Some(name.clone()),
                provenance(computed, index, true),
                trust_markers_of(&property_sets),
            )
        }
        None => (None, None, BTreeMap::new()),
    };
    Ok(ExplainElementOut {
        model_id: entry.id.clone(),
        id: element_id.map_or_else(|| id.to_string(), |element_id| element_id.to_string()),
        exported: found.is_some(),
        name,
        provenance: provenance_out,
        trust_markers,
        decisions,
        capped_reasons,
        file: computed.file.clone(),
    })
}

pub fn get_element(entry: &Entry, computed: &Computed, id: &str) -> Result<ElementDetail, String> {
    let model = &computed.model;
    let indices = find_entities(computed, id);
    let Some(&index) = indices.first() else {
        return Err(format!(
            "no element has Revit ElementId or IFC GlobalId {id:?} in this model. Use list_elements to find ids."
        ));
    };
    let Some(IfcEntity::BuildingElement {
        name,
        type_guid,
        predefined_type,
        storey_index,
        property_set,
        location_feet,
        rotation_radians,
        extrusion,
        host_element_index,
        solid_shape,
        representation_map_index,
        ..
    }) = model.entities.get(index)
    else {
        return Err(format!("{id:?} is not a building element"));
    };
    let provenance = provenance(computed, index, true)
        .ok_or_else(|| format!("{id:?} is not a building element"))?;
    let level = storey_index.and_then(|storey| {
        model.building_storeys.get(storey).map(|found| LevelRefOut {
            index: storey,
            name: found.name.clone(),
            elevation_feet: found.elevation_feet,
        })
    });
    let host_id = host_element_index
        .and_then(|host| model.entities.get(host))
        .and_then(|host| match host {
            IfcEntity::BuildingElement { type_guid, .. } => type_guid.clone(),
            _ => None,
        });
    let body = if representation_map_index.is_some() {
        "mapped"
    } else if solid_shape.is_some() {
        "solid"
    } else if extrusion.is_some() {
        "extrusion"
    } else {
        "none"
    };

    let property_sets = element_property_sets(model, index, property_set);
    let trust_markers = trust_markers_of(&property_sets);
    let other_entities = indices
        .iter()
        .skip(1)
        .filter_map(|&other| match model.entities.get(other)? {
            IfcEntity::BuildingElement { ifc_type, name, .. } => Some(EntityRefOut {
                ifc_type: ifc_type.clone(),
                name: name.clone(),
                global_id: computed.global_ids.get(other).cloned().flatten(),
            }),
            _ => None,
        })
        .collect();

    Ok(ElementDetail {
        model_id: entry.id.clone(),
        id: type_guid.clone(),
        name: name.clone(),
        level,
        predefined_type: predefined_type.clone(),
        location_feet: *location_feet,
        rotation_degrees: rotation_radians
            .filter(|_| location_feet.is_some())
            .map(f64::to_degrees),
        host_id,
        body: body.to_string(),
        provenance,
        file: computed.file.clone(),
        trust_markers,
        property_sets,
        element_type: type_guid.as_deref().and_then(|tag| type_out(model, tag)),
        other_entities,
    })
}

pub fn rooms(entry: &Entry, computed: &Computed) -> RoomsOut {
    let rooms = room_rows(&computed.model)
        .into_iter()
        .filter_map(|row| {
            Some(RoomOut {
                id: non_empty(&row.revit_element_id),
                number: non_empty(&row.number),
                name: non_empty(&row.name),
                level: non_empty(&row.level),
                level_elevation_feet: row.level_elevation_feet,
                area_square_feet: row.area_square_feet,
                area_square_metres: row
                    .area_square_feet
                    .map(|area| area * SQUARE_METRES_PER_SQUARE_FOOT),
                provenance: provenance(computed, row.entity_index, false)?,
            })
        })
        .collect();
    RoomsOut {
        model_id: entry.id.clone(),
        rooms,
    }
}
