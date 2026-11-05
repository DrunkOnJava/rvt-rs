//! The MCP server: ten tools over the models in [`Store`].
//!
//! Decoding and exporting run on tokio's blocking pool, so the server keeps
//! answering the protocol while a large model decodes. A request that carries
//! a progress token gets `notifications/progress`; `notifications/cancelled`
//! cancels the walk through [`WalkerControl`], which stops without writing
//! anything partial. The server never asks the client for anything.

use std::sync::Arc;
use std::time::{Duration, Instant};

use rmcp::model::{
    CacheScope, CallToolResult, ContentBlock, Implementation, JsonObject, ListToolsResult,
    PaginatedRequestParams, ProgressNotificationParam, ProtocolVersion, ServerCapabilities,
    ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, Json, RoleServer, ServerHandler, tool, tool_handler, tool_router};
use rvt::control::{CancelToken, ProgressEvent, Stage, WalkerControl};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::args::{
    ExplainElementArgs, ExportGltfArgs, ExportIfcArgs, GetElementArgs, ListElementsArgs, ModelArgs,
    phase_of,
};
use crate::config::Failure;
use crate::exports::{self, Exported};
use crate::schema::{
    DiagnosticsOut, ElementDetail, ExplainElementOut, ExportOut, LevelsOut, ListElementsOut,
    RoomsOut, SummaryOut,
};
use crate::store::{Computed, Entry, Store};
use crate::views::{self, ElementFilter};

/// How long a client may keep the tool list: the inventory is fixed per binary.
const TOOL_LIST_TTL_MS: u64 = 3_600_000;

/// The fewest milliseconds between two progress notifications of one stage.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

const DEFAULT_PAGE: u32 = 100;
const MAX_PAGE: u32 = 1000;

const INSTRUCTIONS: &str = "rvt-rs-mcp reads Autodesk Revit files (.rvt, .rfa, .rte, .rft) with the rvt library: local, read-only on the model, stdio only, no network and no Revit install. Call open_model with a file path to get a model_id, then pass the model_id (or the path) to any tool. A model_id is opaque; it lives while this server runs and while the model is among the most recently used ones the server keeps, so when a tool says the id is not open, call open_model again. Every answer comes from the decoded model the IFC export writes, with provenance on each element. rvt-rs decodes a subset of a Revit file: the not_read field of the summary and docs/status.md in the rvt-rs repository say what is missing.";

macro_rules! not_read {
    () => {
        "What is not read: rvt-rs decodes a subset of a Revit file, and this server answers from the model the IFC export writes. An element's body is often only its record's bounding box (provenance.body_source says which); family files give no family geometry; an element's property sets hold the parameters the export writes, and Revit's own parameter sets need export_ifc with options.revit_property_sets; the Revit Mark parameter is not decoded; files saved by Revit 2015 to 2022, 2026 and 2027 are experimental, and 2008 to 2014 more so (release_support in the summary); get_diagnostics lists what the export skipped or approximated and why. docs/status.md in the rvt-rs repository (https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/status.md) states what is read per release and category."
    };
}

macro_rules! short_not_read {
    () => {
        "Not read: family geometry, the Revit Mark parameter, and any body but the record's bounding box where provenance.body_source says so; see the summary's not_read and docs/status.md in the rvt-rs repository."
    };
}

/// The server: tools over a [`Store`] of decoded models.
#[derive(Clone)]
pub struct RvtServer {
    store: Arc<Store>,
}

impl RvtServer {
    pub fn new(store: Arc<Store>) -> Self {
        Self { store }
    }
}

fn fail(message: impl Into<String>) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(message)])
}

