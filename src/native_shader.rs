//! Explicit advanced-material kernel helpers.
//!
//! The equations operate on evaluated, linear shader inputs. This API does not
//! select a renderer backend, resolve textures, interpret color spaces, or
//! reproduce a whole image.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// A saved shader input and its ownership witnesses. Connected assets are
/// references into the accompanying appearance tree, not evaluated samples.
#[derive(Debug, Serialize)]
pub struct BoundInput {
    pub property_name: String,
    pub source_object: usize,
    pub saved_value: Option<serde_json::Value>,
    pub unit_type_id: Option<String>,
    pub connected_objects: Vec<usize>,
}

#[derive(Debug, Serialize)]
pub struct GenericBinding {
    pub status: &'static str,
    pub kernel_profile: &'static str,
    pub complete_shader_reproduction: bool,
    pub channels: std::collections::BTreeMap<&'static str, BoundInput>,
    pub bitmap_textures: std::collections::BTreeMap<String, crate::native_texture::BitmapBinding>,
    pub missing_channels: Vec<&'static str>,
    pub unresolved: Vec<&'static str>,
}

/// Bind Generic inputs without flattening scalar/map alternatives or guessing
/// the view-selected renderer. Unsupported schemas return no Generic binding.
pub fn bind_generic(root: &crate::native_appearance::Node) -> Option<GenericBinding> {
    if root.name != "GenericSchema" {
        return None;
    }
    let mut channels = std::collections::BTreeMap::new();
    let mut missing_channels = Vec::new();
    let mut bitmap_textures = std::collections::BTreeMap::new();
    for (channel, name) in [
        ("diffuse", "generic_diffuse"),
        ("diffuse_color_space", "generic_diffuse_colorspace"),
        ("diffuse_texture_fade", "generic_diffuse_image_fade"),
        ("glossiness", "generic_glossiness"),
        ("normal_reflectivity", "generic_reflectivity_at_0deg"),
        ("grazing_reflectivity", "generic_reflectivity_at_90deg"),
        ("metal", "generic_is_metal"),
        ("transparency", "generic_transparency"),
        (
            "transparency_texture_fade",
            "generic_transparency_image_fade",
        ),
        ("cutout", "generic_cutout_opacity"),
        ("index_of_refraction", "generic_refraction_index"),
        ("translucency", "generic_refraction_translucency_weight"),
        ("emissive_filter", "generic_self_illum_filter_map"),
        ("emissive_luminance", "generic_self_illum_luminance"),
        (
            "emissive_temperature",
            "generic_self_illum_color_temperature",
        ),
        ("bump", "generic_bump_map"),
        ("bump_amount", "generic_bump_amount"),
        ("tint_enabled", "common_Tint_toggle"),
        ("tint", "common_Tint_color"),
        ("color_by_object", "color_by_object"),
    ] {
        let matches: Vec<_> = root.properties.iter().filter(|p| p.name == name).collect();
        if let [p] = matches.as_slice() {
            for (index, connected) in p.connected.iter().enumerate() {
                if let Some(bitmap) = crate::native_texture::bind_bitmap(connected) {
                    bitmap_textures.insert(format!("{channel}/{index}"), bitmap);
                }
            }
            channels.insert(
                channel,
                BoundInput {
                    property_name: name.into(),
                    source_object: p.object_index,
                    saved_value: p.value.clone(),
                    unit_type_id: p.unit_type_id.clone(),
                    connected_objects: p.connected.iter().map(|n| n.object_index).collect(),
                },
            );
        } else {
            missing_channels.push(channel);
        }
    }
    Some(GenericBinding {
        status: "bound_saved_generic_inputs",
        kernel_profile: "advanced_material_explicit_inputs_v1",
        complete_shader_reproduction: false,
        channels,
        bitmap_textures,
        missing_channels,
        unresolved: vec![
            "view/backend and translation-option selection",
            "saved color-space interpretation and tone mapping",
            "connected texture resources, UVs and sampling",
            "bump, translucency and illumination evaluation",
            "rendered-image parity",
        ],
    })
}

fn unit(value: f64) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

