//! Revit material → PBR material mapping (VW1-06).
//!
//! Downstream viewers (Three.js, WebGL, glTF exporters) consume
//! materials in the physically-based-rendering (PBR) format that
//! the glTF 2.0 specification standardized:
//!
//! - Base color as linear RGBA, 0-1 range
//! - Metallic factor (0 for most dielectrics, 1 for bare metals)
//! - Roughness factor (0 = mirror, 1 = chalk)
//! - Alpha (0 = fully transparent, 1 = opaque)
//!
//! Revit materials (see [`crate::ifc::MaterialInfo`]) ship with a
//! packed `0x00BBGGRR` color and a 0-1 transparency. This module
//! translates that into a [`PbrMaterial`] with sensible defaults:
//! Revit doesn't carry PBR metadata natively, so heuristics based
//! on material name classify the output as metal, glass, wood,
//! concrete, or generic-dielectric.
//!
//! The mapping is deliberate: real PBR material authoring is a
//! design task, not a reverse-engineering task, so this module
//! picks reasonable values that make scenes render recognizably
//! in a viewer. Callers that want authoritative PBR should
//! override per-material from their own material library.

use super::MaterialInfo;
use serde::{Deserialize, Serialize};

/// A physically-based material ready for glTF 2.0 emission.
///
/// Field ranges match the glTF spec exactly so the serializer is
/// a straight copy — no extra math at emit time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PbrMaterial {
    /// Linear RGB, 0-1. Viewers apply gamma correction on display.
    pub base_color_rgb: [f32; 3],
    /// Alpha, 0-1. Source: Revit `transparency` (inverted — Revit
    /// 0 = opaque, PBR 0 = transparent).
    pub alpha: f32,
    /// 0 for dielectrics (wood, concrete, paint); 1 for bare metals.
    pub metallic: f32,
    /// 0 = mirror-smooth, 1 = chalky-matte. Heuristic-picked from
    /// material name.
    pub roughness: f32,
    /// Render as double-sided? glTF default is false; Revit glass
    /// and thin geometry benefit from true.
    pub double_sided: bool,
}

impl Default for PbrMaterial {
    /// A middle-ground dielectric — mid-grey, opaque, non-metallic,
    /// neither mirror nor chalk. Useful when the material's source
    /// data is entirely absent.
    fn default() -> Self {
        Self {
            base_color_rgb: [0.75, 0.75, 0.75],
            alpha: 1.0,
            metallic: 0.0,
            roughness: 0.6,
            double_sided: false,
        }
    }
}

impl PbrMaterial {
    /// Construct a PBR material from a decoded Revit
    /// [`MaterialInfo`]. Unpacks the `0x00BBGGRR` color, converts
    /// Revit's opaque-0 transparency to PBR's opaque-1 alpha, and
    /// applies a name-driven heuristic to pick metallic /
    /// roughness / double-sided.
    pub fn from_material_info(info: &MaterialInfo) -> Self {
        let base = color_from_packed(info.color_packed);
        let alpha = alpha_from_transparency(info.transparency);
        let (metallic, roughness, double_sided) = classify_from_name(&info.name);
        Self {
            base_color_rgb: base,
            alpha,
            metallic,
            roughness,
            double_sided,
        }
    }
}

/// Unpack Revit's `0x00BBGGRR` byte-order into a linear-space RGB
/// triple. Missing color → mid-grey.
fn color_from_packed(packed: Option<u32>) -> [f32; 3] {
    let Some(p) = packed else {
        // No color — emit linear-space mid-grey (sRGB 0.75 → ~0.522
        // linear) so callers that mix defaulted + coloured materials
        // render consistently.
        let linear = srgb_to_linear(0.75);
        return [linear, linear, linear];
    };
    let r = ((p & 0x0000_00FF) as f32) / 255.0;
    let g = (((p & 0x0000_FF00) >> 8) as f32) / 255.0;
    let b = (((p & 0x00FF_0000) >> 16) as f32) / 255.0;
    // Revit colors are sRGB; glTF wants linear. Apply the standard
    // gamma approximation (pow 2.2) for viewer-accurate colors.
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b)]
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn alpha_from_transparency(t: Option<f64>) -> f32 {
    match t {
        Some(v) => ((1.0 - v.clamp(0.0, 1.0)) as f32).clamp(0.0, 1.0),
        None => 1.0,
    }
}

