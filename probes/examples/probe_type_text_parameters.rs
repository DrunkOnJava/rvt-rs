//! RE-289 (#35, W7): which BuiltInParameter holds a family type's Manufacturer
//! text, and whether its Fire Rating '-' is stored.
//!
//! FACT under test: a type's text parameters sit in its value block as
//! `i64 BuiltInParameter · u32 n · UTF-16 × n` (RE-77). Revit's IFC4 export of
//! the BIM4LCA office writes `Pset_ManufacturerTypeInformation.Manufacturer`
//! on 6 family types ('Thorlux Lighting', 'IDO', 'Ido', 'Korpinen Oy',
//! 'Väinö Korpinen Oy') and `Pset_DoorCommon.FireRating` on door types, one of
//! them '-'. This probe lists, for each such text, the parameter id and the
//! owner ElementId of every entry that holds it, so the id can be read against
//! Revit's export (type Tags 1414255 IDO, 2575395 Ido, 1611986 Väinö Korpinen
//! Oy, 3365322 and 3376911 Thorlux Lighting, 1533300 Korpinen Oy, 2391770 '-').
//!
//! Output: one line per entry: `owner parameter text`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_type_text_parameters -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;

const TEXTS: [&str; 8] = [
    "Thorlux Lighting",
    "IDO",
    "Ido",
    "Korpinen Oy",
    "Väinö Korpinen Oy",
    "EI60",
    "EI30",
    "-",
];

const OWNERS_OF_DASH: [u64; 1] = [2_391_770];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut mark = [0xffu8; 59];
    mark[56] = 0;
    mark[57] = 0;
    mark[58] = 0;
    let mut seen: BTreeMap<(u64, i64, &str), usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let blocks: Vec<usize> = memchr::memmem::find_iter(buf, &mark).collect();
        for text in TEXTS {
            let units: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let mut pattern = u32::try_from(text.encode_utf16().count())?
                .to_le_bytes()
                .to_vec();
            pattern.extend_from_slice(&units);
            for at in memchr::memmem::find_iter(buf, &pattern) {
                let Some(entry) = at.checked_sub(8).and_then(|s| buf.get(s..at)) else {
                    continue;
                };
                let index = blocks.partition_point(|start| *start < at);
                let Some(start) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
                    continue;
                };
                let Some(owner) = start
                    .checked_sub(8)
                    .and_then(|s| buf.get(s..*start))
                    .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
                    .map(u64::from_le_bytes)
                else {
                    continue;
                };
                let parameter = i64::from_le_bytes(<[u8; 8]>::try_from(entry)?);
                if !(-2_000_000..0).contains(&parameter) {
                    continue;
                }
                if text == "-" && !OWNERS_OF_DASH.contains(&owner) {
                    continue;
                }
                *seen.entry((owner, parameter, text)).or_default() += 1;
            }
        }
    }
    for ((owner, parameter, text), count) in seen {
        println!("{owner} {parameter} {text} x{count}");
    }
    Ok(())
}
