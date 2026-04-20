#![no_main]

//! Fuzz target for the ADocument entry-point detector (SEC-22).
//!
//! `walker::find_adocument_start_with_schema` picks the byte offset
//! in a decompressed `Global/Latest` stream where the ADocument
//! record's fields begin. It does this in two passes:
//!
//!   1. `heuristic_find` — scans for a sequential-id table followed
//!      by an 8-zero signature, with light-weight validity checks on
//!      the next two `u32`s.
//!   2. A scoring-based, byte-aligned brute-force scan from offset
//!      `0x100` to `len - 256`, trial-walking a caller-supplied
//!      `ClassEntry` schema for every offset and picking the highest-
//!      scoring one that clears a threshold.
//!
//! Both paths index into the input with arithmetic that depends on
//! attacker-controlled bytes (length prefixes, table markers, u32
//! cursors). Panics or unbounded loops on adversarial bytes here
//! propagate up into `walker::read_adocument` and, eventually,
//! `RevitFile::summarize` — which is on the hot path for any
//! consumer parsing an untrusted `.rvt`.
//!
//! Approach: (a) synthetic `ClassEntry` + the exposed fuzz hook
//! `walker::__fuzz_find_adocument_start`. We build a minimal
//! ADocument-shaped schema with 13 fields covering Pointer,
//! ElementId, and Container{kind=0x0e} — the three variants that
//! `trial_walk` actually recognises. That forces the scoring branch
//! to run on fuzzer bytes (which the byte-only heuristic path alone
//! would not exercise in isolation) and catches panics in both
//! strategies with one target.
//!
//! Approach (b) — targeting just `heuristic_find` — would not cover
//! the scoring branch at all, which is the branch added most
//! recently and has the most attacker-reachable arithmetic.

use libfuzzer_sys::fuzz_target;
use rvt::formats::{ClassEntry, FieldEntry, FieldType};
use rvt::walker::__fuzz_find_adocument_start;

/// Build a synthetic 13-field ADocument-shaped `ClassEntry` that
/// `trial_walk` can score against. Field types cover all three
/// variants the trial walker actually dispatches on — Pointer,
/// ElementId, and the 2-column Container with `kind = 0x0e`. The
/// last three fields are ElementIds because `walk_score` scores
/// the final three fields' (tag, id) pairs.
fn synth_adocument_schema() -> ClassEntry {
    let make_field = |name: &str, field_type: FieldType| FieldEntry {
        name: name.to_string(),
        cpp_type: None,
        field_type: Some(field_type),
    };
    ClassEntry {
        name: "ADocument".to_string(),
        offset: 0,
        fields: vec![
            make_field("m_ptr_a", FieldType::Pointer { kind: 2 }),
            make_field("m_ptr_b", FieldType::Pointer { kind: 2 }),
            make_field("m_ptr_c", FieldType::Pointer { kind: 2 }),
            make_field("m_ptr_d", FieldType::Pointer { kind: 2 }),
            make_field("m_ptr_e", FieldType::Pointer { kind: 2 }),
            make_field(
                "m_container",
                FieldType::Container {
                    kind: 0x0e,
                    cpp_signature: None,
                    body: Vec::new(),
                },
            ),
            make_field(
                "m_ptr_f",
                FieldType::Pointer { kind: 2 },
            ),
            make_field("m_ptr_g", FieldType::Pointer { kind: 2 }),
            make_field("m_ptr_h", FieldType::Pointer { kind: 2 }),
            make_field("m_ptr_i", FieldType::Pointer { kind: 2 }),
            make_field("m_ownerFamilyId", FieldType::ElementId),
            make_field(
                "m_ownerFamilyContainingGroupId",
                FieldType::ElementId,
            ),
            make_field("m_devBranchInfo", FieldType::ElementId),
        ],
        tag: Some(0x0001),
        parent: None,
        declared_field_count: Some(13),
        was_parent_only: false,
        ancestor_tag: None,
    }
}

fuzz_target!(|data: &[u8]| {
    let schema = synth_adocument_schema();

    // Path 1: schema-aware detection (heuristic + scoring brute
    // force). This is the path the production walker runs.
    let _ = __fuzz_find_adocument_start(data, Some(&schema));

    // Path 2: byte-only heuristic path. Separate call keeps the
    // fuzzer's coverage map distinguishing the two strategies.
    let _ = __fuzz_find_adocument_start(data, None);
});
