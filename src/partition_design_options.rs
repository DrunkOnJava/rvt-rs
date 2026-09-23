//! Design option sets and their primary options (#319, RE-40).
//!
//! An element that belongs to a design option carries the option's
//! ElementId at `+0x2a` of its element record
//! ([`crate::partition_element_records::DESIGN_OPTION_OFFSET`]). Revit's own
//! IFC export writes the main model and each option set's primary option,
//! and leaves the other options out. Which option is primary is stored in
//! the set's name entry:
//!
//! ```text
//! option-set partition record (OST_DesignOptionSets, in the RE-35 chain)
//!   +0x00  u64  the set's ElementId
//!   +0x12  i64  -2006112
//!   +0x56  u32  k
//!   +0x5a  k*8  the set's own ElementId, then its k - 1 options
//!
//! option-set name entry
//!   u32  k - 1
//!   ...  the same options, u64 each, in the same order
//!   u32  1
//!   u32  n, the name's length in UTF-16 code units
//!   n*2  the set's name, UTF-16LE
//!   u64  the ElementId of the set's primary option
//! ```
//!
//! On Autodesk's Snowdon Towers 2024 architectural sample the two sets are
//! "Bandstand Options" (four options, primary 1500724) and "Bleacher
//! Seating" (two options, primary 2059970). Each set's name entry occurs
//! once in the file, and its primary is the option whose elements Revit's
//! IFC4 export holds. Of the placed instances of recovered categories, the
//! 27 in a non-primary option (9 floors, 8 walls and 9 slab edges in
//! 2059967, one generic model in 1500727) are all missing from it, and the
//! 29 in 2059970 are all in it.
//!
//! The option records themselves (`OST_DesignOptions`) differ only in their
//! ids, and a set lists its options in ascending id order, so neither says
//! which option is primary.
//!
//! # Fail closed
//!
//! A set whose name entry is missing, or whose entries disagree about the
//! primary, stays unresolved, and elements in its options are kept as
//! before. Only an option of a resolved set that is not its primary is
//! reported as non-primary.

use crate::partition_element_records::{
    CATEGORY_OFFSET, PartitionElementRecord, bbox_marker, partition_record_chain,
    supports_revit_version,
};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Autodesk `BuiltInCategory.OST_DesignOptions`.
pub const OST_DESIGN_OPTIONS: i64 = -2_006_114;

/// Autodesk `BuiltInCategory.OST_DesignOptionSets`.
pub const OST_DESIGN_OPTION_SETS: i64 = -2_006_112;

/// Offset, from the start of an option-set partition record, of the `u32`
/// count that opens the set's id list.
pub const OPTION_LIST_OFFSET: usize = 0x56;

/// Most options a set is read with. A count above it is not a set list.
pub const OPTIONS_PER_SET_MAX: usize = 64;

/// Longest set name, in UTF-16 code units, read from a name entry.
pub const SET_NAME_MAX_UNITS: usize = 256;

/// A design option set whose primary option was read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignOptionSet {
    /// The set's ElementId.
    pub set_id: u32,
    /// The set's name as stored.
    pub name: String,
    /// The set's options, in the order the set lists them.
    pub options: Vec<u32>,
    /// The primary option, one of [`Self::options`].
    pub primary: u32,
}

/// Every design option set of a file.
#[derive(Debug, Clone, Default)]
pub struct DesignOptions {
    /// Sets whose primary option was read, by set ElementId.
    pub sets: BTreeMap<u32, DesignOptionSet>,
    /// Sets found in a partition record chain whose name entry is missing
    /// or ambiguous. Their options are neither primary nor non-primary.
    pub unresolved_sets: BTreeSet<u32>,
}

impl DesignOptions {
    /// Whether `option` belongs to a resolved set and is not its primary.
    pub fn is_non_primary(&self, option: u32) -> bool {
        self.sets
            .values()
            .any(|set| set.primary != option && set.options.contains(&option))
    }

