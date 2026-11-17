//! RE-490: a blend (`BlendElem`) of two straight-edged outlines is rebuilt
//! from its records, and agrees with the mesh Revit saved for it.
//!
//! The volume is checked on a constructed frustum; the saved-mesh agreement
//! on two families read from `RVT_FAMILY_BLEND_DIR` (not redistributed here):
//! `Column_Square_Tollerance.rfa` (one blend) and DynamoRevit's
//! `test__System__FamilyDocument__familyDocumentTests.rfa` (four). Skips what
//! is absent.

use rvt::RevitFile;
use rvt::research::native_form_geometry::{self as nfg, Blend, BlendSection, FormDocument, Frame};
use rvt::research::native_parameters::{GraphLimits, decode_graph_with_limits};
use rvt::research::native_saved_mesh;
use std::collections::BTreeSet;
use std::path::PathBuf;

const LIMITS: GraphLimits = GraphLimits {
    max_values: 2_000_000,
    max_objects: 500_000,
    max_depth: 128,
};

fn square(half: f64) -> Vec<[f64; 2]> {
    vec![[-half, -half], [half, -half], [half, half], [-half, half]]
}

fn section(half: f64, offset: f64) -> BlendSection {
    BlendSection {
        sketch_id: 1,
        plane_id: 2,
        frame: Frame {
            origin: [0.0; 3],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        },
        vertices: square(half),
        offset,
    }
}

#[test]
fn a_square_frustum_has_its_closed_form_volume() {
    // Sides 2 and 4 (areas 4 and 16), 3 apart: h/3 * (A1 + A2 + sqrt(A1 A2)).
    let blend = Blend {
        id: 1,
        cutting: false,
        sections: [section(1.0, 0.0), section(2.0, 3.0)],
    };
    let expected = 3.0 / 3.0 * (4.0 + 16.0 + (4.0f64 * 16.0).sqrt());
    assert!((blend.volume().expect("volume") - expected).abs() < 1e-12);
    let (low, high) = blend.bounding_box();
    assert_eq!((low, high), ([-2.0, -2.0, 0.0], [2.0, 2.0, 3.0]));
    // The offsets exchanged: the same solid turned over, the same volume.
    assert!((blend.swapped().volume().expect("volume") - expected).abs() < 1e-12);
}

#[test]
fn sections_wound_opposite_ways_are_refused() {
    let mut second = section(2.0, 3.0);
    second.vertices.reverse();
    let blend = Blend {
        id: 1,
        cutting: false,
        sections: [section(1.0, 0.0), second],
    };
    assert!(blend.volume().is_err());
}

/// (file, blends) measured in RE-490.
const FILES: [(&str, usize); 2] = [
    ("Column_Square_Tollerance.rfa", 1),
    ("test__System__FamilyDocument__familyDocumentTests.rfa", 4),
];

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[test]
fn blends_agree_with_their_saved_meshes() {
    let Some(dir) = std::env::var_os("RVT_FAMILY_BLEND_DIR").map(PathBuf::from) else {
        eprintln!("skip: RVT_FAMILY_BLEND_DIR unset");
        return;
    };
    for (name, expected) in FILES {
        let path = dir.join(name);
        if !path.exists() {
            eprintln!("skip: {} absent", path.display());
            continue;
        }
        let mut file = RevitFile::open(&path).expect("open");
        let document = nfg::open(&mut file).expect("opens");
        let mut forms = FormDocument::default();
        nfg::walk(&mut file, &document, &[102], |entry| {
            if matches!(
                entry.class,
                Some("BlendElem" | "CurveElem" | "SketchPlane" | "VarSketch")
            ) && let Ok(graph) =
                decode_graph_with_limits(entry.body, &document.registry, &LIMITS)
            {
                forms.insert(entry.id, graph);
            }
            Ok(())
        })
        .expect("walk");
        let ids = forms.blend_ids();
        assert_eq!(ids.len(), expected, "{name}: blends");
        let wanted: BTreeSet<u64> = ids.iter().copied().collect();
        let mut meshes = std::collections::BTreeMap::new();
        nfg::walk(&mut file, &document, &[103], |entry| {
            if entry.class == Some("GElement")
                && wanted.contains(&entry.id)
                && let Ok(graph) = decode_graph_with_limits(entry.body, &document.registry, &LIMITS)
            {
                meshes.insert(entry.id, graph);
            }
            Ok(())
        })
        .expect("walk graphics");
        for id in ids {
            let blend = forms
                .blend(id)
                .unwrap_or_else(|error| panic!("{name}: blend {id}: {error}"));
            let graphics = native_saved_mesh::graphics(&meshes[&id]);
            assert!(graphics.diagnostics.is_empty() && graphics.unbounded_faces == 0);
            let vertices: Vec<[f64; 3]> = graphics
                .primitives
                .iter()
                .flat_map(|p| p.vertices.iter().copied())
                .collect();
            let mesh_volume: f64 = graphics
                .primitives
                .iter()
                .flat_map(|p| {
                    p.triangles.iter().map(|t| {
                        let [a, b, c] = t.map(|i| p.vertices[i as usize]);
                        let cross = [
                            b[1] * c[2] - b[2] * c[1],
                            b[2] * c[0] - b[0] * c[2],
                            b[0] * c[1] - b[1] * c[0],
                        ];
                        (a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0
                    })
                })
                .sum::<f64>()
                .abs();
            let volume = blend.volume().expect("volume");
            assert!(
                (volume - mesh_volume).abs() <= 1e-9 * mesh_volume,
                "{name}: blend {id}: volume {volume} against the mesh's {mesh_volume}"
            );
            let farthest = |b: &Blend| {
                b.corners()
                    .iter()
                    .map(|c| {
                        vertices
                            .iter()
                            .map(|v| distance(*c, *v))
                            .fold(f64::INFINITY, f64::min)
                    })
                    .fold(0.0, f64::max)
            };
            assert!(farthest(&blend) < 1e-6, "{name}: blend {id}: corners");
            // The other pairing of offsets with sketches does not fit.
            assert!(farthest(&blend.swapped()) > 0.05, "{name}: blend {id}");
        }
    }
}