/// Independent normal/grazing reflectance, with an explicit curve exponent.
pub fn custom_reflectance(cosine: f64, normal: f64, grazing: f64, power: f64) -> Result<f64> {
    ensure!(
        unit(cosine) && unit(normal) && unit(grazing),
        "reflectance input outside [0,1]"
    );
    ensure!(
        power.is_finite() && power >= 0.0,
        "invalid reflectance exponent"
    );
    ensure!(
        cosine < 1.0 || power > 0.0,
        "undefined zero-to-zero shader power"
    );
    Ok(normal + (grazing - normal) * (1.0 - cosine).powf(power))
}

/// Schlick curve using the index-of-refraction branch.
pub fn ior_reflectance(cosine: f64, ior: f64) -> Result<f64> {
    ensure!(ior.is_finite() && ior > 0.0, "invalid index of refraction");
    custom_reflectance(cosine, ((ior - 1.0) / (ior + 1.0)).powi(2), 1.0, 5.0)
}

/// Blinn-Phong alternative to the Ward distribution.
pub fn blinn_phong(normal_half: f64, glossiness: f64) -> Result<f64> {
    ensure!(
        unit(normal_half) && unit(glossiness),
        "invalid Blinn-Phong input"
    );
    ensure!(
        normal_half > 0.0 || glossiness > 0.0,
        "undefined zero-to-zero shader power"
    );
    Ok(normal_half.powf(128.0 * glossiness))
}

/// Isotropic Ward lobe. Inputs are unit normal, unit half-vector and
/// dot products [N.L, N.V, N.H, V.H] from the shader geometry stage.
pub fn ward_isotropic(n: [f64; 3], h: [f64; 3], dots: [f64; 4], glossiness: f64) -> Result<f64> {
    ensure!(
        dots.into_iter()
            .all(|v| v.is_finite() && (-1.0..=1.0).contains(&v))
            && unit(glossiness),
        "invalid Ward scalar input"
    );
    for vector in [n, h] {
        ensure!(
            vector.into_iter().all(f64::is_finite),
            "nonfinite Ward vector"
        );
        ensure!(
            (vector.into_iter().map(|x| x * x).sum::<f64>() - 1.0).abs() < 1e-5,
            "Ward vector must be normalized"
        );
    }
    let nh = n.into_iter().zip(h).map(|(a, b)| a * b).sum::<f64>();
    ensure!((nh - dots[2]).abs() < 1e-5, "Ward normal/half dot mismatch");
    let incidence = dots[0] * dots[1];
    // Preserve the GPU's float32 branch boundary. A widened f64 literal would
    // incorrectly reject the exact representable shader epsilon.
    if (dots[0] as f32) * (dots[1] as f32) < 1e-6_f32 {
        return Ok(0.0);
    }
    ensure!(dots[2] > 0.0, "singular Ward half vector");
    let projected = h
        .into_iter()
        .zip(n)
        .map(|(a, b)| (a - dots[2] * b).powi(2))
        .sum::<f64>();
    let sharpness = 2.0_f64.powf(8.0 * glossiness);
    let sharpness = if sharpness > 80.0 {
        80.0 + (sharpness - 80.0).sqrt()
    } else {
        sharpness
    };
    let scale = dots[0] / (4.0 * std::f64::consts::PI * incidence.sqrt());
    let result = [(1.0, 0.5), (0.5, 1.0), (0.25, 1.5)]
        .into_iter()
        .map(|(width, weight)| {
            let precision = (sharpness * width).powi(2);
            weight * precision * (-precision * projected / dots[2].powi(2)).exp()
        })
        .sum::<f64>()
        * scale;
    ensure!(result.is_finite(), "nonfinite Ward result");
    Ok(result)
}

/// Evaluated channels entering the advanced-material combine stage.
/// Colors/irradiances must already be in the chosen renderer's linear domain.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombineInputs {
    pub cutout: f64,
    pub emissive: [f64; 3],
    pub ambient_level: f64,
    pub ambient_color: [f64; 3],
    pub environment_irradiance: [f64; 3],
    pub diffuse_level: f64,
    pub diffuse_color: [f64; 3],
    pub diffuse_light: [f64; 3],
    pub specular_level: f64,
    pub reflectance: f64,
    pub specular_color: [f64; 3],
    pub specular_light: [f64; 3],
    pub environment_specular: [f64; 3],
    pub transmission_color: [f64; 3],
    pub transparency: f64,
    pub extra_alpha: f64,
    pub override_alpha: bool,
    pub unlit_scale: f64,
    pub energy_conservation: bool,
    pub metal: bool,
}

