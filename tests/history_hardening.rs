//! Hostile input to the history-stream readers behind GlobalIds: `EpisodeHistory::parse` and
//! `project_guid` given hand-made bytes that no real file holds (a stream that ends early,
//! counts that disagree, a table length past the stream or overflowing it, a header whose
//! copies differ). Real files drive the same readers through the exporter in
//! `tests/revit_global_ids.rs`; these are the malformed cases they never reach, moved here
//! from the module's unit tests (Griffin, 2026-11-27: broken-file tests belong with the
//! hostile-input suite).

use rvt::research::revit_global_ids::{
    EPISODE_COUNT_OFFSET, EPISODE_TABLE_OFFSET, EPISODE_TERMINATOR, EpisodeHistory,
    PROJECT_GUID_OFFSETS, project_guid,
};

/// A history as `EpisodeHistory::parse` reads it: the count at `0x0e`, a table
/// of `table` words, the count again and the entries (a GUID and a last byte).
fn history(
    count_at_head: u32,
    table: u32,
    count_in_list: u32,
    entries: &[([u8; 16], u8)],
) -> Vec<u8> {
    let mut out = vec![0u8; EPISODE_TABLE_OFFSET];
    out[EPISODE_COUNT_OFFSET..EPISODE_COUNT_OFFSET + 4]
        .copy_from_slice(&count_at_head.to_le_bytes());
    out.extend_from_slice(&table.to_le_bytes());
    for _ in 0..table.min(64) {
        out.extend_from_slice(&0u32.to_le_bytes());
    }
    out.extend_from_slice(&count_in_list.to_le_bytes());
    for (guid, last) in entries {
        out.extend_from_slice(guid);
        out.push(*last);
    }
    out
}

#[test]
fn a_history_that_ends_early_or_disagrees_is_refused() {
    let entries = [
        ([1u8; 16], EPISODE_TERMINATOR),
        ([2u8; 16], EPISODE_TERMINATOR),
    ];
    assert!(EpisodeHistory::parse(&[]).is_none());
    assert!(EpisodeHistory::parse(&[0u8; EPISODE_TABLE_OFFSET]).is_none());
    // The count at the head and the count before the list differ.
    assert!(EpisodeHistory::parse(&history(2, 3, 3, &entries)).is_none());
    // The list holds fewer entries than it says.
    assert!(EpisodeHistory::parse(&history(3, 3, 3, &entries)).is_none());
    // A table length that would run past the stream, and one that overflows.
    assert!(EpisodeHistory::parse(&history(2, 1_000, 2, &entries)).is_none());
    assert!(EpisodeHistory::parse(&history(2, u32::MAX, 2, &entries)).is_none());
}

#[test]
fn an_entry_closed_by_another_byte_is_read_like_the_rest() {
    let entries = [
        ([1u8; 16], EPISODE_TERMINATOR),
        ([2u8; 16], 0x2a),
        ([3u8; 16], EPISODE_TERMINATOR),
    ];
    let parsed = EpisodeHistory::parse(&history(3, 2, 3, &entries)).expect("parses");
    assert_eq!(parsed.len(), 3);
    // Episodes count from the last entry, whatever byte closes each one.
    assert_eq!(parsed.episode_guid(0), Some([3u8; 16]));
    assert_eq!(parsed.episode_guid(1), Some([2u8; 16]));
    assert_eq!(parsed.episode_guid(2), Some([1u8; 16]));
    assert_eq!(parsed.episode_guid(3), None);
}

/// A history header with `copies` at the project GUID's four offsets.
fn header(copies: [[u8; 16]; 4]) -> Vec<u8> {
    let mut out = vec![0u8; EPISODE_TABLE_OFFSET];
    for (at, copy) in PROJECT_GUID_OFFSETS.iter().zip(copies) {
        out[*at..*at + 16].copy_from_slice(&copy);
    }
    out
}

#[test]
fn a_header_read_wrongly_gives_no_project_guid() {
    let guid = [7u8; 16];
    // Too short for the last copy, with or without the other three.
    assert_eq!(project_guid(&[]), None);
    assert_eq!(
        project_guid(&header([guid; 4])[..PROJECT_GUID_OFFSETS[3] + 15]),
        None
    );
    // Any one copy that differs.
    for odd in [0, 1, 2, 3] {
        let mut copies = [guid; 4];
        copies[odd] = [8u8; 16];
        assert_eq!(project_guid(&header(copies)), None, "copy {odd} differs");
    }
    // All four equal and zero: an unset header, not a GUID.
    assert_eq!(project_guid(&header([[0u8; 16]; 4])), None);
}

#[test]
fn four_equal_copies_are_the_project_guid() {
    let guid: [u8; 16] = std::array::from_fn(|i| i as u8 + 1);
    assert_eq!(project_guid(&header([guid; 4])), Some(guid));
    // Whatever follows the header is not read.
    let mut longer = header([guid; 4]);
    longer.extend_from_slice(&[0xff; 40]);
    assert_eq!(project_guid(&longer), Some(guid));
}

#[test]
fn an_empty_list_has_no_episode() {
    let parsed = EpisodeHistory::parse(&history(0, 0, 0, &[])).expect("parses");
    assert!(parsed.is_empty());
    assert_eq!(parsed.episode_guid(0), None);
}
