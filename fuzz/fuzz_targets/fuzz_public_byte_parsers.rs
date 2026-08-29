#![no_main]

//! Fuzz remaining public byte-parsing entry points not covered by a
//! dedicated narrow target (Lane Ten / M8-01).
//!
//! Exercises:
//! - `compression::find_gzip_offsets` / `inflate_all_chunks`
//! - `class_index::extract_class_names`
//! - `arc_wall_record::ArcWallRecord::{find_all_with_limits,decode_*}`
//! - `rect_opening_index::ArcWallRectOpeningIndex`
//! - `ifc::share::decode_from_fragment`
//!
//! Each call must be panic-free on arbitrary bytes. Allocation is
//! bounded via `WalkerLimits` on the ArcWall scan path.

use libfuzzer_sys::fuzz_target;
use rvt::arc_wall_record::ArcWallRecord;
use rvt::class_index::extract_class_names;
use rvt::compression::{find_gzip_offsets, inflate_all_chunks};
use rvt::ifc::share::decode_from_fragment;
use rvt::rect_opening_index::ArcWallRectOpeningIndex;
use rvt::walker::WalkerLimits;

fuzz_target!(|data: &[u8]| {
    let _ = find_gzip_offsets(data);
    let _ = inflate_all_chunks(data);
    let _ = extract_class_names(data);
    let _ = decode_from_fragment(std::str::from_utf8(data).unwrap_or(""));

    let limits = WalkerLimits {
        max_scan_bytes: 64 * 1024,
        max_candidates: 256,
        max_trial_offsets: 4_096,
        max_per_record_decode_bytes: 16 * 1024,
        max_container_records: 64,
    };
    let _ = ArcWallRecord::find_all_with_limits(data, limits);
    let _ = ArcWallRecord::scan_standard_for_revit_version_with_limits(2023, data, limits);
    let _ = ArcWallRecord::decode_standard(data, 0);
    let _ = ArcWallRecord::decode_trailer(data, 0);
    let _ = ArcWallRectOpeningIndex::decode(data, 0);
    let _ = ArcWallRectOpeningIndex::find_all_for_revit_version(2024, data);
});
