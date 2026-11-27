use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{FaceMesh, face};
use serde_json::{Value, json};
use std::{collections::BTreeMap, process::Command};

const TOLERANCE: f64 = 1e-7;

fn reference(index: Option<usize>) -> Value {
    json!({"pointer_token": index.map_or(0, |value| value + 1), "offset": 0})
}

fn face_fields(surface: usize, ring: usize) -> Value {
    json!({
        "m_pFirstLoop": reference(Some(ring)),
        "m_pSurf": reference(Some(surface)),
        "m_faceRegions": [],
        "m_faceFlags_v9": 0,
        "m_renderStyleId": 0,
    })
}

fn ring_fields(owner: usize, start: usize) -> Value {
    json!({
        "m_pFace": reference(Some(owner)),
        "m_next": reference(Some(start)),
        "m_nextLoop": reference(None),
        "m_open": false,
    })
}

fn edge_fields(
    owners: [Option<usize>; 2],
    successors: [Option<usize>; 2],
    samples: &[[[f64; 2]; 2]],
) -> Value {
    let point = |uv| json!({"uv": uv});
    json!({
        "m_pFace": owners.map(reference),
        "m_next": successors.map(reference),
        "m_firstAndLastEdgePnts": [point(samples[0]), point(*samples.last().unwrap())],
        "m_interiorEdgePnts": samples[1..samples.len() - 1].iter().copied().map(point).collect::<Vec<_>>(),
    })
}

fn single_sided(owner: usize, successor: usize, from: [f64; 2], to: [f64; 2]) -> Value {
    edge_fields(
        [Some(owner), None],
        [Some(successor), None],
        &[[from, [0., 0.]], [to, [0., 0.]]],
    )
}

fn object_graph<const N: usize>(records: [(&str, Value); N]) -> ObjectGraph {
    let objects: Vec<_> = records
        .into_iter()
        .enumerate()
        .map(|(index, (class, fields))| {
            json!({
                "class_tag": 0, "class_name": class, "token": index + 1,
                "start": 0, "fields_end": 0, "fields": fields,
            })
        })
        .collect();
    serde_json::from_value(json!({"consumed_bytes": 0, "objects": objects, "edges": []}))
        .expect("generated face graph")
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    let [dx, dy, dz] = std::array::from_fn(|axis| a[axis] - b[axis]);
    dx.hypot(dy).hypot(dz)
}

const PLANE_FACE: usize = 0;
const REV_FACE: usize = 1;
const PLANE: usize = 2;
const REV: usize = 3;
const PROFILE: usize = 4;
const PLANE_LOOP: usize = 5;
const REV_LOOP: usize = 6;
const SHARED: usize = 7;
const PLANE_IN: usize = 8;
const PLANE_OUT: usize = 9;
const REV_RIGHT: usize = 10;
const REV_TOP: usize = 11;
const REV_LEFT: usize = 12;

