//! Explicit texture resources and OGS texture-stage evaluation.
//! Saved material bindings do not authorize path lookup. Callers supply bytes;
//! image rows become bottom-to-top texel rows. UVs and sampler state are explicit.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Serialize)]
pub struct TextureImage {
    pub width: usize,
    pub height: usize,
    pub source_sha256: Option<String>,
    pixels: Vec<[f64; 4]>,
}
impl TextureImage {
    /// RGBA values in row-major order, first row at texture v=0.
    pub fn new(width: usize, height: usize, pixels: Vec<[f64; 4]>) -> Result<Self> {
        ensure!(
            width > 0 && height > 0 && width <= 8192 && height <= 8192,
            "texture dimensions outside budget"
        );
        ensure!(
            width.checked_mul(height) == Some(pixels.len()) && pixels.len() <= 16_777_216,
            "texture pixel population outside budget"
        );
        ensure!(
            pixels
                .iter()
                .flatten()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v)),
            "texture texel outside normalized finite range"
        );
        Ok(Self {
            width,
            height,
            pixels,
            source_sha256: None,
        })
    }
    /// Decode a bounded caller-supplied PNG/JPEG/BMP. No implicit gamma,
    /// EXIF orientation, network access, resource search or embedded-image claim.
    pub fn from_encoded(bytes: &[u8]) -> Result<Self> {
        ensure!(
            bytes.len() <= 64 * 1024 * 1024,
            "encoded texture exceeds budget"
        );
        let mut reader =
            image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format()?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(128 * 1024 * 1024);
        reader.limits(limits);
        let decoded = reader.decode()?;
        ensure!(
            usize::try_from(decoded.width())?
                .checked_mul(usize::try_from(decoded.height())?)
                .is_some_and(|n| n <= 16_777_216),
            "decoded texture exceeds pixel budget"
        );
        let rgba = decoded.to_rgba32f();
        let width = rgba.width() as usize;
        let height = rgba.height() as usize;
        ensure!(
            width.checked_mul(height).is_some_and(|n| n <= 16_777_216),
            "decoded texture exceeds pixel budget"
        );
        let pixels = rgba
            .as_raw()
            .chunks_exact(width * 4)
            .rev()
            .flat_map(|row| {
                row.chunks_exact(4)
                    .map(|p| std::array::from_fn(|i| f64::from(p[i])))
            })
            .collect();
        let mut result = Self::new(width, height, pixels)?;
        result.source_sha256 = Some(format!("{:x}", Sha256::digest(bytes)));
        Ok(result)
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Wrap {
    Repeat,
    ClampToEdge,
    MirroredRepeat,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    Nearest,
    Linear,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sampler {
    pub u: Wrap,
    pub v: Wrap,
    pub filter: Filter,
}

fn coordinate(v: f64, wrap: Wrap) -> f64 {
    match wrap {
        Wrap::Repeat => v.rem_euclid(1.0),
        Wrap::ClampToEdge => v.clamp(0.0, 1.0),
        Wrap::MirroredRepeat => {
            let x = v.rem_euclid(2.0);
            if x <= 1.0 { x } else { 2.0 - x }
        }
    }
}
fn index(i: i64, n: usize, wrap: Wrap) -> usize {
    match wrap {
        Wrap::Repeat => i.rem_euclid(n as i64) as usize,
        Wrap::ClampToEdge | Wrap::MirroredRepeat => i.clamp(0, n as i64 - 1) as usize,
    }
}
pub fn sample(image: &TextureImage, sampler: Sampler, uv: [f64; 2]) -> Result<[f64; 4]> {
    ensure!(
        uv.iter().all(|v| v.is_finite() && v.abs() <= 1e8),
        "texture coordinates outside finite precision scope"
    );
    let u = f64::from((coordinate(f64::from(uv[0] as f32), sampler.u) as f32) * image.width as f32);
    let v =
        f64::from((coordinate(f64::from(uv[1] as f32), sampler.v) as f32) * image.height as f32);
    let pixel = |x, y| {
        image.pixels
            [index(y, image.height, sampler.v) * image.width + index(x, image.width, sampler.u)]
    };
    if matches!(sampler.filter, Filter::Nearest) {
        return Ok(pixel(u.floor() as i64, v.floor() as i64));
    }
    let x = (u - 0.5).floor();
    let y = (v - 0.5).floor();
    let tx = u - 0.5 - x;
    let ty = v - 0.5 - y;
    let a = pixel(x as i64, y as i64);
    let b = pixel(x as i64 + 1, y as i64);
    let c = pixel(x as i64, y as i64 + 1);
    let d = pixel(x as i64 + 1, y as i64 + 1);
    let tx = tx as f32;
    let ty = ty as f32;
    Ok(std::array::from_fn(|i| {
        let low = tx.mul_add(b[i] as f32 - a[i] as f32, a[i] as f32);
        let high = tx.mul_add(d[i] as f32 - c[i] as f32, c[i] as f32);
        f64::from(ty.mul_add(high - low, low))
    }))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TextureInputs {
    pub uv: [f64; 3],
    /// Row-major matrix applied to `[u, v, w, 1]`, without perspective division.
    pub transform: [[f64; 4]; 4],
    pub tint: [f64; 3],
    pub invert: bool,
    pub linearize: bool,
}
fn transformed(input: &TextureInputs) -> Result<[f64; 2]> {
    ensure!(
        input
            .uv
            .iter()
            .chain(input.transform.iter().flatten())
            .chain(input.tint.iter())
            .all(|v| v.is_finite() && v.abs() <= 1e8),
        "nonfinite or excessive texture-stage input"
    );
    Ok(std::array::from_fn(|i| {
        f64::from(
            (0..3)
                .map(|j| input.transform[i][j] as f32 * input.uv[j] as f32)
                .sum::<f32>()
                + input.transform[i][3] as f32,
        )
    }))
}
pub fn srgb_to_linear(value: f64) -> Result<f64> {
    ensure!(
        value.is_finite() && (0.0..=1.0).contains(&value),
        "sRGB channel outside [0,1]"
    );
    Ok(if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    })
}
/// OGS programmable-linearization branch: filter first, then linearize RGB,
/// multiply tint, then invert RGB. Alpha is preserved throughout.
pub fn texture_stage(
    image: &TextureImage,
    sampler: Sampler,
    input: &TextureInputs,
) -> Result<[f64; 4]> {
    let mut value = sample(image, sampler, transformed(input)?)?;
    for (i, v) in value.iter_mut().take(3).enumerate() {
        if input.linearize {
            *v = srgb_to_linear(*v)?;
        }
        *v *= input.tint[i];
        if input.invert {
            *v = 1.0 - *v;
        }
    }
    ensure!(
        value.iter().all(|v| v.is_finite()),
        "texture stage overflow"
    );
    Ok(value)
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NormalInputs {
    pub texture: TextureInputs,
    pub tangent: [f64; 3],
    pub bitangent: [f64; 3],
    pub normal: [f64; 3],
    pub bump_scale: f64,
}
fn normalized(v: [f64; 3]) -> Result<[f64; 3]> {
    let len = f64::from(
        v.iter()
            .map(|x| (*x as f32) * (*x as f32))
            .sum::<f32>()
            .sqrt(),
    );
    ensure!(len.is_finite() && len > 1e-12, "degenerate texture normal");
    Ok(v.map(|x| f64::from(x as f32 / len as f32)))
}
/// Tangent-space normal or precomputed two-channel bump-gradient branch.
pub fn normal_stage(
    image: &TextureImage,
    sampler: Sampler,
    input: &NormalInputs,
    gradient: bool,
) -> Result<[f64; 3]> {
    ensure!(
        input.bump_scale.is_finite() && input.bump_scale.abs() <= 1e8,
        "invalid bump scale"
    );
    let basis = [input.tangent, input.bitangent, input.normal];
    for (i, axis) in basis.iter().enumerate() {
        ensure!(
            axis.iter().all(|v| v.is_finite())
                && (axis.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() < 1e-6,
            "texture basis is not unit length"
        );
        for other in &basis[..i] {
            ensure!(
                axis.iter()
                    .zip(other)
                    .map(|(a, b)| a * b)
                    .sum::<f64>()
                    .abs()
                    < 1e-6,
                "texture basis is not orthogonal"
            );
        }
    }
    let color = sample(image, sampler, transformed(&input.texture)?)?;
    let color = color.map(|v| v as f32);
    let scale = input.bump_scale as f32;
    let local = if gradient {
        [color[0] * scale, color[1] * scale, 1.0]
    } else {
        [
            (2.0 * color[0] - 1.0) * scale,
            (2.0 * color[1] - 1.0) * scale,
            2.0 * color[2] - 1.0,
        ]
    };
    normalized(std::array::from_fn(|i| {
        f64::from((0..3).map(|j| local[j] * basis[j][i] as f32).sum::<f32>())
    }))
}

#[derive(Debug, Serialize)]
pub struct BitmapBinding {
    pub source_object: usize,
    pub status: &'static str,
    pub properties: std::collections::BTreeMap<String, crate::native_shader::BoundInput>,
    pub physical_parameters: Option<BitmapParameters>,
    pub physical_parameter_diagnostic: Option<String>,
    pub embedded_resource_status: &'static str,
    pub unresolved: Vec<&'static str>,
}
pub fn bind_bitmap(node: &crate::native_appearance::Node) -> Option<BitmapBinding> {
    if !node.properties.iter().any(|p| {
        p.name == "BaseSchema"
            && p.value.as_ref().and_then(|v| v.as_str()) == Some("UnifiedBitmapSchema")
    }) {
        return None;
    }
    let mut properties = std::collections::BTreeMap::new();
    for p in &node.properties {
        if !(p.name.starts_with("texture_")
            || p.name.starts_with("unifiedbitmap_")
            || p.name.starts_with("common_Tint"))
        {
            continue;
        }
        if properties
            .insert(
                p.name.clone(),
                crate::native_shader::BoundInput {
                    property_name: p.name.clone(),
                    source_object: p.object_index,
                    saved_value: p.value.clone(),
                    unit_type_id: p.unit_type_id.clone(),
                    connected_objects: p.connected.iter().map(|n| n.object_index).collect(),
                },
            )
            .is_some()
        {
            return None;
        }
    }
    let physical = bitmap_parameters(&properties);
    let (physical_parameters, physical_parameter_diagnostic) = match physical {
        Ok(v) => (Some(v), None),
        Err(e) => (None, Some(format!("{e:#}"))),
    };
    Some(BitmapBinding {
        physical_parameters,
        physical_parameter_diagnostic,
        source_object: node.object_index,
        status: "bound_saved_unified_bitmap_inputs",
        properties,
        embedded_resource_status: "not_resolved",
        unresolved: vec![
            "caller-supplied resource bytes required",
            "raw planar UV projection is separate; final backend bitmap transform is not selected",
            "filter/blur and backend sampler-state selection",
            "active renderer and final image parity",
        ],
    })
}

/// OGS planar transform constructor: S * Rz * T, for column-vector inputs.
/// Angles are radians. Row-major output matches `TextureInputs::transform`.
/// This constructor does not choose the backend's subsequent UV flip.
pub fn planar_transform(
    translation: [f64; 3],
    angle_radians: f64,
    scale: [f64; 3],
) -> Result<[[f64; 4]; 4]> {
    ensure!(
        translation
            .iter()
            .chain(scale.iter())
            .chain(std::iter::once(&angle_radians))
            .all(|v| v.is_finite() && v.abs() <= 1e8),
        "texture transform outside finite precision scope"
    );
    let t = translation.map(|v| v as f32);
    let scale = scale.map(|v| v as f32);
    let (sin, cos) = (angle_radians as f32).sin_cos();
    let m = [
        [
            scale[0] * cos,
            -scale[0] * sin,
            0.,
            scale[0] * (cos * t[0] - sin * t[1]),
        ],
        [
            scale[1] * sin,
            scale[1] * cos,
            0.,
            scale[1] * (sin * t[0] + cos * t[1]),
        ],
        [0., 0., scale[2], scale[2] * t[2]],
        [0., 0., 0., 1.],
    ];
    Ok(m.map(|r| r.map(f64::from)))
}

/// Unit-aware saved UnifiedBitmap parameters, not an implicit renderer choice.
#[derive(Debug, Serialize)]
pub struct BitmapParameters {
    pub physical_scale_feet: [f64; 2],
    pub physical_offset_feet: [f64; 2],
    pub w_angle_degrees: f64,
    pub normalized_offset: [f64; 2],
    pub normalized_scale: [f64; 2],
    pub uv_scale: f64,
    pub repeat: [bool; 2],
    pub invert: bool,
    pub rgb_amount: f64,
    pub map_channel: i64,
    pub resource_reference: String,
}
fn bitmap_parameters(
    p: &std::collections::BTreeMap<String, crate::native_shader::BoundInput>,
) -> Result<BitmapParameters> {
    let property = |name: &str| {
        p.get(name)
            .ok_or_else(|| anyhow::anyhow!("bitmap property missing: {name}"))
    };
    let value = |name: &str| {
        property(name)?
            .saved_value
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("bitmap value absent: {name}"))
    };
    let number = |name: &str| {
        value(name)?
            .as_f64()
            .filter(|v| v.is_finite())
            .ok_or_else(|| anyhow::anyhow!("bitmap number invalid: {name}"))
    };
    let boolean = |name: &str| {
        value(name)?
            .as_bool()
            .ok_or_else(|| anyhow::anyhow!("bitmap boolean invalid: {name}"))
    };
    let feet = |name: &str| -> Result<f64> {
        let conversion = match property(name)?.unit_type_id.as_deref() {
            Some("autodesk.unit.unit:inches-1.0.1") => 1. / 12.,
            Some("autodesk.unit.unit:feet-1.0.1") => 1.,
            Some("autodesk.unit.unit:millimeters-1.0.1") => 1. / 304.8,
            Some("autodesk.unit.unit:centimeters-1.0.1") => 1. / 30.48,
            Some("autodesk.unit.unit:meters-1.0.1") => 1. / 0.3048,
            _ => anyhow::bail!("bitmap distance unit unqualified: {name}"),
        };
        let converted = number(name)? * conversion;
        ensure!(
            converted.is_finite(),
            "bitmap converted distance overflow: {name}"
        );
        Ok(converted)
    };
    let result = BitmapParameters {
        physical_scale_feet: [
            feet("texture_RealWorldScaleX")?,
            feet("texture_RealWorldScaleY")?,
        ],
        physical_offset_feet: [
            feet("texture_RealWorldOffsetX")?,
            feet("texture_RealWorldOffsetY")?,
        ],
        w_angle_degrees: number("texture_WAngle")?,
        normalized_offset: [number("texture_UOffset")?, number("texture_VOffset")?],
        normalized_scale: [number("texture_UScale")?, number("texture_VScale")?],
        uv_scale: number("texture_UVScale")?,
        repeat: [boolean("texture_URepeat")?, boolean("texture_VRepeat")?],
        invert: boolean("unifiedbitmap_Invert")?,
        rgb_amount: number("unifiedbitmap_RGBAmount")?,
        map_channel: value("texture_MapChannel")?
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("bitmap map channel invalid"))?,
        resource_reference: value("unifiedbitmap_Bitmap")?
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("bitmap resource reference invalid"))?
            .into(),
    };
    ensure!(
        result
            .physical_scale_feet
            .iter()
            .all(|v| *v > 0. && v.is_finite()),
        "bitmap physical scale must be positive"
    );
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bilinear_repeat_seam_and_clamp_are_distinct() {
        let image = TextureImage::new(2, 1, vec![[0., 0., 0., 0.], [1., 1., 1., 1.]]).unwrap();
        let mut sampler = Sampler {
            u: Wrap::Repeat,
            v: Wrap::ClampToEdge,
            filter: Filter::Linear,
        };
        assert_eq!(sample(&image, sampler, [0., 0.5]).unwrap(), [0.5; 4]);
        sampler.u = Wrap::ClampToEdge;
        assert_eq!(sample(&image, sampler, [0., 0.5]).unwrap(), [0.; 4]);
        assert_eq!(sample(&image, sampler, [1., 0.5]).unwrap(), [1.; 4]);
    }
    #[test]
    fn invalid_resources_and_coordinates_refuse() {
        assert!(TextureImage::new(1, 1, vec![[f64::NAN; 4]]).is_err());
        assert!(TextureImage::from_encoded(b"not an image").is_err());
        let image = TextureImage::new(1, 1, vec![[0.; 4]]).unwrap();
        assert!(
            sample(
                &image,
                Sampler {
                    u: Wrap::Repeat,
                    v: Wrap::Repeat,
                    filter: Filter::Nearest
                },
                [f64::INFINITY, 0.]
            )
            .is_err()
        );
    }
    #[test]
    fn png_resource_preserves_alpha_and_declares_bottom_row_first() {
        let image =
            TextureImage::from_encoded(include_bytes!("../tests/fixtures/texture-palette.png"))
                .unwrap();
        let sampler = Sampler {
            u: Wrap::ClampToEdge,
            v: Wrap::ClampToEdge,
            filter: Filter::Nearest,
        };
        let bottom = sample(&image, sampler, [0.25, 0.25]).unwrap();
        assert_eq!(&bottom[..3], &[0., 0., 1.]);
        assert!((bottom[3] - 64. / 255.).abs() < 1e-7);
        assert_eq!(
            sample(&image, sampler, [0.25, 0.75]).unwrap(),
            [1., 0., 0., 1.]
        );
        assert!(image.source_sha256.is_some());
    }
    #[test]
    fn filtered_normal_near_midgray_retains_gpu_precision() {
        let pixels = vec![
            [0.02, 0.3, 0.9, 0.2],
            [0.9, 0.05, 0.2, 0.4],
            [0.3, 0.8, 0.04, 0.7],
            [0.7, 0.2, 0.6, 1.],
        ]
        .into_iter()
        .map(|p: [f64; 4]| p.map(|v| f64::from(v as f32)))
        .collect();
        let image = TextureImage::new(2, 2, pixels).unwrap();
        let input = NormalInputs {
            texture: TextureInputs {
                uv: [f64::from(0.99_f32), f64::from(0.51_f32), 0.],
                transform: [
                    [2., 0., 0., 0.125],
                    [0., 0.5, 0., f64::from(-0.2_f32)],
                    [0., 0., 1., 0.],
                    [0., 0., 0., 1.],
                ],
                tint: [1.; 3],
                invert: false,
                linearize: false,
            },
            tangent: [1., 0., 0.],
            bitangent: [0., 1., 0.],
            normal: [0., 0., 1.],
            bump_scale: f64::from(0.7_f32),
        };
        let got = normal_stage(
            &image,
            Sampler {
                u: Wrap::Repeat,
                v: Wrap::Repeat,
                filter: Filter::Linear,
            },
            &input,
            false,
        )
        .unwrap();
        // Independent original OGS GLSL oracle normal-1269. Weighted-sum
        // interpolation loses an ULP before 2*blue-1, amplified by normalize.
        let expected = [
            -0.8230595588684082,
            -0.5672436356544495,
            0.02841803804039955,
        ];
        for (a, b) in got.into_iter().zip(expected) {
            assert!((a - b).abs() <= 2e-7 + 5e-6 * b.abs());
        }
    }
}
