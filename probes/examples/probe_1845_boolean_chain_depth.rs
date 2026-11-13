//! #1845: whether a long chain of boolean cuts overflows a 1 MiB stack when it
//! is dropped, cloned, printed or serialised.
//!
//! FACT under test: an elevation-profile cut writes one `IfcBooleanResult` per
//! cut, nested down its first operand (`wall_profile::cut_body`), up to
//! `SKETCH_MAX_CURVES` (4,096) curves of a profile. #1848 writes and meshes such
//! a chain by a loop, but `SolidShape` is `Debug`, `Clone` and `Serialize` by
//! derive and drops its boxes by the compiler's glue, and each of those walks
//! the chain by recursion. The audit left it open: "whether that overflows the
//! viewer's stack needs a build to say".
//!
//! Hypothesis, registered before the first run: a 4,096-level chain survives all
//! four on a 1 MiB stack (the viewer's wasm stack; a native thread of that size
//! stands in for it), and each breaks only at a depth many times that.
//!
//! First run (Measure 43420945188, depths 1,024 to 1,048,576): refuted for two
//! of the four. `drop` holds to 16,384 and breaks at 65,536; `json` the same;
//! `clone` and `debug` hold at 1,024 and break at 4,096. A profile has at most
//! `SKETCH_MAX_CURVES` (4,096) curves and a cut region needs two or more, so a
//! wall's chain is at most about 2,048 levels: the depths between are what the
//! second run reads, registered before it as: `clone` and `debug` hold at 2,048.
//!
//! Second run (Measure 43422082815, depths 1,024 to 65,536): they do. `drop` and
//! `json` hold to 16,384 and break at 65,536; `clone` holds to 3,072 and breaks
//! at 4,096; `debug` holds to 2,048 and breaks at 3,072. A profile's curves make
//! a region of one curve (a circle) at best, so a wall's chain can reach 4,096.
//!
//! The probe ignores the file it is given. For each operation and depth it runs
//! a child of itself, with the chain built by a loop and the operation run on a
//! thread of exactly 1 MiB, and prints how the child ended, so an overflow
//! (which aborts the process) costs one line and not the run.
//!
//! Output:
//!
//!   file <path>
//!   stack_bytes <n>
//!   <drop|clone|debug|json> <depth> <ok|exit <code>|signal <n>>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1845_boolean_chain_depth -- MODEL.rvt

use rvt::ifc::entities::{IfcBooleanOp, SolidShape};
use std::process::{Command, ExitStatus, Stdio};

/// The viewer's wasm stack, which a thread of this size stands in for.
const STACK_BYTES: usize = 1 << 20;

/// What is done to the chain.
const OPERATIONS: [&str; 4] = ["drop", "clone", "debug", "json"];

/// Chain depths: 4,096 is `SKETCH_MAX_CURVES`, the most curves a profile has, and
/// 2,048 the most cuts it can make.
const DEPTHS: [usize; 7] = [1_024, 1_536, 2_048, 3_072, 4_096, 16_384, 65_536];

fn leaf() -> SolidShape {
    SolidShape::FacetedBrep {
        vertices_feet: Vec::new(),
        triangles: Vec::new(),
    }
}

/// A chain of `depth` cuts, built by a loop so building it recurses nowhere.
fn chain(depth: usize) -> SolidShape {
    let mut shape = leaf();
    for _ in 0..depth {
        shape = SolidShape::BooleanResult {
            op: IfcBooleanOp::Difference,
            operand_a: Box::new(shape),
            operand_b: Box::new(leaf()),
        };
    }
    shape
}

/// Runs `operation` on a chain of `depth` on a thread of `STACK_BYTES`. Every
/// operation but `drop` leaks the chain, so that it is the only one run.
fn child(operation: &str, depth: usize) {
    let operation = operation.to_string();
    let handle = std::thread::Builder::new()
        .stack_size(STACK_BYTES)
        .spawn(move || {
            let shape = chain(depth);
            match operation.as_str() {
                "drop" => drop(shape),
                "clone" => {
                    let copy = shape.clone();
                    std::mem::forget(copy);
                    std::mem::forget(shape);
                }
                "debug" => {
                    let text = format!("{shape:?}");
                    std::hint::black_box(text.len());
                    std::mem::forget(shape);
                }
                "json" => {
                    let text = serde_json::to_string(&shape).expect("serialise");
                    std::hint::black_box(text.len());
                    std::mem::forget(shape);
                }
                other => panic!("no operation {other}"),
            }
        })
        .expect("spawn the chain's thread");
    handle.join().expect("the chain's thread");
}

fn describe(status: ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return format!("signal {signal}");
        }
    }
    match status.code() {
        Some(0) => "ok".to_string(),
        Some(code) => format!("exit {code}"),
        None => "unknown".to_string(),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--child") {
        let depth = args[3].parse().expect("a depth");
        child(&args[2], depth);
        return;
    }
    println!("file {}", args.get(1).map_or("-", String::as_str));
    println!("stack_bytes {STACK_BYTES}");
    let exe = std::env::current_exe().expect("this probe's path");
    for operation in OPERATIONS {
        for depth in DEPTHS {
            let status = Command::new(&exe)
                .args(["--child", operation, &depth.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("run a child");
            println!("{operation} {depth} {}", describe(status));
        }
    }
}
