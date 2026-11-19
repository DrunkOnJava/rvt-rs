//! RE-459 (#878, #1929): the byte that closes each entry of `Global/History` on a
//! Revit 2018 file whose history `EpisodeHistory::parse` refuses.
//!
//! FACT under test: RE-48 reads the history as a count, a `u32`-counted table, the
//! count again and that many 17-byte entries, each a GUID and `0x28`, and
//! `EpisodeHistory::parse` returns `None` when any entry ends in another byte. On
//! the spent Vitoria "Before" model (Revit 2018, 1,127 entries) four entries are
//! not closed by `0x28` (RE-208's census finds its runs of closed entries with one
//! open entry between them), so `revit_global_ids` returns nothing and every
//! element gets a GlobalId derived from the document. This probe prints which byte
//! closes each entry, and the entries that do not end in `0x28` with their
//! neighbours, to say whether those four are a different kind of entry.
//!
//! It also prints the project's base GUID, which the header holds four times, and the
//! file's Levels, for `tools/re/history_base_vs_ifc.py`, which asks Revit's export
//! whether its project, building, site and storeys are that GUID with an id XORed in.
//!
//! Output:
//!   history <bytes> count0e=<u32|none> table66=<u32|none> list_count=<u32|none> parse=<ok N | none>
//!   base <offset> <32 hex, stored order>    at 0x16, 0x26, 0x36 and 0x56
//!   episode <number> <32 hex, stored order>  the oldest (0) and the newest episode's GUID
//!   stored <parameter id> <22 characters>   each GlobalId the file stores in UTF-16 after a
//!                                            built-in parameter's id and the length 22 (#878)
//!   level <ElementId> <elevation ft> <name>
//!   terminators <byte hex>=<entries> ...
//!   odd <index> <offset> <hex of the entry> before=<hex of the entry before> after=<hex of the one after>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re459_history_terminators -- FILE.rvt

use rvt::research::partition_level_records::recover_partition_levels;
use rvt::research::revit_global_ids::{
    EPISODE_COUNT_OFFSET, EPISODE_ENTRY_LEN, EPISODE_TABLE_OFFSET, EPISODE_TERMINATOR,
    EpisodeHistory, IFC_BUILDING_GUID_PARAMETER, IFC_GUID_PARAMETER, IFC_PROJECT_GUID_PARAMETER,
    IFC_SITE_GUID_PARAMETER, expand_ifc_guid,
};
use rvt::{RevitFile, compression, streams};
use std::collections::{BTreeMap, BTreeSet};

/// Where the header holds the project's base GUID: three copies in a row, then 16
/// zero bytes, then a fourth that ends where the table's count begins.
const BASE_GUID_AT: [usize; 4] = [0x16, 0x26, 0x36, 0x56];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn inflate(rf: &mut RevitFile, name: &str) -> Result<Vec<u8>, String> {
    let stored = rf.read_stream(name).map_err(|e| e.to_string())?;
    let prepared = compression::prepare_stream_for_inflate(name, &stored);
    let data = prepared.as_ref();
    for offset in [0, 4, 8, 16] {
        if compression::has_gzip_magic(data, offset) {
            if let Ok(out) = compression::inflate_at(data, offset) {
                return Ok(out);
            }
        }
    }
    compression::inflate_at_auto(data)
        .map(|(_, out)| out)
        .map_err(|e| e.to_string())
}

/// The built-in parameters a GlobalId is stored under, and `-1` for one stored under none.
const STORED_PARAMETERS: [i32; 5] = [
    IFC_GUID_PARAMETER,
    IFC_PROJECT_GUID_PARAMETER,
    IFC_BUILDING_GUID_PARAMETER,
    IFC_SITE_GUID_PARAMETER,
    -1,
];

/// The 22-character GlobalId stored in UTF-16 at `at`, when it is one.
fn utf16_global_id(buf: &[u8], at: usize) -> Option<String> {
    let units = buf.get(at..at.checked_add(44)?)?;
    let text: String = units
        .chunks_exact(2)
        .map(|unit| (unit[1] == 0).then_some(char::from(unit[0])))
        .collect::<Option<_>>()?;
    expand_ifc_guid(&text).map(|_| text)
}

