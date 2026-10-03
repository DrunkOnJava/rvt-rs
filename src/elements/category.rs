//! `Category` + `Subcategory` — Revit classes that organize elements
//! into a taxonomy (Walls, Doors, Floors, ...). Every renderable
//! element references a Category; categories reference Subcategories
//! for finer-grained styling (Core, Finish, Insulation for a wall).
//!
//! # Why these matter
//!
//! The IFC exporter needs Category to emit the right `IfcWall` vs
//! `IfcSlab` vs `IfcDoor` etc. (see `ifc::category_map`). The web
//! viewer uses Categories for the layer toggle panel — one checkbox
//! per Category.
//!
//! # Typical field shape (names stable 2016–2026)
//!
//! Category:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | Display name ("Walls", "Doors", ...) |
//! | `m_builtin` | Primitive u32 | Builtin category enum value (or 0) |
//! | `m_parent_id` | ElementId | Parent category reference (0 if top-level) |
//! | `m_is_cuttable` | Primitive bool | Whether elements can be cut in section views |
//!
//! Subcategory extends Category with:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_line_color` | Primitive u32 | AutoCAD-style packed RGB color |
//! | `m_line_weight_cut` | Primitive u32 | Line weight when cut |
//! | `m_line_weight_projection` | Primitive u32 | Line weight in projection |
//! | `m_line_pattern_id` | ElementId | Reference to LinePattern |
//! | `m_material_id` | ElementId | Default material |

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// Registered [`ElementDecoder`] for the `Category` class.
pub struct CategoryDecoder;

impl ElementDecoder for CategoryDecoder {
    fn class_name(&self) -> &'static str {
        "Category"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Category" {
            return Err(Error::BasicFileInfo(format!(
                "CategoryDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered [`ElementDecoder`] for the `Subcategory` class.
pub struct SubcategoryDecoder;

impl ElementDecoder for SubcategoryDecoder {
    fn class_name(&self) -> &'static str {
        "Subcategory"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Subcategory" {
            return Err(Error::BasicFileInfo(format!(
                "SubcategoryDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded Category.
#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub name: Option<String>,
    pub builtin_id: Option<i64>,
    pub parent_category_id: Option<u32>,
    pub is_cuttable: Option<bool>,
}

impl Category {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self {
            name: None,
            builtin_id: None,
            parent_category_id: None,
            is_cuttable: None,
        };
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("builtin" | "builtincategory", InstanceField::Integer { value, .. }) => {
                    out.builtin_id = Some(*value);
                }
                ("parentid" | "parentcategoryid", InstanceField::ElementId { id, .. }) => {
                    out.parent_category_id = Some(*id);
                }
                ("iscuttable", InstanceField::Bool(b)) => out.is_cuttable = Some(*b),
                _ => {}
            }
        }
        out
    }

    /// `true` when this category has no parent — i.e. it's a top-level
    /// category like "Walls" or "Doors" rather than a subdivision.
    pub fn is_top_level(&self) -> bool {
        matches!(self.parent_category_id, None | Some(0))
    }
}

/// Typed view of a decoded Subcategory.
#[derive(Debug, Clone, PartialEq)]
pub struct Subcategory {
    pub name: Option<String>,
    pub parent_category_id: Option<u32>,
    /// Packed RGB color `0xAABBGGRR` — low byte is R.
    pub line_color: Option<u32>,
    pub line_weight_cut: Option<u32>,
    pub line_weight_projection: Option<u32>,
    pub line_pattern_id: Option<u32>,
    pub material_id: Option<u32>,
}

impl Subcategory {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self {
            name: None,
            parent_category_id: None,
            line_color: None,
            line_weight_cut: None,
            line_weight_projection: None,
            line_pattern_id: None,
            material_id: None,
        };
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("parentcategoryid" | "parentid", InstanceField::ElementId { id, .. }) => {
                    out.parent_category_id = Some(*id)
                }
                ("linecolor", InstanceField::Integer { value, .. }) => {
                    out.line_color = Some(*value as u32);
                }
                ("lineweightcut", InstanceField::Integer { value, .. }) => {
                    out.line_weight_cut = Some(*value as u32);
                }
                ("lineweightprojection", InstanceField::Integer { value, .. }) => {
                    out.line_weight_projection = Some(*value as u32);
                }
                ("linepatternid", InstanceField::ElementId { id, .. }) => {
                    out.line_pattern_id = Some(*id);
                }
                ("materialid", InstanceField::ElementId { id, .. }) => {
                    out.material_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }

    /// Decompose the packed RGB color into (red, green, blue) bytes.
    /// Returns None when line_color wasn't decoded.
    pub fn rgb(&self) -> Option<(u8, u8, u8)> {
        let c = self.line_color?;
        Some((
            (c & 0xFF) as u8,
            ((c >> 8) & 0xFF) as u8,
            ((c >> 16) & 0xFF) as u8,
        ))
    }
}