    /// Every option of a resolved set other than its primary.
    pub fn non_primary_options(&self) -> BTreeSet<u32> {
        self.sets
            .values()
            .flat_map(|set| set.options.iter().copied().filter(|o| *o != set.primary))
            .collect()
    }

    /// Whether `record` is in a non-primary design option, which Revit's
    /// export leaves out.
    pub fn excludes(&self, record: &PartitionElementRecord) -> bool {
        record
            .design_option
            .is_some_and(|option| self.is_non_primary(option))
    }
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The options an option-set partition record lists, or `None` when the
/// record in `buf[start..end]` is not one: its category must be
/// [`OST_DESIGN_OPTION_SETS`], and the list at [`OPTION_LIST_OFFSET`] must
/// start with the record's own `set_id`, stay inside the record, and name
/// at least one option distinct from the set.
pub fn option_set_members(buf: &[u8], start: usize, end: usize, set_id: u64) -> Option<Vec<u32>> {
    let category = read_u64(buf, start.checked_add(CATEGORY_OFFSET)?)? as i64;
    if category != OST_DESIGN_OPTION_SETS {
        return None;
    }
    let list_at = start.checked_add(OPTION_LIST_OFFSET)?;
    let count = read_u32(buf, list_at)? as usize;
    if !(2..=OPTIONS_PER_SET_MAX + 1).contains(&count) {
        return None;
    }
    let ids_at = list_at.checked_add(4)?;
    if ids_at.checked_add(count.checked_mul(8)?)? > end {
        return None;
    }
    if read_u64(buf, ids_at)? != set_id {
        return None;
    }
    let mut options = Vec::with_capacity(count - 1);
    for index in 1..count {
        let id = read_u64(buf, ids_at + index * 8)?;
        let id = u32::try_from(id).ok()?;
        if u64::from(id) == set_id || options.contains(&id) {
            return None;
        }
        options.push(id);
    }
    Some(options)
}

/// Every `(name, primary)` the name entries in `buf` give for the set that
/// lists `options`. A primary that is not one of `options`, or a name with
/// a control character or an unpaired surrogate, is not an entry.
pub fn find_set_entries(buf: &[u8], options: &[u32]) -> Vec<(String, u32)> {
    let mut needle = Vec::with_capacity(8 + options.len() * 8);
    needle.extend_from_slice(&(options.len() as u32).to_le_bytes());
    for option in options {
        needle.extend_from_slice(&u64::from(*option).to_le_bytes());
    }
    needle.extend_from_slice(&1u32.to_le_bytes());
    let mut out = Vec::new();
    for hit in memchr::memmem::find_iter(buf, &needle) {
        let length_at = hit + needle.len();
        let Some(units) = read_u32(buf, length_at).map(|n| n as usize) else {
            continue;
        };
        if !(1..=SET_NAME_MAX_UNITS).contains(&units) {
            continue;
        }
        let name_at = length_at + 4;
        let Some(bytes) = buf.get(name_at..name_at + units * 2) else {
            continue;
        };
        let code_units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let Ok(name) = String::from_utf16(&code_units) else {
            continue;
        };
        if name.trim().is_empty() || name.chars().any(char::is_control) {
            continue;
        }
        let Some(primary) = read_u64(buf, name_at + units * 2).and_then(|v| u32::try_from(v).ok())
        else {
            continue;
        };
        if options.contains(&primary) {
            out.push((name, primary));
        }
    }
    out
}

/// The design option sets of a whole file and their primary options.
/// Empty where the release's record shape is not proven.
/// [`crate::RevitFile::design_options`] memoises it.
pub fn compute_design_options(rf: &mut RevitFile) -> Result<DesignOptions> {
    let mut out = DesignOptions::default();
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version).filter(|_| supports_revit_version(version)) else {
        return Ok(out);
    };
    let streams = rf.partition_stream_names();
    let mut members: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in partition_record_chain(buf, &marker) {
            let Ok(set_id) = u32::try_from(span.element_id) else {
                continue;
            };
            if let Some(options) = option_set_members(buf, span.start, span.end, span.element_id) {
                // A set framed twice with different lists is unresolved.
                match members.get(&set_id) {
                    Some(held) if *held != options => {
                        out.unresolved_sets.insert(set_id);
                    }
                    _ => {
                        members.insert(set_id, options);
                    }
                }
            }
        }
    }
    for (set_id, options) in members {
        if out.unresolved_sets.contains(&set_id) {
            continue;
        }
        let mut entries: BTreeSet<(String, u32)> = BTreeSet::new();
        for stream in &streams {
            let Ok(inflated) = rf.inflated_partition(stream) else {
                continue;
            };
            entries.extend(find_set_entries(inflated.bytes(), &options));
        }
        let primaries: BTreeSet<u32> = entries.iter().map(|(_, primary)| *primary).collect();
        match (primaries.len(), entries.into_iter().next()) {
            (1, Some((name, primary))) => {
                out.sets.insert(
                    set_id,
                    DesignOptionSet {
                        set_id,
                        name,
                        options,
                        primary,
                    },
                );
            }
            _ => {
                out.unresolved_sets.insert(set_id);
            }
        }
    }
    Ok(out)
}

