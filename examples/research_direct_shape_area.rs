use rvt::{
    native_parameters::ObjectGraph, native_saved_material_quantities::direct_shape_type_areas,
    native_saved_mesh::GraphicsMeshes,
};
use serde_json::{Value, json};
use std::{
    fs::File,
    io::{BufRead, BufReader},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() != 4 {
        return Err("records.jsonl scene.json NEW_RESULT.json".into());
    }
    let mut owner: Option<ObjectGraph> = None;
    let mut graphics: Option<ObjectGraph> = None;
    for l in BufReader::new(File::open(&a[1])?).lines() {
        let r: Value = serde_json::from_str(&l?)?;
        if r["channel"] == 102 {
            owner = Some(serde_json::from_value(r["graph"].clone())?);
        }
        if r["channel"] == 103 {
            graphics = Some(serde_json::from_value(r["graph"].clone())?);
        }
    }
    let scene: Value = serde_json::from_reader(File::open(&a[2])?)?;
    let meshes: GraphicsMeshes =
        serde_json::from_value(scene["scene"]["elements"][0]["meshes"].clone())?;
    let (areas, source) = direct_shape_type_areas(
        &owner.ok_or("owner")?,
        &graphics.ok_or("graphics")?,
        &meshes,
    )?;
    let out = File::options().write(true).create_new(true).open(&a[3])?;
    serde_json::to_writer_pretty(out, &json!({"areas":areas,"source":source}))?;
    Ok(())
}
