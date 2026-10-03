//! RE-150 (probe): what the schema-directed walker finds in `Global/Latest`
//! with the tags it uses now, and with each class's own tag (B24, #154).
//!
//! `walker::scan_candidates` indexes classes by `ClassEntry::tag`, which
//! RE-146 showed is the tag of the class's base, not its own. A class's own
//! tag is its definition ordinal (`formats::schema_classes`, RE-133). This
//! runs the candidate scan twice on each file's `Global/Latest`: with the
//! schema as `parse_schema` reads it, and with every tagged class given its
//! own tag. For each run it prints the candidates at the production
//! threshold, the ten commonest classes, and how many candidates' header
//! tag is the own tag of the class they are decoded as.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re150_walker_tags -- FILE.rvt ...

use rvt::walker::{self, WalkerLimits};
use rvt::{RevitFile, compression, formats, streams};
use std::collections::{BTreeMap, HashMap};

fn summary(
    label: &str,
    schema: &formats::SchemaTable,
    data: &[u8],
    own: &HashMap<String, u16>,
) -> String {
    let result = walker::scan_candidates_with_limits(
        schema,
        data,
        walker::PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
    );
    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    let mut own_tag = 0usize;
    for c in &result.candidates {
        *by_class.entry(c.class_name.clone()).or_default() += 1;
        own_tag += usize::from(own.get(&c.class_name) == Some(&c.class_tag));
    }
    let mut top: Vec<_> = by_class.into_iter().collect();
    top.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
    let top: Vec<String> = top
        .iter()
        .take(10)
        .map(|(c, n)| format!("{c:?}:{n}"))
        .collect();
    format!(
        "\"{label}\":{{\"candidates\":{},\"header_tag_is_own_tag\":{own_tag},\"top\":{{{}}}}}",
        result.candidates.len(),
        top.join(",")
    )
}

fn probe(path: &str) -> rvt::Result<String> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf
        .basic_file_info()
        .map_or_else(|_| "null".to_string(), |i| i.version.to_string());
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_stream_at(streams::FORMATS_LATEST, &formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let own: HashMap<String, u16> = formats::schema_classes(&formats_d)
        .classes
        .into_iter()
        .map(|c| (c.name, c.tag))
        .collect();
    let mut rewritten = schema.clone();
    for class in &mut rewritten.classes {
        if class.tag.is_some() {
            class.tag = own.get(&class.name).copied();
        }
    }
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let (_, data) = compression::inflate_stream_auto(streams::GLOBAL_LATEST, &raw)?;
    Ok(format!(
        "{{\"file\":{file:?},\"revit\":{revit},\"global_latest_bytes\":{},{},{}}}",
        data.len(),
        summary("base_tags", &schema, &data, &own),
        summary("own_tags", &rewritten, &data, &own)
    ))
}

fn main() {
    // Measure passes flags such as `--records` after the paths.
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", error.to_string()),
        }
    }
}