/// Placed instances of the
/// [`crate::partition_element_records::RECOVERED_CATEGORIES`] that are in a
/// non-primary design option, one per ElementId, keyed by class name. The
/// export leaves them out, as Revit's does; this counts them so the
/// diagnostics can say so.
pub fn scan_non_primary_option_instances(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<BTreeMap<String, usize>> {
    use crate::partition_element_records::{RECOVERED_CATEGORIES, scan_category_records_multi};
    let mut by_class = BTreeMap::new();
    if !supports_revit_version(revit_version) {
        return Ok(by_class);
    }
    let options = rf.design_options();
    if options.sets.is_empty() {
        return Ok(by_class);
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => records.into_iter().map(|r| r.id_primary).collect(),
        Err(_) => return Ok(by_class),
    };
    let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
    let records = scan_category_records_multi(rf, revit_version, &categories, &declared)?;
    let mut excluded: BTreeMap<u32, i64> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| r.is_exported_instance() && options.excludes(r))
    {
        excluded.insert(record.element_id, record.builtin_category);
    }
    for category in excluded.into_values() {
        if let Some((_, class)) = RECOVERED_CATEGORIES.iter().find(|(c, _)| *c == category) {
            *by_class.entry((*class).to_string()).or_insert(0) += 1;
        }
    }
    Ok(by_class)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SET: u32 = 2_059_966;
    const OPTION_A: u32 = 2_059_967;
    const OPTION_B: u32 = 2_059_970;

    /// A partition-record-shaped span for an option set, list at `+0x56`.
    fn set_record(set_id: u32, options: &[u32]) -> Vec<u8> {
        let mut buf = vec![0xffu8; OPTION_LIST_OFFSET];
        buf[0..8].copy_from_slice(&u64::from(set_id).to_le_bytes());
        buf[CATEGORY_OFFSET..CATEGORY_OFFSET + 8]
            .copy_from_slice(&(OST_DESIGN_OPTION_SETS as u64).to_le_bytes());
        buf.extend_from_slice(&(options.len() as u32 + 1).to_le_bytes());
        buf.extend_from_slice(&u64::from(set_id).to_le_bytes());
        for option in options {
            buf.extend_from_slice(&u64::from(*option).to_le_bytes());
        }
        buf.extend_from_slice(&[0u8; 32]);
        buf
    }

    /// The name entry that closes with the set's primary.
    fn set_entry(options: &[u32], name: &str, primary: u32) -> Vec<u8> {
        let mut buf = vec![0xffu8; 56];
        buf.extend_from_slice(&[0, 0, 0]);
        buf.extend_from_slice(&(options.len() as u32).to_le_bytes());
        for option in options {
            buf.extend_from_slice(&u64::from(*option).to_le_bytes());
        }
        buf.extend_from_slice(&1u32.to_le_bytes());
        let units: Vec<u16> = name.encode_utf16().collect();
        buf.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for unit in units {
            buf.extend_from_slice(&unit.to_le_bytes());
        }
        buf.extend_from_slice(&u64::from(primary).to_le_bytes());
        buf.extend_from_slice(&0xadu32.to_le_bytes());
        buf
    }

    #[test]
    fn an_option_set_record_lists_its_options_after_its_own_id() {
        let buf = set_record(SET, &[OPTION_A, OPTION_B]);
        assert_eq!(
            option_set_members(&buf, 0, buf.len(), u64::from(SET)),
            Some(vec![OPTION_A, OPTION_B])
        );
        // Another record's id, another category or a list running past the
        // record's end is not a set.
        assert_eq!(option_set_members(&buf, 0, buf.len(), 1), None);
        assert_eq!(option_set_members(&buf, 0, 0x60, u64::from(SET)), None);
        let mut option_record = buf.clone();
        option_record[CATEGORY_OFFSET..CATEGORY_OFFSET + 8]
            .copy_from_slice(&(OST_DESIGN_OPTIONS as u64).to_le_bytes());
        assert_eq!(
            option_set_members(&option_record, 0, option_record.len(), u64::from(SET)),
            None
        );
    }

    #[test]
    fn a_set_name_entry_gives_the_primary_option() {
        let mut buf = vec![0u8; 13];
        buf.extend(set_entry(
            &[OPTION_A, OPTION_B],
            "Bleacher Seating",
            OPTION_B,
        ));
        assert_eq!(
            find_set_entries(&buf, &[OPTION_A, OPTION_B]),
            vec![("Bleacher Seating".to_string(), OPTION_B)]
        );
        // The options must match in order and number.
        assert!(find_set_entries(&buf, &[OPTION_B, OPTION_A]).is_empty());
        assert!(find_set_entries(&buf, &[OPTION_A]).is_empty());
    }

    #[test]
    fn a_primary_outside_the_set_or_a_control_character_is_not_an_entry() {
        let outside = set_entry(&[OPTION_A, OPTION_B], "Bleacher Seating", SET);
        assert!(find_set_entries(&outside, &[OPTION_A, OPTION_B]).is_empty());
        let control = set_entry(&[OPTION_A, OPTION_B], "Bleacher\u{1}", OPTION_B);
        assert!(find_set_entries(&control, &[OPTION_A, OPTION_B]).is_empty());
    }

    fn record_in(option: Option<u32>) -> PartitionElementRecord {
        PartitionElementRecord {
            stream: "Partitions/68".into(),
            offset: 0,
            element_id: 2_062_310,
            flags: 0,
            builtin_category: crate::partition_element_records::OST_FLOORS,
            container: crate::partition_element_records::CONTAINER_NONE,
            placement_kind: crate::partition_element_records::PLACEMENT_KIND_INSTANCE,
            bbox_feet: [0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            preceding_reference: None,
            owner_reference: None,
            references: Vec::new(),
            id_from_enclosing_record: true,
            design_option: option,
        }
    }

    #[test]
    fn only_a_resolved_set_s_other_options_are_excluded() {
        let mut options = DesignOptions::default();
        options.sets.insert(
            SET,
            DesignOptionSet {
                set_id: SET,
                name: "Bleacher Seating".into(),
                options: vec![OPTION_A, OPTION_B],
                primary: OPTION_B,
            },
        );
        assert!(options.excludes(&record_in(Some(OPTION_A))));
        assert!(!options.excludes(&record_in(Some(OPTION_B))));
        assert!(!options.excludes(&record_in(None)));
        // An option no resolved set lists is kept (fail closed).
        assert!(!options.excludes(&record_in(Some(1_500_727))));
        assert_eq!(options.non_primary_options(), BTreeSet::from([OPTION_A]));
    }
}
