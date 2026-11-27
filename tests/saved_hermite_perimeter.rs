use rvt::research::native_parameters::ObjectGraph;
use serde_json::{Value, json};
use std::process::Command;

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

fn hermite_perimeter(gap: f64, returning_cubic: bool) -> ObjectGraph {
    const FACE: usize = 0;
    const SURFACE: usize = 1;
    const RING: usize = 2;
    const FIRST: usize = 3;
    let v = if returning_cubic {
        vec![0., 0.5, 0.5 + gap, 1.]
    } else {
        vec![0., 1.]
    };
    let nodes: Vec<_> = v
        .iter()
        .flat_map(|&y| {
            [0., 1.].map(move |x| {
                let slope = if returning_cubic && y == 0.5 {
                    100.
                } else if returning_cubic && y == 0.5 + gap {
                    -100.
                } else {
                    0.
                };
                json!({
                    "m_iPoint": [x,y,0.],
                    "m_iTangent": [[1., 0., 0.], [0., 1., slope]],
                    "m_iMixedDer": [0., 0., 0.],
                })
            })
        })
        .collect();
    object_graph([
        ("Face", face_fields(SURFACE, RING)),
        (
            "HermiteSurf",
            json!({
                "m_uParams": [0.,1.], "m_vParams": v, "m_periodic": [false,false],
                "m_orientFlag": true, "m_Envelope": {"m_corners":[[0.,0.],[1.,1.]]},
                "m_NodeArray": nodes,
            }),
        ),
        ("EdgeLoop", ring_fields(FACE, FIRST)),
        ("Edge", single_sided(FACE, FIRST + 1, [0., 0.], [1., 0.])),
        ("Edge", single_sided(FACE, FIRST + 2, [1., 0.], [1., 0.5])),
        (
            "Edge",
            single_sided(FACE, FIRST + 3, [1., 0.5 + gap], [1., 1.]),
        ),
        ("Edge", single_sided(FACE, FIRST + 4, [1., 1.], [0., 1.])),
        ("Edge", single_sided(FACE, RING, [0., 1.], [0., 0.])),
    ])
}

#[test]
fn hermite_perimeter_patches_only_a_short_positive_gap() {
    use rvt::research::native_parametric_mesh::{face as parametric_face, face_grid};
    const CASE: &str = "RVT_2085_SYNTHETIC_HERMITE_CASE";
    const PERIMETER: &str = "RVT_2085_HERMITE_PERIMETER";
    let Ok(case) = std::env::var(CASE) else {
        for case in [
            "disabled",
            "joined",
            "overlap",
            "too_wide",
            "returning_cubic",
            "exact",
        ] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command.args([
                "--exact",
                "saved_hermite_perimeter::hermite_perimeter_patches_only_a_short_positive_gap",
            ]);
            command.env(CASE, case).env_remove(PERIMETER);
            if case != "disabled" {
                command.env(PERIMETER, "1");
            }
            let output = command.output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(stdout.contains("running 1 test"), "{stdout}");
            assert!(output.status.success(), "{case}: {stdout}");
        }
        return;
    };
    let gap = match case.as_str() {
        "disabled" | "joined" | "returning_cubic" => 5e-7,
        "overlap" => -5e-7,
        "too_wide" => 2e-6,
        "exact" => 0.,
        _ => panic!("unknown generated case"),
    };
    let returning_cubic = case == "returning_cubic";
    let graph = hermite_perimeter(gap, returning_cubic);
    if returning_cubic {
        // The two saved endpoints are 5e-7 ft apart. Opposing tangents on
        // the intervening cubic take the midpoint more than 1e-6 ft away.
        use rvt::research::native_parametric_surface::ParametricSurface;
        let surface = ParametricSurface::read(&graph, 1).expect("generated Hermite surface");
        let midpoint = surface.evaluate(1., 0.5 + gap / 2.).unwrap();
        assert!(midpoint[2].abs() > 1e-6);
    }
    if matches!(
        case.as_str(),
        "disabled" | "overlap" | "too_wide" | "returning_cubic"
    ) {
        let error = face_grid(&graph, 0).expect_err("invalid rectangle should be refused");
        assert!(
            error.to_string().contains("incomplete trim perimeter"),
            "{error}"
        );
        return;
    }
    let grid = face_grid(&graph, 0).expect("expected valid Hermite perimeter");
    assert_eq!(grid.rectangular, case == "exact");
    let mesh = parametric_face(&graph, 0).expect("expected a complete Hermite trim");
    let area: f64 = mesh
        .triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = triangle.map(|index| mesh.vertices[index as usize]);
            let twice_area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            assert!(twice_area > 0., "inverted Hermite triangle");
            twice_area / 2.
        })
        .sum();
    assert!(
        (area - 1.).abs() < 1e-10,
        "Hermite trim did not cover the square"
    );
}
