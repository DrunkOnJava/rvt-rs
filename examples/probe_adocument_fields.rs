use rvt::{RevitFile, walker};

fn main() {
    let path = "/private/tmp/rvt-corpus-probe/magnetar/Revit/Revit_IFC5_Einhoven.rvt";
    let mut rf = RevitFile::open(path).unwrap();
    let d = walker::read_adocument_lossy(&mut rf).unwrap();
    println!(
        "ADocument: {} fields, entry=0x{:x}",
        d.value.fields.len(),
        d.value.entry_offset
    );
    for (i, (name, value)) in d.value.fields.iter().enumerate() {
        println!("  {:2}. {} = {:?}", i, name, value);
    }
}