/// A section of a unit cylinder written as a surface of revolution over a line.
/// The three planar samples are the projection of the revolution samples.
fn model(heights: [f64; 3], reverse_record: bool) -> (ObjectGraph, [[f64; 3]; 3]) {
    let angles = [0.0_f64, 0.4, 1.0];
    let plane = angles.map(|u| [u.cos(), u.sin()]);
    let curved = std::array::from_fn::<_, 3, _>(|index| [angles[index], heights[index]]);
    let mut shared = std::array::from_fn::<_, 3, _>(|index| [plane[index], curved[index]]);
    let mut faces = [Some(PLANE_FACE), Some(REV_FACE)];
    let mut successors = [Some(PLANE_IN), Some(REV_RIGHT)];
    if reverse_record {
        shared.reverse();
        for sample in &mut shared {
            sample.swap(0, 1);
        }
        faces.swap(0, 1);
        successors.swap(0, 1);
    }
    let objects = [
        ("Face", face_fields(PLANE, PLANE_LOOP)),
        ("Face", face_fields(REV, REV_LOOP)),
        (
            "Plane",
            json!({
                "m_origin": [0., 0., 0.], "m_xVec": [1., 0., 0.],
                "m_yVec": [0., 1., 0.], "m_orientFlag": false,
            }),
        ),
        (
            "SurfRev",
            json!({
                "m_center": [0., 0., 0.], "m_xVec": [1., 0., 0.],
                "m_yVec": [0., 1., 0.], "m_zVec": [0., 0., 1.],
                "m_orientFlag": true, "m_Envelope": {"m_corners": [[0., 0.], [1., 1.]]},
                "m_pProfileCurve": reference(Some(PROFILE)),
            }),
        ),
        (
            "GLine",
            json!({"m_origin": [1., 0., 0.], "m_dirVec": [0., 0., 1.], "m_endParams": [0., 1.]}),
        ),
        ("EdgeLoop", ring_fields(PLANE_FACE, SHARED)),
        ("EdgeLoop", ring_fields(REV_FACE, SHARED)),
        ("Edge", edge_fields(faces, successors, &shared)),
        (
            "Edge",
            single_sided(PLANE_FACE, PLANE_OUT, plane[2], [0., 0.]),
        ),
        (
            "Edge",
            single_sided(PLANE_FACE, PLANE_LOOP, [0., 0.], plane[0]),
        ),
        ("Edge", single_sided(REV_FACE, REV_TOP, curved[2], [1., 1.])),
        ("Edge", single_sided(REV_FACE, REV_LEFT, [1., 1.], [0., 1.])),
        (
            "Edge",
            single_sided(REV_FACE, REV_LOOP, [0., 1.], curved[0]),
        ),
    ];
    (object_graph(objects), plane.map(|[x, y]| [x, y, 0.]))
}
struct ChordSegment {
    ends: [[f64; 3]; 2],
    forward: bool,
}

/// Inspect emitted triangle edges independently of the subdivision helper.
fn chain_on_span(mesh: &FaceMesh, first: [f64; 3], last: [f64; 3]) -> Vec<ChordSegment> {
    let axis: [_; 3] = std::array::from_fn(|k| last[k] - first[k]);
    let length_squared = axis.iter().map(|d| d * d).sum::<f64>();
    let slack = TOLERANCE / length_squared.sqrt();
    let parameter = |point: [f64; 3]| -> Option<f64> {
        let t = (0..3).map(|k| (point[k] - first[k]) * axis[k]).sum::<f64>() / length_squared;
        let projected = std::array::from_fn(|k| first[k] + t * axis[k]);
        ((-slack..=1. + slack).contains(&t) && distance(point, projected) <= TOLERANCE).then_some(t)
    };
    let mut uses = BTreeMap::new();
    for &[a, b, c] in &mesh.triangles {
        for (i, j) in [(a, b), (b, c), (c, a)] {
            let (count, _) = uses.entry((i.min(j), i.max(j))).or_insert((0, [i, j]));
            *count += 1;
        }
    }
    let mut segments = Vec::new();
    for (count, [i, j]) in uses.into_values() {
        if count != 1 {
            continue;
        }
        let a = mesh.vertices[i as usize];
        let b = mesh.vertices[j as usize];
        if let (Some(s), Some(t)) = (parameter(a), parameter(b)) {
            assert!((s - t).abs() > slack, "collapsed chord segment");
            let forward = s < t;
            let ends = if forward { [a, b] } else { [b, a] };
            segments.push((s.min(t), ChordSegment { ends, forward }));
        }
    }
    segments.sort_unstable_by(|a, b| a.0.total_cmp(&b.0));
    let mut previous = first;
    for (_, ChordSegment { ends: [a, b], .. }) in &segments {
        assert!(
            distance(previous, *a) <= TOLERANCE,
            "crack along saved chord"
        );
        previous = *b;
    }
    assert!(
        distance(previous, last) <= TOLERANCE,
        "incomplete saved chord"
    );
    segments.into_iter().map(|(_, segment)| segment).collect()
}

