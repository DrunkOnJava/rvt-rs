//! Quick peek at what fields HostObjAttr has in the schema — it's
//! the ONLY class my scan_candidates is matching against real
//! Global/Latest bytes, which means its field layout is so permissive
//! it succeeds at random offsets. Need to understand why to tune the
//! filter.

use rvt::{RevitFile, compression, formats, streams};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let mut rf = RevitFile::open(&path).unwrap();
    let raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    let (_, decomp) = compression::inflate_at_auto(&raw).unwrap();
    let schema = formats::parse_schema(&decomp).unwrap();

    let target = "HostObjAttr";
    let cls = schema.classes.iter().find(|c| c.name == target).unwrap();
    println!(
        "{}: tag={:?} parent={:?} fields={}",
        target,
        cls.tag,
        cls.parent,
        cls.fields.len()
    );
    for (i, f) in cls.fields.iter().enumerate() {
        println!("  {i:2}. {} : {:?}", f.name, f.field_type);
    }

    println!("\n---\nFor reference, Wall class:");
    if let Some(wall) = schema.classes.iter().find(|c| c.name == "Wall") {
        println!(
            "Wall: tag={:?} parent={:?} fields={}",
            wall.tag,
            wall.parent,
            wall.fields.len()
        );
        for (i, f) in wall.fields.iter().enumerate().take(10) {
            println!("  {i:2}. {} : {:?}", f.name, f.field_type);
        }
        if wall.fields.len() > 10 {
            println!("  … {} more", wall.fields.len() - 10);
        }
    }
}
