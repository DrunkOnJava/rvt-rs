//! Spreadsheet schedules (CSV) of an exported [`IfcModel`].
//!
//! Two schedules, one row per element, in storey order:
//!
//! - [`elements_csv`]: every building element — Revit ElementId, IFC type,
//!   level, material, placement, body size where decoded, the host wall of a
//!   door or window, and where the body came from.
//! - [`rooms_csv`]: every `IfcSpace` with its recovered room number, name
//!   and, where the room's outline was read from its stored solid (RE-101),
//!   its plan area.
//!
//! Only decoded values are written; an unknown is an empty cell, never a
//! guess. A room that keeps its record's bounding box has no area: the area
//! of a bounding box is not the area of a room.
//!
//! Output is RFC 4180 CSV with CRLF line ends. The cells come from an
//! untrusted file, so a text cell that a spreadsheet would read as a formula
//! (leading `=`, `+`, `-`, `@`, tab or carriage return) is prefixed with `'`
//! (the OWASP CSV-injection guidance); numeric cells are written by this
//! module and never need it.

use super::IfcModel;
use super::entities::{Extrusion, IfcEntity, ProfileDef, PropertyValue};
use super::export_content::{ROOM_NAME_PROPERTY, ROOM_NUMBER_PROPERTY};

const FEET_TO_METRES: f64 = 0.3048;

/// Length unit for the size and position columns.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum LengthUnit {
    /// Revit's internal unit; column suffix `_ft`.
    #[default]
    Feet,
    /// Column suffix `_m`.
    Metres,
}

impl LengthUnit {
    fn suffix(self) -> &'static str {
        match self {
            LengthUnit::Feet => "ft",
            LengthUnit::Metres => "m",
        }
    }

    fn convert(self, feet: f64) -> f64 {
        match self {
            LengthUnit::Feet => feet,
            LengthUnit::Metres => feet * FEET_TO_METRES,
        }
    }

    fn convert_area(self, square_feet: f64) -> f64 {
        self.convert(self.convert(square_feet))
    }
}

/// How a schedule is written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CsvOptions {
    pub unit: LengthUnit,
    /// Start with a UTF-8 byte-order mark. Excel on Windows needs it to read
    /// non-ASCII names (`Büro`, `会议室`) correctly; most other tools do not.
    pub excel_bom: bool,
}

/// Column names of [`elements_csv`] for `unit`.
pub fn element_columns(unit: LengthUnit) -> Vec<String> {
    let u = unit.suffix();
    [
        "revit_element_id".to_string(),
        "ifc_type".into(),
        "predefined_type".into(),
        "name".into(),
        "level".into(),
        format!("level_elevation_{u}"),
        "material".into(),
        format!("x_{u}"),
        format!("y_{u}"),
        format!("z_{u}"),
        "rotation_deg".into(),
        format!("width_{u}"),
        format!("depth_{u}"),
        format!("height_{u}"),
        "host_revit_element_id".into(),
        "body_source".into(),
        "profile_resolved".into(),
    ]
    .to_vec()
}

/// Column names of [`rooms_csv`] for `unit`.
pub fn room_columns(unit: LengthUnit) -> Vec<String> {
    let u = unit.suffix();
    vec![
        "revit_element_id".to_string(),
        "number".into(),
        "name".into(),
        "level".into(),
        format!("level_elevation_{u}"),
        format!("area_{u}2"),
        "body_source".into(),
    ]
}

