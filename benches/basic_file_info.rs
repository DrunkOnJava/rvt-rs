//! BasicFileInfo parser benchmark (Q-05).
//!
//! The BasicFileInfo stream is UTF-16LE plain text with a handful
//! of key-value lines (Revit Build, Locale, Identity, Worksharing,
//! etc.). The parser is hot-path for `rvt-info` / `rvt-history` /
//! any downstream tool that just needs version + build-tag. This
//! bench pins the cost of a full parse on a realistic-size buffer.

use criterion::{Criterion, criterion_group, criterion_main};
use rvt::basic_file_info::BasicFileInfo;

fn synth_bfi() -> Vec<u8> {
    // Realistic-ish BasicFileInfo contents, UTF-16LE encoded.
    let text = "Worksharing: Not enabled\r\n\
                Username: griffin\r\n\
                Central Model Path: \r\n\
                Format: 2024\r\n\
                Build: 20230322_1500(x64)\r\n\
                Last Save Path: C:\\test\\sample.rvt\r\n\
                Open Workset Default: 3\r\n\
                Project Spark File: 0\r\n\
                Central Model Identity: GUID-placeholder\r\n\
                Locale when saved: enu\r\n\
                All Local Changes Saved To Central: 0\r\n\
                Central model's version number corresponding to the last reload latest or remote-saved: 0\r\n\
                Unique Document GUID: 00000000-0000-0000-0000-000000000000\r\n\
                Unique Document Increments: 0\r\n";
    // UTF-16LE encode with BOM.
    let mut out = vec![0xff, 0xfe];
    for c in text.encode_utf16() {
        out.extend_from_slice(&c.to_le_bytes());
    }
    out
}

fn bench_from_bytes(c: &mut Criterion) {
    let data = synth_bfi();
    c.bench_function("BasicFileInfo::from_bytes", |b| {
        b.iter(|| BasicFileInfo::from_bytes(&data).unwrap())
    });
}

criterion_group!(benches, bench_from_bytes);
criterion_main!(benches);
