//! End to end: the built `rvt-rs-mcp` driven over real stdio.
//!
//! The server is the binary cargo built, started as a child process, and the
//! client is rmcp's own, so the test exercises what an MCP client does. It
//! runs in both lifecycles (`server/discover` at 2026-07-28, `initialize` at
//! 2025-11-25) on the committed synthetic fixture under `corpus/tier1`, and on
//! Einhoven when `RVT_PROJECT_CORPUS_DIR` names the project corpus. Exports are
//! compared byte for byte with `rvt-ifc` and `rvt-gltf` when those binaries
//! sit beside the server (they do after `cargo build --all-targets`, which
//! CI runs first; `RVT_REQUIRE_CLI_PARITY=1` makes their absence a failure).
//! Raw JSON-RPC lines check the unsupported-version error, progress
//! notifications, that stdout carries nothing but JSON, that the server opens
//! no socket, and that it exits when stdin closes.

use std::ffi::OsStr;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, ClientConfig, Implementation,
    ProtocolVersion,
};
use rmcp::service::RunningService;
use rmcp::transport::{ConfigureCommandExt, TokioChildProcess};
use rmcp::{ClientLifecycleMode, ClientServiceExt, RoleClient, ServiceExt};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const SERVER: &str = env!("CARGO_BIN_EXE_rvt-rs-mcp");
const EPOCH: &str = "1700000000";
const TOOLS: [&str; 9] = [
    "export_gltf",
    "export_ifc",
    "get_diagnostics",
    "get_element",
    "get_summary",
    "list_elements",
    "list_levels",
    "list_rooms",
    "open_model",
];

type Client = RunningService<RoleClient, ClientConfig>;

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn fixture() -> PathBuf {
    repository().join("corpus/tier1/architectural-2024/architectural-2024.rvt")
}

/// Einhoven, when the project corpus is present.
fn einhoven() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR")?;
    let path = Path::new(&dir).join("Revit_IFC5_Einhoven.rvt");
    path.is_file().then_some(path)
}

/// A fresh folder for one test's exports.
fn scratch(name: &str) -> PathBuf {
    let dir =
        Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("mcp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the scratch folder");
    dir
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A binary built beside the server, such as `rvt-ifc`.
fn sibling(name: &str) -> Option<PathBuf> {
    let found = Path::new(SERVER)
        .parent()?
        .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if found.is_file() {
        return Some(found);
    }
    assert!(
        std::env::var_os("RVT_REQUIRE_CLI_PARITY").is_none(),
        "RVT_REQUIRE_CLI_PARITY is set but {} was not built beside the server",
        found.display()
    );
    None
}

fn object(value: Value) -> serde_json::Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("tool arguments must be an object, not {other}"),
    }
}

async fn call(client: &Client, name: &'static str, arguments: Value) -> CallToolResult {
    client
        .call_tool(CallToolRequestParams::new(name).with_arguments(object(arguments)))
        .await
        .unwrap_or_else(|error| panic!("tools/call {name}: {error}"))
}

