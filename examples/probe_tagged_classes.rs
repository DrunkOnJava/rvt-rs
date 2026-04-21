//! RE-11 follow-up — list all 80 tagged schema classes with their
//! field counts and parents. The "interesting" class names I tested
//! (Wall, Floor, Door, Level) don't have tags — they're likely
//! abstract parents. Real element tags must be on their subtypes.

use rvt::{RevitFile, compression, formats, streams};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let mut rf = RevitFile::open(&path).unwrap();
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    let formats_d = compression::inflate_at(&formats_raw, 0).unwrap();
    let schema = formats::parse_schema(&formats_d).unwrap();

    let mut tagged: Vec<&formats::ClassEntry> =
        schema.classes.iter().filter(|c| c.tag.is_some()).collect();
    tagged.sort_by_key(|c| c.tag.unwrap());

    println!(
        "{} tagged classes (of {} total schema classes):",
        tagged.len(),
        schema.classes.len()
    );
    println!("{:>6}  {:>3}  {:<40}  {}", "tag", "F#", "name", "parent");
    println!("{}", "-".repeat(100));
    for c in &tagged {
        println!(
            "0x{:04x}  {:>3}  {:<40}  {}",
            c.tag.unwrap(),
            c.fields.len(),
            c.name,
            c.parent.as_deref().unwrap_or("-")
        );
    }

    // Also report untagged classes that ARE in the interesting set.
    let interesting = [
        "Wall",
        "Floor",
        "Door",
        "Window",
        "Stair",
        "Column",
        "Beam",
        "Roof",
        "Ceiling",
        "Level",
        "Grid",
        "FamilyInstance",
        "Room",
    ];
    println!("\nInteresting classes (tag/fields/parent):");
    for name in interesting {
        if let Some(c) = schema.classes.iter().find(|c| c.name == name) {
            println!(
                "  {:<20}  tag={}  fields={}  parent={}  was_parent_only={}",
                c.name,
                c.tag.map(|t| format!("0x{:04x}", t)).unwrap_or("-".into()),
                c.fields.len(),
                c.parent.as_deref().unwrap_or("-"),
                c.was_parent_only
            );
        }
    }
}
