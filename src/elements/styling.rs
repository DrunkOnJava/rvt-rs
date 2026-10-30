//! `Material` — Revit's appearance vocabulary. Every renderable
//! element references one for its appearance.

use super::level::normalise_field_name;
use crate::walker::{DecodedElement, InstanceField};

/// Typed view of a decoded Material.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Material {
    pub name: Option<String>,
    /// Packed RGB, low byte = R.
    pub color: Option<u32>,
    /// 0..1 — 0 is fully opaque.
    pub transparency: Option<f64>,
    /// 0..1 — surface roughness (1/shininess).
    pub shininess: Option<f64>,
    /// IDs of per-view appearance / physical / thermal assets.
    pub appearance_asset_id: Option<crate::ElementId>,
    pub physical_asset_id: Option<crate::ElementId>,
    pub thermal_asset_id: Option<crate::ElementId>,
    /// Surface + cut fill pattern IDs.
    pub surface_pattern_id: Option<crate::ElementId>,
    pub cut_pattern_id: Option<crate::ElementId>,
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
                    out.appearance_asset_id = Some(crate::ElementId::from(*id));
                }
                ("physicalassetid", InstanceField::ElementId { id, .. }) => {
                    out.physical_asset_id = Some(crate::ElementId::from(*id));
                }
                ("thermalassetid", InstanceField::ElementId { id, .. }) => {
                    out.thermal_asset_id = Some(crate::ElementId::from(*id));
                }
                ("surfacepatternid", InstanceField::ElementId { id, .. }) => {
                    out.surface_pattern_id = Some(crate::ElementId::from(*id));
                }
                ("cutpatternid", InstanceField::ElementId { id, .. }) => {
                    out.cut_pattern_id = Some(crate::ElementId::from(*id));
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
