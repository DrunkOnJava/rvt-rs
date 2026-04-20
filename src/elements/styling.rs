//! Material / FillPattern / LinePattern / LineStyle — Revit's
//! visual styling vocabulary. Every renderable element references at
//! least one of these for its appearance.
//!
//! Grouped in one module because they're small + closely related; the
//! usual pattern is one decoder per file, but these three are always
//! consulted together at render time so co-locating them keeps
//! relevant context together for contributors.

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

simple_decoder!(MaterialDecoder, "Material");
simple_decoder!(FillPatternDecoder, "FillPattern");
simple_decoder!(LinePatternDecoder, "LinePattern");
simple_decoder!(LineStyleDecoder, "LineStyle");

/// Typed view of a decoded Material.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Material {
    pub name: Option<String>,
    /// Packed RGB, low byte = R.
    pub color: Option<u32>,
    /// 0..1 — 0 is fully opaque.
    pub transparency: Option<f64>,
    /// 0..1 — surface roughness (1/shininess).
    pub shininess: Option<f64>,
    /// IDs of per-view appearance / physical / thermal assets.
    pub appearance_asset_id: Option<u32>,
    pub physical_asset_id: Option<u32>,
    pub thermal_asset_id: Option<u32>,
    /// Surface + cut fill pattern IDs.
    pub surface_pattern_id: Option<u32>,
    pub cut_pattern_id: Option<u32>,
}

impl Material {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("color", InstanceField::Integer { value, .. }) => out.color = Some(*value as u32),
                ("transparency", InstanceField::Float { value, .. }) => {
                    out.transparency = Some(*value);
                }
                ("shininess", InstanceField::Float { value, .. }) => {
                    out.shininess = Some(*value);
                }
                ("appearanceassetid", InstanceField::ElementId { id, .. }) => {
                    out.appearance_asset_id = Some(*id);
                }
                ("physicalassetid", InstanceField::ElementId { id, .. }) => {
                    out.physical_asset_id = Some(*id);
                }
                ("thermalassetid", InstanceField::ElementId { id, .. }) => {
                    out.thermal_asset_id = Some(*id);
                }
                ("surfacepatternid", InstanceField::ElementId { id, .. }) => {
                    out.surface_pattern_id = Some(*id);
                }
                ("cutpatternid", InstanceField::ElementId { id, .. }) => {
                    out.cut_pattern_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }

    /// Decompose packed color into (R, G, B).
    pub fn rgb(&self) -> Option<(u8, u8, u8)> {
        let c = self.color?;
        Some((
            (c & 0xFF) as u8,
            ((c >> 8) & 0xFF) as u8,
            ((c >> 16) & 0xFF) as u8,
        ))
    }
}

/// Typed view of a decoded FillPattern (surface or cut hatch).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillPattern {
    pub name: Option<String>,
    /// True when the pattern is oriented to the host element's
    /// local coordinate system (e.g. brick courses follow wall
    /// direction); false for drafting patterns that stay world-aligned.
    pub is_model_pattern: Option<bool>,
}

impl FillPattern {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("ismodelpattern", InstanceField::Bool(b)) => {
                    out.is_model_pattern = Some(*b);
                }
                _ => {}
            }
        }
        out
    }
}

/// Typed view of a decoded LinePattern (dash/dot pattern for lines).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LinePattern {
    pub name: Option<String>,
    /// Dash/space sequence in model units. Positive = dash, negative
    /// = gap, 0 = dot. Schema exposes this as a vector field; when
    /// that decodes land we'll populate it, for now keep the field
    /// for future use.
    pub pattern: Option<Vec<f64>>,
}

impl LinePattern {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        // Only `name` is decodable today; the pattern-sequence field
        // is a Vector<f64> which needs the walker's Vector variant to
        // land before it can be read. Keep the struct shape stable
        // so callers don't break when that landing happens.
        for (field_name, value) in &decoded.fields {
            if let ("name", InstanceField::String(s)) =
                (normalise_field_name(field_name).as_str(), value)
            {
                out.name = Some(s.clone());
            }
        }
        out
    }
}

/// Typed view of a decoded LineStyle.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineStyle {
    pub name: Option<String>,
    pub weight: Option<u32>,
    /// Packed RGB.
    pub color: Option<u32>,
    pub pattern_id: Option<u32>,
}