/// Return straight RGBA before tone mapping, tint, clipping and compositing.
pub fn combine(p: &CombineInputs) -> Result<[f64; 4]> {
    ensure!(
        [
            p.cutout,
            p.ambient_level,
            p.diffuse_level,
            p.specular_level,
            p.reflectance,
            p.transparency,
            p.extra_alpha,
            p.unlit_scale
        ]
        .into_iter()
        .all(unit),
        "combine coefficient outside [0,1]"
    );
    for color in [p.diffuse_color, p.specular_color, p.transmission_color] {
        ensure!(
            color.into_iter().all(unit),
            "combine reflectance color outside [0,1]"
        );
    }
    for light in [
        p.emissive,
        p.ambient_color,
        p.environment_irradiance,
        p.diffuse_light,
        p.specular_light,
        p.environment_specular,
    ] {
        ensure!(
            light.into_iter().all(|v| v.is_finite() && v >= 0.0),
            "invalid combine radiance"
        );
    }
    let reflection = p.specular_level * p.reflectance;
    let reflected_energy = reflection * p.specular_color.into_iter().sum::<f64>() / 3.0;
    let transmitted_energy = 0.8 * p.transparency * (1.0 - reflected_energy);
    let diffuse_energy = if p.energy_conservation {
        1.0 - transmitted_energy - reflected_energy
    } else {
        1.0
    };
    let opacity = if p.energy_conservation {
        1.0 - transmitted_energy
    } else {
        1.0 - 0.5 * p.transparency
    };
    let luminance = p
        .specular_color
        .into_iter()
        .zip([0.212671, 0.715160, 0.072169])
        .map(|(v, w)| v * w)
        .sum::<f64>();
    let diffuse_strength = p.diffuse_level
        * if p.metal {
            1.0 - luminance * reflection
        } else {
            1.0
        };
    let diffuse_strength = diffuse_strength + (1.0 - diffuse_strength) * p.transparency;
    let mut rgba = [0.0; 4];
    for (i, value) in rgba[..3].iter_mut().enumerate() {
        let color =
            p.diffuse_color[i] + (p.transmission_color[i] - p.diffuse_color[i]) * p.transparency;
        let diffuse = diffuse_strength
            * color
            * (p.diffuse_light[i] + p.environment_irradiance[i])
            * diffuse_energy;
        let ambient = p.unlit_scale * p.ambient_level * color * p.ambient_color[i] * diffuse_energy;
        let specular = reflection
            * p.specular_color[i]
            * if p.metal { color } else { 1.0 }
            * (p.specular_light[i] + p.unlit_scale * p.environment_specular[i]);
        *value = p.unlit_scale * p.emissive[i]
            + (ambient + diffuse + specular) / if p.energy_conservation { opacity } else { 1.0 };
    }
    rgba[3] = p.cutout * p.extra_alpha * if p.override_alpha { 1.0 } else { opacity };
    ensure!(
        rgba.into_iter().all(f64::is_finite),
        "nonfinite combine result"
    );
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reflectance_endpoints_and_invalid_inputs() {
        assert_eq!(custom_reflectance(1.0, 0.2, 0.8, 5.0).unwrap(), 0.2);
        assert!((custom_reflectance(0.0, 0.2, 0.8, 5.0).unwrap() - 0.8).abs() < 1e-15);
        assert!(ior_reflectance(0.5, -1.0).is_err());
        assert!(custom_reflectance(f64::NAN, 0.2, 0.8, 5.0).is_err());
    }
    #[test]
    fn ward_rejects_inconsistent_geometry_and_accepts_backlight_limit() {
        assert!(ward_isotropic([0., 0., 1.], [0., 0., 1.], [1., 1., 0.5, 1.], 0.5).is_err());
        assert_eq!(
            ward_isotropic([0., 0., 1.], [0., 0., 1.], [0., 1., 1., 1.], 0.5).unwrap(),
            0.
        );
    }
    #[test]
    fn ward_uses_shader_precision_at_epsilon() {
        let epsilon = f64::from(1e-6_f32);
        assert!(
            ward_isotropic([0., 0., 1.], [0., 0., 1.], [epsilon, 1., 1., 1.], 0.5).unwrap() > 0.0
        );
        let below = f64::from(f32::from_bits(1e-6_f32.to_bits() - 1));
        assert_eq!(
            ward_isotropic([0., 0., 1.], [0., 0., 1.], [below, 1., 1., 1.], 0.5).unwrap(),
            0.0
        );
    }
}
