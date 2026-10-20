//! #852 (probe): what the schema path's typed decoders return on real
//! files.
//!
//! `elements::decode_instance_prefer_typed_with_limits` sends every
//! `Global/Latest` candidate of a class in `elements::MVP_TYPED_CLASSES`
//! to that class's registered decoder, and each decoder rejects only a
//! wrong schema name. This prints, for each of those classes, the fields
//! the file's schema declares, the candidates `walker::scan_candidates`
//! finds at `PRODUCTION_ELEMENT_MIN_SCORE`, and every row
//! `walker::iter_elements` yields from them: its ElementId, offset, score
//! and each field's decoded kind (`Bytes` where the walk could not read
//! the field as its schema type). A required-field check for a class can
//! then be chosen from the rows of real instances, not guessed.
//!
//! Usage:
//!   cargo run --profile ci --example probe_typed_schema_rows -- FILE.rvt [...]

use rvt::walker::{self, InstanceField};
use rvt::{RevitFile, compression, elements, formats, streams};
use std::collections::{BTreeMap, HashMap};

fn kind(value: &InstanceField) -> String {
    match value {
        InstanceField::Pointer { raw } => format!("Pointer({:x},{:x})", raw[0], raw[1]),
        InstanceField::ElementId { tag, id } => format!("ElementId({tag}:{id})"),
        InstanceField::RefContainer { col_a, .. } => format!("RefContainer[{}]", col_a.len()),
        InstanceField::Integer { value, size, .. } => format!("Int{}({value})", size * 8),
        InstanceField::Float { value, size } => format!("F{}({value})", size * 8),
        InstanceField::Bool(b) => format!("Bool({b})"),
        InstanceField::Guid(_) => "Guid".into(),
        InstanceField::String(s) => format!("String({:?})", s.chars().take(40).collect::<String>()),
        InstanceField::Vector(v) => format!("Vector[{}]", v.len()),
        InstanceField::Bytes(b) => format!("Bytes[{}]", b.len()),
    }
}

fn probe(path: &str) -> rvt::Result<()> {
    let mut rf = RevitFile::open(path)?;
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_stream_at(streams::FORMATS_LATEST, &formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let (_, data) = compression::inflate_stream_auto(streams::GLOBAL_LATEST, &raw)?;

    for class in elements::MVP_TYPED_CLASSES {
        match schema.classes.iter().find(|c| c.name == *class) {
            Some(c) => {
                let fields: Vec<String> = c
                    .fields
                    .iter()
                    .map(|f| format!("{}:{:?}", f.name, f.field_type))
                    .collect();
                println!(
                    "schema {class} parent={:?} fields={} [{}]",
                    c.parent,
                    c.fields.len(),
                    fields.join(" | ")
                );
            }
            None => println!("schema {class} absent"),
        }
    }

    let candidates = walker::scan_candidates(&schema, &data, walker::PRODUCTION_ELEMENT_MIN_SCORE);
    let mut score_at: HashMap<usize, i64> = HashMap::new();
    let mut candidates_by_class: BTreeMap<&str, usize> = BTreeMap::new();
    for c in &candidates {
        if let Some(class) = elements::MVP_TYPED_CLASSES
            .iter()
            .find(|k| **k == c.class_name)
        {
            *candidates_by_class.entry(*class).or_default() += 1;
            score_at.insert(c.offset, c.score);
        }
    }
    for class in elements::MVP_TYPED_CLASSES {
        println!(
            "candidates {class} {}",
            candidates_by_class.get(class).copied().unwrap_or(0)
        );
    }

    let mut rows_by_class: BTreeMap<String, (usize, usize, usize)> = BTreeMap::new();
    for e in walker::iter_elements(&mut rf)? {
        if e.provenance.source_stream.as_deref() != Some(streams::GLOBAL_LATEST)
            || !elements::MVP_TYPED_CLASSES.contains(&e.class.as_str())
        {
            continue;
        }
        let offset = e.provenance.source_offset.unwrap_or(usize::MAX);
        let bytes_fields = e
            .fields
            .iter()
            .filter(|(_, v)| matches!(v, InstanceField::Bytes(_)))
            .count();
        let entry = rows_by_class.entry(e.class.clone()).or_default();
        entry.0 += 1;
        entry.1 += usize::from(e.id.is_some());
        entry.2 += usize::from(bytes_fields > 0);
        let fields: Vec<String> = e
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={}", kind(value)))
            .collect();
        println!(
            "row {} id={} offset=0x{offset:x} score={} decoder={} bytes_fields={bytes_fields} len={} [{}]",
            e.class,
            e.id.map_or("-".into(), |id| id.to_string()),
            score_at.get(&offset).map_or("?".into(), |s| s.to_string()),
            e.provenance.decoder.as_deref().unwrap_or("-"),
            e.byte_range.len(),
            fields.join(" | "),
        );
    }
    for (class, (rows, with_id, with_bytes)) in &rows_by_class {
        println!("summary {class} rows={rows} with_id={with_id} with_bytes_field={with_bytes}");
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    if paths.is_empty() {
        eprintln!("usage: probe_typed_schema_rows FILE.rvt [...]");
        std::process::exit(2);
    }
    for path in &paths {
        println!("file {}", path.rsplit('/').next().unwrap_or(path));
        if let Err(e) = probe(path) {
            println!("error {e}");
        }
    }
}
