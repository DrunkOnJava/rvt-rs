//! RE-242 (#1016, W7): whether a model keeps the IFC export setup's
//! "Export base quantities" choice.
//!
//! FACT under test: revit-ifc (CLEANROOM.md source 5) saves an export setup
//! in the model as JSON text naming each setting (`IFCExportConfiguration`,
//! RE-239 §4), and writes base quantities (`Qto_` sets) only when its
//! `ExportBaseQuantities` setting is true. Hypothesis: where a model keeps a
//! setup, its `ExportBaseQuantities` is true when Revit's reference export
//! writes `Qto_` sets and false when it writes none.
//!
//! Output: for each UTF-16 or UTF-8 `"ExportBaseQuantities"` in every
//! stream, stored and inflated, every setting with a true, false or number
//! value in the 4,000 characters around it (string settings, which hold
//! names and paths, are not printed):
//!   cfg <stream> <utf8|utf16> <occurrence> <setting>=<value> ...
//! then the occurrences per encoding:
//!   total <utf8|utf16> <count>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re242_export_setup -- MODEL.rvt

use rvt::RevitFile;

const NEEDLE: &str = "\"ExportBaseQuantities\"";
const WINDOW: usize = 2000;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let utf16: Vec<u8> = NEEDLE.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let mut totals = [0usize; 2];
    for name in rf.stream_names() {
        let Ok(stored) = rf.read_stream(&name) else {
            continue;
        };
        let mut chunks = rvt::compression::inflate_all_chunks_for_stream(&name, &stored);
        chunks.push(stored);
        for chunk in &chunks {
            for at in memchr::memmem::find_iter(chunk, NEEDLE.as_bytes()) {
                totals[0] += 1;
                let from = at.saturating_sub(WINDOW);
                let to = (at + WINDOW).min(chunk.len());
                let text = String::from_utf8_lossy(&chunk[from..to]);
                println!("cfg {name} utf8 {} {}", totals[0], settings(&text));
            }
            for at in memchr::memmem::find_iter(chunk, &utf16) {
                totals[1] += 1;
                let from = at - (at.min(2 * WINDOW) & !1);
                let to = (at + 2 * WINDOW).min(chunk.len());
                let units: Vec<u16> = chunk[from..to]
                    .chunks_exact(2)
                    .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                    .collect();
                let text = String::from_utf16_lossy(&units);
                println!("cfg {name} utf16 {} {}", totals[1], settings(&text));
            }
        }
    }
    println!("total utf8 {}", totals[0]);
    println!("total utf16 {}", totals[1]);
    Ok(())
}

/// Every `"Setting":value` in `text` whose value is true, false or a number.
fn settings(text: &str) -> String {
    let mut out = Vec::new();
    for (colon, _) in text.match_indices("\":") {
        let Some(open) = text[..colon].rfind('"') else {
            continue;
        };
        let key = &text[open + 1..colon];
        let value = &text[colon + 2..];
        let end = value
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-'))
            .unwrap_or(value.len());
        let value = &value[..end];
        let plain = value == "true"
            || value == "false"
            || (!value.is_empty() && value.parse::<f64>().is_ok());
        if plain && !key.is_empty() && key.chars().all(|c| c.is_ascii_alphanumeric()) {
            out.push(format!("{key}={value}"));
        }
    }
    out.join(" ")
}
