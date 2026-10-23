//! List the elements of a Revit file, counted by class, with the ElementId
//! and provenance of the first few.
//!
//! ```sh
//! cargo run --example basics_elements -- path/to/project.rvt
//! ```
//!
//! Without an argument it opens the synthetic project file the repository
//! ships, `corpus/tier1/architectural-2024/architectural-2024.rvt`.

use std::collections::BTreeMap;

use rvt::RevitFile;

const FIXTURE: &str = "corpus/tier1/architectural-2024/architectural-2024.rvt";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| FIXTURE.to_string());
    let mut rf = RevitFile::open(&path)?;

    let mut by_class: BTreeMap<String, usize> = BTreeMap::new();
    for (index, element) in rvt::walker::iter_elements(&mut rf)?.enumerate() {
        if index < 5 {
            println!(
                "{} {:?} from {:?}",
                element.class, element.id, element.provenance
            );
        }
        *by_class.entry(element.class).or_default() += 1;
    }
    for (class, count) in &by_class {
        println!("{count:6} {class}");
    }
    println!("{} elements", by_class.values().sum::<usize>());
    Ok(())
}
