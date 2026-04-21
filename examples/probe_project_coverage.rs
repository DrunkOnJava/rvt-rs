use rvt::{RevitFile, compression, formats, streams};

fn main() {
    let files = [
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt",
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/2024_Core_Interior.rvt",
    ];
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