#[test]
fn synthetic_plane_and_revolution_share_trim_subdivision() {
    const CASE: &str = "RVT_2085_SYNTHETIC_PLANE_CASE";
    const ANCHOR: &str = "RVT_2085_SHARED_PLANE_ANCHOR";
    const SUBDIVISION: &str = "RVT_2085_SHARED_CHORD_SUBDIVISIONS";
    let Ok(case) = std::env::var(CASE) else {
        // Each arm exercises the ordinary face entry point with process-local
        // research options. Other tests never see their environment changes.
        let qualified = concat!(
            module_path!(),
            "::synthetic_plane_and_revolution_share_trim_subdivision"
        );
        let (_, name) = qualified.split_once("::").unwrap();
        for case in [
            "default",
            "anchor",
            "subdivide",
            "reverse_record",
            "too_wide",
            "interior",
            "two_ends",
        ] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args([name, "--exact", "--nocapture"])
                .env(CASE, case)
                .env_remove(ANCHOR)
                .env_remove(SUBDIVISION);
            if case != "default" {
                command.env(ANCHOR, "1");
            }
            if case != "default" && case != "anchor" {
                command.env(SUBDIVISION, "1");
            }
            let output = command.output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                output.status.success() && stdout.contains("running 1 test"),
                "{case}\n{stdout}\n{stderr}"
            );
            print!("{stdout}");
        }
        return;
    };

    let heights = match case.as_str() {
        "too_wide" => [0.0005, 0., 0.],
        "interior" => [0., 0.0002, 0.],
        "two_ends" => [0.0002, 0., 0.0002],
        _ => [0.0002, 0., 0.],
    };
    let (graph, saved) = model(heights, case == "reverse_record");
    let revolution = face(&graph, REV_FACE).unwrap().unwrap();
    assert!(!revolution.triangles.is_empty());
    let plane = face(&graph, PLANE_FACE);
    if matches!(case.as_str(), "too_wide" | "interior" | "two_ends") {
        let error = plane.unwrap_err().to_string();
        assert!(
            error.contains("shared curved edge leaves adjacent plane"),
            "unexpected refusal: {error}"
        );
        return;
    }

    let plane = plane.unwrap().unwrap();
    if case == "default" {
        // The earlier plane anchor already permits this face, but changes only
        // the planar side. The revolution still lacks the common endpoint.
        let has_endpoint = |mesh: &FaceMesh| {
            mesh.vertices
                .iter()
                .any(|&point| distance(point, saved[0]) <= TOLERANCE)
        };
        assert!(has_endpoint(&plane));
        assert!(!has_endpoint(&revolution));
        return;
    }
    let reversed = (case == "subdivide").then(|| {
        let mut reversed = revolution.clone();
        for triangle in &mut reversed.triangles {
            triangle.swap(1, 2);
        }
        reversed
    });
    let mut counts = [0, 0];
    for span in saved.windows(2) {
        let planar = chain_on_span(&plane, span[0], span[1]);
        let curved = chain_on_span(&revolution, span[0], span[1]);
        counts[0] += planar.len();
        counts[1] += curved.len();
        if case != "anchor" {
            assert_eq!(planar.len(), curved.len());
            for (pair, other) in planar.iter().zip(&curved) {
                assert!(distance(pair.ends[0], other.ends[0]) <= TOLERANCE);
                assert!(distance(pair.ends[1], other.ends[1]) <= TOLERANCE);
                assert_ne!(
                    pair.forward, other.forward,
                    "shared chord has the same traversal"
                );
            }
        }
        if let Some(reversed) = &reversed {
            let flipped = chain_on_span(reversed, span[0], span[1]);
            assert_eq!(flipped.len(), planar.len());
            for ((stock, flipped), planar) in curved.iter().zip(flipped).zip(&planar) {
                assert_eq!(stock.ends, flipped.ends);
                assert_eq!(planar.forward, flipped.forward);
            }
        }
    }
    assert!(
        counts[1] > 2,
        "fixture must contain inserted curved vertices"
    );
    if case == "anchor" {
        assert_eq!(counts[0], 2, "planar trim had two stored spans");
    }
    println!(
        "{case}: planar segments {}, revolution segments {}",
        counts[0], counts[1]
    );
}