/// One row per building element (rooms included, as `IFCSPACE`).
pub fn elements_csv(model: &IfcModel, options: &CsvOptions) -> String {
    let unit = options.unit;
    let mut rows: Vec<(f64, String, Vec<String>)> = Vec::new();
    for (entity_index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            predefined_type,
            storey_index,
            material_index,
            property_set,
            location_feet,
            rotation_radians,
            extrusion,
            host_element_index,
            material_layer_set_index,
            material_profile_set_index,
            solid_shape,
            representation_map_index,
        } = entity
        else {
            continue;
        };
        let storey = storey_index.and_then(|i| model.building_storeys.get(i));
        // Same precedence as the STEP writer: profile set > layer set >
        // single material > constituent set.
        // A profile set names its section; its one material is the material.
        let material = material_profile_set_index
            .and_then(|i| model.material_profile_sets.get(i))
            .map(|set| match set.profiles.as_slice() {
                [profile] => model
                    .materials
                    .get(profile.material_index)
                    .map_or_else(|| set.name.clone(), |m| m.name.clone()),
                _ => set.name.clone(),
            })
            .or_else(|| {
                material_layer_set_index
                    .and_then(|i| model.material_layer_sets.get(i))
                    .map(|m| m.name.clone())
            })
            .or_else(|| {
                material_index
                    .and_then(|i| model.materials.get(i))
                    .map(|m| m.name.clone())
            })
            // Then a family instance's materials (RE-82), or a wall's layer
            // materials where its body is not a layer set (RE-88), each
            // once, in the order the STEP writer lists them.
            .or_else(|| {
                let set = model
                    .material_constituent_sets
                    .iter()
                    .find(|set| set.elements.contains(&entity_index))?;
                let mut names: Vec<&str> = Vec::new();
                for name in set
                    .material_indices
                    .iter()
                    .filter_map(|&i| model.materials.get(i).map(|m| m.name.as_str()))
                {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
                (!names.is_empty()).then(|| names.join("; "))
            })
            .unwrap_or_default();
        // Size columns describe the body the writer emits: only the
        // extrusion path, and width / depth only for a rectangle.
        let body_is_extrusion = solid_shape.is_none() && representation_map_index.is_none();
        let (width, depth, height) = match extrusion.as_ref().filter(|_| body_is_extrusion) {
            Some(e) => {
                let (w, d) = match &e.profile_override {
                    None => (Some(e.width_feet), Some(e.depth_feet)),
                    Some(ProfileDef::Rectangle {
                        width_feet,
                        depth_feet,
                    }) => (Some(*width_feet), Some(*depth_feet)),
                    Some(_) => (None, None),
                };
                (w, d, Some(e.height_feet))
            }
            None => (None, None, None),
        };
        let host_id = host_element_index
            .and_then(|i| model.entities.get(i))
            .and_then(|host| match host {
                IfcEntity::BuildingElement { type_guid, .. } => type_guid.clone(),
                _ => None,
            })
            .unwrap_or_default();
        let property = |key: &str| {
            property_set
                .as_ref()
                .and_then(|set| set.properties.iter().find(|p| p.name == key))
                .map(|p| &p.value)
        };
        let body_source = match property("BodySource") {
            Some(PropertyValue::Text(t)) => t.clone(),
            _ => String::new(),
        };
        let profile_resolved = match property("ProfileResolved") {
            Some(PropertyValue::Boolean(b)) => b.to_string(),
            _ => String::new(),
        };
        let length = |v: Option<f64>| number(v.map(|f| unit.convert(f)));
        let cells = vec![
            text(type_guid.as_deref().unwrap_or_default()),
            text(ifc_type),
            text(predefined_type.as_deref().unwrap_or_default()),
            text(name),
            text(storey.map(|s| s.name.as_str()).unwrap_or_default()),
            length(storey.map(|s| s.elevation_feet)),
            text(&material),
            length(location_feet.map(|p| p[0])),
            length(location_feet.map(|p| p[1])),
            length(location_feet.map(|p| p[2])),
            number(
                rotation_radians
                    .filter(|_| location_feet.is_some())
                    .map(f64::to_degrees),
            ),
            length(width),
            length(depth),
            length(height),
            text(&host_id),
            text(&body_source),
            profile_resolved,
        ];
        let elevation = storey.map_or(f64::INFINITY, |s| s.elevation_feet);
        rows.push((elevation, sort_key(type_guid.as_deref(), ifc_type), cells));
    }
    render(element_columns(unit), rows, options)
}

