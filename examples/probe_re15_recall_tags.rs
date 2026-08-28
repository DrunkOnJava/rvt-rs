//! RE-15-01..04 (#81–#84) — recall baseline tag / class inventory.
use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn filtered_count(buf: &[u8], tag: u16) -> usize {
    let mut n = 0usize;
    for i in 0..buf.len().saturating_sub(3) {
        let v = u16::from_le_bytes([buf[i], buf[i + 1]]);
        if v == tag && buf[i + 2] == 0 && buf[i + 3] == 0 {
            n += 1;
        }
    }
    n
}

fn tag_of(schema: &formats::SchemaTable, name: &str) -> Option<u16> {
    schema
        .classes
        .iter()
        .find(|c| c.name == name)
        .and_then(|c| c.tag)
}

fn probe(file: &str, partition: &str) {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/{file}");
    let mut rf = match RevitFile::open(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{file}: {e}");
            return;
        }
    };
    let version = rf.basic_file_info().map(|b| b.version).unwrap_or(0);
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).expect("formats");
    let formats_d = compression::inflate_at(&formats_raw, 0).expect("inflate");
    let schema = formats::parse_schema(&formats_d).expect("schema");
    let targets = [
        "ArcWall",
        "VWall",
        "ArcWallRectOpening",
        "VWallRectOpening",
        "AnalyticalModelSlab",
        "HostObjAttr",
        "WallCGDriver",
        "ArcWallCGDriver",
    ];
    let mut tags: BTreeMap<&str, Option<u16>> = BTreeMap::new();
    for name in targets {
        tags.insert(name, tag_of(&schema, name));
    }
    let raw = rf.read_stream(partition).expect("partition");
    let concat: Vec<u8> = compression::inflate_all_chunks(&raw)
        .into_iter()
        .flatten()
        .collect();
    println!(
        "\n=== {file} (Revit {version}) {partition} — {} B ===",
        concat.len()
    );
    println!("  {:>24}  {:>8}  {:>10}", "class", "tag", "filt_hits");
    for (name, tag) in &tags {
        match tag {
            Some(t) => println!(
                "  {name:>24}  0x{t:04x}  {:>10}",
                filtered_count(&concat, *t)
            ),
            None => println!("  {name:>24}  {:>8}  {:>10}", "—", "—"),
        }
    }
}

fn main() {
    println!("RE-15 recall-tag inventory (#81–#84 baselines)");
    probe("Revit_IFC5_Einhoven.rvt", "Partitions/5");
    probe("2024_Core_Interior.rvt", "Partitions/46");
}
