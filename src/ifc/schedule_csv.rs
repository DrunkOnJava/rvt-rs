//! Spreadsheet schedules (CSV) of an exported [`IfcModel`].
//!
//! Two schedules, one row per element, in storey order:
//!
//! - [`elements_csv`]: every building element — Revit ElementId, IFC type,
//!   level, material, placement, body size where decoded, the host wall of a
//!   door or window, and where the body came from.
//! - [`rooms_csv`]: every `IfcSpace` with its recovered room number and name.
//!
//! Only decoded values are written; an unknown is an empty cell, never a
//! guess. The room schedule has no area column: on the files that decode
//! rooms today the room body is the element record's bounding box
//! (`BodySource = partition_element_record_bbox`, `ProfileResolved = false`),
//! and the area of a bounding box is not the area of a room.
//!
//! Output is RFC 4180 CSV with CRLF line ends. The cells come from an
//! untrusted file, so a text cell that a spreadsheet would read as a formula
//! (leading `=`, `+`, `-`, `@`, tab or carriage return) is prefixed with `'`
//! (the OWASP CSV-injection guidance); numeric cells are written by this
//! module and never need it.

use super::IfcModel;
use super::entities::{IfcEntity, ProfileDef, PropertyValue};
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
        "body_source".into(),
    ]
}

