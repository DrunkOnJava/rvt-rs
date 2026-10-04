//! Room number / name / host Level from the Revit 2024 partition
//! parameter block a room owns (#90, RE-29).
//!
//! RE-22 found that a per-instance parameter *value* sits at a fixed
//! negative offset from two copies of the owning `ElementId`, and used
//! that to read the `IFC Export As` override. A room's number and name
//! are carried by the same shape, read from the other end: anchor on
//! the owning `ElementId` and walk **forward**.
//!
//! ```text
//! +0x000  u64  owning room ElementId
//! +0x03c  u64  owning room ElementId   (confirmation; must agree)
//! +0x044  u64  host Level ElementId
//! +0x1f5  u32  room number length, in UTF-16 code units
//! +0x1f9  2*n  room Number, UTF-16LE
//! +…+ 8   u32  room name length, in UTF-16 code units   (8 bytes past
//! +…+12   2*m  room Name, UTF-16LE                       the number)
//! ```
//!
//! The name's offset is not fixed because the number's length is not:
//! the name's length prefix sits [`NAME_GAP_AFTER_NUMBER`] bytes past
//! the end of the number string, which is why the block is parsed
//! forward from the owner rather than backward from either string.
//!
//! # Honesty
//!
//! - Every offset here is **measured**, on one file, over the 232
//!   blocks that frame its 116 rooms. The parameter block's own header
//!   is not decoded; these are fixed displacements inside it, exactly
//!   as RE-22's two owner offsets are.
//! - The scan is anchored on an ElementId the caller supplies, and it
//!   requires the confirmation copy at [`OWNER_CONFIRM_OFFSET`] to
//!   carry the same id. Measured over every one of the 26 425 ids
//!   `Global/ElemTable` declares on `2024_Core_Interior.rvt`: the
//!   framing accepts **116** of them and they are exactly the 116
//!   rooms Revit's own export emits as `IfcSpace` — no other declared
//!   id accepts it at all.
//! - Recovery is **all or nothing per room**. A room framed by two
//!   blocks that disagree resolves to nothing rather than to either.
//!   On the recorded edge all 116 rooms are framed twice and no pair
//!   disagrees.
//! - The recovered `(number, name)` pairs were scored against the
//!   reference export: 116 of 116 names equal the `IfcSpace.LongName`
//!   Revit writes and 116 of 116 numbers equal its `IfcSpace.Name`.
//! - The Level slot at [`LEVEL_OFFSET`] is returned verbatim. Nothing
//!   here checks that it is a Level — the caller does, against the
//!   `OST_Levels` id set, the same way
//!   [`crate::element_record_level_refs`] does.
//!
//! See `reports/element-framing/RE-29-room-instance-and-envelope.md`.

use crate::{Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where this framing is corpus-proven.
pub const ROOM_PARAMETER_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Bytes from the owning ElementId to its confirmation copy.
pub const OWNER_CONFIRM_OFFSET: usize = 0x3c;

/// Bytes from the owning ElementId to the host Level ElementId.
pub const LEVEL_OFFSET: usize = 0x44;

/// Bytes from the owning ElementId to the number's `u32` length prefix.
pub const NUMBER_LENGTH_OFFSET: usize = 505;

/// Bytes from the end of the number string to the name's length prefix.
pub const NAME_GAP_AFTER_NUMBER: usize = 8;

/// Longest number / name the scan will accept, in UTF-16 code units.
pub const MAX_VALUE_CHARS: usize = 64;

/// Value recorded for where a recovered room number / name came from.
pub const ROOM_PARAMETER_SOURCE: &str = "partition_room_parameter_block";

/// One accepted room parameter block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomParameters {
    /// Stream the block was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the owning ElementId in the inflated stream.
    pub offset: usize,
    /// The room this block belongs to.
    pub element_id: u32,
    /// Room number, e.g. `"66"`.
    pub number: String,
    /// Room name, e.g. `"Stair 2"`.
    pub name: String,
    /// Raw Level slot at [`LEVEL_OFFSET`], verbatim.
    pub level_slot: u64,
}

impl RoomParameters {
    /// The Level ElementId this block names, when it is one of
    /// `level_ids`. Fail-closed on a slot outside the `u32` range or
    /// outside the recovered Level set.
    pub fn level_element_id(&self, level_ids: &BTreeSet<u32>) -> Option<u32> {
        let id = u32::try_from(self.level_slot).ok()?;
        level_ids.contains(&id).then_some(id)
    }
}

