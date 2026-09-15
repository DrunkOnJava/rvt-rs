//! Measure saved native scene JSONL; reference metadata is never an input.
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    io::{BufRead, Write},
};
fn main() -> anyhow::Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    anyhow::ensure!(
        a.len() == 5,
        "SCENE OWNER102_JSONL GRAPHICS103_JSONL NEW_OUTPUT"
    );
    let mut owners = BTreeMap::new();
    for line in std::io::BufReader::new(std::fs::File::open(&a[2])?).lines() {
        let v: Value = serde_json::from_str(&line?)?;
        if v["channel"] == 102
            && matches!(v["class_name"].as_str(), Some("SWall" | "Floor"))
            && !v["graph"].is_null()
        {
            owners.insert(
                v["identity"]["element_id"].as_u64().unwrap(),
                serde_json::from_value::<rvt::native_parameters::ObjectGraph>(v["graph"].clone())?,
            );
        }
    }
    let mut graphics = BTreeMap::new();
    for line in std::io::BufReader::new(std::fs::File::open(&a[3])?).lines() {
        let v: Value = serde_json::from_str(&line?)?;
        let id = v["identity"]["element_id"].as_u64().unwrap();
        if owners.contains_key(&id) && !v["graph"].is_null() {
            graphics.insert(
                id,
                serde_json::from_value::<rvt::native_parameters::ObjectGraph>(v["graph"].clone())?,
            );
        }
    }
    let mut out = std::io::BufWriter::new(
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&a[4])?,
    );
    for line in std::io::BufReader::new(std::fs::File::open(&a[1])?).lines() {
        let v: Value = serde_json::from_str(&line?)?;
        let id = v["id"].as_u64().unwrap();
        let m: rvt::native_saved_mesh::GraphicsMeshes =
            serde_json::from_value(v["meshes"].clone())?;
        let metrics = rvt::native_saved_metrics::compute(&m);
        let area = match (owners.get(&id), graphics.get(&id)) {
            (Some(o), Some(g)) => {
                if o.objects[0].class_name == "SWall" {
                    if o.objects
                        .iter()
                        .any(|o| o.class_name == "CWallGridFaceGStep")
                    {
                        rvt::native_saved_metrics::curtain_wall_area(o, g, &m)
                    } else {
                        rvt::native_saved_metrics::straight_wall_area(o, g, &m)
                    }
                } else {
                    rvt::native_saved_metrics::floor_area(o, g, &m)
                }
            }
            _ => Err("owner outside qualified host area scope".into()),
        };
        serde_json::to_writer(
            &mut out,
            &json!({"id":id,"metrics":metrics,"host_area":area.as_ref().ok(),"host_area_diagnostic":area.err()}),
        )?;
        writeln!(out)?;
    }
    Ok(())
}
