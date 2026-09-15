//! Evaluate explicit numerical shader inputs; never load a native oracle at runtime.
use anyhow::{Context, Result, bail};
use rvt::native_shader::{self, CombineInputs};
use serde_json::{Value, json};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 3,
        "usage: research_shader_kernels INPUT_JSON NEW_OUTPUT_JSON"
    );
    let input: Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut results = Vec::new();
    for case in input["cases"].as_array().context("missing cases")? {
        let p = &case["input"];
        let scalar = |name: &str| -> Result<f64> {
            p[name]
                .as_f64()
                .with_context(|| format!("missing scalar {name}"))
        };
        let vec3 =
            |name: &str| -> Result<[f64; 3]> { Ok(serde_json::from_value(p[name].clone())?) };
        let boolean = |name: &str| -> Result<bool> {
            p[name]
                .as_bool()
                .with_context(|| format!("missing bool {name}"))
        };
        let result = (|| -> Result<Value> {
            Ok(match case["kernel"].as_str().context("missing kernel")? {
                "ward_iso" => json!(native_shader::ward_isotropic(
                    vec3("n")?,
                    vec3("h")?,
                    serde_json::from_value(p["dots"].clone())?,
                    scalar("glossiness")?
                )?),
                "blinn" | "blinn_phong" => json!(native_shader::blinn_phong(
                    p["dots"][2].as_f64().context("missing NH")?,
                    scalar("glossiness")?
                )?),
                "custom_f" => json!(native_shader::custom_reflectance(
                    scalar("nx")?,
                    scalar("f0")?,
                    scalar("f1")?,
                    scalar("fresnel_power")?
                )?),
                "schlick_f" => json!(native_shader::custom_reflectance(
                    scalar("nx")?,
                    scalar("f0")?,
                    1.0,
                    5.0
                )?),
                "schlick_ior" => json!(native_shader::ior_reflectance(
                    scalar("nx")?,
                    scalar("ior")?
                )?),
                "ior_to_reflectance" => {
                    let v = native_shader::ior_reflectance(1.0, scalar("ior")?)?;
                    json!([v, v, v])
                }
                "am_combine" => json!(native_shader::combine(&CombineInputs {
                    cutout: scalar("cutout")?,
                    emissive: vec3("Ce")?,
                    ambient_level: scalar("Ka")?,
                    ambient_color: vec3("La")?,
                    environment_irradiance: vec3("Ei")?,
                    diffuse_level: scalar("Kd")?,
                    diffuse_color: vec3("Cd")?,
                    diffuse_light: vec3("Ld")?,
                    specular_level: scalar("Ks")?,
                    reflectance: scalar("Fs")?,
                    specular_color: vec3("Cs")?,
                    specular_light: vec3("Ls")?,
                    environment_specular: vec3("Es")?,
                    transmission_color: vec3("Ct")?,
                    transparency: scalar("transp")?,
                    extra_alpha: p["extra"][3].as_f64().context("missing extra alpha")?,
                    override_alpha: boolean("extraOverrideAlpha")?,
                    unlit_scale: scalar("unlit")?,
                    energy_conservation: boolean("energyConservation")?,
                    metal: boolean("specularIsMetal")?,
                })?),
                other => bail!("unknown shader kernel {other}"),
            })
        })();
        results.push(match result {
            Ok(value) => json!({"id":case["id"],"kernel":case["kernel"],"output":value}),
            Err(e) => json!({"id":case["id"],"kernel":case["kernel"],"refusal":format!("{e:#}")}),
        });
    }
    let output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])?;
    serde_json::to_writer_pretty(
        output,
        &json!({"scope":"explicit shader kernel inputs; not Revit backend or rendered-image parity","cases":results}),
    )?;
    Ok(())
}