/// Whether this release's room parameter framing is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    ROOM_PARAMETER_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// A UTF-16LE string of `chars` code units at `at`, rejected when it
/// holds a control character.
fn utf16_at(buf: &[u8], at: usize, chars: usize) -> Option<String> {
    if !(1..=MAX_VALUE_CHARS).contains(&chars) {
        return None;
    }
    let bytes = buf.get(at..at.checked_add(chars.checked_mul(2)?)?)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    if text.chars().any(char::is_control) {
        return None;
    }
    Some(text)
}

/// Decode the room parameter block whose owning ElementId sits at
/// `owner_at`, fail-closed.
///
/// Returns `None` unless the confirmation copy at
/// [`OWNER_CONFIRM_OFFSET`] carries the same `element_id` and both
/// strings decode as non-empty, control-free UTF-16LE.
pub fn decode_at(
    stream: &str,
    buf: &[u8],
    owner_at: usize,
    element_id: u32,
) -> Option<RoomParameters> {
    if read_u64(buf, owner_at)? != u64::from(element_id) {
        return None;
    }
    let confirm_at = owner_at.checked_add(OWNER_CONFIRM_OFFSET)?;
    if read_u64(buf, confirm_at)? != u64::from(element_id) {
        return None;
    }
    let level_slot = read_u64(buf, owner_at.checked_add(LEVEL_OFFSET)?)?;

    let number_len_at = owner_at.checked_add(NUMBER_LENGTH_OFFSET)?;
    let number_chars = read_u32(buf, number_len_at)? as usize;
    let number_at = number_len_at.checked_add(4)?;
    let number = utf16_at(buf, number_at, number_chars)?;

    let name_len_at = number_at
        .checked_add(number_chars.checked_mul(2)?)?
        .checked_add(NAME_GAP_AFTER_NUMBER)?;
    let name_chars = read_u32(buf, name_len_at)? as usize;
    let name = utf16_at(buf, name_len_at.checked_add(4)?, name_chars)?;

    Some(RoomParameters {
        stream: stream.to_string(),
        offset: owner_at,
        element_id,
        number,
        name,
        level_slot,
    })
}

/// Every accepted room parameter block in `buf` for an id in `rooms`.
pub fn find_room_parameters(
    stream: &str,
    buf: &[u8],
    rooms: &BTreeSet<u32>,
) -> Vec<RoomParameters> {
    let mut out = Vec::new();
    let (Some(&low), Some(&high)) = (rooms.first(), rooms.last()) else {
        return out;
    };
    if buf.len() < 8 {
        return out;
    }
    // Most offsets hold no room id; the range test settles them before
    // the set lookup.
    let (low, high) = (u64::from(low), u64::from(high));
    for owner_at in 0..=(buf.len() - 8) {
        let raw = u64::from_le_bytes(buf[owner_at..owner_at + 8].try_into().expect("8 bytes"));
        if raw == 0 || raw < low || raw > high {
            continue;
        }
        let element_id = raw as u32;
        if !rooms.contains(&element_id) {
            continue;
        }
        if let Some(block) = decode_at(stream, buf, owner_at, element_id) {
            out.push(block);
        }
    }
    out
}

/// Scan every `Partitions/*` stream for the parameter block of each
/// room in `rooms`, keeping only rooms whose blocks all agree.
///
/// Returns an empty map for unsupported releases (fail closed).
pub fn scan_room_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
    rooms: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, RoomParameters>> {
    if !supports_revit_version(revit_version) || rooms.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut found: BTreeMap<u32, Vec<RoomParameters>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for block in find_room_parameters(&stream, inflated.bytes(), rooms) {
            found.entry(block.element_id).or_default().push(block);
        }
    }
    Ok(resolve_unique(found))
}

/// Keep one block per room, and only when every block found for it
/// agrees on the number, the name and the Level slot.
pub fn resolve_unique(found: BTreeMap<u32, Vec<RoomParameters>>) -> BTreeMap<u32, RoomParameters> {
    let mut out = BTreeMap::new();
    for (id, blocks) in found {
        let Some(first) = blocks.first() else {
            continue;
        };
        let agree = blocks.iter().all(|block| {
            block.number == first.number
                && block.name == first.name
                && block.level_slot == first.level_slot
        });
        if agree {
            out.insert(id, first.clone());
        }
    }
    out
}

/// BuiltInParameter `ROOM_FINISH_FLOOR`, a room's Floor Finish (RE-153).
pub const ROOM_FINISH_FLOOR_PARAMETER: i64 = -1_006_903;
/// Bytes of a data object's header on Revit 2024 and later: `i32 ElementId ·
/// i32 0 · u32 Adler-32 · i32 size · i32 class`.
pub const DATA_OBJECT_HEADER: usize = 20;
/// How far before a parameter entry its data object's header is looked for.
pub const DATA_OBJECT_SEARCH: usize = 200_000;

