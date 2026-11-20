use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{CylinderTessellationProfile, cylinder_face};
use serde_json::{Value, json};

/// A two-ring partial cylinder keeps its hole even when a ring knot lies
/// close to a regular tessellation knot. Every point is synthetic.
#[test]
fn cylinder_two_loops_keep_the_hole_across_close_u_knots() {
    let pointer = |token: u32| json!({ "pointer_token": token, "offset": 0 });
    let object = |class: &str, token: u32, fields: Value| {
        json!({ "class_tag": 1, "class_name": class, "token": token,
                "start": 0, "fields_end": 0, "fields": fields })
    };
    let point = |uv: [f64; 2]| json!({ "uv": [uv, [0., 0.]] });
    let rings = [
        [[0., 0.], [2., 0.], [2., 2.], [0., 2.]],
        [
            [0.5 - 1.5e-9, 0.5],
            [1.5 - 1.5e-9, 0.5],
            [1.5 - 1.5e-9, 1.5],
            [0.5 - 0.3e-9, 1.5],
        ],
    ];
    let mut objects = vec![
        object(
            "Face",
            1,
            json!({
                "m_faceRegions": [], "m_pSurf": pointer(2), "m_pFirstLoop": pointer(3),
                "m_faceFlags_v9": 0, "m_renderStyleId": 0
            }),
        ),
        object(
            "CylSurf",
            2,
            json!({
                "m_center": [0., 0., 0.], "m_radius": 1.,
                "m_xVec": [1., 0., 0.], "m_yVec": [0., 1., 0.], "m_zVec": [0., 0., 1.],
                "m_Envelope": { "m_corners": [[0., 0.], [2., 2.]] }, "m_orientFlag": true
            }),
        ),
    ];
    for (index, ring) in rings.iter().enumerate() {
        let token = 3 + 5 * index as u32;
        let next_loop = if index == 0 { token + 5 } else { 0 };
        objects.push(object(
            "EdgeLoop",
            token,
            json!({
                "m_open": false, "m_pFace": pointer(1), "m_next": pointer(token + 1),
                "m_nextLoop": pointer(next_loop)
            }),
        ));
        for edge in 0..4 {
            let next = if edge == 3 {
                token
            } else {
                token + 2 + edge as u32
            };
            objects.push(object(
                "Edge",
                token + 1 + edge as u32,
                json!({
                    "m_pFace": [pointer(1), pointer(0)], "m_next": [pointer(next), pointer(0)],
                    "m_firstAndLastEdgePnts": [point(ring[edge]), point(ring[(edge + 1) % 4])],
                    "m_interiorEdgePnts": []
                }),
            ));
        }
    }
    let graph: ObjectGraph = serde_json::from_value(json!({
        "consumed_bytes": 0, "edges": [], "objects": objects
    }))
    .expect("synthetic cylinder graph");
    let mesh = cylinder_face(
        &graph,
        0,
        CylinderTessellationProfile {
            // A 0.13-radian limit yields 16 equal regular strips.
            chord_error: 2. * (0.13_f64 / 4.).sin().powi(2),
            max_v_edge: 1.,
        },
    )
    .expect("a valid two-ring cylinder trim");
    let area: f64 = mesh
        .triangles
        .iter()
        .map(|triangle| {
            let [a, b, c] = triangle.map(|index| {
                let [x, y, z] = mesh.vertices[index as usize];
                [y.atan2(x), z]
            });
            ((b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])).abs() / 2.
        })
        .sum();
    assert!((area - 3.0000000006).abs() < 1e-10, "UV area {area}");
}
