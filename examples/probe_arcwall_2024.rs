//! RE-13 — verify ArcWall decoder generalises to Revit 2024.
//!
//! On Revit 2023 (Einhoven) ArcWall's tag is 0x0191. For Revit 2024,
//! the class_tag_map documents tag drift per release; this probe
//! checks whether ArcWall in 2024 still uses 0x0191 or has drifted,
//! and whether the record envelope (variant 0x07fa, fixed header
//! 0x00088004) is identical.
//!
//! Expected outcomes:
//!   - Tag may drift by a small amount (2023 ArcWall=0x0191 → 2024 ArcWall=0x0192?)
//!   - Record envelope (variant + fixed header + coord block) should
//!     be stable.
//!
//! Output:
//!   - ArcWall tag observed on 2024 Core Interior Partitions/46.
//!   - Count of records passing the filter on that tag.
//!   - First 2-3 hex dumps for visual confirmation of envelope.

use rvt::arc_wall_record::{ARC_WALL_VARIANT_STANDARD, ArcWallRecord, SCHEMA_FAMILY_MARKER};
use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let file = "2024_Core_Interior.rvt";
    let path = format!("{project_dir}/{file}");

    let mut rf = match RevitFile::open(&path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot open {path}: {e}");
            return;
        }
    };

    // Resolve ArcWall tag from the schema.
    let Ok(formats_raw) = rf.read_stream(streams::FORMATS_LATEST) else {
        eprintln!("cannot read Formats/Latest");
        return;
    };
    let Ok(formats_d) = compression::inflate_at(&formats_raw, 0) else {
        eprintln!("cannot inflate Formats/Latest");
        return;
    };
    let Ok(schema) = formats::parse_schema(&formats_d) else {
        eprintln!("cannot parse schema");
        return;
    };
    let arc_wall_tag_2024 = schema
        .classes
        .iter()
        .find(|c| c.name == "ArcWall")
        .and_then(|c| c.tag);
    println!(
        "=== {file} — ArcWall tag in 2024 schema: {} ===",
        arc_wall_tag_2024
            .map(|t| format!("0x{t:04x}"))
            .unwrap_or_else(|| "(not found)".to_string())
    );

    if arc_wall_tag_2024.is_none() {
        // ArcWall not in 2024 schema — might be renamed.
        let mut candidates = Vec::new();
        for c in &schema.classes {
            if c.name.contains("Wall") {
                candidates.push((c.name.clone(), c.tag));
            }
        }
        println!("  Wall-containing class names in 2024 schema:");
        for (name, tag) in &candidates {
            println!(
                "    {name} tag={}",
                tag.map(|t| format!("0x{t:04x}")).unwrap_or("—".to_string())
            );
        }
        return;
    }
    let arc_wall_tag_2024 = arc_wall_tag_2024.unwrap();

    // Scan Partitions/46 (the large one) for this tag.
    let Ok(raw) = rf.read_stream("Partitions/46") else {
        eprintln!("cannot read Partitions/46");
        return;
    };
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();

    // Raw occurrences with the filter prefix.
    let mut filtered_occurrences: Vec<usize> = Vec::new();
    for i in 0..concat.len().saturating_sub(3) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v == arc_wall_tag_2024 && concat[i + 2] == 0x00 && concat[i + 3] == 0x00 {
            filtered_occurrences.push(i);
        }
    }
    println!(
        "  Partitions/46 ({} B): {} filtered occurrences of tag 0x{arc_wall_tag_2024:04x}",
        concat.len(),
        filtered_occurrences.len()
    );

    // Variant marker distribution at +0x10 across these occurrences.
    let mut variant_hist: BTreeMap<u16, usize> = BTreeMap::new();
    for &off in &filtered_occurrences {
        if off + 0x12 < concat.len() {
            let v = u16::from_le_bytes([concat[off + 0x10], concat[off + 0x11]]);
            *variant_hist.entry(v).or_insert(0) += 1;
        }
    }
    println!("\n  Variant marker distribution at +0x10:");
    let mut sorted: Vec<(u16, usize)> = variant_hist.into_iter().collect();
    sorted.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
    for (v, c) in sorted.iter().take(10) {
        let label = match *v {
            ARC_WALL_VARIANT_STANDARD => " (standard)",
            0x0821 => " (compound)",
            _ => "",
        };
        println!("    0x{v:04x}{label}: {c} records");
    }

    // Fixed header constant at +0x04 across standard-variant records.
    let standard_offsets: Vec<usize> = filtered_occurrences
        .iter()
        .filter(|&&off| {
            off + 0x12 < concat.len()
                && u16::from_le_bytes([concat[off + 0x10], concat[off + 0x11]])
                    == ARC_WALL_VARIANT_STANDARD
        })
        .copied()
        .collect();

    if !standard_offsets.is_empty() {
        let mut fh_hist: BTreeMap<u32, usize> = BTreeMap::new();
        for &off in &standard_offsets {
            let fh = u32::from_le_bytes([
                concat[off + 0x04],
                concat[off + 0x05],
                concat[off + 0x06],
                concat[off + 0x07],
            ]);
            *fh_hist.entry(fh).or_insert(0) += 1;
        }
        let mut sorted_fh: Vec<(u32, usize)> = fh_hist.into_iter().collect();
        sorted_fh.sort_by_key(|(_, c)| std::cmp::Reverse(*c));
        println!(
            "\n  Fixed header (+0x04) distribution across {} standard-variant records:",
            standard_offsets.len()
        );
        for (fh, c) in sorted_fh.iter().take(5) {
            let label = if *fh == SCHEMA_FAMILY_MARKER {
                " (SCHEMA_FAMILY_MARKER — matches 2023)"
            } else {
                ""
            };
            println!("    0x{fh:08x}{label}: {c} records");
        }
    }

    // Try the actual decoder.
    println!("\n  Decoder invocation on standard-variant records:");
    let mut ok = 0usize;
    let mut err = 0usize;
    // The decoder is hardcoded to tag 0x0191. If tag drifted, we need
    // to patch bytes before calling it. For this probe, just try and
    // report.
    for &off in standard_offsets.iter().take(5) {
        match ArcWallRecord::decode_standard(&concat, off) {
            Ok(rec) => {
                ok += 1;
                println!(
                    "    @{off:>8}: start=({:.3}, {:.3}, {:.3}), end=({:.3}, {:.3}, {:.3})",
                    rec.coords[0],
                    rec.coords[1],
                    rec.coords[2],
                    rec.coords[3],
                    rec.coords[4],
                    rec.coords[5]
                );
            }
            Err(e) => {
                err += 1;
                println!("    @{off:>8}: ERR {e}");
            }
        }
    }

    println!(
        "\n  Summary: tag 0x{arc_wall_tag_2024:04x}, {} filtered, \
         {} standard-variant, {ok} decoded ok, {err} err on sample of 5",
        filtered_occurrences.len(),
        standard_offsets.len()
    );
    if arc_wall_tag_2024 != 0x0191 {
        println!(
            "\n  NOTE: 2024 ArcWall tag is 0x{arc_wall_tag_2024:04x}, not 0x0191. \
             The ArcWallRecord decoder needs a tag parameter or version-keyed \
             constant to work on 2024 files. Patch accordingly."
        );
    }
}