/// One row per `IfcSpace`, with the room number and name recovered from the
/// file (#90, RE-29), and the area of the outline read from the room's
/// stored solid (RE-101) where it was.
pub fn rooms_csv(model: &IfcModel, options: &CsvOptions) -> String {
    let unit = options.unit;
    let mut rows = Vec::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid,
            storey_index,
            property_set,
            extrusion,
            ..
        } = entity
        else {
            continue;
        };
        if ifc_type != "IFCSPACE" {
            continue;
        }
        let property = |key: &str| -> String {
            match property_set
                .as_ref()
                .and_then(|set| set.properties.iter().find(|p| p.name == key))
                .map(|p| &p.value)
            {
                Some(PropertyValue::Text(t)) => t.clone(),
                _ => String::new(),
            }
        };
        let storey = storey_index.and_then(|i| model.building_storeys.get(i));
        let number_cell = property(ROOM_NUMBER_PROPERTY);
        let cells = vec![
            text(type_guid.as_deref().unwrap_or_default()),
            text(&number_cell),
            text(&property(ROOM_NAME_PROPERTY)),
            text(storey.map(|s| s.name.as_str()).unwrap_or_default()),
            number(storey.map(|s| unit.convert(s.elevation_feet))),
            number(
                (property("ProfileSource")
                    == crate::partition_room_boundaries::ROOM_OUTLINE_SOURCE)
                    .then(|| extrusion.as_ref().and_then(outline_area_square_feet))
                    .flatten()
                    .map(|area| unit.convert_area(area)),
            ),
            text(&property("BodySource")),
        ];
        let elevation = storey.map_or(f64::INFINITY, |s| s.elevation_feet);
        rows.push((elevation, natural_key(&number_cell), cells));
    }
    render(room_columns(unit), rows, options)
}

/// The plan area of a body's polygon profile, its voids taken out.
fn outline_area_square_feet(extrusion: &Extrusion) -> Option<f64> {
    let ring = |points: &[(f64, f64)]| {
        points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
            .sum::<f64>()
            .abs()
            / 2.0
    };
    match extrusion.profile_override.as_ref()? {
        ProfileDef::ArbitraryClosed { points } => Some(ring(points)),
        ProfileDef::ArbitraryWithVoids { points, voids } => {
            Some(ring(points) - voids.iter().map(|void| ring(void)).sum::<f64>())
        }
        _ => None,
    }
}

fn render(
    header: Vec<String>,
    mut rows: Vec<(f64, String, Vec<String>)>,
    options: &CsvOptions,
) -> String {
    rows.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    let mut out = String::new();
    if options.excel_bom {
        out.push('\u{FEFF}');
    }
    let header: Vec<String> = header.iter().map(|h| escape(h)).collect();
    out.push_str(&header.join(","));
    out.push_str("\r\n");
    for (_, _, cells) in rows {
        out.push_str(&cells.join(","));
        out.push_str("\r\n");
    }
    out
}

/// Sort ElementIds numerically inside a level, then by IFC type.
fn sort_key(revit_id: Option<&str>, ifc_type: &str) -> String {
    format!("{}:{ifc_type}", natural_key(revit_id.unwrap_or_default()))
}

/// Zero-pad the digit runs of `s` so that `"9"` sorts before `"10"`.
fn natural_key(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 16);
    let mut digits = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            digits.push(c);
            continue;
        }
        if !digits.is_empty() {
            out.push_str(&format!("{digits:0>20}"));
            digits.clear();
        }
        out.push(c);
    }
    if !digits.is_empty() {
        out.push_str(&format!("{digits:0>20}"));
    }
    out
}

/// A text cell: formula-neutralised, then RFC 4180-escaped.
fn text(value: &str) -> String {
    if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        escape(&format!("'{value}"))
    } else {
        escape(value)
    }
}

fn escape(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Up to four decimals, trailing zeros dropped; empty for unknown or
/// non-finite values.
fn number(value: Option<f64>) -> String {
    let Some(v) = value.filter(|v| v.is_finite()) else {
        return String::new();
    };
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}
