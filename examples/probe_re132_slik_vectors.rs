//! RE-132 (probe): STE1200's `element_header` vectors for the MIT files
//! (Discussion #112, pack 260928).
//!
//! Steffen (SLIK Architekten) cut eight records from Einhoven (Revit 2023)
//! and Core Interior (Revit 2024) with his own reader, each with its offset in
//! the page-stripped inflated stream, its ElementId, class, `BuiltInCategory`,
//! flags word and box, and gave the files' SHA-256. This looks each record up
//! in what rvt-rs reads from the file with that hash and prints, per vector,
//! which fields agree, and where rvt-rs frames the record. Nothing here is
//! read from his reader: the expected values are his, in `VECTORS`.
//!
//! The flags word is `m_abFlags4Bytes`, the `u32` at `+0x46` of a record laid
//! out as 2024 lays it out (RE-128). The box is his length, thickness and
//! height: the longer and the shorter plan side of the record's box, and its
//! height, in millimetres. For a record rvt-rs does not return, it prints
//! whether the id is declared in `Global/ElemTable` and what a 2023 record's
//! decode asks of the bytes at his offset.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re132_slik_vectors -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
use sha2::{Digest, Sha256};

/// How closely a box side must match, millimetres. His values are rounded to
/// 0.1 mm.
const DIMENSION_TOLERANCE_MM: f64 = 0.15;

const FEET_TO_MM: f64 = 304.8;

/// Bytes from a 2023 record's ElementId to its marker (RE-81).
const ID_TO_MARKER_2023: isize = 52;

/// Bytes from a 2023 record's `BuiltInCategory` to its marker (RE-81).
const CATEGORY_BEFORE_MARKER_2023: isize = 38;

struct Vector {
    /// SHA-256 of the `.rvt`.
    file: &'static str,
    id: u32,
    stream: &'static str,
    offset: usize,
    class: &'static str,
    category: i64,
    flags: u32,
    /// Length, thickness and height, millimetres.
    dims_mm: [f64; 3],
}

const EINHOVEN: &str = "d3a0c6d37d3f47a1726bc5aa7fe3880ed3c13bbe819b5e64680f6710b15aa948";
const CORE_INTERIOR: &str = "c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014";

const VECTORS: &[Vector] = &[
    Vector {
        file: EINHOVEN,
        id: 2808,
        stream: "Partitions/0",
        offset: 240_469,
        class: "SWall",
        category: -2_000_011,
        flags: 0x929,
        dims_mm: [6000.0, 200.0, 2998.6],
    },
    Vector {
        file: EINHOVEN,
        id: 2921,
        stream: "Partitions/0",
        offset: 1_632_829,
        class: "SWall",
        category: -2_000_011,
        flags: 0x939,
        dims_mm: [3251.2, 152.4, 3048.0],
    },
    Vector {
        file: EINHOVEN,
        id: 3637,
        stream: "Partitions/0",
        offset: 2_382_177,
        class: "SWall",
        category: -2_000_011,
        flags: 0x939,
        dims_mm: [2286.0, 152.4, 3048.0],
    },
    Vector {
        file: EINHOVEN,
        id: 5317,
        stream: "Partitions/0",
        offset: 258_214,
        class: "FamilyInstance",
        category: -2_000_014,
        flags: 0x928,
        dims_mm: [609.6, 574.5, 1219.2],
    },
    Vector {
        file: CORE_INTERIOR,
        id: 87_758,
        stream: "Partitions/46",
        offset: 2_458_768,
        class: "SWall",
        category: -2_000_011,
        flags: 0x1929,
        dims_mm: [44_805.6, 304.8, 9144.0],
    },
    Vector {
        file: CORE_INTERIOR,
        id: 87_762,
        stream: "Partitions/46",
        offset: 2_461_236,
        class: "SWall",
        category: -2_000_011,
        flags: 0x1929,
        dims_mm: [44_805.6, 304.8, 9144.0],
    },
    Vector {
        file: CORE_INTERIOR,
        id: 22_071,
        stream: "Partitions/46",
        offset: 3_044_354,
        class: "SWall",
        category: -2_000_011,
        flags: 0x1929,
        dims_mm: [44_805.6, 304.8, 9144.0],
    },
    Vector {
        file: CORE_INTERIOR,
        id: 20_953,
        stream: "Partitions/46",
        offset: 8_144,
        class: "Floor",
        category: -2_000_032,
        flags: 0x928,
        dims_mm: [55_575.2, 37_067.6, 50.8],
    },
];

fn sha256_hex(bytes: &[u8]) -> String {
    let digits: Vec<String> = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    digits.join("")
}

/// Longer plan side, shorter plan side and height of a record's box, mm.
fn dims_mm(bbox: [f64; 6]) -> [f64; 3] {
    let (dx, dy, dz) = (
        (bbox[3] - bbox[0]) * FEET_TO_MM,
        (bbox[4] - bbox[1]) * FEET_TO_MM,
        (bbox[5] - bbox[2]) * FEET_TO_MM,
    );
    [dx.max(dy), dx.min(dy), dz]
}