/// The ElementId of the data object whose payload holds the byte at `at`
/// (RE-153): the last header before it whose `size` bytes of payload end in
/// the size again and enclose `at`, and whose Adler-32, over the class word
/// and the payload less its last 4 bytes, verifies. Revit 2024 and later.
pub fn enclosing_data_object(buf: &[u8], at: usize) -> Option<u32> {
    let low = at.saturating_sub(DATA_OBJECT_SEARCH);
    let mut p = at.checked_sub(DATA_OBJECT_HEADER)?;
    loop {
        if let Some(object) = verified_data_object(buf, p).filter(|object| object.end > at) {
            return Some(object.element_id);
        }
        if p <= low {
            return None;
        }
        p -= 1;
    }
}

/// Every verified data object in `buf` (RE-153), with its header's offset,
/// in one pass: a header is tried only where its zero word, its size and the
/// size that ends its payload agree, before its Adler-32 is computed. Use it
/// where [`enclosing_data_object`] would be called for many offsets of one
/// buffer, each a backward search of up to [`DATA_OBJECT_SEARCH`] bytes.
pub fn data_objects(buf: &[u8]) -> Vec<(usize, DataObject)> {
    let u32_at = |p: usize| u32::from_le_bytes(buf[p..p + 4].try_into().expect("4 bytes"));
    let mut out = Vec::new();
    let mut p = 0;
    let (mut failed, mut failed_bytes, mut gap_steps) = (0usize, 0usize, 0usize);
    let mut failed_time = std::time::Duration::ZERO;
    let mut ok_time = std::time::Duration::ZERO;
    while p + DATA_OBJECT_HEADER <= buf.len() {
        let size = u32_at(p + 12) as usize;
        let end = p + DATA_OBJECT_HEADER + size;
        let plausible =
            u32_at(p + 4) == 0 && size >= 4 && end <= buf.len() && u32_at(end - 4) as usize == size;
        let verified = if plausible {
            let t = std::time::Instant::now();
            let verified = verified_data_object(buf, p);
            if verified.is_some() {
                ok_time += t.elapsed();
            } else {
                failed_time += t.elapsed();
                failed += 1;
                failed_bytes += size;
            }
            verified
        } else {
            None
        };
        match verified {
            Some(object) => {
                out.push((p, object));
                p = object.end;
            }
            None => {
                gap_steps += 1;
                p += 1
            }
        }
    }
    if buf.len() > 1_000_000 {
        eprintln!(
            "[subtiming] data_objects counters: {} bytes, {} objects in {:.3}s, {failed} failed plausible headers of {failed_bytes} bytes in {:.3}s, {gap_steps} single steps",
            buf.len(),
            out.len(),
            ok_time.as_secs_f64(),
            failed_time.as_secs_f64()
        );
    }
    out
}

/// A data object whose header starts at a known offset (RE-153).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataObject {
    /// The ElementId in its header.
    pub element_id: u32,
    /// Its class word.
    pub class: u32,
    /// The offset just past its payload.
    pub end: usize,
}

/// The data object whose header starts at `p`, when its payload ends in its
/// size again and its Adler-32, over the class word and the payload less its
/// last 4 bytes, verifies. Revit 2024 and later.
pub fn verified_data_object(buf: &[u8], p: usize) -> Option<DataObject> {
    let u32_at = |p: usize| {
        buf.get(p..p.checked_add(4)?)
            .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
    };
    if u32_at(p.checked_add(4)?)? != 0 {
        return None;
    }
    let (id, sum, size, class) = (u32_at(p)?, u32_at(p + 8)?, u32_at(p + 12)?, u32_at(p + 16)?);
    let size = size as usize;
    let end = p.checked_add(DATA_OBJECT_HEADER)?.checked_add(size)?;
    if size < 4 || end > buf.len() || u32_at(end - 4) != Some(size as u32) {
        return None;
    }
    let mut adler = simd_adler32::Adler32::new();
    adler.write(&class.to_le_bytes());
    adler.write(&buf[p + DATA_OBJECT_HEADER..end - 4]);
    (adler.finish() == sum).then_some(DataObject {
        element_id: id,
        class,
        end,
    })
}

