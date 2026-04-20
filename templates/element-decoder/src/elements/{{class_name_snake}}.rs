//! `{{class_name}}` — {{struct_description}}
//!
//! TODO: document field shape from real fixture. Run
//! `cargo run --release --bin rvt-schema -- _corpus/<file>.rfa | grep -A 20 "class {{class_name}}"`
//! and record the fields below once the class is confirmed in the
//! schema dump.
//!
//! # Typical Revit field shape (stable 2016–2026)
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | TODO — usually present on named elements |
//! | `m_TODO` | TODO | TODO |
//!
//! Schema may include additional fields that are version-dependent;
//! the typed struct captures only the stable semantic subset. Raw
//! fields remain available via the underlying `DecodedElement.fields`
//! vector for callers that need them.

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

simple_decoder!({{class_name}}Decoder, "{{class_name}}");

/// Typed view of a decoded `{{class_name}}`. Convenience wrapper on
/// top of [`DecodedElement`]; call [`{{class_name}}Decoder::decode`]
/// first, then [`{{class_name}}::from_decoded`] to project into this
/// struct.
///
/// TODO: extend with class-specific fields. Add one `Option<T>` per
/// field you care about, then pattern-match in `from_decoded` below.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct {{class_name}} {
    pub name: Option<String>,
    // TODO: add typed fields, e.g.:
    // pub foo: Option<i64>,
    // pub bar: Option<f64>,
    // pub some_id: Option<u32>,
}

impl {{class_name}} {
    /// Extract the typed `{{class_name}}` view from a generic
    /// `DecodedElement`. Missing or wrong-typed fields land as `None`
    /// — callers that need strict "all fields present" semantics
    /// should check each.
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                // TODO: add further patterns, e.g.:
                // ("foo", InstanceField::Integer { value, .. }) => out.foo = Some(*value),
                // ("bar", InstanceField::Float { value, .. }) => out.bar = Some(*value),
                // ("someid", InstanceField::ElementId { id, .. }) => out.some_id = Some(*id),
                _ => {}
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::ClassEntry;

    fn wrong_schema() -> ClassEntry {
        ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    #[test]
    fn {{class_name_snake}}_rejects_wrong_schema() {
        assert!(
            {{class_name}}Decoder
                .decode(&[], &wrong_schema(), &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn {{class_name_snake}}_from_decoded() {
        let fields = vec![(
            "m_name".into(),
            InstanceField::String("test-{{class_name_snake}}".into()),
        )];
        let decoded = DecodedElement {
            id: None,
            class: "{{class_name}}".into(),
            fields,
            byte_range: 0..0,
        };
        let v = {{class_name}}::from_decoded(&decoded);
        assert_eq!(v.name.as_deref(), Some("test-{{class_name_snake}}"));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "{{class_name}}".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        let v = {{class_name}}::from_decoded(&empty);
        assert!(v.name.is_none());
    }

    #[test]
    fn class_name() {
        assert_eq!({{class_name}}Decoder.class_name(), "{{class_name}}");
    }
}
