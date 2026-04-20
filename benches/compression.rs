//! Compression-path benchmarks (Q-05).
//!
//! Exercises the truncated-gzip encode + inflate + chunk-discovery
//! functions on synthetic payloads of three sizes (1 KiB, 64 KiB,
//! 1 MiB). Payloads are random-ish structured bytes, not true
//! incompressible randomness, so the compression ratio matches
//! what Revit streams tend to produce.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rvt::compression::{inflate_at, truncated_gzip_encode};
use rvt::partitions::find_chunks;

fn synth_payload(len: usize) -> Vec<u8> {
    // Semi-structured bytes — repeat a 23-byte pattern then xor
    // with a rolling counter. Gives a realistic-ish compression
    // ratio (~2-3x) without being a pathological all-zeros run.
    let pat = b"revit-fake-stream-xbeef\x00";
    let mut out = Vec::with_capacity(len);
    for i in 0..len {
        out.push(pat[i % pat.len()] ^ ((i as u8).wrapping_mul(37)));
    }
    out
}

fn bench_encode(c: &mut Criterion) {
    let mut group = c.benchmark_group("truncated_gzip_encode");
    for &size in &[1 << 10, 1 << 16, 1 << 20] {
        let data = synth_payload(size);
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, d| {
            b.iter(|| truncated_gzip_encode(d).unwrap())
        });
    }
    group.finish();
}

fn bench_inflate(c: &mut Criterion) {
    let mut group = c.benchmark_group("inflate_at");
    for &size in &[1 << 10, 1 << 16, 1 << 20] {
        // Prepare a compressed payload once — the benchmark measures
        // inflate, not encode.
        let data = synth_payload(size);
        let encoded = truncated_gzip_encode(&data).unwrap();
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &encoded, |b, e| {
            b.iter(|| inflate_at(e, 0).unwrap())
        });
    }
    group.finish();
}

fn bench_find_chunks(c: &mut Criterion) {
    // A partition stream typically concatenates several
    // truncated-gzip chunks back-to-back. Stitch 8 × 64 KiB chunks
    // to mimic a realistic partition.
    let mut stitched = Vec::new();
    for _ in 0..8 {
        let data = synth_payload(1 << 16);
        let encoded = truncated_gzip_encode(&data).unwrap();
        stitched.extend_from_slice(&encoded);
    }
    let mut group = c.benchmark_group("find_chunks");
    group.throughput(Throughput::Bytes(stitched.len() as u64));
    group.bench_function("8x64KiB", |b| b.iter(|| find_chunks(&stitched)));
    group.finish();
}

criterion_group!(benches, bench_encode, bench_inflate, bench_find_chunks);
criterion_main!(benches);
