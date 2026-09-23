//! RE-45: an element exports as the IFC entity and predefined type its own
//! export parameters, or its type's, name.
//!
//! On `2024_Core_Interior.rvt` two floors carry "Export to IFC As" `ifcSlab`
//! with predefined type `ROOF`, and Revit's own export writes both as
//! `IFCSLAB(… .ROOF.)`; twenty carry `IfcShadingDevice`. Corpus-gated through
//! `RVT_PROJECT_CORPUS_DIR`.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use rvt::partition_schema_mvp::{IFC_EXPORT_AS_FIELD, IFC_PREDEFINED_TYPE_FIELD};
use rvt::walker::{DecodedElement, InstanceField};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn core_interior() -> Option<PathBuf> {
    let path =
        PathBuf::from(std::env::var_os("RVT_PROJECT_CORPUS_DIR")?).join("2024_Core_Interior.rvt");
    path.exists().then_some(path)
}

fn field<'a>(element: &'a DecodedElement, wanted: &str) -> Option<&'a str> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text) if name == wanted => Some(text.as_str()),
        _ => None,
    })
}

/// `(entity, PredefinedType)` of every IFCSLAB and IFCSHADINGDEVICE by Tag.
fn slab_like_by_tag(step: &str) -> BTreeMap<u32, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let entity = entity.trim();
        if entity != "IFCSLAB" && entity != "IFCSHADINGDEVICE" {
            continue;
        }
        // The Tag is the last quoted argument and the predefined type the
        // enumeration after it.
        let Some((head, predefined)) = args.trim_end_matches(");").rsplit_once(',') else {
            continue;
        };
        let Some(tag) = head
            .rsplit_once(",'")
            .and_then(|(_, tag)| tag.trim_end_matches('\'').parse().ok())
        else {
            continue;
        };
        out.insert(
            tag,
            (entity.to_string(), predefined.trim_matches('.').to_string()),
        );
    }
    out
}

#[test]
fn core_interior_export_overrides_are_revits() {
    let Some(path) = core_interior() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR does not hold 2024_Core_Interior.rvt");
        return;
    };
    let mut rf = RevitFile::open(&path).expect("open");
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        2024,
        rvt::walker::WalkerLimits::default(),
    )
    .expect("partition MVP");
    let mut roofs: Vec<u32> = mvp
        .slabs
        .iter()
        .filter(|slab| field(slab, IFC_PREDEFINED_TYPE_FIELD) == Some("ROOF"))
        .filter_map(|slab| slab.id)
        .collect();
    roofs.sort_unstable();
    assert_eq!(roofs, [20912, 70325]);
    for slab in mvp
        .slabs
        .iter()
        .filter(|slab| roofs.contains(&slab.id.unwrap_or(0)))
    {
        assert_eq!(field(slab, IFC_EXPORT_AS_FIELD), Some("ifcSlab"));
    }
    let shading = mvp
        .slabs
        .iter()
        .filter(|slab| field(slab, IFC_EXPORT_AS_FIELD) == Some("IfcShadingDevice"))
        .count();
    assert_eq!(shading, 20);

    // The export writes what Revit's does for those Tags.
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = slab_like_by_tag(&write_step(&result.model));
    for tag in [20912, 70325] {
        assert_eq!(
            ours[&tag],
            ("IFCSLAB".to_string(), "ROOF".to_string()),
            "{tag}"
        );
    }
    let roof_rows = ours.values().filter(|(_, p)| p == "ROOF").count();
    assert_eq!(roof_rows, 2);
    let shading_rows = ours
        .values()
        .filter(|(e, _)| e == "IFCSHADINGDEVICE")
        .count();
    assert_eq!(shading_rows, 20);
}
