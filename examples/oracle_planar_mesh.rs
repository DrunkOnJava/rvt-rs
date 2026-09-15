//! Research tessellation of bounded saved planar topology from Rust graph JSONL.
//! Input must be oracle_embedded_content output; no API geometry is consumed.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
};
fn token(v: &Value) -> Result<u64> {
    v["pointer_token"]
        .as_u64()
        .ok_or_else(|| anyhow::anyhow!("pointer absent"))
}
fn vec3(v: &Value) -> Result<[f64; 3]> {
    let a: Vec<f64> = serde_json::from_value(v.clone())?;
    ensure!(
        a.len() == 3 && a.iter().all(|x| x.is_finite()),
        "invalid vector"
    );
    Ok([a[0], a[1], a[2]])
}
fn mesh(row: &Value) -> Result<Value> {
    ensure!(
        row["channel"] == 103
            && row["class_name"] == "GElement"
            && row["status"] == "complete_bounded_graph",
        "requires complete channel103 GElement"
    );
    let objects = row["graph"]["objects"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("objects absent"))?;
    let mut map = BTreeMap::new();
    for o in objects {
        let t = o["token"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("object token absent"))?;
        ensure!(map.insert(t, o).is_none(), "ambiguous object token");
    }
    let get = |t: u64, name: &str| -> Result<&Value> {
        let o = map
            .get(&t)
            .ok_or_else(|| anyhow::anyhow!("unresolved pointer {t}"))?;
        ensure!(
            o["class_name"] == name,
            "unsupported topology class, expected {name}, got {}",
            o["class_name"]
        );
        Ok(&o["fields"])
    };
    let root = get(0, "GElement")?;
    let nodes = root["m_subNodes"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("nodes absent"))?;
    ensure!(nodes.len() == 1, "multiple geometry nodes unsupported");
    let geom = get(token(&nodes[0])?, "Geometry")?;
    let faces = geom["m_pFaces"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("faces absent"))?;
    ensure!(!faces.is_empty(), "empty geometry");
    let mut vertices: Vec<[f64; 3]> = Vec::new();
    let mut triangles: Vec<[usize; 3]> = Vec::new();
    let mut materials = Vec::new();
    let mut edge_sides = BTreeSet::new();
    for face_pointer in faces {
        let ft = token(face_pointer)?;
        let f = get(ft, "Face")?;
        ensure!(
            f["m_faceRegions"].as_array().is_some_and(Vec::is_empty),
            "face regions unsupported"
        );
        let p = get(token(&f["m_pSurf"])?, "Plane")?;
        ensure!(p["m_orientFlag"] == true, "unqualified plane orientation");
        let origin = vec3(&p["m_origin"])?;
        let x = vec3(&p["m_xVec"])?;
        let y = vec3(&p["m_yVec"])?;
        let lt = token(&f["m_pFirstLoop"])?;
        let lp = get(lt, "EdgeLoop")?;
        ensure!(
            lp["m_open"] == false && token(&lp["m_nextLoop"])? == 0 && token(&lp["m_pFace"])? == ft,
            "open/multiple/inconsistent loops unsupported"
        );
        let mut current = token(&lp["m_next"])?;
        let mut previous = lt;
        let mut seen = BTreeSet::new();
        let mut uvs: Vec<[f64; 2]> = Vec::new();
        let mut ends = Vec::new();
        while current != lt {
            ensure!(
                seen.insert(current) && seen.len() <= 100000,
                "edge traversal cycle/budget"
            );
            let e = get(current, "Edge")?;
            ensure!(
                e["m_interiorEdgePnts"]
                    .as_array()
                    .is_some_and(Vec::is_empty),
                "curved/sampled edge unsupported"
            );
            let refs = e["m_pFace"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("edge faces absent"))?;
            let slots: Vec<_> = refs
                .iter()
                .enumerate()
                .filter_map(|(i, r)| (token(r).ok() == Some(ft)).then_some(i))
                .collect();
            ensure!(slots.len() == 1 && slots[0] < 2, "ambiguous edge side");
            let side = slots[0];
            ensure!(
                edge_sides.insert((current, side)) && token(&e["m_prev"][side])? == previous,
                "inconsistent edge predecessor"
            );
            let uv = |end: usize| -> Result<[f64; 2]> {
                let v: Vec<f64> =
                    serde_json::from_value(e["m_firstAndLastEdgePnts"][end]["uv"][side].clone())?;
                ensure!(
                    v.len() == 2 && v.iter().all(|x| x.is_finite()),
                    "invalid UV"
                );
                Ok([v[0], v[1]])
            };
            let flags = e["m_flags"]
                .as_u64()
                .ok_or_else(|| anyhow::anyhow!("edge flags absent"))?;
            ensure!(flags == 6 || flags == 7, "unqualified edge flags");
            let first = side ^ (flags as usize & 1);
            uvs.push(uv(first)?);
            ends.push(uv(1 - first)?);
            previous = current;
            current = token(&e["m_next"][side])?;
        }
        ensure!(
            uvs.len() >= 3 && token(&lp["m_prev"])? == previous,
            "invalid closed loop"
        );
        for i in 0..uvs.len() {
            let next = uvs[(i + 1) % uvs.len()];
            ensure!(
                (ends[i][0] - next[0]).abs() < 1e-9 && (ends[i][1] - next[1]).abs() < 1e-9,
                "edge endpoints disagree"
            );
        }
        let mut signs = BTreeSet::new();
        for i in 0..uvs.len() {
            let a = uvs[i];
            let b = uvs[(i + 1) % uvs.len()];
            let c = uvs[(i + 2) % uvs.len()];
            let z = (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]);
            ensure!(z.abs() > 1e-12, "degenerate polygon");
            signs.insert(z > 0.0);
        }
        ensure!(signs.len() == 1, "nonconvex face unsupported");
        let offset = vertices.len();
        for uv in uvs {
            vertices.push(std::array::from_fn(|i| {
                origin[i] + uv[0] * x[i] + uv[1] * y[i]
            }));
        }
        for i in 1..vertices.len() - offset - 1 {
            triangles.push([offset, offset + i, offset + i + 1]);
            materials.push(f["m_renderStyleId"].clone());
        }
    }
    let edges = geom["m_pEdges"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("geometry edges absent"))?;
    ensure!(
        edges.len() * 2 == edge_sides.len()
            && edges.iter().all(|e| token(e)
                .is_ok_and(|t| edge_sides.contains(&(t, 0)) && edge_sides.contains(&(t, 1)))),
        "not a closed two-sided topology"
    );
    // Two nonparallel planes define a straight intersection; verify both UV
    // descriptions agree in 3D before calling an endpoint-only edge linear.
    for ep in edges {
        let edge = get(token(ep)?, "Edge")?;
        let mut points = Vec::new();
        let mut normals = Vec::new();
        for side in 0..2 {
            let face = get(token(&edge["m_pFace"][side])?, "Face")?;
            let plane = get(token(&face["m_pSurf"])?, "Plane")?;
            let o = vec3(&plane["m_origin"])?;
            let x = vec3(&plane["m_xVec"])?;
            let y = vec3(&plane["m_yVec"])?;
            let normal: [f64; 3] = std::array::from_fn(|i| {
                x[(i + 1) % 3] * y[(i + 2) % 3] - x[(i + 2) % 3] * y[(i + 1) % 3]
            });
            normals.push(normal);
            let mut side_points = Vec::new();
            for end in 0..2 {
                let uv: Vec<f64> = serde_json::from_value(
                    edge["m_firstAndLastEdgePnts"][end]["uv"][side].clone(),
                )?;
                ensure!(uv.len() == 2, "invalid endpoint UV");
                side_points.push(std::array::from_fn::<_, 3, _>(|i| {
                    o[i] + uv[0] * x[i] + uv[1] * y[i]
                }));
            }
            points.push(side_points);
        }
        let cross: [f64; 3] = std::array::from_fn(|i| {
            normals[0][(i + 1) % 3] * normals[1][(i + 2) % 3]
                - normals[0][(i + 2) % 3] * normals[1][(i + 1) % 3]
        });
        ensure!(
            cross.iter().map(|v| v * v).sum::<f64>() > 1e-20,
            "coplanar boundary linearity unqualified"
        );
        for (a, b) in points[0].iter().zip(&points[1]) {
            for (x, y) in a.iter().zip(b) {
                ensure!(
                    (x - y).abs() < 1e-9,
                    "adjacent face endpoint geometry disagrees"
                );
            }
        }
    }
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in &vertices {
        for i in 0..3 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    let center = vertices[0];
    let mut volume = 0.0;
    for t in &triangles {
        let a: [f64; 3] = std::array::from_fn(|i| vertices[t[0]][i] - center[i]);
        let b: [f64; 3] = std::array::from_fn(|i| vertices[t[1]][i] - center[i]);
        let c: [f64; 3] = std::array::from_fn(|i| vertices[t[2]][i] - center[i]);
        volume += (0..3)
            .map(|i| a[i] * (b[(i + 1) % 3] * c[(i + 2) % 3] - b[(i + 2) % 3] * c[(i + 1) % 3]))
            .sum::<f64>()
            / 6.0;
    }
    ensure!(volume > 0.0, "nonpositive outward signed volume");
    Ok(
        json!({"status":"bounded_planar_mesh","local_element_id":row["local_element_id"],"content_key_raw_hex":row["content_key_raw_hex"],"source":row["source"],"coordinate_space":"saved_content_feet","vertices":vertices,"triangles":triangles,"triangle_render_style_refs":materials,"render_style_namespace_resolved":false,"bounds":[min,max],"signed_volume_ft3":volume}),
    )
}
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() >= 4, "INPUT_JSONL NEW_OUTPUT_JSONL LOCAL_ID...");
    let ids: BTreeSet<u64> = args[3..]
        .iter()
        .map(|x| x.parse())
        .collect::<std::result::Result<_, _>>()?;
    let mut seen = BTreeSet::new();
    let mut output = BufWriter::new(
        File::options()
            .write(true)
            .create_new(true)
            .open(&args[2])?,
    );
    let mut failed = false;
    for line in BufReader::new(File::open(&args[1])?).lines() {
        let row: Value = serde_json::from_str(&line?)?;
        let Some(id) = row["local_element_id"].as_u64() else {
            continue;
        };
        if row["channel"] != 103 || !ids.contains(&id) {
            continue;
        }
        ensure!(seen.insert(id), "duplicate physical owner requires routing");
        let result = match mesh(&row) {
            Ok(value) => value,
            Err(e) => {
                failed = true;
                json!({"status":"unsupported_mesh","local_element_id":id,"diagnostic":format!("{e:#}"),"source":row["source"]})
            }
        };
        serde_json::to_writer(&mut output, &result)?;
        output.write_all(b"\n")?;
    }
    for missing in ids.difference(&seen) {
        failed = true;
        serde_json::to_writer(
            &mut output,
            &json!({"status":"missing_record","local_element_id":missing}),
        )?;
        output.write_all(b"\n")?;
    }
    output.flush()?;
    std::process::exit(if failed { 2 } else { 0 });
}
