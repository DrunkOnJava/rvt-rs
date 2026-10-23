//! Open a Revit file and print what it is: its release, build, title, when
//! it was last saved, and what its schema declares.
//!
//! ```sh
//! cargo run --example basics_inspect -- path/to/model.rvt
//! ```
//!
//! Without an argument it opens the synthetic project file the repository
//! ships, `corpus/tier1/architectural-2024/architectural-2024.rvt`.

use rvt::RevitFile;

const FIXTURE: &str = "corpus/tier1/architectural-2024/architectural-2024.rvt";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| FIXTURE.to_string());

    let meta = rvt::metadata::read_metadata(&path)?;
    println!("Revit {}", meta.revit_version);
    println!("title: {}", meta.title.as_deref().unwrap_or("-"));
    println!("last saved: {}", meta.last_saved.as_deref().unwrap_or("-"));

    let mut rf = RevitFile::open(&path)?;
    let summary = rf.summarize_strict()?;
    println!("build: {}", summary.build.as_deref().unwrap_or("-"));
    println!("streams: {}", summary.streams.len());
    println!("schema classes: {}", summary.class_name_count);
    Ok(())
}
