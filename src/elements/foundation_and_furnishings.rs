//! `StructuralFoundation` + `Furniture` + `FurnitureSystem` +
//! `Casework` + `Rebar` — the "equipment and reinforcement" tier of
//! elements. All share the generic FamilyInstance placement shape,
//! so we re-use the common location+host pattern.
//!
//! # Field shape
//!
//! StructuralFoundation (mapping: `IfcFooting`):
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level (top of footing) |
//! | `m_host_id` | ElementId | Usually the column above; 0 for isolated footings |
//! | `m_symbol_id` | ElementId | FamilySymbol (size + material) |
//! | `m_location_x`, `_y`, `_z` | f64 | Insertion point |
//!
//! Furniture / FurnitureSystem / Casework (mapping: `IfcFurniture`):
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Host level |
//! | `m_host_id` | ElementId | 0 for free-standing; wall ID for wall-hosted |
//! | `m_symbol_id` | ElementId | Family symbol |
//! | `m_location_x`, `_y`, `_z` | f64 | Insertion point |
//! | `m_rotation` | f64 | Rotation (radians) |
//!
//! Rebar (mapping: `IfcReinforcingBar`):
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_host_id` | ElementId | The element the rebar is embedded in |
//! | `m_quantity` | Primitive u32 | Number of bars in the set |
//! | `m_bar_type_id` | ElementId | Rebar type (diameter + steel spec) |
//! | `m_length` | f64 | Per-bar length in feet |

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

simple_decoder!(StructuralFoundationDecoder, "StructuralFoundation");
simple_decoder!(FurnitureDecoder, "Furniture");
simple_decoder!(FurnitureSystemDecoder, "FurnitureSystem");
simple_decoder!(CaseworkDecoder, "Casework");
simple_decoder!(RebarDecoder, "Rebar");

/// Fields shared by the foundation + furnishing decoders (all are
/// FamilyInstance subtypes with the same placement + host shape).
#[derive(Debug, Clone, Copy, Default)]
struct PlacedCommon {
    location: Option<Point3>,
    rotation_radians: Option<f64>,
    level_id: Option<u32>,
    host_id: Option<u32>,
    symbol_id: Option<u32>,
}

fn collect_common(decoded: &DecodedElement) -> PlacedCommon {
    let mut lx = None;
    let mut ly = None;
    let mut lz = None;
    let mut out = PlacedCommon::default();
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

/// Typed view for StructuralFoundation.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StructuralFoundation {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
}

impl StructuralFoundation {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let c = collect_common(decoded);
        Self {
            level_id: c.level_id,
            host_id: c.host_id,
            symbol_id: c.symbol_id,
            location: c.location,
        }
    }

    /// Isolated footing (column footing) when no host is specified;
    /// strip/mat footings will have a host wall/slab.
    pub fn is_isolated(&self) -> bool {
        matches!(self.host_id, None | Some(0))
    }
}

/// Typed view for Furniture / FurnitureSystem / Casework.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Furnishing {
    pub level_id: Option<u32>,
    pub host_id: Option<u32>,
    pub symbol_id: Option<u32>,
    pub location: Option<Point3>,
    pub rotation_radians: Option<f64>,
}

impl Furnishing {
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

    pub fn is_wall_hosted(&self) -> bool {
        !matches!(self.host_id, None | Some(0))
    }
}

/// Typed view for Rebar.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rebar {
    pub host_id: Option<u32>,
    pub bar_type_id: Option<u32>,
    pub quantity: Option<u32>,
    pub length_feet: Option<f64>,
}

impl Rebar {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                ("bartypeid" | "typeid", InstanceField::ElementId { id, .. }) => {
                    out.bar_type_id = Some(*id);
                }
                ("quantity", InstanceField::Integer { value, .. }) => {
                    out.quantity = Some(*value as u32);
                }
                ("length", InstanceField::Float { value, .. }) => {
                    out.length_feet = Some(*value);
                }
                _ => {}
            }
        }
        out
    }

    /// Total linear footage = quantity × per-bar length.
    pub fn total_length_feet(&self) -> Option<f64> {
        Some(self.quantity? as f64 * self.length_feet?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundation_rejects_wrong_schema() {
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
            StructuralFoundationDecoder
                .decode(&[], &wrong, &HandleIndex::new())
                .is_err()
        );
    }

    #[test]
    fn foundation_isolated_detection() {
        let isolated = StructuralFoundation {
            host_id: Some(0),
            ..Default::default()
        };
        let attached = StructuralFoundation {
            host_id: Some(42),
            ..Default::default()
        };
        assert!(isolated.is_isolated());
        assert!(!attached.is_isolated());
        assert!(StructuralFoundation::default().is_isolated());
    }

    #[test]
    fn furnishing_host_detection() {
        let free = Furnishing {
            host_id: Some(0),
            ..Default::default()
        };
        let hosted = Furnishing {
            host_id: Some(7),
            ..Default::default()
        };
        assert!(!free.is_wall_hosted());
        assert!(hosted.is_wall_hosted());
        assert!(!Furnishing::default().is_wall_hosted());
    }

    #[test]
    fn rebar_total_length() {
        let r = Rebar {
            quantity: Some(20),
            length_feet: Some(12.5),
            ..Default::default()
        };
        assert_eq!(r.total_length_feet(), Some(250.0));
        let missing = Rebar {
            quantity: Some(5),
            length_feet: None,
            ..Default::default()
        };
        assert_eq!(missing.total_length_feet(), None);
    }

    #[test]
    fn furnishing_from_fields() {
        let fields = vec![
            (
                "m_location_x".into(),
                InstanceField::Float {
                    value: 1.0,
                    size: 8,
                },
            ),
            (
                "m_location_y".into(),
                InstanceField::Float {
                    value: 2.0,
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
                "m_rotation".into(),
                InstanceField::Float {
                    value: std::f64::consts::FRAC_PI_2,
                    size: 8,
                },
            ),
            (
                "m_symbol_id".into(),
                InstanceField::ElementId { tag: 0, id: 99 },
            ),
        ];
        let decoded = DecodedElement {
            id: None,
            class: "Furniture".into(),
            fields,
            byte_range: 0..0,
        };
        let f = Furnishing::from_decoded(&decoded);
        assert_eq!(f.location, Some(Point3::new(1.0, 2.0, 0.0)));
        assert_eq!(f.symbol_id, Some(99));
        assert!((f.rotation_radians.unwrap() - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn class_names() {
        assert_eq!(
            StructuralFoundationDecoder.class_name(),
            "StructuralFoundation"
        );
        assert_eq!(FurnitureDecoder.class_name(), "Furniture");
        assert_eq!(FurnitureSystemDecoder.class_name(), "FurnitureSystem");
        assert_eq!(CaseworkDecoder.class_name(), "Casework");
        assert_eq!(RebarDecoder.class_name(), "Rebar");
    }
}