impl LineStyle {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("weight" | "lineweight", InstanceField::Integer { value, .. }) => {
                    out.weight = Some(*value as u32);
                }
                ("color", InstanceField::Integer { value, .. }) => out.color = Some(*value as u32),
                ("patternid" | "linepatternid", InstanceField::ElementId { id, .. }) => {
                    out.pattern_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{ClassEntry, FieldEntry, FieldType};

    fn synth_material_schema() -> ClassEntry {
        ClassEntry {
            name: "Material".to_string(),
            offset: 0,
            fields: vec![
                FieldEntry {
                    name: "m_name".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::String),
                },
                FieldEntry {
                    name: "m_color".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    }),
                },
                FieldEntry {
                    name: "m_transparency".to_string(),
                    cpp_type: None,
                    field_type: Some(FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    }),
                },
            ],
            tag: Some(1),
            parent: None,
            declared_field_count: Some(3),
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    fn synth_material_bytes() -> Vec<u8> {
        let mut b = Vec::new();
        let name = "Concrete";
        b.extend_from_slice(&(name.chars().count() as u32).to_le_bytes());
        for ch in name.encode_utf16() {
            b.extend_from_slice(&ch.to_le_bytes());
        }
        b.extend_from_slice(&0x00ABCDEFu32.to_le_bytes()); // color
        b.extend_from_slice(&0.25_f64.to_le_bytes()); // transparency
        b
    }

    #[test]
    fn material_decoder_roundtrip() {
        let decoded = MaterialDecoder
            .decode(
                &synth_material_bytes(),
                &synth_material_schema(),
                &HandleIndex::new(),
            )
            .unwrap();
        let m = Material::from_decoded(&decoded);
        assert_eq!(m.name.as_deref(), Some("Concrete"));
        assert_eq!(m.color, Some(0x00ABCDEF));
        assert!((m.transparency.unwrap() - 0.25).abs() < 1e-9);
    }

    #[test]
    fn material_rgb_decomposition() {
        let m = Material {
            color: Some(0x00112233),
            ..Default::default()
        };
        assert_eq!(m.rgb(), Some((0x33, 0x22, 0x11)));
    }

    #[test]
    fn all_four_decoders_reject_wrong_schema() {
        let wrong_schemas_and_decoders: Vec<(&str, &dyn ElementDecoder)> = vec![
            ("NotMaterial", &MaterialDecoder),
            ("NotFillPattern", &FillPatternDecoder),
            ("NotLinePattern", &LinePatternDecoder),
            ("NotLineStyle", &LineStyleDecoder),
        ];
        for (schema_name, decoder) in wrong_schemas_and_decoders {
            let schema = ClassEntry {
                name: schema_name.to_string(),
                offset: 0,
                fields: vec![],
                tag: None,
                parent: None,
                declared_field_count: None,
                was_parent_only: false,
                ancestor_tag: None,
            };
            assert!(
                decoder.decode(&[], &schema, &HandleIndex::new()).is_err(),
                "{} should reject schema '{}'",
                decoder.class_name(),
                schema_name
            );
        }
    }

    #[test]
    fn decoder_class_names() {
        assert_eq!(MaterialDecoder.class_name(), "Material");
        assert_eq!(FillPatternDecoder.class_name(), "FillPattern");
        assert_eq!(LinePatternDecoder.class_name(), "LinePattern");
        assert_eq!(LineStyleDecoder.class_name(), "LineStyle");
    }

    #[test]
    fn typed_views_tolerate_empty_decoded() {
        let empty = DecodedElement {
            id: None,
            class: "Any".to_string(),
            fields: vec![],
            byte_range: 0..0,
        };
        let m = Material::from_decoded(&empty);
        let f = FillPattern::from_decoded(&empty);
        let l = LinePattern::from_decoded(&empty);
        let s = LineStyle::from_decoded(&empty);
        assert!(m.name.is_none() && m.color.is_none());
        assert!(f.name.is_none() && f.is_model_pattern.is_none());
        assert!(l.name.is_none() && l.pattern.is_none());
        assert!(s.name.is_none() && s.weight.is_none());
    }
}
