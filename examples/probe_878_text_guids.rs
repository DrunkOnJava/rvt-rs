//! #878: every GUID a file holds as text, for the base of the project's,
//! site's and building's GlobalIds, which Revit's export takes from
//! `ExportUtils.GetExportId` of the fake ElementIds -16, -15 and -14 and
//! which is none of the elements' GUIDs, the history's episodes or
//! BasicFileInfo's Unique Document GUID.
//!
//! FACT under test: the base is a GUID the file stores as text, XORed with
//! 0xFFFFFFF0 in its last 32 bits for the project.
//!
//! Output, per distinct GUID text (8-4-4-4-12 hex, ASCII or UTF-16) in any
//! stream, raw or inflated:
//!   guid <stream> <text>
//!
//! `tools/re/text_guids_vs_ifc.py` tests each against Revit's export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_878_text_guids -- FILE.rvt

use rvt::RevitFile;
use std::collections::BTreeSet;

fn guids(data: &[u8], step: usize, out: &mut BTreeSet<String>) {
    let unit = |i: usize| -> Option<u8> {
        let at = i * step;
        let b = *data.get(at)?;
        (step == 1 || *data.get(at + 1)? == 0).then_some(b)
    };
    let n = data.len() / step;
    let mut i = 0;
    while i + 36 <= n {
        let ok = (0..36).all(|k| match unit(i + k) {
            Some(c) => {
                if [8, 13, 18, 23].contains(&k) {
                    c == b'-'
                } else {
                    c.is_ascii_hexdigit()
                }
            }
            None => false,
        });
        if ok {
            let text: String = (0..36)
                .filter_map(|k| unit(i + k))
                .map(char::from)
                .collect();
            out.insert(text.to_ascii_lowercase());
            i += 36;
        } else {
            i += 1;
        }
    }
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    for name in rf.stream_names() {
        let mut found = BTreeSet::new();
        if let Ok(raw) = rf.read_stream(&name) {
            guids(&raw, 1, &mut found);
            guids(&raw, 2, &mut found);
            if raw.len() > 1 {
                guids(&raw[1..], 2, &mut found);
            }
        }
        if name.starts_with("Partitions/") {
            if let Ok(inflated) = rf.inflated_partition(&name) {
                let bytes = inflated.bytes();
                guids(bytes, 1, &mut found);
                guids(bytes, 2, &mut found);
                if bytes.len() > 1 {
                    guids(&bytes[1..], 2, &mut found);
                }
            }
        } else {
            let prepared = rvt::compression::prepare_stream_for_inflate(
                &name,
                &rf.read_stream(&name).unwrap_or_default(),
            );
            if let Ok((_, out)) = rvt::compression::inflate_at_auto(prepared.as_ref()) {
                guids(&out, 1, &mut found);
                guids(&out, 2, &mut found);
                if out.len() > 1 {
                    guids(&out[1..], 2, &mut found);
                }
            }
        }
        for text in found {
            println!("guid {name} {text}");
        }
    }
    Ok(())
}
