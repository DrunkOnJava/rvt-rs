//! RE-365: the I section of each type on Revit 2017 to 2023, read from the 32-bit
//! value block (`f64 · ff×4 · i32`), one line per type, for comparing with the
//! same project's 2024 save.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re365_i_sections_32 -- \
//!     samples/2023-rstbasicsampleproject.rvt 2023 > 2023.txt
//! cargo run --profile ci -p rvt-probes --example probe_re365_i_sections_32 -- \
//!     samples/2024-rstbasicsampleproject.rvt 2024 > 2024.txt
//! diff 2023.txt 2024.txt
//! ```

use rvt::RevitFile;
use rvt::elem_table::{declared_ids, parse_records};
use rvt::research::partition_type_parameters::type_i_sections;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, path, release] = &args[..] else {
        eprintln!("usage: probe_re365_i_sections_32 <file.rvt> <release>");
        std::process::exit(2);
    };
    let mut file = RevitFile::open(path).expect("open the project");
    let release: u32 = release.parse().expect("a release year");
    let records = parse_records(&mut file).expect("read the element table");
    let types = declared_ids(&records);
    for (id, section) in type_i_sections(&mut file, release, &types) {
        println!(
            "{id} width {:.6} depth {:.6} web {:.6} flange {:.6}",
            section.width_feet, section.depth_feet, section.web_feet, section.flange_feet
        );
    }
}
