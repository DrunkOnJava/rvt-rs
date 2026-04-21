//! Q5.2 — FieldType classification coverage on real project files.
//!
//! For each `.rvt` under the corpus dir, parses `Formats/Latest` and
//! counts how many of the schema's declared fields the 11-discriminator
//! `FieldType` enum classified cleanly vs fell through to `Unknown`.
//!
//! Validates the "100% classified" claim across project files (was
//! originally measured on family files only). Path resolves via
//! `RVT_PROJECT_CORPUS_DIR` (default
//! `/private/tmp/rvt-corpus-probe/magnetar/Revit`).

use rvt::{RevitFile, compression, formats, streams};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let p2023 = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let p2024 = format!("{project_dir}/2024_Core_Interior.rvt");
    let files = [p2023.as_str(), p2024.as_str()];
    for path in files {
        let mut rf = RevitFile::open(path).unwrap();
        let raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
        let decomp = compression::inflate_at_auto(&raw).unwrap().1;
        let schema = formats::parse_schema(&decomp).unwrap();
        let mut total = 0;
        let mut unknown = 0;
        for cls in &schema.classes {
            for f in &cls.fields {
                total += 1;
                match &f.field_type {
                    Some(formats::FieldType::Unknown { .. }) | None => unknown += 1,
                    _ => {}
                }
            }
        }
        println!(
            "{}: {} fields, {} unknown ({:.2}% classified)",
            path.rsplit('/').next().unwrap(),
            total,
            unknown,
            (total - unknown) as f64 * 100.0 / total as f64
        );
    }
}
