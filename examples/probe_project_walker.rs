use rvt::{RevitFile, walker};

fn main() {
    let files = [
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt",
        "/private/tmp/rvt-corpus-probe/magnetar/Revit/2024_Core_Interior.rvt",
    ];
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
