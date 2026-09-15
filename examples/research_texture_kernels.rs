//! Evaluate explicit texture-stage inputs against separately generated witnesses.
use anyhow::{Context, Result};
use rvt::native_texture::{self, Filter, NormalInputs, Sampler, TextureImage, TextureInputs, Wrap};
use serde_json::{Value, json};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 3,
        "usage: research_texture_kernels INPUT_JSON NEW_OUTPUT_JSON"
    );
    let input: Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut rows = Vec::new();
    for case in input["cases"].as_array().context("missing cases")? {
        let p = &case["input"];
        let result = (|| -> Result<Value> {
            let image = TextureImage::new(
                usize::try_from(p["width"].as_u64().context("width")?)?,
                usize::try_from(p["height"].as_u64().context("height")?)?,
                serde_json::from_value(p["pixels_rgba"].clone())?,
            )?;
            let wrap = match p["wrap"].as_str().context("wrap")? {
                "repeat" => Wrap::Repeat,
                "clamp" => Wrap::ClampToEdge,
                "mirror" => Wrap::MirroredRepeat,
                _ => anyhow::bail!("unknown wrap"),
            };
            let filter = match p["filter"].as_str().context("filter")? {
                "nearest" => Filter::Nearest,
                "linear" => Filter::Linear,
                _ => anyhow::bail!("unknown filter"),
            };
            let sampler = Sampler {
                u: wrap,
                v: wrap,
                filter,
            };
            let matrix: [f64; 16] = serde_json::from_value(p["transform"].clone())?;
            let texture = TextureInputs {
                uv: serde_json::from_value(p["uv"].clone())?,
                transform: std::array::from_fn(|r| std::array::from_fn(|c| matrix[c * 4 + r])),
                tint: if p["tint"].is_null() {
                    [1.; 3]
                } else {
                    serde_json::from_value(p["tint"].clone())?
                },
                invert: p["invert"].as_bool().unwrap_or(false),
                linearize: p["linearize"].as_bool().unwrap_or(false),
            };
            match case["kernel"].as_str().context("kernel")? {
                "texture" => Ok(json!(native_texture::texture_stage(
                    &image, sampler, &texture
                )?)),
                kernel @ ("normal" | "bump") => {
                    let n = NormalInputs {
                        texture,
                        tangent: serde_json::from_value(p["tangent"].clone())?,
                        bitangent: serde_json::from_value(p["bitangent"].clone())?,
                        normal: serde_json::from_value(p["normal"].clone())?,
                        bump_scale: p["bump_scale"].as_f64().context("bump_scale")?,
                    };
                    Ok(json!(native_texture::normal_stage(
                        &image,
                        sampler,
                        &n,
                        kernel == "bump"
                    )?))
                }
                _ => anyhow::bail!("unsupported kernel"),
            }
        })();
        rows.push(match result {
            Ok(output) => json!({"id":case["id"],"kernel":case["kernel"],"output":output}),
            Err(e) => json!({"id":case["id"],"kernel":case["kernel"],"refusal":format!("{e:#}")}),
        });
    }
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&args[2])?;
    serde_json::to_writer_pretty(file, &json!({"cases":rows}))?;
    Ok(())
}