#[test]
fn shared_revolution_anchors_preserve_public_saved_bodies() {
    use rvt::ifc::{ExportNote, ExportOptions, run_export};
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};

    const CHILD: &str = "RVT_TEST_SHARED_BOUNDARY_EXPORT";
    const SIGNATURE: &str = "shared-boundary-export ";
    if let Some(fixture) = std::env::var_os(CHILD) {
        let mut file = rvt::RevitFile::open(Path::new(&fixture)).expect("open the public sample");
        let run =
            run_export(&mut file, &ExportOptions::default()).expect("export the public sample");
        assert!(run.unread.is_empty(), "public sample export is incomplete");
        let saved = run
            .notes
            .iter()
            .find_map(|note| match note {
                ExportNote::SavedMeshes { replaced, .. } => Some(*replaced),
                _ => None,
            })
            .expect("Saved body replacement report");
        let fingerprint = |bytes: Vec<u8>| hex::encode(Sha256::digest(bytes));
        let model = fingerprint(serde_json::to_vec(&run.model).expect("serialize model"));
        let diagnostics =
            fingerprint(serde_json::to_vec(&run.diagnostics).expect("serialize diagnostics"));
        println!("{SIGNATURE}{}", json!([saved, model, diagnostics]));
        return;
    }

    let qualified = concat!(
        module_path!(),
        "::shared_revolution_anchors_preserve_public_saved_bodies"
    );
    let (_, name) = qualified.split_once("::").unwrap();
    let Some(directory) = std::env::var_os("RVT_SAMPLE_PROJECTS_DIR") else {
        eprintln!("skipping: RVT_SAMPLE_PROJECTS_DIR is unset");
        return;
    };
    let directory = PathBuf::from(directory);
    let export = |fixture: &Path, shared: bool| {
        let mut command = Command::new(std::env::current_exe().unwrap());
        for (name, _) in
            std::env::vars_os().filter(|(name, _)| name.to_string_lossy().starts_with("RVT_2085_"))
        {
            command.env_remove(name);
        }
        command
            .args([name, "--exact", "--nocapture"])
            .env(CHILD, fixture);
        if shared {
            command
                .env("RVT_2085_SHARED_PLANE_ANCHOR", "1")
                .env("RVT_2085_SHARED_CHORD_SUBDIVISIONS", "1");
        }
        let output = command.output().expect("run the isolated export");
        let stdout = String::from_utf8(output.stdout).expect("read export signature");
        assert!(
            output.status.success(),
            "{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let signature = stdout
            .lines()
            .find_map(|line| line.strip_prefix(SIGNATURE))
            .expect("find the executed public fixture result");
        serde_json::from_str::<(usize, String, String)>(signature).unwrap()
    };
    for (sample, expected_saved) in [
        ("2023-racbasicsampleproject.rvt", 366),
        ("2024-racbasicsampleproject.rvt", 348),
    ] {
        let fixture = directory.join(sample);
        if !fixture.is_file() {
            eprintln!("skipping: public fixture {sample} is unavailable");
            continue;
        }
        let before = export(&fixture, false);
        let after = export(&fixture, true);
        assert_eq!(
            before.0, expected_saved,
            "{sample}: expected Saved body count"
        );
        assert_eq!(after.0, before.0, "{sample}: changed Saved body count");
        assert_eq!(after.1, before.1, "{sample}: changed IFC model");
        assert_eq!(after.2, before.2, "{sample}: changed diagnostics");
        println!("{sample}: {} Saved bodies preserved", after.0);
    }
}
