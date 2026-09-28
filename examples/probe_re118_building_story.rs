//! RE-118: a Revit Level keeps its Building Story setting in its data.
//!
//! FACT: past a Level's name, the first `ff × 8 · 00 01` is followed by an
//! `f64`, a `u64` ElementId and a byte that is 1 on a building story and 0
//! on any other Level. Against the Building Story parameter of Snowdon
//! Towers' VIM export it is exact on all 18 architectural and 19 structural
//! Levels (12 stories, 7 not); on Core Interior's 15 and RE1's 2 Levels it
//! is 1, and each is a storey of Revit's own IFC export. Only a building
//! story is a storey of Revit's export.
//!
//! The probe prints each Level with its elevation and setting.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re118_building_story -- MODEL.rvt

use rvt::RevitFile;

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let levels =
        rvt::partition_level_records::recover_partition_levels(&mut rf, version).expect("levels");
    for level in levels {
        println!(
            "{}\t{}\t{:.4} ft\tbuilding story {:?}",
            level.element_id, level.name, level.elevation_feet, level.building_story
        );
    }
}
