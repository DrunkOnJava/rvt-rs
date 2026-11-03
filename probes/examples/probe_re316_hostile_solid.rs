//! RE-316: how long the room outline readers take on a crafted solid.
//!
//! FACT under test: a partition is untrusted input, and the readers that
//! resolve a room's stored solid ([`outline_of_solids`], the walk of RE-101,
//! and [`resolved_outline_of_solids`], RE-316) do bounded work on any solid:
//! the walk reads no more than `SOLID_FACES_MAX` faces, the span no more than
//! the solid's list of objects counts and `RESOLVE_FACES_MAX`, and
//! `region_boundary` gives up before it builds more than `BOUNDARY_PIECES_MAX`
//! pieces however often its segments cross.
//!
//! The probe builds solids in memory, runs both readers on each and prints the
//! time they took and whether they gave an outline. No model is read.
//!
//! - `grid`: a square grid of full-length wall faces, `n` across and `n` down,
//!   whose segments cross `n * n` times (512 faces at most);
//! - `collinear`: wall faces that overlap along one line;
//! - `rooms`: concentric squares, the shape of a room with holes;
//! - `unbounded list`: faces whose list of objects counts more than the reader
//!   resolves;
//! - `no list`: faces with no list of objects.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_hostile_solid

use std::time::Instant;

use rvt::research::partition_room_boundaries::{
    FACE_TAG, ROOM_SOLID_TAG, outline_of_solids, resolved_outline_of_solids,
};

const HEIGHT: f64 = 10.0;

fn put(buf: &mut Vec<u8>, values: &[f64]) {
    for value in values {
        buf.extend_from_slice(&value.to_le_bytes());
    }
}

/// A wall face along `direction` (a unit vector in plan) from `from` for
/// `length` feet, from the floor to `HEIGHT`, in the 174-byte layout.
fn wall(from: (f64, f64), direction: (f64, f64), length: f64) -> Vec<u8> {
    let mut face = Vec::with_capacity(174);
    face.extend_from_slice(&FACE_TAG);
    face.extend_from_slice(&[0xff; 16]);
    put(&mut face, &[0.0, 0.0, HEIGHT, length]);
    face.resize(85, 0xff);
    face.push(1);
    put(&mut face, &[from.0, from.1, 0.0]);
    put(&mut face, &[0.0, 0.0, 1.0]);
    put(&mut face, &[direction.0, direction.1, 0.0]);
    face.resize(174, 0xff);
    face
}

/// A solid's bytes: the room's id, the solid tag, the two boxes' room, the
/// list of objects (`list` of them, or none) and `faces`.
fn solid(faces: &[Vec<u8>], list: Option<u32>) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.extend_from_slice(&7u64.to_le_bytes());
    buf.extend_from_slice(&ROOM_SOLID_TAG);
    buf.extend_from_slice(&[0u8; 96]);
    if let Some(count) = list {
        buf.extend_from_slice(&[0x04, 0xc0, 0x08, 0x00]);
        buf.extend_from_slice(&count.to_le_bytes());
        buf.extend_from_slice(&4u32.to_le_bytes());
        for token in 0..count {
            buf.extend_from_slice(&[0x97, 0x06]);
            buf.extend_from_slice(&(token + 5).to_le_bytes());
        }
    }
    buf.extend_from_slice(&[0u8; 16]);
    for face in faces {
        buf.extend_from_slice(face);
    }
    buf.extend_from_slice(&[0u8; 64]);
    buf
}

fn run(name: &str, faces: &[Vec<u8>], list: Option<u32>, extent: f64) {
    let buf = solid(faces, list);
    let bbox = [0.0, 0.0, 0.0, extent, extent, HEIGHT];
    let started = Instant::now();
    let walked = outline_of_solids(&buf, &[0], bbox);
    let walk_ms = started.elapsed().as_secs_f64() * 1000.0;
    let started = Instant::now();
    let resolved = resolved_outline_of_solids(&buf, &[0], bbox);
    let resolve_ms = started.elapsed().as_secs_f64() * 1000.0;
    println!(
        "{name}: {} faces, {} bytes; walk {walk_ms:.1} ms ({}), resolve {resolve_ms:.1} ms ({})",
        faces.len(),
        buf.len(),
        if walked.is_some() { "outline" } else { "none" },
        if resolved.is_some() {
            "outline"
        } else {
            "none"
        },
    );
}

fn main() {
    for n in [10usize, 40, 100, 256] {
        let extent = n as f64 + 1.0;
        let mut faces = Vec::new();
        for k in 0..n {
            faces.push(wall((0.0, k as f64 + 0.5), (1.0, 0.0), extent));
            faces.push(wall((k as f64 + 0.5, 0.0), (0.0, 1.0), extent));
        }
        run(
            &format!("grid {n} x {n}"),
            &faces,
            Some(faces.len() as u32),
            extent,
        );
    }
    let overlapping: Vec<Vec<u8>> = (0..512)
        .map(|k| wall((k as f64 * 0.01, 0.0), (1.0, 0.0), 100.0))
        .collect();
    run("collinear 512", &overlapping, Some(512), 100.0);
    let squares: Vec<Vec<u8>> = (0..64)
        .flat_map(|k| {
            let a = k as f64 * 0.5;
            let side = 100.0 - 2.0 * a;
            [
                wall((a, a), (1.0, 0.0), side),
                wall((a + side, a), (0.0, 1.0), side),
                wall((a + side, a + side), (-1.0, 0.0), side),
                wall((a, a + side), (0.0, -1.0), side),
            ]
        })
        .collect();
    run("rooms 64 concentric squares", &squares, Some(256), 100.0);
    run("unbounded list", &squares, Some(100_000), 100.0);
    run("no list", &squares, None, 100.0);
    let many: Vec<Vec<u8>> = (0..5000)
        .map(|k| wall((0.0, k as f64 * 0.01), (1.0, 0.0), 100.0))
        .collect();
    run("5000 faces", &many, Some(512), 100.0);
}