fn error_text(result: &CallToolResult) -> String {
    assert_eq!(result.is_error, Some(true), "expected a tool error");
    result
        .content
        .iter()
        .filter_map(|block| block.as_text().map(|text| text.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A successful result's structured content, also given as the same JSON text.
fn answer(result: &CallToolResult) -> Value {
    assert_ne!(
        result.is_error,
        Some(true),
        "tool error: {}",
        result
            .content
            .iter()
            .filter_map(|block| block.as_text().map(|text| text.text.as_str()))
            .collect::<String>()
    );
    let structured = result
        .structured_content
        .clone()
        .expect("structuredContent");
    let text = result
        .content
        .iter()
        .find_map(|block| block.as_text())
        .expect("the same JSON as a text block");
    let parsed: Value = serde_json::from_str(&text.text).expect("the text block is JSON");
    assert_eq!(
        parsed, structured,
        "text block and structuredContent differ"
    );
    structured
}

async fn start_discover() -> Client {
    ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("rvt-rs-mcp-e2e", "0"),
    )
    .serve_with_lifecycle(
        transport(),
        ClientLifecycleMode::Discover {
            preferred_versions: vec![ProtocolVersion::V_2026_07_28],
        },
    )
    .await
    .expect("server/discover")
}

async fn start_initialize() -> Client {
    ClientConfig::new(
        ClientCapabilities::default(),
        Implementation::new("rvt-rs-mcp-e2e", "0"),
    )
    .with_protocol_version(ProtocolVersion::V_2025_11_25)
    .serve(transport())
    .await
    .expect("initialize")
}

fn transport() -> TokioChildProcess {
    TokioChildProcess::new(tokio::process::Command::new(SERVER).configure(|command| {
        command.env("SOURCE_DATE_EPOCH", EPOCH);
    }))
    .expect("spawn rvt-rs-mcp")
}

/// Every tool, in one lifecycle, on `model`.
async fn drive(client: &Client, modern: bool, model: &Path, out: &Path, real_project: bool) {
    let path = model.to_string_lossy().into_owned();

    // The tool list: nine tools, each with an output schema; fresh for an hour
    // (and result types present) only for a 2026-07-28 peer.
    let list = client.list_tools(None).await.expect("tools/list");
    let names: Vec<String> = list
        .tools
        .iter()
        .map(|tool| tool.name.to_string())
        .collect();
    assert_eq!(names, TOOLS, "tools/list is sorted by name");
    for tool in &list.tools {
        assert!(
            tool.output_schema.is_some(),
            "{} declares no outputSchema",
            tool.name
        );
    }
    for tool in &list.tools {
        assert!(
            tool.description
                .as_deref()
                .is_some_and(|text| text.contains("docs/status.md")),
            "{} does not say what is not read and point to docs/status.md",
            tool.name
        );
    }
    if modern {
        assert!(
            list.ttl_ms.is_some_and(|ttl| ttl > 0),
            "ttlMs: {:?}",
            list.ttl_ms
        );
        assert!(
            list.result_type
                .as_ref()
                .is_some_and(|kind| kind.is_complete())
        );
    } else {
        assert!(
            list.result_type.is_none(),
            "a 2025-11-25 peer gets no resultType"
        );
    }

    // open_model: an opaque handle and the summary.
    let opened = call(client, "open_model", json!({ "model": path })).await;
    assert_eq!(opened.result_type.is_some(), modern);
    let summary = answer(&opened);
    let model_id = summary["model_id"].as_str().expect("model_id").to_string();
    assert!(
        model_id.starts_with("rvt-") && model_id.len() == 20,
        "{model_id}"
    );
    assert!(
        !path.contains(&model_id) && !model_id.contains("tier1"),
        "the handle carries no path"
    );
    assert_eq!(summary["file_kind"], "rvt");
    assert!(
        summary["not_read"]
            .as_str()
            .is_some_and(|text| text.contains("docs/status.md"))
    );
    assert!(summary["diagnostics"]["confidence_level"].is_string());
    assert!(summary["file"]["revit_release"].is_number(), "{summary}");
    let total = summary["elements"]["total"]
        .as_u64()
        .expect("elements.total");
    if real_project {
        assert!(total > 0, "a real project has elements");
    }

    // The same summary by id, and every other tool by id or by path.
    let again = answer(&call(client, "get_summary", json!({ "model_id": model_id })).await);
    assert_eq!(again, summary, "get_summary is open_model's summary");
    let levels = answer(&call(client, "list_levels", json!({ "model": path })).await);
    assert_eq!(
        levels["model_id"], summary["model_id"],
        "a path names the same model"
    );
    assert_eq!(levels["levels"], summary["levels"]);

    // An id this server did not issue is an error that says what to do.
    let unknown = call(
        client,
        "list_levels",
        json!({ "model": "rvt-0000000000000000" }),
    )
    .await;
    assert!(error_text(&unknown).contains("open_model"));

    let listed = answer(
        &call(
            client,
            "list_elements",
            json!({ "model": model_id, "limit": 5 }),
        )
        .await,
    );
    assert_eq!(listed["total"].as_u64(), Some(total));
    let rows = listed["elements"].as_array().expect("elements");
    assert_eq!(listed["returned"].as_u64(), Some(rows.len() as u64));
    assert!(rows.len() <= 5);

    let rooms = answer(&call(client, "list_rooms", json!({ "model": model_id })).await);
    assert!(rooms["rooms"].is_array());

    let diagnostics = answer(&call(client, "get_diagnostics", json!({ "model": model_id })).await);
    assert_eq!(diagnostics["diagnostics"]["schema_version"], 1);
    assert!(diagnostics["diagnostics"]["exported"].is_object());

    // An element that is not there is a tool error, not a protocol error.
    let missing = call(
        client,
        "get_element",
        json!({ "model": model_id, "id": "999999999" }),
    )
    .await;
    assert!(error_text(&missing).contains("list_elements"));

    if let Some(first) = rows.iter().find(|row| row["id"].is_string()) {
        element_checks(client, &model_id, first, out).await;
    }

    // Exports: a summary and a link, the file where it was asked for.
    let ifc = out.join("model.ifc");
    let exported = call(
        client,
        "export_ifc",
        json!({ "model": model_id, "out_path": ifc }),
    )
    .await;
    assert_eq!(exported.result_type.is_some(), modern);
    let summary_ifc = answer(&exported);
    let bytes = std::fs::read(&ifc).expect("the IFC was written");
    assert!(bytes.starts_with(b"ISO-10303-21;"));
    assert_eq!(summary_ifc["output"]["sha256"], sha256_hex(&bytes));
    assert_eq!(
        summary_ifc["output"]["bytes"].as_u64(),
        Some(bytes.len() as u64)
    );
    assert!(summary_ifc["diagnostics"].is_object());
    let link = exported
        .content
        .iter()
        .find_map(|block| block.as_resource_link())
        .expect("a resource_link to the file");
    assert!(
        link.uri.starts_with("file://") && link.uri.ends_with("model.ifc"),
        "{}",
        link.uri
    );
    assert_eq!(link.size, Some(bytes.len() as u64));

    // Refusals: an existing file, the wrong extension, the model itself.
    let exists = call(
        client,
        "export_ifc",
        json!({ "model": model_id, "out_path": ifc }),
    )
    .await;
    assert!(error_text(&exists).contains("overwrite"));
    let wrong = call(
        client,
        "export_ifc",
        json!({ "model": model_id, "out_path": out.join("model.txt") }),
    )
    .await;
    assert!(error_text(&wrong).contains(".ifc"));
    let itself = call(
        client,
        "export_ifc",
        json!({ "model": model_id, "out_path": path, "overwrite": true }),
    )
    .await;
    error_text(&itself);
    assert!(model.is_file(), "the model was not touched");
    let replaced = call(
        client,
        "export_ifc",
        json!({ "model": model_id, "out_path": ifc, "overwrite": true }),
    )
    .await;
    assert_eq!(
        answer(&replaced)["output"]["sha256"],
        summary_ifc["output"]["sha256"]
    );

    // The switches rvt-ifc has change the model, and map onto its flags.
    let switched = out.join("switched.ifc");
    let result = call(
        client,
        "export_ifc",
        json!({
            "model": model_id,
            "out_path": switched,
            "options": { "saved_meshes": "off", "no_base_quantities": true }
        }),
    )
    .await;
    let switched_summary = answer(&result);

    let glb = out.join("model.glb");
    let gltf = answer(
        &call(
            client,
            "export_gltf",
            json!({ "model": model_id, "out_path": glb }),
        )
        .await,
    );
    let glb_bytes = std::fs::read(&glb).expect("the glb was written");
    assert!(glb_bytes.starts_with(b"glTF"));
    assert_eq!(gltf["output"]["sha256"], sha256_hex(&glb_bytes));

    // The exports are what the CLIs write from the same file.
    if let Some(rvt_ifc) = sibling("rvt-ifc") {
        let cli = out.join("cli.ifc");
        let status = Command::new(rvt_ifc)
            .arg(model)
            .arg("-o")
            .arg(&cli)
            .env("SOURCE_DATE_EPOCH", EPOCH)
            .status()
            .expect("run rvt-ifc");
        assert!(status.success(), "rvt-ifc failed");
        assert_eq!(
            std::fs::read(&cli).expect("rvt-ifc output"),
            bytes,
            "export_ifc differs from rvt-ifc"
        );

        let cli_switched = out.join("cli-switched.ifc");
        let status = Command::new(sibling("rvt-ifc").expect("rvt-ifc"))
            .arg(model)
            .arg("-o")
            .arg(&cli_switched)
            .args(["--no-saved-meshes", "--no-base-quantities"])
            .env("SOURCE_DATE_EPOCH", EPOCH)
            .status()
            .expect("run rvt-ifc");
        assert!(status.success());
        assert_eq!(
            sha256_hex(&std::fs::read(&cli_switched).expect("rvt-ifc output")),
            switched_summary["output"]["sha256"]
                .as_str()
                .expect("sha256"),
            "export_ifc with options differs from rvt-ifc with the same flags"
        );
    }
    if let Some(rvt_gltf) = sibling("rvt-gltf") {
        let cli = out.join("cli.glb");
        let status = Command::new(rvt_gltf)
            .arg(model)
            .arg("-o")
            .arg(&cli)
            .status()
            .expect("run rvt-gltf");
        assert!(status.success(), "rvt-gltf failed");
        assert_eq!(
            std::fs::read(&cli).expect("rvt-gltf output"),
            glb_bytes,
            "export_gltf differs from rvt-gltf"
        );
    }
}

/// `get_element` on the first listed element: provenance, lookup by either id,
/// and the GlobalId in the IFC the export writes.
async fn element_checks(client: &Client, model_id: &str, row: &Value, out: &Path) {
    let id = row["id"].as_str().expect("element id");
    let detail = answer(
        &call(
            client,
            "get_element",
            json!({ "model": model_id, "id": id }),
        )
        .await,
    );
    assert_eq!(detail["id"], row["id"]);
    assert!(detail["property_sets"].is_array());
    assert!(detail["trust_markers"].is_object());
    assert!(detail["file"]["revit_release"].is_number());
    assert_eq!(
        detail["provenance"], row["provenance"],
        "list and detail agree"
    );
    assert_eq!(detail["provenance"]["element_id"], row["id"]);
    let category = detail["provenance"]["category"].as_str().expect("category");
    let global_id = detail["provenance"]["global_id"]
        .as_str()
        .expect("every building element has a GlobalId")
        .to_string();
    assert_eq!(global_id.len(), 22, "{global_id}");

    let by_global = answer(
        &call(
            client,
            "get_element",
            json!({ "model": model_id, "id": global_id }),
        )
        .await,
    );
    assert_eq!(
        by_global["id"], row["id"],
        "a GlobalId finds the same element"
    );

    let by_category = answer(
        &call(
            client,
            "list_elements",
            json!({ "model": model_id, "category": category.to_lowercase(), "limit": 1000 }),
        )
        .await,
    );
    assert!(by_category["total"].as_u64().is_some_and(|total| total > 0));
    for found in by_category["elements"].as_array().expect("elements") {
        assert_eq!(found["provenance"]["category"], category);
    }

    let ifc = out.join("element-check.ifc");
    let written = answer(
        &call(
            client,
            "export_ifc",
            json!({ "model": model_id, "out_path": ifc }),
        )
        .await,
    );
    assert!(
        written["building_elements"]
            .as_u64()
            .is_some_and(|count| count > 0)
    );
    let text = String::from_utf8_lossy(&std::fs::read(&ifc).expect("IFC")).into_owned();
    assert!(
        text.contains(&format!("('{global_id}',")),
        "GlobalId {global_id} is not in the IFC the export wrote"
    );
}

#[tokio::test]
async fn discover_lifecycle_on_the_synthetic_fixture() {
    let out = scratch("discover");
    let client = start_discover().await;
    let info = client.peer_info().expect("server info");
    assert_eq!(info.protocol_version, ProtocolVersion::V_2026_07_28);
    drive(&client, true, &fixture(), &out, false).await;
    client.cancel().await.expect("close the client");
}

#[tokio::test]
async fn initialize_lifecycle_on_the_synthetic_fixture() {
    let out = scratch("initialize");
    let client = start_initialize().await;
    let info = client.peer_info().expect("server info");
    assert_eq!(info.protocol_version, ProtocolVersion::V_2025_11_25);
    drive(&client, false, &fixture(), &out, false).await;
    client.cancel().await.expect("close the client");
}

#[tokio::test]
async fn discover_lifecycle_on_einhoven() {
    let Some(model) = einhoven() else {
        return;
    };
    let out = scratch("einhoven");
    let client = start_discover().await;
    drive(&client, true, &model, &out, true).await;
    client.cancel().await.expect("close the client");
}

/// A server on raw pipes.
struct Raw {
    child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<Result<Value, String>>,
}

impl Raw {
    fn start(args: &[&OsStr]) -> Self {
        let mut child = Command::new(SERVER)
            .args(args)
            .env("SOURCE_DATE_EPOCH", EPOCH)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn rvt-rs-mcp");
        let stdin = child.stdin.take();
        let stdout = child.stdout.take().expect("stdout");
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let parsed = serde_json::from_str::<Value>(&line).map_err(|error| {
                    format!("stdout carried a line that is not JSON: {line:?}: {error}")
                });
                if sender.send(parsed).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            stdin,
            lines,
        }
    }

    fn send(&mut self, message: &Value) {
        let stdin = self.stdin.as_mut().expect("stdin is open");
        writeln!(stdin, "{message}").expect("write a request");
        stdin.flush().expect("flush");
    }

    fn recv(&self) -> Value {
        self.lines
            .recv_timeout(Duration::from_secs(120))
            .expect("the server answered in time")
            .unwrap_or_else(|problem| panic!("{problem}"))
    }

    /// The messages up to and including the response to `id`.
    fn until_response(&self, id: i64) -> Vec<Value> {
        let mut seen = Vec::new();
        loop {
            let message = self.recv();
            let done = message["id"] == json!(id);
            seen.push(message);
            if done {
                return seen;
            }
        }
    }
}

fn modern_meta() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": "2026-07-28",
        "io.modelcontextprotocol/clientCapabilities": {}
    })
}