/// The `u32` at `at`, when it lies in `buf`.
fn read_u32(buf: &[u8], at: isize) -> Option<u32> {
    let at = usize::try_from(at).ok()?;
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

/// What a 2023 record's decode asks of the bytes at `offset`, as JSON.
fn diagnose_2023(rf: &mut RevitFile, vector: &Vector) -> String {
    let Ok(inflated) = rf.inflated_partition(vector.stream) else {
        return "null".to_string();
    };
    let buf = inflated.bytes();
    let at = vector.offset as isize;
    let id_here = read_u32(buf, at);
    let category = usize::try_from(at + ID_TO_MARKER_2023 - CATEGORY_BEFORE_MARKER_2023)
        .ok()
        .and_then(|from| buf.get(from..from + 8))
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")));
    let marker = rvt::partition_element_records_2023::record_marker(rf);
    let marker_ok = marker.is_some_and(|(marker, _)| {
        let from = (at + ID_TO_MARKER_2023) as usize;
        buf.get(from..from + 8) == Some(marker.as_slice())
    });
    let header_ok = marker.is_some_and(|(_, tag)| {
        buf.get(vector.offset + 8..vector.offset + 10) == Some(tag.to_le_bytes().as_slice())
    });
    format!(
        "{{\"id_at_offset\":{},\"category_at_offset\":{},\"marker_at_offset_ok\":{marker_ok},\"header_tag_ok\":{header_ok}}}",
        id_here.map_or(-1, i64::from),
        category.unwrap_or(0),
    )
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let bytes = std::fs::read(&path).expect("read the model");
    let digest = sha256_hex(&bytes);
    drop(bytes);
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mine: Vec<&Vector> = VECTORS.iter().filter(|v| v.file == digest).collect();
    println!(
        "{{\"revit\":{version},\"sha256\":\"{digest}\",\"vectors\":{}}}",
        mine.len()
    );
    if mine.is_empty() {
        return Ok(());
    }
    let classes = rf.schema_classes()?;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = if version == 2023 {
        rvt::partition_element_records_2023::scan_records(&mut rf, version)
    } else {
        per::scan_category_records_multi(
            &mut rf,
            version,
            &[per::OST_WALLS, per::OST_WINDOWS, per::OST_FLOORS],
            &declared,
        )?
    };
    let mut all_agree = 0;
    for vector in &mine {
        let is_declared = declared.contains(&vector.id);
        let found: Vec<&per::PartitionElementRecord> = records
            .iter()
            .filter(|record| record.element_id == vector.id)
            .collect();
        let best = found
            .iter()
            .find(|record| record.stream == vector.stream)
            .or_else(|| found.first())
            .copied();
        let Some(record) = best else {
            let diagnosis = if version == 2023 {
                diagnose_2023(&mut rf, vector)
            } else {
                "null".to_string()
            };
            println!(
                "{{\"id\":{},\"found\":0,\"declared_in_elem_table\":{is_declared},\"records_read\":{},\"diagnosis\":{diagnosis}}}",
                vector.id,
                records.len()
            );
            continue;
        };
        let class_name = classes
            .by_tag(record.class_tag)
            .map(|class| class.name.as_str());
        let class_text = class_name.unwrap_or("");
        let dims = dims_mm(record.bbox_feet);
        // A 2023 record's start, laid out as 2024 lays it out, is 28 bytes
        // before its ElementId.
        let flags_at = if version == 2023 {
            record.offset as isize + ID_TO_MARKER_2023 - per::BBOX_MARKER_OFFSET as isize + 0x46
        } else {
            record.offset as isize + 0x46
        };
        let flags = rf
            .inflated_partition(&record.stream)
            .ok()
            .and_then(|inflated| read_u32(inflated.bytes(), flags_at));
        let stream_ok = record.stream == vector.stream;
        let category_ok = record.builtin_category == vector.category;
        let class_ok = class_name == Some(vector.class);
        let flags_ok = flags == Some(vector.flags);
        let dims_ok = (0..3).all(|k| (dims[k] - vector.dims_mm[k]).abs() <= DIMENSION_TOLERANCE_MM);
        let offset_delta = record.offset as i64 - vector.offset as i64;
        if stream_ok && category_ok && class_ok && flags_ok && dims_ok {
            all_agree += 1;
        }
        println!(
            "{{\"id\":{},\"found\":{},\"declared_in_elem_table\":{is_declared},\"stream\":{:?},\"stream_ok\":{stream_ok},\"offset\":{},\"offset_delta\":{offset_delta},\"category\":{},\"category_ok\":{category_ok},\"class\":{:?},\"class_tag\":{},\"class_ok\":{class_ok},\"flags_at_0x46\":{},\"expected_flags\":{},\"flags_ok\":{flags_ok},\"dims_mm\":{:?},\"expected_dims_mm\":{:?},\"dims_ok\":{dims_ok}}}",
            vector.id,
            found.len(),
            record.stream,
            record.offset,
            record.builtin_category,
            class_text,
            record.class_tag,
            flags.map_or(-1, i64::from),
            vector.flags,
            dims,
            vector.dims_mm,
        );
    }
    println!(
        "{{\"summary\":true,\"vectors\":{},\"all_fields_agree\":{all_agree}}}",
        mine.len()
    );
    eprintln!(
        "Revit {version}: {all_agree} of {} vectors agree",
        mine.len()
    );
    Ok(())
}
