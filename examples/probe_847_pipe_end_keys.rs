//! #847 (probe): the connector entry keys at each end of a pipe.
//!
//! RE-131: a pipe's ends are points of the partition's connector entries,
//! `u64 element · u32 connector index · u32 1 · f64 × 3 point`. rvt-rs keeps
//! the points and drops the keys. For each pipe production draws as a
//! cylinder, this finds every entry in the pipe's partition whose point is
//! one of its two ends and prints the entry's element and connector index,
//! so `tools/re/pipe_end_keys_vs_ifc.py` can say which connector of the pipe
//! each end is, against the ports Revit's export places there.
//!
//! Output, one line per end:
//!   end <pipe ElementId> <first|second> <x> <y> <z> keys <element>:<index> ...
//!
//! Usage:
//!   cargo run --profile ci --example probe_847_pipe_end_keys -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::pipe_body_from_fields;

const SAME_POINT_FEET: f64 = 1e-4;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut pipes = Vec::new();
    for e in rvt::walker::iter_elements(&mut rf)? {
        if e.class != "Pipe" {
            continue;
        }
        let stream = e.fields.iter().find_map(|(name, value)| match value {
            rvt::walker::InstanceField::String(stream) if name == "m_source_stream" => {
                Some(stream.clone())
            }
            _ => None,
        });
        let (Some(id), Some(body), Some(stream)) = (e.id, pipe_body_from_fields(&e.fields), stream)
        else {
            continue;
        };
        pipes.push((id, body, stream));
    }
    let marker = 1u32.to_le_bytes();
    for (id, body, stream) in &pipes {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            println!("pipe {id}: partition {stream} not read");
            continue;
        };
        let buf = inflated.bytes();
        let mut keys: [Vec<String>; 2] = [Vec::new(), Vec::new()];
        for at in memchr::memmem::find_iter(buf, marker.as_slice()) {
            let (Some(element), Some(index), Some(point)) = (
                at.checked_sub(12)
                    .and_then(|from| buf.get(from..from + 8))
                    .map(|b| u64::from_le_bytes(b.try_into().unwrap_or([0; 8]))),
                at.checked_sub(4)
                    .and_then(|from| buf.get(from..from + 4))
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4]))),
                (0..3)
                    .map(|axis| {
                        let from = at + 4 + 8 * axis;
                        buf.get(from..from + 8)
                            .map(|b| f64::from_le_bytes(b.try_into().unwrap_or([0; 8])))
                    })
                    .collect::<Option<Vec<f64>>>(),
            ) else {
                continue;
            };
            for (slot, end) in [body.start, body.end].iter().enumerate() {
                let near = (0..3).all(|axis| (point[axis] - end[axis]).abs() <= SAME_POINT_FEET);
                if near {
                    keys[slot].push(format!("{element}:{index}"));
                    // #834: the 64 bytes after the point, to look for a flow
                    // direction.
                    let after: String = buf
                        .get(at + 28..(at + 92).min(buf.len()))
                        .unwrap_or(&[])
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    println!("entry {id} {slot} {element}:{index} {after}");
                }
            }
        }
        for (slot, end) in [body.start, body.end].iter().enumerate() {
            println!(
                "end {id} {} {:.6} {:.6} {:.6} keys {}",
                if slot == 0 { "first" } else { "second" },
                end[0],
                end[1],
                end[2],
                keys[slot].join(" ")
            );
        }
    }
    Ok(())
}
