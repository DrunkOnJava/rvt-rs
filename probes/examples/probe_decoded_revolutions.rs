//! Exercise the shared Rust reader directly on standalone RFA bytes.
//! Usage: probe_decoded_revolutions FILE.rfa NEW_OUTPUT [--refusal CASE]
//! Refusal cases mutate decoded native graphs only; source files never change.
use anyhow::{Context, Result, ensure};
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::{
    native_form_geometry::{self, Curve, FormDocument},
    native_parameters::{self, ObjectGraph},
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufWriter, Read},
    path::Path,
};

fn digest(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn mutate(records: &mut Vec<(u64, ObjectGraph)>, case: &str) -> Result<()> {
    let profile = records
        .iter()
        .position(|(_, g)| {
            g.objects
                .first()
                .is_some_and(|o| o.class_name == "CurveElem")
                && g.objects
                    .iter()
                    .any(|o| o.class_name == "RevolutionCurveData" && o.fields["m_axis"] == false)
        })
        .context("no profile")?;
    let form = records
        .iter()
        .position(|(_, g)| g.objects[0].class_name == "RevolutionElem")
        .context("no form")?;
    if case == "duplicate_record" {
        records.push(records[profile].clone());
        return Ok(());
    }
    if case == "missing_curve" {
        records.remove(profile);
        return Ok(());
    }
    if case == "void" {
        records[form].1.objects[0].fields["m_cutting"] = json!(true);
        return Ok(());
    }
    if case == "duplicate_angle" {
        let p = records[form]
            .1
            .objects
            .iter_mut()
            .find(|o| o.class_name == "ParamValueSetDouble")
            .context("no parameters")?;
        let set = p.fields["m_paramSet"]
            .as_array_mut()
            .context("parameter array")?;
        set.push(set[0].clone());
        return Ok(());
    }
    if case == "no_axis" {
        for (_, g) in records {
            for o in &mut g.objects {
                if o.class_name == "RevolutionCurveData" {
                    o.fields["m_axis"] = json!(false);
                }
            }
        }
        return Ok(());
    }
    let graph = &mut records[profile].1;
    match case {
        "profile_gap" => {
            // Shorten the first profile curve at an internal join. The final
            // join still closes, so a check of only that join misses the gap.
            let class = if graph.objects.iter().any(|o| o.class_name == "GArc") {
                "GArc"
            } else {
                "GLine"
            };
            let curve = graph
                .objects
                .iter_mut()
                .find(|o| o.class_name == class)
                .context("profile primitive")?;
            let end = curve.fields["m_endParams"][1]
                .as_f64()
                .context("curve end parameter")?;
            curve.fields["m_endParams"][1] = json!(end - 1e-5);
        }
        "flipped_curve" => {
            graph
                .objects
                .iter_mut()
                .find(|o| o.class_name == "CurveElemDriver")
                .context("driver")?
                .fields["m_flip"] = json!(true);
        }
        "old_axis" => {
            graph
                .objects
                .iter_mut()
                .find(|o| o.class_name == "RevolutionCurveData")
                .context("data")?
                .fields["m_bOldAxis"] = json!(true);
        }
        "missing_driver_edge" => {
            let driver = graph
                .objects
                .iter()
                .position(|o| o.class_name == "CurveElemDriver")
                .context("driver")?;
            let offset = graph.objects[driver].fields["m_pCrv"]["offset"]
                .as_u64()
                .context("offset")?;
            graph.edges.retain(|e| {
                !(e.source_object_index == driver && e.pointer_offset as u64 == offset)
            });
        }
        "negative_radius" | "nonunit_arc" | "filled_arc" | "unsupported_curve" => {
            let arc = graph
                .objects
                .iter_mut()
                .find(|o| o.class_name == "GArc")
                .context("arc case needs curved fixture")?;
            match case {
                "negative_radius" => arc.fields["m_radius"] = json!(-1.0),
                "nonunit_arc" => arc.fields["m_xVec"] = json!([2.0, 0.0, 0.0]),
                "filled_arc" => arc.fields["m_bFilled"] = json!(true),
                _ => arc.class_name = "GEllipse".into(),
            }
        }
        _ => anyhow::bail!("unknown refusal case"),
    }
    Ok(())
}

fn curve_json(curve: &Curve) -> Fv {
    match curve {
        Curve::Line { from, to } => json!({"kind":"line","from":from,"to":to}),
        Curve::Arc {
            center,
            x_axis,
            y_axis,
            radius,
            start,
            end,
        } => {
            json!({"kind":"arc", "center":center,"x_axis":x_axis,"y_axis":y_axis,"radius":radius,"start":start,"end":end})
        }
    }
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    ensure!(
        args.len() == 2 || (args.len() == 4 && args[2] == "--refusal"),
        "usage: probe_decoded_revolutions FILE.rfa NEW_OUTPUT [--refusal CASE]"
    );
    let path = Path::new(&args[0]);
    let output = Path::new(&args[1]);
    ensure!(
        path.extension()
            .is_some_and(|v| v.eq_ignore_ascii_case("rfa")),
        "standalone RFA required"
    );
    ensure!(!output.exists(), "output directory must be new");
    let before = digest(path)?;
    let mut file = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, true))?;
    let document = native_form_geometry::open(&mut file)?;
    let mut records = Vec::new();
    let limits = native_parameters::GraphLimits {
        max_values: 200_000,
        max_objects: 100_000,
        max_depth: 128,
    };
    let walked = native_form_geometry::walk(&mut file, &document, &[102], |entry| {
        if !matches!(
            entry.class,
            Some("RevolutionElem" | "CurveElem" | "SketchPlane" | "VarSketch")
        ) {
            return Ok(());
        }
        ensure!(records.len() < 2000, "record budget");
        let graph =
            native_parameters::decode_graph_with_limits(entry.body, &document.registry, &limits)?;
        ensure!(graph.consumed_bytes == entry.body.len(), "incomplete graph");
        records.push((entry.id, graph));
        Ok(())
    })?;
    ensure!(walked.unframed_groups == 0, "unframed groups");
    let refusal = args.get(3);
    if let Some(case) = refusal {
        mutate(&mut records, case)?;
    }
    let mut forms = FormDocument::default();
    for (id, graph) in records {
        forms.insert(id, graph);
    }
    let ids = forms.revolution_ids();
    ensure!(!ids.is_empty(), "no revolution records");
    fs::create_dir_all(output)?;
    let mut results = Vec::new();
    for id in ids {
        match forms.revolution(id) {
            Err(reason) => {
                results.push(json!({"id":id,"status":"refused","reason":reason}));
            }
            Ok(form) => {
                ensure!(refusal.is_none(), "mutated native evidence was accepted");
                let mesh = form.mesh().map_err(anyhow::Error::msg)?;
                let data = json!({"schema":"decoded-revolution-v1","source_sha256":before,
                    "file":path.file_name().context("filename")?.to_string_lossy(),"units":"feet",
                    "id":id,"sketch_id":form.sketch_id,"plane_id":form.plane_id,"curve_ids":form.curve_ids,
                    "axis_curve_id":form.axis_curve_id,"axis_origin":form.axis_origin,"axis_direction":form.axis_direction,
                    "radial_direction":form.radial_direction,"start":form.start,"end":form.end,
                    "profile":form.profile.loops[0].curves.iter().map(curve_json).collect::<Vec<_>>(),
                    "profile_area":form.profile.area,"vertices":mesh.vertices,"triangles":mesh.triangles});
                let name = format!("revolution-{id}.json");
                serde_json::to_writer(
                    BufWriter::new(fs::File::create(output.join(&name))?),
                    &data,
                )?;
                results.push(json!({"id":id,"status":"decoded","mesh":name}));
            }
        }
    }
    ensure!(digest(path)? == before, "source changed");
    let passed = results.iter().all(|r| {
        r["status"]
            == if refusal.is_some() {
                "refused"
            } else {
                "decoded"
            }
    });
    let report = json!({"passed":passed,"source_sha256":before,"release":document.release,"refusal_case":refusal,"forms":results});
    fs::write(
        output.join("result.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    ensure!(passed, "some native forms were not decoded");
    Ok(())
}
