//! Probe the framing of `Global/Latest` — do instances have a class
//! tag prefix we can scan for? What sits between records?
//!
//! Strategy: find the ADocument entry via the walker, dump the 32 bytes
//! before and after, then look for any recognisable class-tag pattern
//! between ADocument's end and the next apparent record start.

use rvt::{RevitFile, compression, formats, streams, walker};

fn main() {
    let paths = [
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt",
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/2024_Core_Interior.rvt",
    ];
    for path in paths {
        let Ok(mut rf) = RevitFile::open(path) else {
            println!("{path}: open failed");
            continue;
        };
        let Ok(formats_raw) = rf.read_stream(streams::FORMATS_LATEST) else {
            continue;
        };
        let Ok((_, formats_d)) = compression::inflate_at_auto(&formats_raw) else {
            continue;
        };
        let Ok(schema) = formats::parse_schema(&formats_d) else {
            continue;
        };
        let Some(adoc) = schema.classes.iter().find(|c| c.name == "ADocument") else {
            continue;
        };

        let Ok(latest_raw) = rf.read_stream(streams::GLOBAL_LATEST) else {
            continue;
        };
        let Ok((_, d)) = compression::inflate_at_auto(&latest_raw) else {
            continue;
        };

        let detection = walker::detect_adocument_start(&d, Some(adoc));
        let Some(adoc_offset) = detection.offset else {
            println!("{path}: ADocument not detected");
            continue;
        };

        println!(
            "\n{}: Global/Latest={} B, ADocument at 0x{:x}, ADocument tag={:?}",
            path.rsplit('/').next().unwrap(),
            d.len(),
            adoc_offset,
            adoc.tag
        );

        // Dump 32 bytes before and 64 bytes at the entry offset
        let before_start = adoc_offset.saturating_sub(32);
        print!("  bytes before 0x{:x}: ", adoc_offset);
        for i in before_start..adoc_offset {
            print!("{:02x} ", d[i]);
        }
        println!();

        print!("  bytes at/after:       ");
        for i in adoc_offset..(adoc_offset + 32).min(d.len()) {
            print!("{:02x} ", d[i]);
        }
        println!();

        // Look for recognisable class tags as u16 LE in the 256 bytes
        // before ADocument. The schema has class tags like 0x0061,
        // 0x006b, etc. — if those appear as a u16 aligned to a 4-byte
        // boundary in the gap, we've found a per-instance class-tag
        // framing pattern.
        let known_tags: std::collections::HashSet<u16> =
            schema.classes.iter().filter_map(|c| c.tag).collect();
        let scan_start = adoc_offset.saturating_sub(256);
        let mut tag_hits: Vec<(usize, u16)> = Vec::new();
        for i in (scan_start..adoc_offset.saturating_sub(2)).step_by(2) {
            let v = u16::from_le_bytes([d[i], d[i + 1]]);
            if v != 0 && known_tags.contains(&v) {
                tag_hits.push((i, v));
            }
        }
        if !tag_hits.is_empty() {
            println!("  known tags in 256 B before ADocument:");
            for (off, tag) in tag_hits.iter().take(15) {
                let name = schema
                    .classes
                    .iter()
                    .find(|c| c.tag == Some(*tag))
                    .map(|c| c.name.as_str())
                    .unwrap_or("?");
                println!(
                    "    0x{:06x} (rel -0x{:x})  tag=0x{:04x}  {}",
                    off,
                    adoc_offset - off,
                    tag,
                    name
                );
            }
            if tag_hits.len() > 15 {
                println!("    ... +{} more", tag_hits.len() - 15);
            }
        }

        // Also scan AFTER ADocument to see if there's a similar pattern
        let scan_end = (adoc_offset + 2048).min(d.len().saturating_sub(2));
        let mut after_hits = 0usize;
        for i in (adoc_offset + 32..scan_end).step_by(2) {
            let v = u16::from_le_bytes([d[i], d[i + 1]]);
            if v != 0 && known_tags.contains(&v) {
                after_hits += 1;
            }
        }
        println!(
            "  known tags in 2 KB after ADocument+32: {after_hits} hits ({:.1}/KB)",
            after_hits as f64 / 2.0
        );
    }
}
