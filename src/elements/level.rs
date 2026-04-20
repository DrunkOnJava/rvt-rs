//! `Level` — Revit class representing a floor level (e.g. "Level 1",
//! "Ground Floor", "Roof"). One of the simplest non-trivial elements
//! to decode, which makes it the reference example for
//! [`crate::walker::ElementDecoder`] implementations.
//!
//! Typical Revit field shape (names stable 2016–2026):
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Display name, e.g. "Level 1" |
//! | `m_elevation` | Primitive f64 | Height above project base, in feet |
//! | `m_levelTypeId` | ElementIdRef | Reference to the LevelType |
//! | `m_isBuildingStory` | Primitive bool | `true` when this level participates in storey count |
//!
//! Schema may include additional fields (e.g. `m_scopeBoxId`, visibility
//! hints) that are version-dependent; the typed struct captures only
//! the stable semantic subset. Raw fields remain available via the
//! underlying `DecodedElement.fields` vector for callers that need
//! them.

use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Registered [`ElementDecoder`] for the `Level` class.
pub struct LevelDecoder;

impl ElementDecoder for LevelDecoder {
    fn class_name(&self) -> &'static str {
        "Level"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Level" {
            return Err(Error::BasicFileInfo(format!(
                "LevelDecoder received wrong schema: {}",
                schema.name
            )));
        }
        // Use the generic decoder to produce field-by-field
        // InstanceField values. This handles the byte-level walk +
        // short-input safety. We then pattern-match into typed
        // values where we recognise the field names + shapes.
        let decoded = crate::walker::decode_instance(bytes, 0, schema);
        Ok(decoded)
    }
}

/// Typed view of a decoded Level. Convenience wrapper on top of
/// [`DecodedElement`]; call [`LevelDecoder::decode`] first, then
/// [`Level::from_decoded`] to project into this struct.
#[derive(Debug, Clone, PartialEq)]
pub struct Level {
    pub name: Option<String>,
    pub elevation_feet: Option<f64>,
    pub level_type_id: Option<u32>,
    pub is_building_story: Option<bool>,
}

impl Level {
    /// Extract the typed `Level` view from a generic `DecodedElement`.
    /// Missing or wrong-typed fields land as `None` — callers that
    /// need strict "all fields present" semantics should check each.
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self {
            name: None,
            elevation_feet: None,
            level_type_id: None,
            is_building_story: None,
        };
        for (field_name, value) in &decoded.fields {
            // Revit uses both camelCase and snake_case in schema
            // depending on version. Match both + tolerate m_ prefix.
            let normalised = normalise_field_name(field_name);
            match (normalised.as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("elevation", InstanceField::Float { value, .. }) => {
                    out.elevation_feet = Some(*value);
                }
                ("leveltypeid", InstanceField::ElementId { id, .. }) => {
                    out.level_type_id = Some(*id);
                }
                ("isbuildingstory", InstanceField::Bool(b)) => {
                    out.is_building_story = Some(*b);
                }
                _ => {}
            }
        }
        out
    }
}

/// Normalise a Revit field name: strip `m_` prefix, lowercase,
/// drop underscores. `m_LevelTypeId` / `m_level_type_id` /
/// `levelTypeId` all collapse to `"leveltypeid"`.
///
/// Exposed pub(crate) so other `ElementDecoder` implementations
/// in this crate can reuse the canonical form — Revit schema
/// field-name casing varies across versions, so every decoder
/// needs this normalisation pass before pattern-matching.
pub(crate) fn normalise_field_name(name: &str) -> String {
    let stripped = name.strip_prefix("m_").unwrap_or(name);
    stripped
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_level_schema() -> ClassEntry {
        ClassEntry {
            name: "Level".to_string(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_name".to_string(),
                    cpp_type: Some("String".into()),
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_elevation".to_string(),
                    cpp_type: Some("double".into()),
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
                FieldEntry {
                    name: "m_levelTypeId".to_string(),
                    cpp_type: Some("ElementId".into()),
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_isBuildingStory".to_string(),
                    cpp_type: Some("bool".into()),
                    field_type: Some(FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
            ],
            tag: Some(0x1234),
            parent: None,
            declared_field_count: Some(4),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_level_bytes() -> Vec<u8> {
        // Matches synth_level_schema in field order.
        let mut bytes = Vec::new();

        // m_name: UTF-16LE length-prefixed "Level 1"
        let name = "Level 1";
        bytes.extend_from_slice(&(name.chars().count() as u32).to_le_bytes());
        for ch in name.encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }

        // m_elevation: f64 = 10.0 feet
        bytes.extend_from_slice(&10.0_f64.to_le_bytes());

        // m_levelTypeId: [u32 tag=0][u32 id=42]
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&42u32.to_le_bytes());

        // m_isBuildingStory: bool=true
        bytes.push(1);

        bytes
    }

    #[test]
    fn level_decoder_rejects_wrong_schema() {
        let wrong_schema = ClassEntry {
            name: "Wall".to_string(),
            ..synth_level_schema()
        };
        let decoder = LevelDecoder;
        let result = decoder.decode(&[], &wrong_schema, &HandleIndex::new());
        assert!(result.is_err());
    }

    #[test]
    fn level_decoder_decodes_all_fields() {
        let schema = synth_level_schema();
        let bytes = synth_level_bytes();
        let decoder = LevelDecoder;
        let decoded = decoder
            .decode(&bytes, &schema, &HandleIndex::new())
            .unwrap();
        assert_eq!(decoded.class, "Level");
        assert_eq!(decoded.fields.len(), 4);
    }

    #[test]
    fn level_from_decoded_projects_typed_view() {
        let schema = synth_level_schema();
        let bytes = synth_level_bytes();
        let decoder = LevelDecoder;
        let decoded = decoder
            .decode(&bytes, &schema, &HandleIndex::new())
            .unwrap();
        let level = Level::from_decoded(&decoded);
        assert_eq!(level.name.as_deref(), Some("Level 1"));
        assert_eq!(level.elevation_feet, Some(10.0));
        assert_eq!(level.level_type_id, Some(42));
        assert_eq!(level.is_building_story, Some(true));
    }

    #[test]
    fn level_from_decoded_tolerates_missing_fields() {
        // DecodedElement with no fields → all Level fields None.
        let decoded = DecodedElement {
            id: None,
            class: "Level".to_string(),
            fields: vec![],
            byte_range: 0..0,
        };
        let level = Level::from_decoded(&decoded);
        assert!(level.name.is_none());
        assert!(level.elevation_feet.is_none());
        assert!(level.level_type_id.is_none());
        assert!(level.is_building_story.is_none());
    }

    #[test]
    fn normalise_field_name_variants() {
        assert_eq!(normalise_field_name("m_LevelTypeId"), "leveltypeid");
        assert_eq!(normalise_field_name("m_level_type_id"), "leveltypeid");
        assert_eq!(normalise_field_name("levelTypeId"), "leveltypeid");
        assert_eq!(normalise_field_name("name"), "name");
        assert_eq!(normalise_field_name("m_name"), "name");
    }

    #[test]
    fn level_decoder_class_name_stable() {
        assert_eq!(LevelDecoder.class_name(), "Level");
    }
}
