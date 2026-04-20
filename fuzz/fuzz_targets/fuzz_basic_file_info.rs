#![no_main]

//! Fuzz target for [`rvt::basic_file_info::BasicFileInfo::from_bytes`].
//!
//! The `BasicFileInfo` OLE stream carries UTF-16LE metadata present in every
//! Revit file (version, build tag, GUID, original path, locale). The parser
//! sweeps raw bytes through `encoding_rs::UTF_16LE`, then runs a handful of
//! hand-rolled scanners over the decoded text. Adversarial inputs could hit:
//!
//! - unpaired surrogates / odd-length buffers fed to the UTF-16LE decoder
//! - path scanners running past the end of the decoded string
//! - GUID-window scans on strings shorter than 36 bytes
//! - build-tag window scan with pathological ASCII-only prefixes
//!
//! This target feeds raw libFuzzer bytes straight into `from_bytes`. We
//! discard the `Result` — we are looking for panics, UB, and allocation
//! blowups, not parse errors.
//!
//! Wired in per SEC-20.

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = rvt::basic_file_info::BasicFileInfo::from_bytes(data);
});
