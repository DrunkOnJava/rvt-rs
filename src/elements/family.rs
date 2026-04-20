//! `Symbol` + `FamilyInstance` — the two foundational classes of
//! Revit's family system. Every element that isn't a system family
//! (Door, Window, Furniture, Casework, Column, Beam, …) ultimately
//! bottoms out at a FamilyInstance whose shape is defined by a
//! Symbol, which is defined by a Family (.rfa file).
//!
//! # Class hierarchy (conceptual)
//!
//! ```text
//! Family           (the .rfa definition: "W-Shape", "Double-Hung Window")
//!   │
//!   └─ Symbol      (a concrete type: "W16x26", "2060 — 24x72")
//!        │
//!        └─ FamilyInstance  (a placed occurrence in the project)
//! ```
//!
//! We already decode FamilyInstance subclasses (Door, Window,
//! Column, Beam, Furniture…) directly. This module adds the
//! generic FamilyInstance + Symbol decoders so callers can resolve
//! type names and family-level properties for any element that
//! carries a `symbol_id`.
//!
//! # Typical field shape (stable 2016–2026)
//!
//! Symbol:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Type name ("W16x26", "2060 24x72") |
//! | `m_family_id` | ElementId | Parent Family |
//! | `m_category_id` | ElementId | Revit category ("OST_Walls", …) |
//! | `m_is_system_family` | Primitive bool | True for system families (Wall, Floor, …) |
//!
//! FamilyInstance:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_symbol_id` | ElementId | Type definition |
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Host element (0 for model-hosted) |
//! | `m_location_x`, `_y`, `_z` | f64 | Insertion point |
//! | `m_rotation` | f64 | Rotation (radians) |
//! | `m_mirror_x` / `m_mirror_y` | Primitive bool | Mirror flags |
//! | `m_scale` | f64 | Uniform scale (for generic models only) |

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::Point3;
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

simple_decoder!(SymbolDecoder, "Symbol");
simple_decoder!(FamilyInstanceDecoder, "FamilyInstance");

/// Typed view for Symbol (family type definition).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Symbol {
    pub name: Option<String>,
    pub family_id: Option<u32>,
    pub category_id: Option<u32>,
    pub is_system_family: Option<bool>,
}

impl Symbol {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("familyid", InstanceField::ElementId { id, .. }) => {
                    out.family_id = Some(*id);
                }
                ("categoryid", InstanceField::ElementId { id, .. }) => {
                    out.category_id = Some(*id);
                }
                ("issystemfamily" | "systemfamily", InstanceField::Bool(b)) => {
                    out.is_system_family = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// True for built-in system families (Wall, Floor, Roof, etc.)
    /// whose shape is defined in Revit's C++ code rather than a
    /// loadable .rfa. System families can't be swapped at runtime;
    /// loaded families can.
    pub fn is_loadable(&self) -> Option<bool> {
        self.is_system_family.map(|s| !s)
    }
}

/// Typed view for the generic FamilyInstance. Specific subtypes
/// (Door, Window, Column, …) have their own decoders that extract
/// extra scalar fields beyond this common base.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FamilyInstance {
    pub symbol_id: Option<u32>,
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    pub mirror_x: Option<bool>,
    pub mirror_y: Option<bool>,
    pub scale: Option<f64>,
}

impl FamilyInstance {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut lx = None;
        let mut ly = None;
        let mut lz = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("symbolid" | "typeid" | "familysymbolid", InstanceField::ElementId { id, .. }) => {
                    out.symbol_id = Some(*id);
                }
                ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                    out.level_id = Some(*id);
                }
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                ("locationx", InstanceField::Float { value, .. }) => lx = Some(*value),
                ("locationy", InstanceField::Float { value, .. }) => ly = Some(*value),
                ("locationz", InstanceField::Float { value, .. }) => lz = Some(*value),
                ("rotation", InstanceField::Float { value, .. }) => {
                    out.rotation_radians = Some(*value);
                }
                ("mirrorx", InstanceField::Bool(b)) => out.mirror_x = Some(*b),
                ("mirrory", InstanceField::Bool(b)) => out.mirror_y = Some(*b),
                ("scale", InstanceField::Float { value, .. }) => out.scale = Some(*value),
                _ => {}
            }
        }
        if let (Some(x), Some(y), Some(z)) = (lx, ly, lz) {
            out.location = Some(Point3::new(x, y, z));
        }
        out
    }

    /// True when either axis has been mirrored — a hint that the
    /// family's profile is reversed at render time.
    pub fn is_mirrored(&self) -> Option<bool> {
        match (self.mirror_x, self.mirror_y) {
            (Some(x), Some(y)) => Some(x ^ y),
            (Some(x), None) => Some(x),
            (None, Some(y)) => Some(y),
            (None, None) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbol_rejects_wrong_schema() {
        use crate::formats::ClassEntry;
        let wrong = ClassEntry {
            name: "Wall".into(),
            offset: 0,
            fields: vec![],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        assert!(
            SymbolDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn symbol_from_decoded() {
        let fields = vec![
            ("m_name".into(), InstanceField::String("W16x26".into())),
            (
                "m_family_id".into(),
                InstanceField::ElementId { tag: 0, id: 100 },
            ),
            ("m_is_system_family".into(), InstanceField::Bool(false)),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Symbol".into(),
            fields,
            byte_range: 0..0,
        };
        let s = Symbol::from_decoded(&decoded);
        assert_eq!(s.name.as_deref(), Some("W16x26"));
        assert_eq!(s.family_id, Some(100));
        assert_eq!(s.is_system_family, Some(false));
        assert_eq!(s.is_loadable(), Some(true));
    }

    #[test]
    fn symbol_loadable_fallback() {
        let unknown = Symbol::default();
        assert_eq!(unknown.is_loadable(), None);
        let system = Symbol {
            is_system_family: Some(true),
            ..Default::default()
        };
        assert_eq!(system.is_loadable(), Some(false));
    }

    #[test]
    fn family_instance_from_decoded() {
        let fields = vec![
            (
                "m_symbol_id".into(),
                InstanceField::ElementId { tag: 0, id: 42 },
            ),
            (
                "m_location_x".into(),
                InstanceField::Float {
                    value: 5.0,
                    size: 8,
                },
            ),
            (
                "m_location_y".into(),
                InstanceField::Float {
                    value: 10.0,
                    size: 8,
                },
            ),
            (
                "m_location_z".into(),
                InstanceField::Float {
                    value: 0.0,
                    size: 8,
                },
            ),
            ("m_mirror_x".into(), InstanceField::Bool(true)),
            ("m_mirror_y".into(), InstanceField::Bool(false)),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "FamilyInstance".into(),
            fields,
            byte_range: 0..0,
        };
        let fi = FamilyInstance::from_decoded(&decoded);
        assert_eq!(fi.symbol_id, Some(42));
        assert_eq!(fi.location, Some(Point3::new(5.0, 10.0, 0.0)));
        assert_eq!(fi.is_mirrored(), Some(true));
    }

    #[test]
    fn family_instance_double_mirror_is_identity() {
        let fi = FamilyInstance {
            mirror_x: Some(true),
            mirror_y: Some(true),
            ..Default::default()
        };
        assert_eq!(fi.is_mirrored(), Some(false));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "FamilyInstance".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(Symbol::from_decoded(&empty).name.is_none());
        assert!(FamilyInstance::from_decoded(&empty).symbol_id.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(SymbolDecoder.class_name(), "Symbol");
        assert_eq!(FamilyInstanceDecoder.class_name(), "FamilyInstance");
    }
}
