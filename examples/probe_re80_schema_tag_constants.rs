//! RE-80: three "per-release constants" are class tags in the file's own
//! schema.
//!
//! FACTS, each checked here on every file given:
//!
//! 1. The `u16` that opens `Global/ElemTable` (read so far as a per-release
//!    `element_count`: 1174 on 2016, 1411 on 2024) is the serialization tag
//!    of the class `ElemTable` in the file's `Formats/Latest`.
//! 2. The 8-byte element-record marker before the bounding box (RE-32) is
//!    `[tag of Outline][0xFF x 4][tag of ElementParents]`: the record
//!    serializes its box as an `Outline`, then its reference lists as
//!    `ElementParents`.
//! 3. The `u16` in each partition record-chain header (RE-35, "the marker
//!    constant minus 12") is the tag of `ElementHeader`, on 2024 to 2026.
//!    Before 2024 no partition opens with a chain of that shape.
//!
//! A class's tag is its definition ordinal in the schema (RE-76). Verify on
//! the family corpus (2016 to 2026) and a 2024 and a 2025 project:
//!
//! ```text
//! cargo run --profile ci --example probe_re80_schema_tag_constants -- \
//!     "$RVT_SAMPLES_DIR"/*.rfa 2024_Core_Interior.rvt RE1-Architecture.rvt
//! ```
//!
//! Each line reports the three tags, whether the ElemTable header equals the
//! first, how many `[Outline][0xFF x 4]` prefixes the partitions hold and how
//! many of them end in the `ElementParents` tag and are followed by a
//! well-formed box (six finite doubles, each minimum at most its maximum),
//! and how many chain records open the partitions with the `ElementHeader`
//! tag.

use rvt::partition_element_records::partition_record_chain;

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let mut rf = rvt::RevitFile::open(&path)?;
        let version = rf.basic_file_info().map(|b| b.version).unwrap_or(0);
        let classes = rf.schema_classes()?;
        let tag = |name: &str| {
            classes
                .classes
                .iter()
                .find(|c| c.name == name)
                .map(|c| c.tag)
        };
        let (Some(elem_table), Some(outline), Some(parents), Some(header)) = (
            tag("ElemTable"),
            tag("Outline"),
            tag("ElementParents"),
            tag("ElementHeader"),
        ) else {
            println!("{version} {path}: a class is missing from the schema");
            continue;
        };
        let table_header = rvt::elem_table::parse_header(&mut rf)?.element_count;
        let mut prefix = [0xffu8; 6];
        prefix[..2].copy_from_slice(&outline.to_le_bytes());
        // `partition_record_chain` takes the marker and reads the chain's
        // header constant as its last u16 minus 12.
        let mut chain_marker = [0xffu8; 8];
        chain_marker[6..].copy_from_slice(&(header + 12).to_le_bytes());
        let (mut prefixes, mut markers, mut boxes, mut chain) = (0, 0, 0, 0);
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            chain += partition_record_chain(buf, &chain_marker).len();
            for at in memchr::memmem::find_iter(buf, &prefix) {
                prefixes += 1;
                if buf.get(at + 6..at + 8) != Some(&parents.to_le_bytes()[..]) {
                    continue;
                }
                markers += 1;
                let values: Vec<f64> = (0..6)
                    .filter_map(|i| buf.get(at + 8 + i * 8..at + 16 + i * 8))
                    .filter_map(|b| b.try_into().ok().map(f64::from_le_bytes))
                    .collect();
                if values.len() == 6
                    && values.iter().all(|v| v.is_finite())
                    && (0..3).all(|i| values[i] <= values[i + 3])
                {
                    boxes += 1;
                }
            }
        }
        println!(
            "{version} {}: ElemTable {elem_table} (header {table_header}, {}), Outline {outline}, \
             ElementParents {parents}, ElementHeader {header}; [Outline FFx4] {prefixes}, \
             ending ElementParents {markers}, with a box {boxes}; chain records {chain}",
            path.rsplit('/').next().unwrap_or(&path),
            if table_header == elem_table {
                "equal"
            } else {
                "DIFFERENT"
            },
        );
    }
    Ok(())
}
