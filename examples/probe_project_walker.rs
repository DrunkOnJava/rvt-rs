//! L5B-11 partial unblock — does the ADocument walker run cleanly on
//! real `.rvt` project files from Revit 2023 + 2024? Previously
//! documented as "reliable on 2024-2026 families only"; this probe
//! exercises `walker::read_adocument_lossy` on each file and reports
//! `(field count, entry offset, diagnostic count)`.
//!
//! Path resolves via `RVT_PROJECT_CORPUS_DIR` (default
//! `/private/tmp/rvt-corpus-probe/magnetar/Revit`).

use rvt::{RevitFile, walker};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let p2023 = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let p2024 = format!("{project_dir}/2024_Core_Interior.rvt");
    let files = [p2023.as_str(), p2024.as_str()];
    for path in files {
        let mut rf = RevitFile::open(path).unwrap();
        let lossy = walker::read_adocument_lossy(&mut rf);
        match lossy {
            Ok(d) => {
                let warn_count = d.diagnostics.warnings.len();
                let instance_desc = format!(
                    "{} fields, entry=0x{:x}",
                    d.value.fields.len(),
                    d.value.entry_offset
                );
                println!(
                    "{}: {} ({} diagnostics)",
                    path.rsplit('/').next().unwrap(),
                    instance_desc,
                    warn_count
                );
            }
            Err(e) => println!("{}: error — {e}", path.rsplit('/').next().unwrap()),
        }
    }
}
