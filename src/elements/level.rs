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
#[derive(Debug, Clone, PartialEq, Default)]
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
                // `m_height` appears on gen-fixture Levels; real files
                // use `m_elevation`. Accept both so scaffold fixtures
                // project a typed elevation without inventing fields.
                ("elevation" | "height", InstanceField::Float { value, .. }) => {
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