/// Each room's value of the text parameter `parameter`, by ElementId
/// (RE-153), on Revit 2024 and later. The value is an entry `parameter (i64) ·
/// u32 n · UTF-16 × n`, joined to its room by the data object holding it
/// ([`enclosing_data_object`]). A room whose entries disagree gets nothing.
/// On RE1 Architecture the Floor Finish ([`ROOM_FINISH_FLOOR_PARAMETER`]) of
/// 11 of 11 rooms is Revit's `Pset_SpaceCommon.FloorCovering`.
pub fn scan_room_text_parameter(
    rf: &mut RevitFile,
    revit_version: u32,
    rooms: &BTreeSet<u32>,
    parameter: i64,
) -> BTreeMap<u32, String> {
    scan_text_parameter(rf, revit_version, rooms, parameter)
}

/// Each element of `elements`' value of the text parameter `parameter`, a
/// BuiltInParameter or a project or shared parameter's ElementId, by
/// ElementId, on Revit 2024 and later: an entry `parameter (i64) · u32 n ·
/// UTF-16 × n` in the element's own data object ([`enclosing_data_object`],
/// RE-153, RE-156). An element whose entries disagree gets nothing.
pub fn scan_text_parameter(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &BTreeSet<u32>,
    parameter: i64,
) -> BTreeMap<u32, String> {
    if revit_version < 2024 || elements.is_empty() {
        return BTreeMap::new();
    }
    let tag = parameter.to_le_bytes();
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &tag) {
            let Some(value) = utf16_entry(buf, hit + tag.len(), 256) else {
                continue;
            };
            let Some(element) = enclosing_data_object(buf, hit).filter(|id| elements.contains(id))
            else {
                continue;
            };
            match found.get_mut(&element) {
                None => {
                    found.insert(element, Some(value));
                }
                Some(held) => {
                    if held.as_ref() != Some(&value) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(element, value)| Some((element, value?)))
        .collect()
}

/// BuiltInParameter `FUNCTION_PARAM`, a type's Function: 0 interior, 1
/// exterior (RE-158).
pub const FUNCTION_PARAMETER: i64 = -1_001_006;

/// Each element of `elements`' value of the integer parameter `parameter`,
/// by ElementId, on Revit 2024 and later (RE-158): an entry `parameter (i64)
/// · u32 value` in the element's own data object ([`enclosing_data_object`]).
/// An element whose entries disagree gets nothing. On RE1 Architecture the
/// door and floor types' [`FUNCTION_PARAMETER`] is Revit's `IsExternal` on
/// 5 of 5 doors (interior and exterior) and 2 of 2 floors.
pub fn scan_integer_parameter(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &BTreeSet<u32>,
    parameter: i64,
) -> BTreeMap<u32, u32> {
    if revit_version < 2024 || elements.is_empty() {
        return BTreeMap::new();
    }
    let tag = parameter.to_le_bytes();
    let mut found: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &tag) {
            let Some(value) = read_u32(buf, hit + tag.len()) else {
                continue;
            };
            let Some(element) = enclosing_data_object(buf, hit).filter(|id| elements.contains(id))
            else {
                continue;
            };
            match found.get_mut(&element) {
                None => {
                    found.insert(element, Some(value));
                }
                Some(held) => {
                    if *held != Some(value) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(element, value)| Some((element, value?)))
        .collect()
}

/// Prefix of the first string of a project or shared parameter's definition:
/// its group's ForgeTypeId (RE-156).
pub const PARAMETER_GROUP_PREFIX: &str = "autodesk.parameter.group:";
/// How far past its group string a definition's name is looked for.
pub const PARAMETER_NAME_WINDOW: usize = 64;

/// Each project or shared parameter's ElementId by its name, on Revit 2024
/// and later (RE-156). A definition is a data object whose strings are its
/// group's ForgeTypeId ([`PARAMETER_GROUP_PREFIX`]), its name and, for a
/// shared parameter, its GUID (`revit.local.shared:…`); the name is the first
/// `u32 n · UTF-16 × n` entry of 2 or more characters within
/// [`PARAMETER_NAME_WINDOW`] bytes past the group string. On RE1 Electrical
/// and Plumbing `Serial Number` is 490488 and 447886. A name two definitions
/// give different ids gets nothing.
pub fn find_parameter_definitions(rf: &mut RevitFile, revit_version: u32) -> BTreeMap<String, u32> {
    if revit_version < 2024 {
        return BTreeMap::new();
    }
    let needle: Vec<u8> = PARAMETER_GROUP_PREFIX
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut found: BTreeMap<String, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &needle) {
            let Some(group) = hit.checked_sub(4).and_then(|at| utf16_entry(buf, at, 256)) else {
                continue;
            };
            let after = hit + 2 * group.encode_utf16().count();
            let name = (after..after + PARAMETER_NAME_WINDOW).find_map(|at| {
                utf16_entry(buf, at, 256).filter(|name| name.encode_utf16().count() >= 2)
            });
            let (Some(name), Some(id)) = (name, enclosing_data_object(buf, hit)) else {
                continue;
            };
            match found.get_mut(&name) {
                None => {
                    found.insert(name, Some(id));
                }
                Some(held) => {
                    if *held != Some(id) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(name, id)| Some((name, id?)))
        .collect()
}

/// `u32 n · UTF-16 × n` at `at`: 1 to `limit` code units, no control character.
fn utf16_entry(buf: &[u8], at: usize, limit: usize) -> Option<String> {
    let n = usize::try_from(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?)).ok()?;
    if !(1..=limit).contains(&n) {
        return None;
    }
    let units: Vec<u16> = buf
        .get(at + 4..at + 4 + 2 * n)?
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    (!text.chars().any(char::is_control)).then_some(text)
}

/// BuiltInParameter `ROOM_NUMBER` (RE-117).
pub const ROOM_NUMBER_PARAMETER: i64 = -1_006_901;
/// BuiltInParameter `ROOM_NAME` (RE-117).
pub const ROOM_NAME_PARAMETER: i64 = -1_006_900;
/// How far past a room's object its number and name are looked for.
pub const ROOM_PARAMETER_ENTRY_WINDOW: usize = 0x400;
/// Value of `m_room_parameter_source` for a room named by its parameter
/// entries (RE-117).
pub const ROOM_PARAMETER_ENTRY_SOURCE: &str = "partition_room_parameter_entries";

/// Each room's number and name from its parameter entries, by ElementId
/// (RE-117), on a release with an element-data layout
/// ([`crate::partition_names::element_data_layout`]).
///
/// A room's data is an object `01 00 00 00 · id` (a `u32` id on Revit 2023,
/// a `u64` on 2024 and 2025). Its number and name are the first
/// `ROOM_NUMBER · u32 n · UTF-16 × n` entry within
/// [`ROOM_PARAMETER_ENTRY_WINDOW`] past it, immediately followed by
/// `ROOM_NAME · u32 m · UTF-16 × m`, the parameter ids as wide as the
/// release's ElementIds: 377 or 389 bytes past the id on Exemplo_data
/// (2023), 483 to 503 on RE1 Architecture (2025). A room whose copies
/// disagree gets nothing.
pub fn scan_room_parameter_entries(
    rf: &mut RevitFile,
    revit_version: u32,
    rooms: &BTreeSet<u32>,
) -> BTreeMap<u32, (String, String)> {
    let Some(layout) = crate::partition_names::element_data_layout(revit_version) else {
        return BTreeMap::new();
    };
    let (id_len, number_tag, name_tag) = if layout.wide_id {
        (
            8,
            ROOM_NUMBER_PARAMETER.to_le_bytes().to_vec(),
            ROOM_NAME_PARAMETER.to_le_bytes().to_vec(),
        )
    } else {
        let narrow = |id: i64| i32::try_from(id).expect("a 32-bit BuiltInParameter");
        (
            4,
            narrow(ROOM_NUMBER_PARAMETER).to_le_bytes().to_vec(),
            narrow(ROOM_NAME_PARAMETER).to_le_bytes().to_vec(),
        )
    };
    let string_at = |buf: &[u8], at: usize, limit: usize| -> Option<(String, usize)> {
        let n = usize::try_from(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?)).ok()?;
        if !(1..=limit).contains(&n) {
            return None;
        }
        let units: Vec<u16> = buf
            .get(at + 4..at + 4 + 2 * n)?
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let text = String::from_utf16(&units).ok()?;
        (!text.chars().any(char::is_control)).then_some((text, at + 4 + 2 * n))
    };
    let mut found: BTreeMap<u32, Option<(String, String)>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let Some(id) = layout.id_at(buf, hit + 4).filter(|id| rooms.contains(id)) else {
                continue;
            };
            let start = hit + 4 + id_len;
            let end = (start + ROOM_PARAMETER_ENTRY_WINDOW).min(buf.len());
            let Some(window) = buf.get(start..end) else {
                continue;
            };
            let pair = memchr::memmem::find_iter(window, &number_tag).find_map(|at| {
                let at = start + at;
                let (number, after) = string_at(buf, at + number_tag.len(), 64)?;
                (buf.get(after..after + name_tag.len())? == &name_tag[..]).then_some(())?;
                let (name, _) = string_at(buf, after + name_tag.len(), 256)?;
                Some((number, name))
            });
            let Some(pair) = pair else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(pair));
                }
                Some(held) => {
                    if held.as_ref() != Some(&pair) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, pair)| pair.map(|p| (id, p)))
        .collect()
}