/// Name-driven material classifier (VW1-06). Returns `(metallic,
/// roughness, double_sided)` based on keywords in the material
/// name — matches the typical Revit family naming ("Glass —
/// Tinted", "Steel — Painted", "Wood — Oak").
///
/// Heuristics chosen to match Blender's "Principled BSDF" preset
/// values that AEC artists recognize:
///
/// - Glass: metallic 0, roughness 0.05, double-sided true
/// - Metal: metallic 1, roughness 0.2, single-sided
/// - Wood: metallic 0, roughness 0.7, single-sided
/// - Concrete / masonry: metallic 0, roughness 0.9
/// - Paint / ceramic / tile: metallic 0, roughness 0.5
/// - Default (unknown): metallic 0, roughness 0.6
fn classify_from_name(name: &str) -> (f32, f32, bool) {
    let n = name.to_ascii_lowercase();
    if n.contains("glass") || n.contains("glazing") || n.contains("acrylic") {
        return (0.0, 0.05, true);
    }
    if n.contains("metal")
        || n.contains("steel")
        || n.contains("aluminum")
        || n.contains("aluminium")
        || n.contains("copper")
        || n.contains("brass")
        || n.contains("iron")
        || n.contains("bronze")
    {
        return (1.0, 0.2, false);
    }
    if n.contains("wood") || n.contains("timber") || n.contains("oak") || n.contains("pine") {
        return (0.0, 0.7, false);
    }
    if n.contains("concrete")
        || n.contains("masonry")
        || n.contains("stone")
        || n.contains("brick")
        || n.contains("mortar")
    {
        return (0.0, 0.9, false);
    }
    if n.contains("paint") || n.contains("ceramic") || n.contains("tile") {
        return (0.0, 0.5, false);
    }
    (0.0, 0.6, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(name: &str, color: Option<u32>, transparency: Option<f64>) -> MaterialInfo {
        MaterialInfo {
            name: name.into(),
            color_packed: color,
            transparency,
        }
    }

    #[test]
    fn default_material_is_mid_grey_dielectric() {
        let m = PbrMaterial::default();
        assert_eq!(m.base_color_rgb, [0.75, 0.75, 0.75]);
        assert_eq!(m.alpha, 1.0);
        assert_eq!(m.metallic, 0.0);
    }

    #[test]
    fn glass_material_is_double_sided_transparent() {
        let info = mk("Glass — Tinted", Some(0x00FFFFFF), Some(0.5));
        let pbr = PbrMaterial::from_material_info(&info);
        assert!(pbr.double_sided);
        assert!(pbr.roughness < 0.1);
        assert_eq!(pbr.metallic, 0.0);
        assert!((pbr.alpha - 0.5).abs() < 1e-6);
    }

    #[test]
    fn steel_material_is_metallic() {
        let info = mk("Steel — Structural", Some(0x00808080), None);
        let pbr = PbrMaterial::from_material_info(&info);
        assert_eq!(pbr.metallic, 1.0);
        assert!(!pbr.double_sided);
    }

    #[test]
    fn aluminum_also_detected() {
        let pbr = PbrMaterial::from_material_info(&mk("Aluminium Frame", None, None));
        assert_eq!(pbr.metallic, 1.0);
    }

    #[test]
    fn wood_material_is_rough_dielectric() {
        let pbr = PbrMaterial::from_material_info(&mk("Wood — Oak", None, None));
        assert_eq!(pbr.metallic, 0.0);
        assert!(pbr.roughness > 0.5);
    }

    #[test]
    fn concrete_is_very_rough() {
        let pbr = PbrMaterial::from_material_info(&mk("Concrete — Cast-in-Place", None, None));
        assert!(pbr.roughness > 0.8);
    }

    #[test]
    fn unknown_material_gets_default_roughness() {
        let pbr = PbrMaterial::from_material_info(&mk("Custom Material 42", None, None));
        assert_eq!(pbr.metallic, 0.0);
        assert!((pbr.roughness - 0.6).abs() < 1e-6);
        assert!(!pbr.double_sided);
    }

    #[test]
    fn color_unpacks_from_packed_bgr() {
        // 0x00BBGGRR with R=0xFF, G=0x80, B=0x00 → pure-ish red.
        let pbr = PbrMaterial::from_material_info(&mk("Paint", Some(0x000080FF), None));
        // Linear sRGB for 0xFF → 1.0, 0x80 → ~0.216, 0x00 → 0.
        assert!((pbr.base_color_rgb[0] - 1.0).abs() < 0.001);
        assert!(pbr.base_color_rgb[1] > 0.2 && pbr.base_color_rgb[1] < 0.25);
        assert!(pbr.base_color_rgb[2] < 0.01);
    }

    #[test]
    fn color_missing_is_mid_grey() {
        let pbr = PbrMaterial::from_material_info(&mk("Generic", None, None));
        // sRGB 0.75 → linear ~0.522
        for c in pbr.base_color_rgb.iter() {
            assert!(*c > 0.45 && *c < 0.6);
        }
    }

    #[test]
    fn transparency_1_maps_to_alpha_0() {
        let pbr = PbrMaterial::from_material_info(&mk("Ghost", None, Some(1.0)));
        assert_eq!(pbr.alpha, 0.0);
    }

    #[test]
    fn transparency_0_maps_to_alpha_1() {
        let pbr = PbrMaterial::from_material_info(&mk("Solid", None, Some(0.0)));
        assert_eq!(pbr.alpha, 1.0);
    }

    #[test]
    fn transparency_clamps_out_of_range_values() {
        let pbr = PbrMaterial::from_material_info(&mk("Weird", None, Some(2.0)));
        assert_eq!(pbr.alpha, 0.0);
        let pbr = PbrMaterial::from_material_info(&mk("Weird2", None, Some(-1.0)));
        assert_eq!(pbr.alpha, 1.0);
    }

    #[test]
    fn srgb_to_linear_matches_known_values() {
        // sRGB 0 → linear 0
        assert!((srgb_to_linear(0.0) - 0.0).abs() < 1e-6);
        // sRGB 1 → linear 1
        assert!((srgb_to_linear(1.0) - 1.0).abs() < 1e-6);
        // sRGB 0.5 → linear ~0.214
        let v = srgb_to_linear(0.5);
        assert!(v > 0.21 && v < 0.22);
    }

    #[test]
    fn classifier_is_case_insensitive() {
        let (m, _, _) = classify_from_name("GLASS PANE");
        assert_eq!(m, 0.0);
        let (m, _, _) = classify_from_name("gLaSs");
        assert_eq!(m, 0.0);
    }
}
