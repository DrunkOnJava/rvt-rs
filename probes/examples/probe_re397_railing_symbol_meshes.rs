//! RE-397 (#1635): the meshes of the symbols a railing's saved graphics name.
//!
//! FACT under test: a railing's saved graphics are a group of `GInstance`s
//! that name other elements as symbols (`probe_re397_railing_graphics_tree`).
//! On the BIM4LCA office as built the instances of the railings that give a
//! complete mesh are all visible and unrestricted (`m_targets` 0); those that
//! give a partial one hold one more, flagged hidden in the normal draw mode and
//! restricted (`m_targets` 53246); and the `KJO-1` stair railings, which give
//! none, hold only such instances. This reads each symbol as an element of its
//! own, with and without the hidden branches, so the volume of each can be set
//! against Revit's body of its railing.
//!
//! Output, one line per symbol and decode:
//!   symbol <railing> <symbol> <default|hidden> scene=no
//!   symbol <railing> <symbol> <default|hidden> scene=yes <status>
//!     primitives=<n> triangles=<n> diagnostics=<n> volume=<abs signed volume,
//!     ft3> <messages>
//! where <messages> is each distinct diagnostic with its digits replaced by
//! `#`, joined by ` | `, or `-`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_railing_symbol_meshes -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::{extract_at_detail, extract_including_hidden};

/// (railing, symbols its instances name) of the office as built: KAI-3, KAI-4,
/// KAI-1, KAI-5, KJO-1, KAI-2 and KAI-6.
const RAILINGS: [(u64, &[u64]); 7] = [
    (2614259, &[2614260]),
    (1467318, &[2620342, 1467319]),
    (1485880, &[1485881]),
    (2598350, &[2598390, 2598351]),
    (2598353, &[2598386, 2598385, 2598384, 2598374]),
    (1749890, &[1749891]),
    (3076853, &[3076854]),
];

fn message_kind(message: &str) -> String {
    let text = message.split_once(": ").map_or(message, |(_, rest)| rest);
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let symbols: BTreeSet<u64> = RAILINGS
        .iter()
        .flat_map(|(_, named)| named.iter().copied())
        .collect();
    let options = Options {
        selected_ids: symbols,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    for (label, hidden) in [("default", false), ("hidden", true)] {
        let scene = if hidden {
            extract_including_hidden(&mut rf, &options)?
        } else {
            extract_at_detail(&mut rf, &options, 3)?
        };
        let by_id: BTreeMap<u64, _> = scene
            .elements
            .iter()
            .map(|element| (element.identity.element_id, element))
            .collect();
        for (railing, named) in RAILINGS {
            for symbol in named {
                let Some(element) = by_id.get(symbol) else {
                    println!("symbol {railing} {symbol} {label} scene=no");
                    continue;
                };
                let meshes = &element.meshes;
                let mut volume6 = 0.0;
                let mut triangles = 0usize;
                for primitive in &meshes.primitives {
                    for triangle in &primitive.triangles {
                        let [a, b, c] =
                            triangle.map(|i| primitive.vertices.get(i as usize).copied());
                        let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                            continue;
                        };
                        triangles += 1;
                        volume6 += a[0] * (b[1] * c[2] - b[2] * c[1])
                            - a[1] * (b[0] * c[2] - b[2] * c[0])
                            + a[2] * (b[0] * c[1] - b[1] * c[0]);
                    }
                }
                let kinds: BTreeSet<String> =
                    meshes.diagnostics.iter().map(|d| message_kind(d)).collect();
                let messages = if kinds.is_empty() {
                    "-".to_string()
                } else {
                    kinds.into_iter().collect::<Vec<_>>().join(" | ")
                };
                println!(
                    "symbol {railing} {symbol} {label} scene=yes {} primitives={} triangles={triangles} \
                     diagnostics={} volume={:.6} {messages}",
                    element.status,
                    meshes.primitives.len(),
                    meshes.diagnostics.len(),
                    (volume6 / 6.0).abs(),
                );
            }
        }
    }
    Ok(())
}
