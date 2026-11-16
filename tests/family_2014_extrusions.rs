//! RE-489: a standalone family of Revit 2014 is read from its own plain
//! groups, as 2015 to 2027 are, and its extrusions rebuild.
//!
//! The three Revit 2014 families of DynamoRevit's test suite (Apache-2.0,
//! not redistributed here) are read from `RVT_FAMILY_2014_DIR`: every plain
//! group frames, and the extrusions the files hold (6, 3 and 3) all build.
//! Skips what is absent.

use rvt::RevitFile;
use rvt::research::native_form_geometry::{self as nfg, FAMILY_RELEASES, FormDocument};
use rvt::research::native_parameters::{GraphLimits, decode_graph_with_limits};
use std::path::PathBuf;

const LIMITS: GraphLimits = GraphLimits {
    max_values: 2_000_000,
    max_objects: 500_000,
    max_depth: 128,
};

#[test]
fn family_releases_start_at_2014() {
    assert!(FAMILY_RELEASES.contains(&2014));
    assert!(!FAMILY_RELEASES.contains(&2013));
    assert!(FAMILY_RELEASES.contains(&2027));
}

/// (file, extrusions) of the Revit 2014 families measured in RE-489.
const FILES: [(&str, usize); 3] = [
    ("test__System__DividedCurve__DividedCurve.rfa", 6),
    ("test__System__Selection__Selection.0001.rfa", 3),
    ("test__System__Selection__Selection.0002.rfa", 3),
];

#[test]
fn revit_2014_family_extrusions_build() {
    let Some(dir) = std::env::var_os("RVT_FAMILY_2014_DIR").map(PathBuf::from) else {
        eprintln!("skip: RVT_FAMILY_2014_DIR unset");
        return;
    };
    for (name, expected) in FILES {
        let path = dir.join(name);
        if !path.exists() {
            eprintln!("skip: {} absent", path.display());
            continue;
        }
        let mut file = RevitFile::open(&path).expect("open");
        let document = nfg::open(&mut file).expect("a Revit 2014 family opens");
        assert_eq!(document.release, 2014, "{name}");
        let mut forms = FormDocument::default();
        let walked = nfg::walk(&mut file, &document, &[102], |entry| {
            if matches!(
                entry.class,
                Some("ExtrusionElem" | "CurveElem" | "SketchPlane" | "VarSketch")
            ) && let Ok(graph) =
                decode_graph_with_limits(entry.body, &document.registry, &LIMITS)
            {
                forms.insert(entry.id, graph);
            }
            Ok(())
        })
        .expect("walk");
        assert_eq!(walked.unframed_groups, 0, "{name}: unframed groups");
        let ids = forms.extrusion_ids();
        assert_eq!(ids.len(), expected, "{name}: extrusions");
        for id in ids {
            forms
                .extrusion(id)
                .and_then(|extrusion| extrusion.default_solid())
                .unwrap_or_else(|error| panic!("{name}: extrusion {id}: {error}"));
        }
    }
}