#[test]
fn wire_discipline_on_raw_stdio() {
    let mut server = Raw::start(&[]);

    // server/discover lists both lifecycles' versions.
    server.send(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "server/discover",
        "params": { "_meta": modern_meta() }
    }));
    let discovered = server.recv();
    assert_eq!(discovered["id"], 1);
    let versions = discovered["result"]["supportedVersions"]
        .as_array()
        .expect("supportedVersions");
    assert!(versions.contains(&json!("2026-07-28")));
    assert!(versions.contains(&json!("2025-11-25")));
    assert!(discovered["result"]["capabilities"]["tools"].is_object());

    // An unsupported protocol version in _meta is -32022 with the supported list.
    server.send(&json!({
        "jsonrpc": "2.0", "id": 2, "method": "tools/list",
        "params": { "_meta": {
            "io.modelcontextprotocol/protocolVersion": "1999-01-01",
            "io.modelcontextprotocol/clientCapabilities": {}
        } }
    }));
    let refused = server.recv();
    assert_eq!(refused["id"], 2);
    assert_eq!(refused["error"]["code"], -32022, "{refused}");
    assert_eq!(refused["error"]["data"]["requested"], "1999-01-01");
    assert!(
        refused["error"]["data"]["supported"]
            .as_array()
            .is_some_and(|supported| supported.contains(&json!("2026-07-28")))
    );

    // A call with a progress token: progress notifications come first, then
    // the response, in the stateless result shape.
    let mut meta = modern_meta();
    meta["progressToken"] = json!("p1");
    server.send(&json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": {
            "name": "open_model",
            "arguments": { "model": fixture() },
            "_meta": meta
        }
    }));
    let messages = server.until_response(3);
    let (response, notices) = messages.split_last().expect("a response");
    assert!(
        !notices.is_empty(),
        "no progress notification came before the result"
    );
    for notice in notices {
        assert_eq!(notice["method"], "notifications/progress", "{notice}");
        assert_eq!(notice["params"]["progressToken"], "p1");
    }
    assert_eq!(response["result"]["resultType"], "complete", "{response}");
    assert_eq!(response["result"]["isError"], false);
    assert!(response["result"]["structuredContent"]["model_id"].is_string());

    // Opening the model made no socket (checked where /proc lists them).
    #[cfg(target_os = "linux")]
    {
        let fds =
            std::fs::read_dir(format!("/proc/{}/fd", server.child.id())).expect("list the fds");
        for fd in fds.flatten() {
            let target = std::fs::read_link(fd.path()).unwrap_or_default();
            assert!(
                !target.to_string_lossy().starts_with("socket:"),
                "the server holds a socket: {}",
                target.display()
            );
        }
    }

    // Closing stdin ends the server.
    drop(server.stdin.take());
    let started = Instant::now();
    loop {
        match server.child.try_wait().expect("wait") {
            Some(status) => {
                assert!(status.success(), "exit status {status}");
                break;
            }
            None if started.elapsed() > Duration::from_secs(15) => {
                let _ = server.child.kill();
                panic!("the server did not exit after stdin closed");
            }
            None => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[test]
fn a_path_outside_output_dir_is_refused() {
    let out = scratch("output-dir");
    let inside = out.join("inside");
    std::fs::create_dir_all(&inside).expect("create the output folder");
    let mut server = Raw::start(&[OsStr::new("--output-dir"), inside.as_os_str()]);
    let mut export = |id: i64, arguments: Value| -> Value {
        server.send(&json!({
            "jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": { "name": "export_ifc", "arguments": arguments, "_meta": modern_meta() }
        }));
        server.recv()
    };
    let refused = export(
        1,
        json!({ "model": fixture(), "out_path": out.join("outside.ifc") }),
    );
    assert_eq!(refused["result"]["isError"], true, "{refused}");
    let allowed = export(2, json!({ "model": fixture(), "out_path": "relative.ifc" }));
    assert_eq!(allowed["result"]["isError"], false, "{allowed}");
    assert!(!out.join("outside.ifc").exists());
    assert!(inside.join("relative.ifc").is_file());
}
