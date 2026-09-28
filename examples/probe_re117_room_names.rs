//! RE-117: a room keeps its number and name as parameter entries, on Revit
//! 2023, 2024 and 2025.
//!
//! FACT: a room's data is an object `01 00 00 00 · id` (`u32` on 2023, `u64`
//! later), and within 0x400 bytes past it come the entries `-1006901
//! (ROOM_NUMBER) · u32 n · UTF-16 × n` and, right after, `-1006900
//! (ROOM_NAME) · u32 m · UTF-16 × m`, the parameter ids as wide as the
//! ElementIds. They equal Revit's number and name on Core Interior (116),
//! Snowdon Towers (54, against the VIM), RE1 Architecture (11) and
//! Exemplo_data (9).
//!
//! The probe prints each exported room's number and name.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re117_room_names -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;
use std::collections::BTreeSet;

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())
        .expect("partition records");
    let rooms: BTreeSet<u32> = mvp.rooms.iter().filter_map(|room| room.id).collect();
    let found =
        rvt::partition_room_parameters::scan_room_parameter_entries(&mut rf, version, &rooms);
    for id in &rooms {
        println!("{id}\t{:?}", found.get(id));
    }
}
