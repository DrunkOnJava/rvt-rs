//! RE-364: the walls each wall's join lists name on Revit 2015 to 2023, in the
//! 32-bit layout, one line per wall, for comparing with the same project's
//! 2024 save.
//!
//! The ids to read are a file of whitespace-separated wall ElementIds, such as
//! the `Tag`s of an export's `IfcWall`s.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re364_wall_join_lists_32 -- \
//!     samples/2023-racadvancedsampleproject.rvt 2023 walls.txt > 2023.txt
//! cargo run --profile ci -p rvt-probes --example probe_re364_wall_join_lists_32 -- \
//!     samples/2024-racadvancedsampleproject.rvt 2024 walls.txt > 2024.txt
//! diff 2023.txt 2024.txt
//! ```

use rvt::RevitFile;
use rvt::research::partition_compound_structure::scan_wall_join_partners;
use std::collections::BTreeSet;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let [_, path, release, ids] = &args[..] else {
        eprintln!("usage: probe_re364_wall_join_lists_32 <file.rvt> <release> <wall-ids.txt>");
        std::process::exit(2);
    };
    let mut file = RevitFile::open(path).expect("open the project");
    let release: u32 = release.parse().expect("a release year");
    let walls: BTreeSet<u32> = std::fs::read_to_string(ids)
        .expect("read the ids")
        .split_whitespace()
        .map(|id| id.parse().expect("an ElementId"))
        .collect();
    let partners =
        scan_wall_join_partners(&mut file, release, &walls, &walls).expect("read the lists");
    for (wall, named) in partners {
        let named: Vec<String> = named.iter().map(u32::to_string).collect();
        println!("{wall} {}", named.join(","));
    }
}