/// A tool's arguments as its type. The tools take the arguments as a JSON
/// object and read them here, not through rmcp's `Parameters`, so that a
/// missing field or a value of the wrong type is a tool error the model can
/// act on (`isError` with a message), as MCP's tools page asks for input
/// validation, and not a JSON-RPC protocol error the client would show as an
/// opaque failure.
fn parse<T: DeserializeOwned>(tool: &str, arguments: JsonObject) -> Result<T, CallToolResult> {
    serde_json::from_value(Value::Object(arguments)).map_err(|error| {
        fail(format!(
            "Invalid arguments for {tool}: {error}. The tool's inputSchema lists them; `model` is a model_id from open_model or the path of a Revit file."
        ))
    })
}

fn failure_result(failure: Failure) -> CallToolResult {
    match failure {
        Failure::Cancelled => fail("The request was cancelled; nothing was written."),
        Failure::Message(message) => fail(message),
    }
}

/// Run `job` on the blocking pool with a [`WalkerControl`] wired to the
/// request: progress goes out as notifications when the request carries a
/// progress token, and a cancelled request cancels the walk.
async fn run_job<T, F>(context: &RequestContext<RoleServer>, job: F) -> Result<T, CallToolResult>
where
    T: Send + 'static,
    F: FnOnce(&WalkerControl) -> Result<T, Failure> + Send + 'static,
{
    let cancel = CancelToken::new();
    let token = context.meta.get_progress_token();
    let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel::<ProgressEvent>();
    let mut control = WalkerControl::new().with_cancel(cancel.clone());
    if token.is_some() {
        control = control.with_progress(move |event| {
            let _ = events_tx.send(event);
        });
    }
    let mut worker = tokio::task::spawn_blocking(move || job(&control));
    let mut events_open = token.is_some();
    let mut cancel_sent = false;
    let mut sent = 0.0_f64;
    let mut last_stage: Option<Stage> = None;
    let mut last_sent: Option<Instant> = None;
    let joined = loop {
        tokio::select! {
            joined = &mut worker => break joined,
            event = events_rx.recv(), if events_open => match event {
                Some(event) => {
                    let now = Instant::now();
                    let new_stage = last_stage != Some(event.stage);
                    let due = last_sent.is_none_or(|at| now.duration_since(at) >= PROGRESS_INTERVAL);
                    if (new_stage || due)
                        && let Some(token) = &token
                    {
                        sent += 1.0;
                        let total = event.total.map(|total| format!("/{total}")).unwrap_or_default();
                        let message = format!("{} {}{total}", event.stage.as_str(), event.done);
                        let notice = ProgressNotificationParam::new(token.clone(), sent)
                            .with_message(message);
                        let _ = context.peer.notify_progress(notice).await;
                        last_stage = Some(event.stage);
                        last_sent = Some(now);
                    }
                }
                None => events_open = false,
            },
            () = context.ct.cancelled(), if !cancel_sent => {
                cancel.cancel();
                cancel_sent = true;
            }
        }
    };
    if let Some(token) = &token {
        let notice =
            ProgressNotificationParam::new(token.clone(), sent + 1.0).with_message("finished");
        let _ = context.peer.notify_progress(notice).await;
    }
    match joined {
        Ok(result) => result.map_err(failure_result),
        Err(error) => Err(fail(format!(
            "internal error: the decoding task failed: {error}"
        ))),
    }
}

/// The open model of `model` (at `phase`) and its default decode.
fn decode(
    store: &Store,
    model: &str,
    phase: Option<&str>,
    control: &WalkerControl,
) -> Result<(Arc<Entry>, Arc<Computed>), Failure> {
    let entry = store.open(model, phase)?;
    let computed = entry.computed(store.config(), control)?;
    Ok((entry, computed))
}

async fn summary_answer(
    store: &Arc<Store>,
    context: &RequestContext<RoleServer>,
    args: ModelArgs,
) -> Result<Json<SummaryOut>, CallToolResult> {
    let store = Arc::clone(store);
    let out = run_job(context, move |control| {
        let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
        Ok(views::summary(&entry, &computed))
    })
    .await?;
    Ok(Json(out))
}

