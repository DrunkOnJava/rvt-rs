//! RE-367: each curtain mullion and panel type's material, from its native
//! `m_renderStyleId`, one line per type, for comparing a Revit 2015 to 2023 save
//! with the same project's 2024 save.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re367_curtain_materials -- \
//!     samples/2023-racadvancedsampleproject.rvt 2023 > 2023.txt
//! cargo run --profile ci -p rvt-probes --example probe_re367_curtain_materials -- \
//!     samples/2024-racadvancedsampleproject.rvt 2024 > 2024.txt
//! diff 2023.txt 2024.txt
//! ```

use rvt::RevitFile;
use rvt::elem_table::{declared_ids, parse_records};
use rvt::research::partition_curtain_materials::scan_curtain_type_materials;
use rvt::research::partition_materials::{scan_material_names, scan_materials_32};
use rvt::research::partition_schema_mvp::CURTAIN_32BIT_REVIT_VERSIONS;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, path, release] = &args[..] else {
        eprintln!("usage: probe_re367_curtain_materials <file.rvt> <release>");
        std::process::exit(2);
    };
    let mut file = RevitFile::open(path).expect("open the project");
    let release: u32 = release.parse().expect("a release year");
    let records = parse_records(&mut file).expect("read the element table");
    let declared = declared_ids(&records);
    let names = if CURTAIN_32BIT_REVIT_VERSIONS.contains(&release) {
        scan_materials_32(&mut file, release, &declared).1
    } else {
        scan_material_names(&mut file, release, &declared).expect("read the material names")
    };
    let materials = names.keys().copied().collect();
    let types =
        scan_curtain_type_materials(&mut file, release, &materials).expect("read the types");
    for (type_id, material) in types {
        println!("{type_id} {}", names[&material]);
    }
}
