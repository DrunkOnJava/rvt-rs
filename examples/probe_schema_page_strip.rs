use rvt::compression;
use rvt::formats;
use std::env;
use std::io::Read;
use std::path::PathBuf;

fn main() {
    let path = PathBuf::from(env::args().nth(1).expect("path"));
    let mut file = cfb::open(&path).unwrap();
    let mut raw = Vec::new();
    file.open_stream("Formats/Latest")
        .unwrap()
        .read_to_end(&mut raw)
        .unwrap();
    let stripped = compression::strip_revit_page_checksums(&raw);
    for (label, bytes) in [("raw", raw.as_slice()), ("stripped", stripped.as_slice())] {
        let d = compression::inflate_at(bytes, 0).expect("inflate");
        let names = rvt::class_index::extract_class_names(&d).expect("names");
        let schema = formats::parse_schema(&d).expect("schema");
        let field_count: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
        println!(
            "{label}: inflate={} class_names={} schema_classes={} fields={} skipped={}",
            d.len(),
            names.len(),
            schema.classes.len(),
            field_count,
            schema.skipped_records
        );
    }
}
