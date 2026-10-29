//! RE-264 (#1092): which `Formats/Latest` field encodings does Revit 2027
//! add? The 2027 sample family's schema has 334 fields `FieldType::decode`
//! leaves `Unknown`, where every release from 2016 to 2026 has 9 to 12.
//!
//! The probe prints every field of the schema, one per line: class, field,
//! the `FieldType` variant, and the encoding bytes (`FieldType::encode`, the
//! raw bytes for an `Unknown` one), then the Unknown fields' encodings by
//! leading bytes. Run on two releases' files and join the lines on the field
//! name: a field 2026 classifies and 2027 leaves Unknown shows the encoding
//! that changed.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re264_schema_field_encodings -- FILE.rfa
use rvt::RevitFile;
use rvt::formats::FieldType;
use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re264_schema_field_encodings FILE"))?;
    let mut rf = RevitFile::open(&path)?;
    let schema = rf.schema()?;
    let mut unknown: BTreeMap<String, usize> = BTreeMap::new();
    let mut fields = 0usize;
    for class in &schema.classes {
        for field in &class.fields {
            fields += 1;
            let Some(field_type) = &field.field_type else {
                println!("field {}::{} none -", class.name, field.name);
                continue;
            };
            let variant = format!("{field_type:?}");
            let variant = variant.split([' ', '{', '(']).next().unwrap_or("");
            let hex: String = field_type
                .encode()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("field {}::{} {variant} {hex}", class.name, field.name);
            if let FieldType::Unknown { bytes } = field_type {
                let lead: String = bytes.iter().take(4).map(|b| format!("{b:02x}")).collect();
                *unknown.entry(lead).or_default() += 1;
            }
        }
    }
    println!(
        "{path}: {} classes, {fields} fields, {} Unknown",
        schema.classes.len(),
        unknown.values().sum::<usize>()
    );
    for (lead, count) in &unknown {
        println!("unknown lead {lead}: {count}");
    }
    Ok(())
}