/// The tool result of an export: its summary as structured content and text,
/// and a `resource_link` to the file.
fn export_result(exported: Exported) -> Result<CallToolResult, CallToolResult> {
    let value = serde_json::to_value(&exported.out).map_err(|error| {
        fail(format!(
            "internal error: the answer did not serialize: {error}"
        ))
    })?;
    let mut result = CallToolResult::structured(value);
    result
        .content
        .push(ContentBlock::resource_link(exported.link));
    Ok(result)
}

#[tool_router]
impl RvtServer {
    #[tool(
        name = "open_model",
        input_schema = rmcp::handler::server::common::schema_for_input::<ModelArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "Open a Revit file (.rvt, .rfa, .rte or .rft) and decode it once, with the same decode rvt-ifc runs. Every other tool answers from that decoded model and the IFC export is written from it, so an answer and the export cannot disagree. Returns model_id, the Revit release and build, element counts by IFC entity and Revit class, levels, and what was left out (diagnostics, unread streams). ",
            "Handle lifetime: model_id is opaque and stays valid while this server process runs and the model is among the most recently used ones the server keeps (2 by default); after an eviction or a restart a tool returns an error and you call open_model again. Every tool also takes the file path in place of the model_id. ",
            "Phase: pass phase (a phase name or ElementId) with the path to read a project phase other than the last, as rvt-ifc --phase does; the model_id then names that phase, and get_diagnostics lists the rooms it leaves out. ",
            "Reads the file only: no Revit install, no network call, nothing is written. ",
            not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn open_model(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<SummaryOut>, CallToolResult> {
        let args = parse::<ModelArgs>("open_model", arguments)?;
        summary_answer(&self.store, &context, args).await
    }

    #[tool(
        name = "get_summary",
        input_schema = rmcp::handler::server::common::schema_for_input::<ModelArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "The summary open_model returns, for a model_id or a path: release and build, element counts by IFC entity and Revit class, levels, room count and the diagnostics summary (what was read, approximated, skipped and left unread). ",
            not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_summary(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<SummaryOut>, CallToolResult> {
        let args = parse::<ModelArgs>("get_summary", arguments)?;
        summary_answer(&self.store, &context, args).await
    }

    #[tool(
        name = "list_elements",
        input_schema = rmcp::handler::server::common::schema_for_input::<ListElementsArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "List the building elements of a model, optionally by category (IFC entity such as IFCWALL), class (Revit class such as Wall) and level (storey name), paged by limit (100 by default, at most 1000) and offset. Each row has the Revit ElementId, name and long_name (the IFC Name and LongName: an IfcSpace's are its room number and its room name, and name is absent for a room with neither), level and a provenance object: element_id, global_id, category, class, body_source, profile_source, space_outline_source and area_body_refusal, and where the element's record sits: carrier, stream, offset, record_offset, enclosing_record_id, class_tag, class_name and native_record, each present only where the model has it. The rows leave out value_sources, the copy each reader took a value of the body from: get_element lists them. ",
            short_not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_elements(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ListElementsOut>, CallToolResult> {
        let args = parse::<ListElementsArgs>("list_elements", arguments)?;
        let store = Arc::clone(&self.store);
        let out = run_job(&context, move |control| {
            let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
            let filter = ElementFilter {
                category: args
                    .category
                    .as_deref()
                    .filter(|text| !text.trim().is_empty()),
                class: args.class.as_deref().filter(|text| !text.trim().is_empty()),
                level: args.level.as_deref().filter(|text| !text.trim().is_empty()),
                offset: args.offset.unwrap_or(0) as usize,
                limit: args.limit.unwrap_or(DEFAULT_PAGE).clamp(1, MAX_PAGE) as usize,
            };
            Ok(views::list_elements(&entry, &computed, &filter))
        })
        .await?;
        Ok(Json(out))
    }

    #[tool(
        name = "get_element",
        input_schema = rmcp::handler::server::common::schema_for_input::<GetElementArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "One element, by Revit ElementId or IFC GlobalId (the Revit Mark parameter is not decoded, so it cannot be looked up). Returns its property sets and quantities as the IFC export writes them (quantities in the IFC's SI units), its trust markers (the ...Source, ...Resolved and ...Refusal properties of its record, such as BodySource), its type object as the export writes it (element_type: the IFC entity, name and Tag, and the sets it holds; absent where the export writes none), its provenance (the carrier its record was read from, its stream and byte offset, and in value_sources the copy each reader took a value of its body from), and the file's: Revit release and build, schema class count, partition count and ElemTable record layout. ",
            short_not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_element(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ElementDetail>, CallToolResult> {
        let args = parse::<GetElementArgs>("get_element", arguments)?;
        let store = Arc::clone(&self.store);
        let out = run_job(&context, move |control| {
            let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
            views::get_element(&entry, &computed, &args.id.text()).map_err(Failure::Message)
        })
        .await?;
        Ok(Json(out))
    }

    #[tool(
        name = "explain_element",
        input_schema = rmcp::handler::server::common::schema_for_input::<ExplainElementArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "Why an element is in the export as it is, or is not in it: its provenance (with value_sources, the copy each reader took a value of its body from), its trust markers (the ...Source, ...Resolved and ...Refusal properties of its record) and every fail-closed decision recorded for it, each with a stage (element_skipped, body_not_drawn, body_left_as_box, opening_not_cut, outline_not_read, storey_not_bound), a reason code (the reason of the skipped item in get_diagnostics, or body_is_record_box, space_outline_is_record_box, area_body_refused, slab_shape_refused, wall_base_cut_refused), the stream and byte offset its record was read from where known, and a detail. Takes a Revit ElementId whether or not the export holds the element (a left-out element has decisions and no provenance), or the GlobalId of an exported one. The log keeps at most 5000 decisions a reason; capped_reasons names the reasons that dropped some, whose decisions about this element may be missing. A frame with no ElementId has no entry. ",
            short_not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn explain_element(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<ExplainElementOut>, CallToolResult> {
        let args = parse::<ExplainElementArgs>("explain_element", arguments)?;
        let store = Arc::clone(&self.store);
        let out = run_job(&context, move |control| {
            let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
            views::explain_element(&entry, &computed, &args.element.text())
                .map_err(Failure::Message)
        })
        .await?;
        Ok(Json(out))
    }

    #[tool(
        name = "list_rooms",
        input_schema = rmcp::handler::server::common::schema_for_input::<ModelArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "List the rooms and spaces of a model: ElementId, number, name, level, area and provenance, whose space_outline_source says where the outline came from. The area is present only where the outline was read exactly (space_outline_source stored_solid, stored_solid_faces, or area_boundary or area_boundary_joined for an Area); a room that keeps its record's bounding box (record_box) has no area, since the area of a box is not the area of a room. ",
            short_not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_rooms(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<RoomsOut>, CallToolResult> {
        let args = parse::<ModelArgs>("list_rooms", arguments)?;
        let store = Arc::clone(&self.store);
        let out = run_job(&context, move |control| {
            let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
            Ok(views::rooms(&entry, &computed))
        })
        .await?;
        Ok(Json(out))
    }

    #[tool(
        name = "list_levels",
        input_schema = rmcp::handler::server::common::schema_for_input::<ModelArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "List the levels (building storeys) of a model with their elevation in feet and metres and how many building elements the export puts on each. ",
            short_not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn list_levels(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<LevelsOut>, CallToolResult> {
        let args = parse::<ModelArgs>("list_levels", arguments)?;
        let store = Arc::clone(&self.store);
        let out = run_job(&context, move |control| {
            let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
            Ok(views::levels_out(&entry, &computed))
        })
        .await?;
        Ok(Json(out))
    }

    #[tool(
        name = "get_diagnostics",
        input_schema = rmcp::handler::server::common::schema_for_input::<ModelArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "The export diagnostics of a model, as rvt-ifc --diagnostics writes them (docs/schemas/export-diagnostics.schema.json in the rvt-rs repository): what was decoded and exported, what was skipped, approximated or left out with the reason for each, unsupported features, warnings, the confidence level, and the streams a resource limit kept unread. ",
            not_read!()
        ),
        annotations(
            read_only_hint = true,
            destructive_hint = false,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn get_diagnostics(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<Json<DiagnosticsOut>, CallToolResult> {
        let args = parse::<ModelArgs>("get_diagnostics", arguments)?;
        let store = Arc::clone(&self.store);
        let out = run_job(&context, move |control| {
            let (entry, computed) = decode(&store, &args.model, phase_of(&args.phase), control)?;
            Ok(views::diagnostics_out(&entry, &computed))
        })
        .await?;
        Ok(Json(out))
    }

    #[tool(
        name = "export_ifc",
        input_schema = rmcp::handler::server::common::schema_for_input::<ExportIfcArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "Write the model as IFC4 to out_path, which must end in .ifc, must not be the model, and must not exist unless overwrite is true. The file is the one rvt-ifc writes with the same switches (options); SOURCE_DATE_EPOCH fixes the time in its header. If a resource limit kept part of the file unread, nothing is written unless options.allow_partial is true. Returns a summary (entity counts, diagnostics, unread streams, size and sha256) and a resource_link to the file; the IFC is never returned inline. Sends progress notifications when the request carries a progress token, and cancelling it stops the export with nothing written. Writes only the file it is asked for. ",
            short_not_read!()
        ),
        output_schema = rmcp::handler::server::tool::schema_for_output::<ExportOut>(),
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn export_ifc(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, CallToolResult> {
        let args = parse::<ExportIfcArgs>("export_ifc", arguments)?;
        let store = Arc::clone(&self.store);
        let exported = run_job(&context, move |control| {
            exports::ifc(&store, &args, control)
        })
        .await?;
        export_result(exported)
    }

    #[tool(
        name = "export_gltf",
        input_schema = rmcp::handler::server::common::schema_for_input::<ExportGltfArgs>()
            .expect("the input schema is an object"),
        description = concat!(
            "Write the model as a binary glTF 2.0 file to out_path, which must end in .glb, must not be the model, and must not exist unless overwrite is true. The file is the one rvt-gltf writes. If a resource limit kept part of the file unread, nothing is written unless allow_partial is true. Returns a summary (entity counts, unread streams, size and sha256) and a resource_link to the file; the glb is never returned inline. Sends progress notifications when the request carries a progress token, and cancelling it stops the export with nothing written. Writes only the file it is asked for. ",
            short_not_read!()
        ),
        output_schema = rmcp::handler::server::tool::schema_for_output::<ExportOut>(),
        annotations(
            read_only_hint = false,
            destructive_hint = true,
            idempotent_hint = true,
            open_world_hint = false
        )
    )]
    async fn export_gltf(
        &self,
        arguments: JsonObject,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, CallToolResult> {
        let args = parse::<ExportGltfArgs>("export_gltf", arguments)?;
        let store = Arc::clone(&self.store);
        let exported = run_job(&context, move |control| {
            exports::gltf(&store, &args, control)
        })
        .await?;
        export_result(exported)
    }
}

#[tool_handler]
impl ServerHandler for RvtServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("rvt-rs-mcp", env!("CARGO_PKG_VERSION"))
                    .with_title("rvt-rs: Revit files without Revit")
                    .with_description(
                        "Local, read-only MCP server for Autodesk Revit files, over stdio.",
                    ),
            )
            .with_instructions(INSTRUCTIONS)
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let modern = context
            .protocol_version()
            .is_some_and(|version| version >= ProtocolVersion::V_2026_07_28);
        let mut result = ListToolsResult::with_all_items(Self::tool_router().list_all());
        if modern {
            result = result
                .with_ttl_ms(TOOL_LIST_TTL_MS)
                .with_cache_scope(CacheScope::Public);
        }
        Ok(result)
    }
}