/// Every GlobalId the file stores after a built-in parameter's id and the length 22, with
/// that id (the reader of `stored_global_ids` looks in the same place).
fn stored_strings(rf: &mut RevitFile) -> BTreeSet<(i32, String)> {
    let mut out = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for parameter in STORED_PARAMETERS {
            let mut needle = [0u8; 8];
            needle[..4].copy_from_slice(&parameter.to_le_bytes());
            needle[4..].copy_from_slice(&22u32.to_le_bytes());
            for hit in memchr::memmem::find_iter(buf, &needle) {
                if let Some(text) = utf16_global_id(buf, hit + needle.len()) {
                    out.insert((parameter, text));
                }
            }
        }
    }
    out
}

fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: probe_re459_history_terminators FILE.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let history = match inflate(&mut rf, streams::GLOBAL_HISTORY) {
        Ok(history) => history,
        Err(error) => {
            println!("history error {error}");
            return;
        }
    };
    let count = u32_at(&history, EPISODE_COUNT_OFFSET);
    let table = u32_at(&history, EPISODE_TABLE_OFFSET);
    let list_count_at = table.and_then(|skipped| {
        (skipped as usize)
            .checked_mul(4)?
            .checked_add(EPISODE_TABLE_OFFSET + 4)
    });
    let list_count = list_count_at.and_then(|at| u32_at(&history, at));
    let parsed_history = EpisodeHistory::parse(&history);
    let parsed = parsed_history
        .as_ref()
        .map_or("none".to_string(), |parsed| format!("ok {}", parsed.len()));
    println!(
        "history {} count0e={count:?} table66={table:?} list_count={list_count:?} parse={parsed}",
        history.len()
    );
    // The project's base GUID, in the header, four times in the 2018 files read so
    // far (RE-459): the GUID Revit's export XORs the fake ids -16, -15 and -14 into
    // for the project, the building and the site, and a Level's ElementId into for
    // a storey.
    for at in BASE_GUID_AT {
        let bytes = history.get(at..at + 16).map_or("none".to_string(), hex);
        println!("base {at:#x} {bytes}");
    }
    // The oldest and the newest episode's GUID, to say whether the base is one of them.
    if let Some(parsed) = &parsed_history {
        for episode in [0, parsed.len().saturating_sub(1) as u32] {
            let bytes = parsed
                .episode_guid(episode)
                .map_or("none".to_string(), |guid| hex(&guid));
            println!("episode {episode} {bytes}");
        }
    }
    for (parameter, text) in stored_strings(&mut rf) {
        println!("stored {parameter} {text}");
    }
    let version = rf.basic_file_info().map(|info| info.version);
    if let Ok(version) = version {
        for level in recover_partition_levels(&mut rf, version).unwrap_or_default() {
            println!(
                "level {} {:.6} {}",
                level.element_id, level.elevation_feet, level.name
            );
        }
    }
    let (Some(count), Some(at), Some(list_count)) = (count, list_count_at, list_count) else {
        return;
    };
    if list_count != count {
        println!("counts differ");
        return;
    }
    let start = at + 4;
    let entry = |index: usize| {
        let from = start.checked_add(index.checked_mul(EPISODE_ENTRY_LEN)?)?;
        history.get(from..from.checked_add(EPISODE_ENTRY_LEN)?)
    };
    let mut terminators: BTreeMap<u8, usize> = BTreeMap::new();
    let mut odd = Vec::new();
    for index in 0..count as usize {
        let Some(bytes) = entry(index) else {
            println!("entries run past the stream at {index}");
            break;
        };
        *terminators.entry(bytes[EPISODE_ENTRY_LEN - 1]).or_default() += 1;
        if bytes[EPISODE_ENTRY_LEN - 1] != EPISODE_TERMINATOR {
            odd.push(index);
        }
    }
    let line: Vec<String> = terminators
        .iter()
        .map(|(byte, entries)| format!("{byte:02x}={entries}"))
        .collect();
    println!("terminators {}", line.join(" "));
    for index in odd {
        let shown = |at: Option<usize>| at.and_then(entry).map_or("none".to_string(), hex);
        println!(
            "odd {index} {} {} before={} after={}",
            start + index * EPISODE_ENTRY_LEN,
            shown(Some(index)),
            shown(index.checked_sub(1)),
            shown(index.checked_add(1)),
        );
    }
}
