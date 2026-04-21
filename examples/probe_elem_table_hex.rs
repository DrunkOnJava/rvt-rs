//! Hex-dump Global/ElemTable first 512 bytes on family vs project files.
//! Goal: find the record-layout difference that makes parse_records_rough
//! work on family files (45 records) but early-terminate on project files
//! (2 records).

use rvt::{RevitFile, compression, streams};

fn dump(label: &str, bytes: &[u8], from: usize, to: usize) {
    println!("=== {} ({}..{}) ===", label, from, to);
    for row in (from..to.min(bytes.len())).step_by(16) {
        let hex: Vec<_> = bytes[row..row + 16]
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect();
        let ascii: String = bytes[row..row + 16]
            .iter()
            .map(|&b| {
                if b.is_ascii_graphic() || b == b' ' {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();
        println!(
            "  0x{:04x}  {} {}  |{}|",
            row,
            hex[..8].join(" "),
            hex[8..].join(" "),
            ascii
        );
    }
}

fn main() {
    // Resolve paths via env vars so this probe doesn't leak any user's
    // local home directory into the repo (PII guard in CI flags those).
    let family_path = std::env::var("RVT_FAMILY_2024").unwrap_or_else(|_| {
        format!(
            "{}/samples/racbasicsamplefamily-2024.rfa",
            std::env::var("RVT_SAMPLES_DIR").unwrap_or_else(|_| "../../samples".into())
        )
    });
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let project_2023 = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let project_2024 = format!("{project_dir}/2024_Core_Interior.rvt");
    let files = [
        ("FAMILY 2024", family_path.as_str()),
        ("PROJECT 2023", project_2023.as_str()),
        ("PROJECT 2024", project_2024.as_str()),
    ];
    for (label, path) in files {
        let mut rf = RevitFile::open(path).unwrap();
        let raw = rf.read_stream(streams::GLOBAL_ELEM_TABLE).unwrap();
        let d = compression::inflate_at_auto(&raw).unwrap().1;
        println!("\n### {} — {} bytes decompressed", label, d.len());
        dump(label, &d, 0x00, 0x90);
    }
}
