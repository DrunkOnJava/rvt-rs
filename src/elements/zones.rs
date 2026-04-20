//! `Room` + `Area` + `Space` — Revit's spatial-zoning classes.
//!
//! These are the "programmatic" elements — they describe *what a part
//! of the building is for* (Bedroom, Kitchen, Mech Room, Stairwell,
//! Fire Zone A, HVAC Zone 3) rather than *what's physically there*.
//! The IFC exporter maps all three to `IfcSpace` because the IFC4
//! schema collapses Revit's three variants into a single class.
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! Room:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Kitchen", "Bedroom 1" |
//! | `m_number` | String | Room number ("101", "B2-12") |
//! | `m_level_id` | ElementId | Level the room sits on |
//! | `m_upper_limit_id` | ElementId | Top level for the room's bounded height |
//! | `m_upper_offset` | f64 | Offset from upper level |
//! | `m_base_offset` | f64 | Offset from base level |
//! | `m_area` | f64 | Computed floor area (ft²) |
//! | `m_volume` | f64 | Computed volume (ft³) |
//!
//! Area + Space use the same fields with minor naming differences,
//! all collapsed through `normalise_field_name`.

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

macro_rules! simple_decoder {
    ($Struct:ident, $name:literal) => {
        pub struct $Struct;

        impl ElementDecoder for $Struct {
            fn class_name(&self) -> &'static str {
                $name
            }

            fn decode(
                &self,
                bytes: &[u8],
                schema: &formats::ClassEntry,
                _index: &HandleIndex,
            ) -> Result<DecodedElement> {
                if schema.name != $name {
                    return Err(Error::BasicFileInfo(format!(
                        "{} received wrong schema: {}",
                        stringify!($Struct),
                        schema.name
                    )));
                }
                Ok(crate::walker::decode_instance(bytes, 0, schema))
            }
        }
    };
}

simple_decoder!(RoomDecoder, "Room");
simple_decoder!(AreaDecoder, "Area");
simple_decoder!(SpaceDecoder, "Space");

/// Shared typed view for Room/Area/Space — same shape, same code path.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Zone {
    pub name: Option<String>,
    pub number: Option<String>,
    pub level_id: Option<u32>,
    pub upper_limit_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    pub upper_offset_feet: Option<f64>,
    pub area_square_feet: Option<f64>,
    pub volume_cubic_feet: Option<f64>,
}

impl Zone {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("number" | "roomnumber", InstanceField::String(s)) => {
                    out.number = Some(s.clone());
                }
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("upperlimitid" | "upperlevelid", InstanceField::ElementId { id, .. }) => {
                    out.upper_limit_id = Some(*id);
                }
                ("baseoffset", InstanceField::Float { value, .. }) => {
                    out.base_offset_feet = Some(*value);
                }
                ("upperoffset" | "limitoffset", InstanceField::Float { value, .. }) => {
                    out.upper_offset_feet = Some(*value);
                }
                ("area", InstanceField::Float { value, .. }) => {
                    out.area_square_feet = Some(*value);
                }
                ("volume", InstanceField::Float { value, .. }) => {
                    out.volume_cubic_feet = Some(*value);
                }
                _ => {}
            }
        }
        out
    }

    /// Human-readable label — "Number: Name" or just Name/Number if
    /// only one is present.
    pub fn label(&self) -> Option<String> {
        match (&self.number, &self.name) {
            (Some(n), Some(name)) => Some(format!("{n}: {name}")),
            (Some(n), None) => Some(n.clone()),
            (None, Some(name)) => Some(name.clone()),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_room_schema() -> ClassEntry {
        let f64_prim = FieldType::Primitive {
            kind: 0x07,
            size: 8,
        };
        ClassEntry {
            name: "Room".into(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_name".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_number".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_level_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_area".into(),
                    cpp_type: None,
                    field_type: Some(f64_prim),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(4),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_room_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        // m_name = "Kitchen"
        let name = "Kitchen";
        b.extend_from_slice(&(name.chars().count() as u32).to_le_bytes());
        for ch in name.encode_utf16() {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        // m_number = "101"
        let num = "101";
        b.extend_from_slice(&(num.chars().count() as u32).to_le_bytes());
        for ch in num.encode_utf16() {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        // m_level_id = [0, 7]
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&7u32.to_le_bytes());
        // m_area = 150.5 sqft
        b.extend_from_slice(&150.5_f64.to_le_bytes());
        b
    }

    #[test]
    fn room_rejects_wrong_schema() {
        let wrong = ClassEntry {
            name: "Wall".into(),
            ..synth_room_schema()
        };
        assert!(
            RoomDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn room_decodes_name_number_area() {
        let decoded = RoomDecoder
            .decode(
                &synth_room_bytes(),
                &synth_room_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let z = Zone::from_decoded(&decoded);
        assert_eq!(z.name.as_deref(), Some("Kitchen"));
        assert_eq!(z.number.as_deref(), Some("101"));
        assert_eq!(z.level_id, Some(7));
        assert_eq!(z.area_square_feet, Some(150.5));
        assert_eq!(z.label().as_deref(), Some("101: Kitchen"));
    }

    #[test]
    fn label_fallbacks() {
        let name_only = Zone {
            name: Some("Vestibule".into()),
            ..Default::default()
        };
        let number_only = Zone {
            number: Some("B1".into()),
            ..Default::default()
        };
        let empty = Zone::default();
        assert_eq!(name_only.label().as_deref(), Some("Vestibule"));
        assert_eq!(number_only.label().as_deref(), Some("B1"));
        assert_eq!(empty.label(), None);
    }

    #[test]
    fn zone_tolerates_empty() {
        let empty = DecodedElement {
            id: None,
            class: "Space".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let z = Zone::from_decoded(&empty);
        assert!(z.name.is_none() && z.area_square_feet.is_none());
    }

    #[test]
    fn area_and_space_decoders_work_on_same_view() {
        // All three classes decode into the same `Zone` type.
        let a_empty = DecodedElement {
            id: None,
            class: "Area".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let s_empty = DecodedElement {
            id: None,
            class: "Space".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let _a = Zone::from_decoded(&a_empty);
        let _s = Zone::from_decoded(&s_empty);
    }

    #[test]
    fn class_names() {
        assert_eq!(RoomDecoder.class_name(), "Room");
        assert_eq!(AreaDecoder.class_name(), "Area");
        assert_eq!(SpaceDecoder.class_name(), "Space");
    }
}
