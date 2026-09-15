use anyhow::{Result, ensure};
use std::io::{BufRead, Write};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(a.len() == 3, "INPUT_JSONL OUTPUT_NEW_JSONL");
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[2])?;
    for line in std::io::BufReader::new(std::fs::File::open(&a[1])?).lines() {
        let r: serde_json::Value = serde_json::from_str(&line?)?;
        let g: rvt::native_parameters::ObjectGraph = serde_json::from_value(r["graph"].clone())?;
        let mut meshes = Vec::new();
        let mut errors = Vec::new();
        let mut unbounded = 0;
        for (i, o) in g.objects.iter().enumerate() {
            if o.class_name == "Face" {
                match rvt::native_saved_mesh::face(&g, i) {
                    Ok(Some(m)) => meshes.push(m),
                    Ok(None) => unbounded += 1,
                    Err(e) => {
                        errors.push(serde_json::json!({"face_index":i,"error":format!("{e:#}")}))
                    }
                }
            }
        }
        serde_json::to_writer(
            &mut out,
            &serde_json::json!({"id":r["id"],"stream":r["stream"],"source":r["source"],"offset":r["offset"],"physical_only":true,"meshes":meshes,"errors":errors,"unbounded_faces":unbounded}),
        )?;
        writeln!(&mut out)?;
    }
    Ok(())
}