/// One row per building element (rooms included, as `IFCSPACE`).
pub fn elements_csv(model: &IfcModel, options: &CsvOptions) -> String {
    let unit = options.unit;
    let mut rows: Vec<(f64, String, Vec<String>)> = Vec::new();
    for entity in &model.entities {
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
        // single material.
        let material = material_profile_set_index
            .and_then(|i| model.material_profile_sets.get(i))
            .map(|m| m.name.clone())
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
/// file (#90, RE-29). No area — see the module docs.
pub fn rooms_csv(model: &IfcModel, options: &CsvOptions) -> String {
    let unit = options.unit;
    let mut rows = Vec::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid,
            storey_index,
            property_set,
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
            text(&property("BodySource")),
        ];
        let elevation = storey.map_or(f64::INFINITY, |s| s.elevation_feet);
        rows.push((elevation, natural_key(&number_cell), cells));
    }
    render(room_columns(unit), rows, options)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ifc::entities::{Extrusion, Property, PropertySet};
    use crate::ifc::{MaterialInfo, Storey};

    fn element(
        ifc_type: &str,
        id: &str,
        storey_index: Option<usize>,
        extrusion: Option<Extrusion>,
        properties: Vec<Property>,
    ) -> IfcEntity {
        IfcEntity::BuildingElement {
            ifc_type: ifc_type.into(),
            name: format!("{ifc_type}-{id}"),
            type_guid: Some(id.into()),
            predefined_type: None,
            storey_index,
            material_index: None,
            property_set: Some(PropertySet {
                name: "RvtElementRecordGeometry".into(),
                properties,
            }),
            location_feet: Some([1.0, 2.0, 0.0]),
            rotation_radians: Some(std::f64::consts::FRAC_PI_2),
            extrusion,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }
    }

    fn text_prop(name: &str, value: &str) -> Property {
        Property {
            name: name.into(),
            value: PropertyValue::Text(value.into()),
        }
    }

    fn model() -> IfcModel {
        let wall = element(
            "IFCWALL",
            "20796",
            Some(1),
            Some(Extrusion {
                width_feet: 12.5,
                depth_feet: 0.5,
                height_feet: 10.0,
                profile_override: None,
            }),
            vec![
                text_prop("BodySource", "partition_element_record_join_trimmed"),
                Property {
                    name: "ProfileResolved".into(),
                    value: PropertyValue::Boolean(false),
                },
            ],
        );
        let mut door = element("IFCDOOR", "20810", Some(1), None, vec![]);
        if let IfcEntity::BuildingElement {
            host_element_index,
            material_index,
            ..
        } = &mut door
        {
            *host_element_index = Some(0);
            *material_index = Some(0);
        }
        let room = |id: &str, number: &str, name: &str, storey| {
            element(
                "IFCSPACE",
                id,
                Some(storey),
                None,
                vec![
                    text_prop("BodySource", "partition_element_record_bbox"),
                    text_prop(ROOM_NUMBER_PROPERTY, number),
                    text_prop(ROOM_NAME_PROPERTY, name),
                ],
            )
        };
        IfcModel {
            entities: vec![
                wall,
                door,
                room("20901", "10", "Lobby, East", 1),
                room("20902", "9", "=HYPERLINK(\"x\")", 1),
                room("20900", "B1", "Plant", 0),
            ],
            building_storeys: vec![
                Storey {
                    name: "Basement".into(),
                    elevation_feet: -12.0,
                },
                Storey {
                    name: "Level 1".into(),
                    elevation_feet: 0.0,
                },
            ],
            materials: vec![MaterialInfo {
                name: "Wood - Oak".into(),
                color_packed: None,
                transparency: None,
            }],
            ..Default::default()
        }
    }

    fn lines(csv: &str) -> Vec<&str> {
        csv.split("\r\n").filter(|l| !l.is_empty()).collect()
    }

    #[test]
    fn element_schedule_writes_decoded_values_and_blanks_the_rest() {
        let csv = elements_csv(&model(), &CsvOptions::default());
        let rows = lines(&csv);
        assert_eq!(rows[0], element_columns(LengthUnit::Feet).join(","));
        let wall = rows.iter().find(|r| r.starts_with("20796,")).unwrap();
        assert_eq!(
            *wall,
            "20796,IFCWALL,,IFCWALL-20796,Level 1,0,,1,2,0,90,12.5,0.5,10,,partition_element_record_join_trimmed,false"
        );
        let door = rows.iter().find(|r| r.starts_with("20810,")).unwrap();
        assert_eq!(
            *door,
            "20810,IFCDOOR,,IFCDOOR-20810,Level 1,0,Wood - Oak,1,2,0,90,,,,20796,,"
        );
    }

    #[test]
    fn rows_run_in_storey_order_then_by_numeric_id() {
        let csv = elements_csv(&model(), &CsvOptions::default());
        let ids: Vec<&str> = lines(&csv)[1..]
            .iter()
            .map(|r| r.split(',').next().unwrap())
            .collect();
        assert_eq!(ids, ["20900", "20796", "20810", "20901", "20902"]);
    }

    #[test]
    fn metres_convert_lengths_and_rename_columns() {
        let options = CsvOptions {
            unit: LengthUnit::Metres,
            excel_bom: false,
        };
        let csv = elements_csv(&model(), &options);
        let rows = lines(&csv);
        assert!(rows[0].contains("level_elevation_m") && rows[0].contains("height_m"));
        let wall = rows.iter().find(|r| r.starts_with("20796,")).unwrap();
        assert!(wall.contains(",3.81,0.1524,3.048,"), "{wall}");
    }

    #[test]
    fn room_schedule_escapes_and_neutralises_untrusted_names() {
        let csv = rooms_csv(&model(), &CsvOptions::default());
        let rows = lines(&csv);
        assert_eq!(
            rows,
            [
                "revit_element_id,number,name,level,level_elevation_ft,body_source",
                "20900,B1,Plant,Basement,-12,partition_element_record_bbox",
                "20902,9,\"'=HYPERLINK(\"\"x\"\")\",Level 1,0,partition_element_record_bbox",
                "20901,10,\"Lobby, East\",Level 1,0,partition_element_record_bbox",
            ]
        );
    }

    #[test]
    fn excel_option_prepends_a_bom() {
        let options = CsvOptions {
            unit: LengthUnit::Feet,
            excel_bom: true,
        };
        assert!(rooms_csv(&model(), &options).starts_with('\u{FEFF}'));
        assert!(!rooms_csv(&model(), &CsvOptions::default()).starts_with('\u{FEFF}'));
    }

    #[test]
    fn number_formatting_is_compact_and_blank_for_non_finite() {
        assert_eq!(number(Some(12.0)), "12");
        assert_eq!(number(Some(0.30480)), "0.3048");
        assert_eq!(number(Some(-0.00001)), "0");
        assert_eq!(number(Some(f64::NAN)), "");
        assert_eq!(number(None), "");
    }

    #[test]
    fn an_empty_model_is_just_a_header() {
        let csv = elements_csv(&IfcModel::default(), &CsvOptions::default());
        assert_eq!(lines(&csv).len(), 1);
    }
}
