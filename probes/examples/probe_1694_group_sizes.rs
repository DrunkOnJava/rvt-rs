//! #1694: how a partition's inflated bytes divide among channels and groups,
//! and what one walk of the file that inflates every segment costs.
//!
//! FACT under test: the saved-graphics pass reads channel 103 alone. Walking
//! without the handle's inflated copies, each of its reads inflates every
//! segment of each partition it visits (`native_segments::walk_inflated`). A
//! segment's marker names its channel before the segment is inflated, and a
//! group's first marker sits at an offset known without inflating. So a read
//! could inflate only the groups that hold its records, or only its channel's
//! segments. Which of these saves time depends on how many groups a partition
//! holds, how large they are, and what share of the bytes channel 103 is. This
//! prints those counts and times one walk of the file that inflates every
//! segment, as an uncached read does.
//!
//! Output:
//!   partition <n> channel <c> groups=<n> keyed=<n> bytes=<n> largest=<n> records=<n>
//!   channel <c> partitions=<n> groups=<n> keyed=<n> bytes=<n> largest=<n> records=<n>
//!   channel <c> group_bytes p50=<n> p90=<n> p99=<n> max=<n>
//!   file partitions=<n> segments=<n> groups=<n> inflated_bytes=<n> walk_ms=<n>
//!
//! `keyed` counts groups inside a keyed content block (an embedded family's
//! document), which no native read decodes. `records` counts a group's records
//! where it splits into whole ones at the id width the readers use.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1694_group_sizes -- MODEL.rvt

use std::collections::BTreeMap;
use std::time::Instant;

use rvt::compression::prepare_stream_for_inflate;
use rvt::{
    RevitFile, research::native_document, research::native_segments, research::schema_registry,
};

#[derive(Default)]
struct Tally {
    groups: u64,
    keyed: u64,
    bytes: u64,
    largest: u64,
    records: u64,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.groups += other.groups;
        self.keyed += other.keyed;
        self.bytes += other.bytes;
        self.largest = self.largest.max(other.largest);
        self.records += other.records;
    }
}

/// The number of records `bytes` splits into at an id width of 8 or 4 bytes,
/// where every record's two length words agree and the group ends exactly
/// (`native_document::group_records`); 0 where neither width does.
fn record_count(bytes: &[u8]) -> u64 {
    'width: for width in [8usize, 4] {
        let header = width + 8;
        let mut pos = 0usize;
        let mut count = 0u64;
        while pos < bytes.len() {
            let Some(length_at) = pos.checked_add(header) else {
                continue 'width;
            };
            let Some(word) = bytes.get(length_at - 4..length_at) else {
                continue 'width;
            };
            let length = u32::from_le_bytes([word[0], word[1], word[2], word[3]]) as usize;
            let Some(end) = length_at.checked_add(length) else {
                continue 'width;
            };
            let Some(trailer) = bytes.get(end..end.saturating_add(4)) else {
                continue 'width;
            };
            if trailer.len() != 4
                || u32::from_le_bytes([trailer[0], trailer[1], trailer[2], trailer[3]]) as usize
                    != length
            {
                continue 'width;
            }
            count += 1;
            pos = end + 4;
        }
        return count;
    }
    0
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    sorted[(sorted.len() - 1) * p / 100]
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_1694_group_sizes MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let registry =
        schema_registry::parse(&native_document::read_single(&mut rf, "Formats/Latest")?)?;
    let mut names: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|name| name.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort_by_key(|name| name[11..].parse::<u32>().unwrap_or(u32::MAX));
    let mut by_channel: BTreeMap<u64, (u64, Tally)> = BTreeMap::new();
    let mut sizes: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let (mut segments, mut groups, mut inflated) = (0usize, 0usize, 0u64);
    let mut walk_ms = 0u128;
    for name in &names {
        let partition: u32 = name[11..].parse()?;
        let stored = rf.read_stream(name)?;
        let prepared = prepare_stream_for_inflate(name, &stored);
        let mut here: BTreeMap<u64, Tally> = BTreeMap::new();
        let started = Instant::now();
        let stats =
            native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
                let tally = here.entry(source.channel).or_default();
                let length = bytes.len() as u64;
                tally.groups += 1;
                tally.bytes += length;
                tally.largest = tally.largest.max(length);
                if source.content_key.is_some() {
                    tally.keyed += 1;
                } else {
                    tally.records += record_count(bytes);
                    sizes.entry(source.channel).or_default().push(length);
                }
                Ok(())
            })?;
        walk_ms += started.elapsed().as_millis();
        segments += stats.segments;
        groups += stats.groups;
        inflated += stats.inflated_bytes;
        for (channel, tally) in &here {
            println!(
                "partition {partition} channel {channel} groups={} keyed={} bytes={} largest={} records={}",
                tally.groups, tally.keyed, tally.bytes, tally.largest, tally.records
            );
            let entry = by_channel.entry(*channel).or_default();
            entry.0 += 1;
            entry.1.add(tally);
        }
    }
    for (channel, (partitions, tally)) in &by_channel {
        println!(
            "channel {channel} partitions={partitions} groups={} keyed={} bytes={} largest={} records={}",
            tally.groups, tally.keyed, tally.bytes, tally.largest, tally.records
        );
    }
    for (channel, list) in &mut sizes {
        list.sort_unstable();
        println!(
            "channel {channel} group_bytes p50={} p90={} p99={} max={}",
            percentile(list, 50),
            percentile(list, 90),
            percentile(list, 99),
            list.last().copied().unwrap_or(0)
        );
    }
    println!(
        "file partitions={} segments={segments} groups={groups} inflated_bytes={inflated} walk_ms={walk_ms}",
        names.len()
    );
    Ok(())
}
