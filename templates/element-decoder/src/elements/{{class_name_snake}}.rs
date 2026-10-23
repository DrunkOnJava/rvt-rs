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
use crate::walker::{DecodedElement, InstanceField};

/// Typed view of a decoded `{{class_name}}`. Convenience wrapper on
/// top of [`DecodedElement`]: decode the instance with
/// `SimpleDecoder::new("{{class_name}}")`, then call
/// [`{{class_name}}::from_decoded`] to project it into this struct.
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
