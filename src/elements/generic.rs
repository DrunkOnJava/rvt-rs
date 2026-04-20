//! `GenericModel` + `Mass` — the "everything else" classes.
//!
//! - **GenericModel** hosts custom Revit families that don't fit any
//!   built-in category (Entourage, custom site furniture, mock-up
//!   geometry). All three IFC exporters (Autodesk's, Graphisoft's,
//!   and ours) route these through `IfcBuildingElementProxy`.
//! - **Mass** is Revit's conceptual-massing object — abstract volumes
//!   used in early-phase design before they get refined into walls /
//!   slabs / roofs. Also maps to `IfcBuildingElementProxy` with a
//!   `USERDEFINED` predefined type.
//!
//! Both share the same placement + symbol field shape.
//!
//! # Typical field shape (stable 2016–2026)
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | Optional host element |
//! | `m_symbol_id` | ElementId | FamilySymbol |
//! | `m_location_x`, `_y`, `_z` | f64 | Insertion point |
//! | `m_rotation` | f64 | Rotation radians |
//! | `m_category_name` | String | Designer-supplied category label for GenericModel |

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

simple_decoder!(GenericModelDecoder, "GenericModel");
simple_decoder!(MassDecoder, "Mass");

/// Common projection shared by GenericModel + Mass. Same shape as
/// `OpeningCommon` / `PlacedCommon` in sibling modules.
#[derive(Debug, Clone, Copy, Default)]
struct FamilyCommon {
    location: Option<Point3>,
    rotation_radians: Option<f64>,
    level_id: Option<u32>,
    host_id: Option<u32>,
    symbol_id: Option<u32>,
}

fn collect_common(decoded: &DecodedElement) -> FamilyCommon {
    let mut lx = None;
    let mut ly = None;
    let mut lz = None;
    let mut out = FamilyCommon::default();
    for (field_name, value) in &decoded.fields {
        match (normalise_field_name(field_name).as_str(), value) {
            ("locationx", InstanceField::Float { value, .. }) => lx = Some(*value),
            ("locationy", InstanceField::Float { value, .. }) => ly = Some(*value),
            ("locationz", InstanceField::Float { value, .. }) => lz = Some(*value),
            ("rotation", InstanceField::Float { value, .. }) => {
                out.rotation_radians = Some(*value);
            }
            ("levelid" | "hostlevelid", InstanceField::ElementId { id, .. }) => {
                out.level_id = Some(*id);
            }
            ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
            ("symbolid" | "typeid" | "familysymbolid", InstanceField::ElementId { id, .. }) => {
                out.symbol_id = Some(*id);
            }
            _ => {}
        }
    }
    if let (Some(x), Some(y), Some(z)) = (lx, ly, lz) {
        out.location = Some(Point3::new(x, y, z));
    }
    out
}

/// Typed view for GenericModel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GenericModel {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
    pub category_name: Option<String>,
}

impl GenericModel {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        let mut out = Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
            rotation_radians: c.rotation_radians,
            category_name: None,
        };
        for (field_name, value) in &decoded.fields {
            if let ("categoryname" | "category", InstanceField::String(s)) =
                (normalise_field_name(field_name).as_str(), value)
            {
                out.category_name = Some(s.clone());
            }
        }
        out
    }
}

/// Typed view for Mass.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mass {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
}

impl Mass {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
            rotation_radians: c.rotation_radians,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_model_rejects_wrong_schema() {
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
            GenericModelDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn generic_model_from_decoded() {
        let fields = vec![
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
            (
                "m_category_name".into(),
                InstanceField::String("Entourage".into()),
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "GenericModel".into(),
            fields,
            byte_range: 0..0,
        };
        let g = GenericModel::from_decoded(&decoded);
        assert_eq!(g.location, Some(Point3::new(5.0, 10.0, 0.0)));
        assert_eq!(g.category_name.as_deref(), Some("Entourage"));
    }

    #[test]
    fn mass_from_decoded() {
        let fields = vec![
            (
                "m_location_x".into(),
                InstanceField::Float {
                    value: 100.0,
                    size: 8,
                },
            ),
            (
                "m_location_y".into(),
                InstanceField::Float {
                    value: 200.0,
                    size: 8,
                },
            ),
            (
                "m_location_z".into(),
                InstanceField::Float {
                    value: 50.0,
                    size: 8,
                },
            ),
            (
                "m_symbol_id".into(),
                InstanceField::ElementId { tag: 0, id: 42 },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Mass".into(),
            fields,
            byte_range: 0..0,
        };
        let m = Mass::from_decoded(&decoded);
        assert_eq!(m.location, Some(Point3::new(100.0, 200.0, 50.0)));
        assert_eq!(m.symbol_id, Some(42));
    }

    #[test]
    fn empty_tolerance() {
        let empty = DecodedElement {
            id: None,
            class: "Mass".into(),
            fields: vec![],
            byte_range: 0..0,
        };
        assert!(GenericModel::from_decoded(&empty).location.is_none());
        assert!(Mass::from_decoded(&empty).location.is_none());
    }

    #[test]
    fn class_names() {
        assert_eq!(GenericModelDecoder.class_name(), "GenericModel");
        assert_eq!(MassDecoder.class_name(), "Mass");
    }
}
