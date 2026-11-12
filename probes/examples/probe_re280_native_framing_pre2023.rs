//! RE-280 (probe): whether the native record reader's framing holds on
//! Revit 2015 to 2022 (#1158).
//!
//! FACT under test: `native_document::extract_records` admits Revit 2023 to
//! 2027 only. It frames a record group as `id · [u32] · u32 length · body ·
//! u32 length` (the extra word on channels 102 and 103) with a `u32` id on
//! Revit 2023 and a `u64` id from 2024, and it reads a global preamble
//! (`Formats/Latest`, `Global/History`, `Global/ElemTable`,
//! `Global/DocumentIncrementTable`, `Global/Latest`) with contracts measured
//! on 2023 to 2027 only. Whether an older release takes the 2023 layout is
//! unmeasured, and so is whether the records' fields read as the same model's
//! 2023 copy's do.
//!
//! One file (`FILE.rvt`, with `--records` for the lighter run that skips the
//! per-record decoding) or two (`OLDER.rvt NEWER.rvt`, in either order):
//!
//! - the preamble, one stream at a time: the registry, the history, the
//!   element table, the increment table, the extensible-storage catalogue.
//!   A stream that does not read reports, for each of the gzip member
//!   offsets 0, 4, 8 and 16, whether it has the magic and inflates;
//! - the shape of the classes the contracts read (`Element`, `ElementId`,
//!   `Identifier`, `ElemTable` and its record, the segment markers, the
//!   phase classes);
//! - for every group of channels 101 to 103, plain and keyed: whether it
//!   frames, with dual lengths agreeing to its end, and whether it also
//!   matches its declared object count and body bytes, for a `u32` or `u64`
//!   id and each width of the extra field (0, 4, 8 and 12 bytes); groups
//!   that frame at both id widths; the extra word's values;
//! - the channel-102 records of declared elements: the framed id against
//!   the id the decoded owner graph names, `native_element::decode_base`, the
//!   object graph, and the saved parameter sets, each with its histogram of
//!   refusals;
//! - with two files, the same model's records by ElementId: class, every
//!   field of every graph object, the Element base scalars, the parameters
//!   by id, the element index, and the schema.
//!
//! Output is `native framing:` count lines, which hold only counts, schema
//! class and field names and normalised error text, so that a Measure run
//! can keep them for a model whose per-element results are not uploaded
//! (`tools/ci/scrub-holdout-results.sh`), and JSON lines of examples with
//! ElementIds, which it deletes.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re280_native_framing_pre2023 -- FILE.rvt [--records]
//!   cargo run --profile ci -p rvt-probes --example probe_re280_native_framing_pre2023 -- OLDER.rvt NEWER.rvt

use anyhow::Result;
use rvt::RevitFile;
use rvt::compression::{has_gzip_magic, inflate_at, prepare_stream_for_inflate};
use rvt::elem_table;
use rvt::research::native_extensible_storage::Catalog;
use rvt::research::native_parameters::{GraphLimits, GraphObject};
use rvt::research::schema_registry::{Class, Field, Registry};
use rvt::research::{
    native_document, native_element, native_es_catalog, native_index, native_metadata,
    native_parameter_definitions, native_parameters, native_segments, schema_registry,
};
use serde_json::{Map, Value, json};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::hash::{Hash, Hasher};

const MAX_GROUP_BYTES: usize = 256 * 1024 * 1024;
const MAX_STREAM_BYTES: u64 = 512 * 1024 * 1024;
/// Examples printed per kind.
const EXAMPLES: usize = 4;
/// Rows printed per histogram.
const TOP: usize = 20;
/// The classes whose shape is printed: what the contracts read.
const SHAPES: [&str; 14] = [
    "VarParam",
    "Element",
    "ElementId",
    "Identifier",
    "ElemTable",
    "ContentMarker",
    "ContentKey",
    "SegmentMarker",
    "SegmentCheckback",
    "SignatureMarker",
    "DocumentHistory",
    "DocumentIncrementTable",
    "AllProjectPhases",
    "ProjectPhase",
];

/// Where lines go. A `native framing:` line is counts and names only.
struct Out {
    tag: &'static str,
}

impl Out {
    fn line(&self, text: impl AsRef<str>) {
        println!("native framing: {}{}", self.tag, scrub(text.as_ref()));
    }

    fn detail(&self, value: Value) {
        println!("{value}");
    }
}

/// The characters a count line may hold (`tools/ci/scrub-holdout-results.sh`).
fn scrub(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " _:.,;()=#<>/+-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// An error message with its numbers and hex words folded, so that one cause
/// is one histogram row.
fn norm_msg(text: &str) -> String {
    let mut words = Vec::new();
    for word in text.split_whitespace() {
        let core = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
        let hex_like = core.len() >= 8
            && core.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
            && core.chars().any(|c| c.is_ascii_digit());
        words.push(if hex_like { "<hex>" } else { word });
    }
    let joined = words.join(" ");
    let mut folded = String::new();
    let mut in_digits = false;
    for c in joined.chars() {
        if c.is_ascii_digit() {
            if !in_digits {
                folded.push('#');
            }
            in_digits = true;
        } else {
            in_digits = false;
            folded.push(c);
        }
    }
    folded.chars().take(110).collect()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn hash_text(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

#[derive(Default)]
struct Count {
    n: usize,
    example: String,
}

type Tally = BTreeMap<String, Count>;

fn bump(tally: &mut Tally, key: String, example: impl FnOnce() -> String) {
    let entry = tally.entry(key).or_default();
    if entry.n == 0 {
        entry.example = example();
    }
    entry.n += 1;
}

fn top(tally: &Tally, limit: usize) -> Vec<(&String, &Count)> {
    let mut rows: Vec<_> = tally.iter().collect();
    rows.sort_by(|a, b| b.1.n.cmp(&a.1.n).then_with(|| a.0.cmp(b.0)));
    rows.truncate(limit);
    rows
}

fn print_top(out: &Out, label: &str, tally: &Tally) {
    for (key, count) in top(tally, TOP) {
        out.line(format!("{label} {key} x{}", count.n));
    }
}

fn print_examples(out: &Out, label: &str, tally: &Tally) {
    for (key, count) in top(tally, EXAMPLES * 2) {
        out.detail(json!({
            "kind": "example",
            "what": label,
            "key": key,
            "n": count.n,
            "example": count.example,
        }));
    }
}

fn describe_field(registry: &Registry, field: &Field) -> String {
    let mut text = format!(
        "{}:{}.{}.{}",
        field.name, field.base, field.modifier, field.uninterpreted_flags
    );
    for reference in &field.references {
        text.push('>');
        text.push_str(
            registry
                .class(reference.tag)
                .map_or("?", |c| c.name.as_str()),
        );
    }
    text
}

fn describe_class(registry: &Registry, class: &Class) -> String {
    let parent = if class.parent_reference.tag >= 12 {
        registry
            .class(class.parent_reference.tag)
            .map_or("?", |c| c.name.as_str())
    } else {
        "none"
    };
    let fields: Vec<String> = class
        .fields
        .iter()
        .map(|f| describe_field(registry, f))
        .collect();
    format!(
        "tag={} parent={parent} fields={}: {}",
        class.tag,
        class.fields.len(),
        fields.join(",")
    )
}

/// A record's JSON with what differs between any two saves left out: the
/// offsets, the pointer tokens and the class tags (named by their class).
fn normalize(registry: &Registry, value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            if map.contains_key("pointer_token") {
                let label: String = match map
                    .get("class_tag")
                    .and_then(Value::as_u64)
                    .and_then(|t| u16::try_from(t).ok())
                {
                    Some(tag) => registry
                        .class(tag)
                        .map_or("?".to_string(), |c| c.name.clone()),
                    None if map.get("pointer_token").and_then(Value::as_u64) == Some(0) => {
                        "null".to_string()
                    }
                    None => "external".to_string(),
                };
                return json!({ "ptr": label });
            }
            let drop_places = map.contains_key("state") || map.contains_key("raw_value");
            let mut out = Map::new();
            for (key, inner) in map {
                if key == "class_tag" && map.contains_key("class_name") {
                    continue;
                }
                if drop_places && (key == "offset" || key == "start" || key == "end") {
                    continue;
                }
                out.insert(key.clone(), normalize(registry, inner));
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| normalize(registry, v)).collect()),
        other => other.clone(),
    }
}

/// An identifier as the native readers read it: through `m_id` and `m_id64`
/// wrappers to the integer.
fn identifier(value: &Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

fn owner_id(fields: &Value) -> Option<i64> {
    fields.get("m_id").and_then(identifier)
}

/// A way to frame a record: the id's width and the bytes between the id and
/// the length.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Layout {
    id: usize,
    extra: usize,
}

const CANDIDATES: [Layout; 8] = [
    Layout { id: 4, extra: 0 },
    Layout { id: 4, extra: 4 },
    Layout { id: 4, extra: 8 },
    Layout { id: 4, extra: 12 },
    Layout { id: 8, extra: 0 },
    Layout { id: 8, extra: 4 },
    Layout { id: 8, extra: 8 },
    Layout { id: 8, extra: 12 },
];

impl Layout {
    fn header(self) -> usize {
        self.id + self.extra + 4
    }

    fn name(self) -> String {
        format!("id{}+{}", self.id, self.extra)
    }
}

struct Frame {
    id: u64,
    start: usize,
    end: usize,
    extra: u32,
}

struct Framed {
    records: Vec<Frame>,
    body_sum: u64,
}

fn read32(bytes: &[u8], at: usize) -> Result<u32, String> {
    u32_at(bytes, at).ok_or_else(|| format!("short read at {at}"))
}

/// Split a group under `layout`: every record's dual lengths must agree and
/// the records must end with the group.
fn split(bytes: &[u8], layout: Layout) -> Result<Framed, String> {
    let header = layout.header();
    let mut pos = 0;
    let mut records = Vec::new();
    let mut body_sum = 0u64;
    while pos < bytes.len() {
        if bytes.len() - pos < header + 4 {
            return Err(format!("truncated header at {pos}"));
        }
        let id = if layout.id == 4 {
            u64::from(read32(bytes, pos)?)
        } else {
            u64_at(bytes, pos).ok_or_else(|| format!("short read at {pos}"))?
        };
        let extra = if layout.extra >= 4 {
            read32(bytes, pos + layout.id)?
        } else {
            0
        };
        let length = read32(bytes, pos + header - 4)? as usize;
        let start = pos + header;
        let end = start
            .checked_add(length)
            .ok_or_else(|| format!("length overflow at {pos}"))?;
        if end.saturating_add(4) > bytes.len() {
            return Err(format!("body past the group end at {pos}"));
        }
        if read32(bytes, end)? as usize != length {
            return Err(format!("dual lengths disagree at {pos}"));
        }
        records.push(Frame {
            id,
            start,
            end,
            extra,
        });
        body_sum += length as u64;
        pos = end + 4;
    }
    Ok(Framed { records, body_sum })
}

struct Grouping {
    framed: Vec<Layout>,
    declared: Vec<Layout>,
    counted: Vec<Layout>,
    trailer: Vec<Layout>,
    chosen: Option<Framed>,
    kind: &'static str,
    errors: Vec<(Layout, String)>,
}

/// Every candidate layout against one group, and the one a reader would take:
/// the release's expected id width, then the other, each with the channel's
/// usual extra field, where the group's declared counts agree.
fn frame_group(
    source: &native_segments::GroupSource,
    bytes: &[u8],
    expected_width: usize,
) -> Grouping {
    let usual = if source.channel == 101 { 0 } else { 4 };
    let mut framed = Vec::new();
    let mut declared = Vec::new();
    let mut counted = Vec::new();
    let mut trailer = Vec::new();
    let mut errors = Vec::new();
    let mut results: Vec<(Layout, Framed)> = Vec::new();
    for layout in CANDIDATES {
        match split(bytes, layout) {
            Ok(found) => {
                framed.push(layout);
                let count = found.records.len() as u64;
                if count == source.declared_objects {
                    counted.push(layout);
                    if found.body_sum + 4 * count == source.declared_body_bytes {
                        trailer.push(layout);
                    }
                }
                if count == source.declared_objects && found.body_sum == source.declared_body_bytes
                {
                    declared.push(layout);
                }
                results.push((layout, found));
            }
            Err(error) => {
                if layout.extra == usual {
                    errors.push((layout, error));
                }
            }
        }
    }
    let narrow = Layout {
        id: 4,
        extra: usual,
    };
    let wide = Layout {
        id: 8,
        extra: usual,
    };
    let order = if expected_width == 4 {
        [narrow, wide]
    } else {
        [wide, narrow]
    };
    let (pick, kind) = if declared.contains(&order[0]) {
        (Some(order[0]), "expected")
    } else if declared.contains(&order[1]) {
        (Some(order[1]), "other")
    } else if let Some(first) = declared.first() {
        (Some(*first), "nonstandard")
    } else if framed.contains(&order[0]) {
        (Some(order[0]), "lenient")
    } else if framed.contains(&order[1]) {
        (Some(order[1]), "lenient")
    } else {
        (None, "none")
    };
    let chosen = pick.and_then(|layout| {
        let at = results.iter().position(|(found, _)| *found == layout)?;
        Some(results.swap_remove(at).1)
    });
    Grouping {
        framed,
        declared,
        counted,
        trailer,
        chosen,
        kind,
        errors,
    }
}

#[derive(Default)]
struct Hits {
    framed: usize,
    declared: usize,
    counted: usize,
    trailer: usize,
}

#[derive(Default)]
struct ChannelStats {
    groups: usize,
    bytes: u64,
    records: usize,
    layouts: BTreeMap<Layout, Hits>,
    both_widths: usize,
    kinds: BTreeMap<&'static str, usize>,
    extras: BTreeMap<u32, usize>,
}

/// What the Element base holds, for comparing two saves.
#[derive(Clone, PartialEq)]
struct BaseScalars {
    id: i64,
    ids: [i64; 7],
    flags: [bool; 3],
}

const BASE_NAMES: [&str; 10] = [
    "m_assocLevelId",
    "m_famId",
    "m_unplacedOwnerId",
    "m_ownerDBViewId",
    "m_createdPhaseId",
    "m_demolishedPhaseId",
    "m_designOptionId",
    "m_locked",
    "m_moribund",
    "m_dummy",
];

impl BaseScalars {
    fn values(&self) -> [i64; 10] {
        let mut values = [0i64; 10];
        values[..7].copy_from_slice(&self.ids);
        for (slot, flag) in values[7..].iter_mut().zip(self.flags) {
            *slot = i64::from(flag);
        }
        values
    }
}

struct Param {
    id: i64,
    storage: String,
    hash: u64,
    text: String,
}

struct FieldPrint {
    hash: u64,
    text: String,
}

struct Obj {
    class: String,
    fields: BTreeMap<String, FieldPrint>,
}

/// One record's decoding.
struct Rec {
    partition: u32,
    class: String,
    graph_error: Option<String>,
    id_agrees: Option<bool>,
    contract_error: Option<String>,
    base: Result<BaseScalars, String>,
    raw_graph_error: Option<String>,
    metadata_error: Option<String>,
    params: Vec<Param>,
    objects: Vec<Obj>,
}

struct Ctx<'a> {
    registry: &'a Registry,
    es: Option<&'a Catalog>,
    defs: &'a native_parameter_definitions::Registry,
    declared: &'a BTreeSet<u64>,
    expected_width: usize,
    decode: bool,
    compare: bool,
}

fn fingerprint(registry: &Registry, object: &GraphObject) -> Obj {
    let mut fields = BTreeMap::new();
    if let Value::Object(map) = &object.fields {
        for (name, value) in map {
            let text = normalize(registry, value).to_string();
            fields.insert(
                name.clone(),
                FieldPrint {
                    hash: hash_text(&text),
                    text: text.chars().take(100).collect(),
                },
            );
        }
    }
    Obj {
        class: object.class_name.clone(),
        fields,
    }
}

/// The Element base scalars from the decoded root, through either shape of
/// `Identifier` (`m_id` on 2015 to 2023, `m_id64` from 2024).
fn base_from_graph(root: &GraphObject) -> Result<BaseScalars, String> {
    let fields = &root.fields;
    let get = |name: &str| {
        fields
            .get(name)
            .and_then(identifier)
            .ok_or_else(|| format!("no {name}"))
    };
    let flag = |name: &str| {
        fields
            .get(name)
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("no {name}"))
    };
    Ok(BaseScalars {
        id: get("m_id")?,
        ids: [
            get("m_assocLevelId")?,
            get("m_famId")?,
            get("m_unplacedOwnerId")?,
            get("m_ownerDBViewId")?,
            get("m_createdPhaseId")?,
            get("m_demolishedPhaseId")?,
            get("m_designOptionId")?,
        ],
        flags: [flag("m_locked")?, flag("m_moribund")?, flag("m_dummy")?],
    })
}

fn decode_record(ctx: &Ctx<'_>, id: u64, partition: u32, body: &[u8]) -> Rec {
    let class = u16_at(body, 0)
        .and_then(|tag| ctx.registry.class(tag))
        .map_or_else(|| "<unregistered>".to_string(), |c| c.name.clone());
    let contract_error = native_element::decode_base(body, ctx.registry)
        .err()
        .map(|error| norm_msg(&format!("{error:#}")));
    let mut rec = Rec {
        partition,
        class,
        graph_error: None,
        id_agrees: None,
        contract_error,
        base: Err("graph refused".to_string()),
        raw_graph_error: None,
        metadata_error: None,
        params: Vec::new(),
        objects: Vec::new(),
    };
    match native_parameters::decode_graph_with_catalog_usage(
        body,
        ctx.registry,
        &GraphLimits::default(),
        ctx.es,
    ) {
        Err(error) => {
            let raw = format!("{error:#}");
            rec.graph_error = Some(norm_msg(&raw));
            rec.raw_graph_error = Some(raw.chars().take(300).collect());
        }
        Ok((graph, _usage)) => {
            rec.base = match graph.objects.first() {
                Some(root) => base_from_graph(root),
                None => Err("no root".to_string()),
            };
            rec.id_agrees = graph.objects.first().map(|root| {
                owner_id(&root.fields).is_some_and(|named| u64::try_from(named).ok() == Some(id))
            });
            match native_metadata::project_with_definitions(&graph, ctx.defs) {
                Err(error) => rec.metadata_error = Some(norm_msg(&format!("{error:#}"))),
                Ok(metadata) => {
                    for set in &metadata.parameter_sets {
                        for parameter in &set.parameters {
                            let text = normalize(ctx.registry, &parameter.raw_value).to_string();
                            rec.params.push(Param {
                                id: parameter.serialized_parameter_id,
                                storage: parameter.storage_type.clone(),
                                hash: hash_text(&text),
                                text: text.chars().take(80).collect(),
                            });
                        }
                    }
                }
            }
            if ctx.compare {
                rec.objects = graph
                    .objects
                    .iter()
                    .map(|object| fingerprint(ctx.registry, object))
                    .collect();
            }
        }
    }
    rec
}

#[derive(Default)]
struct Walk {
    channels: BTreeMap<(bool, u64), ChannelStats>,
    seen: BTreeMap<u64, usize>,
    copies: BTreeMap<u64, Vec<Rec>>,
    undeclared: Tally,
    classes: Tally,
    unresolved: usize,
    odd_examples: usize,
}

impl Walk {
    fn group(
        &mut self,
        ctx: &Ctx<'_>,
        out: &Out,
        partition: u32,
        source: &native_segments::GroupSource,
        bytes: &[u8],
    ) {
        let keyed = source.content_key.is_some();
        let key = (keyed, source.channel);
        let stats = self.channels.entry(key).or_default();
        stats.groups += 1;
        stats.bytes += bytes.len() as u64;
        if !(101..=103).contains(&source.channel) {
            return;
        }
        let grouping = frame_group(source, bytes, ctx.expected_width);
        for layout in &grouping.framed {
            stats.layouts.entry(*layout).or_default().framed += 1;
        }
        for layout in &grouping.declared {
            stats.layouts.entry(*layout).or_default().declared += 1;
        }
        for layout in &grouping.counted {
            stats.layouts.entry(*layout).or_default().counted += 1;
        }
        for layout in &grouping.trailer {
            stats.layouts.entry(*layout).or_default().trailer += 1;
        }
        let usual = if source.channel == 101 { 0 } else { 4 };
        let narrow = Layout {
            id: 4,
            extra: usual,
        };
        let wide = Layout {
            id: 8,
            extra: usual,
        };
        if grouping.declared.contains(&narrow) && grouping.declared.contains(&wide) {
            stats.both_widths += 1;
        }
        *stats.kinds.entry(grouping.kind).or_default() += 1;
        if (grouping.kind != "expected" || grouping.chosen.is_none())
            && self.odd_examples < EXAMPLES * 3
        {
            self.odd_examples += 1;
            out.detail(json!({
                "kind": "odd_group",
                "partition": partition,
                "channel": source.channel,
                "keyed": keyed,
                "choice": grouping.kind,
                "bytes": bytes.len(),
                "declared_objects": source.declared_objects,
                "declared_body_bytes": source.declared_body_bytes,
                "head": hex(&bytes[..bytes.len().min(48)]),
                "errors": grouping
                    .errors
                    .iter()
                    .map(|(layout, error)| format!("{}: {}", layout.name(), norm_msg(error)))
                    .collect::<Vec<_>>(),
            }));
        }
        let Some(framed) = grouping.chosen else {
            return;
        };
        let stats = self.channels.entry(key).or_default();
        stats.records += framed.records.len();
        if source.channel != 101 {
            for frame in &framed.records {
                if stats.extras.len() < 64 || stats.extras.contains_key(&frame.extra) {
                    *stats.extras.entry(frame.extra).or_default() += 1;
                }
            }
        }
        if keyed {
            return;
        }
        for frame in &framed.records {
            let body = &bytes[frame.start..frame.end];
            let class = u16_at(body, 0)
                .and_then(|tag| ctx.registry.class(tag))
                .map(|c| c.name.clone());
            if class.is_none() {
                self.unresolved += 1;
            }
            let class_name = class.unwrap_or_else(|| "<unregistered>".to_string());
            bump(
                &mut self.classes,
                format!("ch{} {class_name}", source.channel),
                || frame.id.to_string(),
            );
            if source.channel != 102 {
                continue;
            }
            *self.seen.entry(frame.id).or_default() += 1;
            if !ctx.declared.contains(&frame.id) {
                bump(&mut self.undeclared, class_name, || frame.id.to_string());
                continue;
            }
            if ctx.decode {
                self.copies
                    .entry(frame.id)
                    .or_default()
                    .push(decode_record(ctx, frame.id, partition, body));
            }
        }
    }
}

/// Why a global stream does not inflate: what the stored bytes begin with and,
/// at each gzip member offset the readers use, whether it has the magic and
/// inflates.
fn inflate_diagnosis(rf: &mut RevitFile, name: &str) -> String {
    let stored = match rf.read_stream(name) {
        Ok(stored) => stored,
        Err(error) => return format!("unreadable: {}", norm_msg(&error.to_string())),
    };
    let prepared = prepare_stream_for_inflate(name, &stored);
    let data: &[u8] = &prepared;
    let mut parts = vec![format!(
        "stored={} prepared={} head={}",
        stored.len(),
        data.len(),
        hex(&data[..data.len().min(16)])
    )];
    for offset in [0usize, 4, 8, 16] {
        if !has_gzip_magic(data, offset) {
            parts.push(format!("off{offset}=no_magic"));
            continue;
        }
        parts.push(match inflate_at(data, offset) {
            Ok(inflated) => format!("off{offset}=inflates_{}", inflated.len()),
            Err(error) => format!("off{offset}=fails_{}", norm_msg(&error.to_string())),
        });
    }
    parts.join(" ")
}

/// A global stream, one gzip member (`native_document::read_single`).
fn global(rf: &mut RevitFile, out: &Out, name: &str) -> Option<Vec<u8>> {
    match native_document::read_single(rf, name) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            let diagnosis = inflate_diagnosis(rf, name);
            out.line(format!(
                "preamble {name} read failed: {}; {diagnosis}",
                norm_msg(&format!("{error:#}"))
            ));
            None
        }
    }
}

fn step<T>(
    out: &Out,
    name: &str,
    result: Result<T>,
    describe: impl FnOnce(&T) -> String,
) -> Option<T> {
    match result {
        Ok(value) => {
            out.line(format!("preamble {name} ok: {}", describe(&value)));
            Some(value)
        }
        Err(error) => {
            out.line(format!(
                "preamble {name} parse failed: {}",
                norm_msg(&format!("{error:#}"))
            ));
            None
        }
    }
}

/// The five global streams the native readers need, each read on its own.
struct Pre {
    registry: Registry,
    index: Option<native_index::ProjectIndex>,
    es: Option<Catalog>,
    complete: bool,
}

fn preamble(rf: &mut RevitFile, out: &Out) -> Option<Pre> {
    let bytes = global(rf, out, "Formats/Latest")?;
    let registry = match schema_registry::parse(&bytes) {
        Ok(registry) => registry,
        Err(error) => {
            out.line(format!(
                "preamble Formats/Latest parse failed: {}",
                norm_msg(&format!("{error:#}"))
            ));
            return None;
        }
    };
    out.line(format!(
        "preamble Formats/Latest ok: bytes={} classes={} consumed={} terminator_at={} references={}",
        bytes.len(),
        registry.classes.len(),
        registry.consumed_bytes,
        registry.terminator_offset,
        registry.reference_count
    ));
    for name in SHAPES {
        match registry.named(name) {
            Some(class) => out.line(format!("shape {name} {}", describe_class(&registry, class))),
            None => out.line(format!("shape {name} absent")),
        }
    }
    for class in registry
        .classes
        .iter()
        .filter(|c| c.fields.iter().any(|f| f.name == "m_fieldTypeName"))
        .take(4)
    {
        out.line(format!(
            "shape {} {}",
            class.name,
            describe_class(&registry, class)
        ));
    }
    let record = registry
        .named("ElemTable")
        .and_then(|table| table.fields.first())
        .and_then(|field| field.references.first())
        .and_then(|reference| registry.class(reference.tag));
    if let Some(record) = record {
        out.line(format!(
            "shape {} {}",
            record.name,
            describe_class(&registry, record)
        ));
    }
    let episodes = global(rf, out, "Global/History").and_then(|bytes| {
        step(
            out,
            "Global/History",
            native_index::creation_episodes(&bytes, &registry),
            |episodes| format!("episodes={}", episodes.len()),
        )
    });
    let placeholder;
    let (given, label) = match &episodes {
        Some(episodes) => (episodes.as_slice(), "history episodes"),
        None => {
            placeholder = vec![String::new(); 100_000];
            (placeholder.as_slice(), "placeholder episodes")
        }
    };
    let index = global(rf, out, "Global/ElemTable").and_then(|bytes| {
        step(
            out,
            "Global/ElemTable",
            native_index::parse(&bytes, &registry, given),
            |index| {
                format!(
                    "identities={} graveyard={} stride={} {label}",
                    index.identities.len(),
                    index.graveyard_rows.len(),
                    index.row_stride
                )
            },
        )
    });
    let increments = global(rf, out, "Global/DocumentIncrementTable").and_then(|bytes| {
        step(
            out,
            "Global/DocumentIncrementTable",
            native_index::storage_increments(&bytes, &registry),
            |rows| {
                format!(
                    "rows={} stored_elements={} permuted_rows={}",
                    rows.len(),
                    rows.iter().map(|row| row.stored_elements).sum::<u64>(),
                    rows.iter()
                        .filter(|row| !row.episode_permutation.is_empty())
                        .count()
                )
            },
        )
    });
    let es = global(rf, out, "Global/Latest").and_then(|bytes| {
        step(
            out,
            "Global/Latest",
            native_es_catalog::decode(&bytes, &registry).map(|(catalog, _)| catalog),
            |catalog| format!("es_schemas={}", catalog.schemas.len()),
        )
    });
    let complete = episodes.is_some() && index.is_some() && increments.is_some() && es.is_some();
    out.line(format!("preamble complete={complete}"));
    Some(Pre {
        registry,
        index,
        es,
        complete,
    })
}

struct Analysis {
    saved: u32,
    layout: u32,
    pre: Option<Pre>,
    current: BTreeMap<u64, Rec>,
}

/// The copy of each element's record a reader would take: the one the
/// element's stored revision routes to where that holds a copy, else the
/// latest partition's.
fn current_records(
    rf: &mut RevitFile,
    copies: BTreeMap<u64, Vec<Rec>>,
) -> (BTreeMap<u64, Rec>, usize, usize) {
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (id, records) in &copies {
        if let Ok(small) = u32::try_from(*id) {
            holders
                .entry(small)
                .or_default()
                .extend(records.iter().map(|rec| rec.partition));
        }
    }
    let routed = native_document::current_partitions(rf, &holders);
    let mut current = BTreeMap::new();
    let (mut by_route, mut by_latest) = (0usize, 0usize);
    for (id, mut records) in copies {
        if records.is_empty() {
            continue;
        }
        let wanted = u32::try_from(id)
            .ok()
            .and_then(|small| routed.get(&small).copied());
        let at = match wanted.and_then(|p| records.iter().position(|rec| rec.partition == p)) {
            Some(at) => {
                by_route += 1;
                at
            }
            None => {
                by_latest += 1;
                records
                    .iter()
                    .enumerate()
                    .max_by_key(|(_, rec)| rec.partition)
                    .map_or(0, |(at, _)| at)
            }
        };
        current.insert(id, records.swap_remove(at));
    }
    (current, by_route, by_latest)
}

fn analyze(path: &str, out: &Out, decode: bool, compare: bool) -> Result<Analysis> {
    let mut rf = RevitFile::open(path)?;
    let saved = rf.basic_file_info()?.version;
    let layout = rf.layout_release().unwrap_or(saved);
    let name = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    out.line(format!(
        "file saved={saved} layout={layout} decode={decode}"
    ));
    out.detail(json!({
        "kind": "file",
        "side": out.tag.trim(),
        "file": name,
        "saved": saved,
        "layout": layout,
    }));
    let Some(pre) = preamble(&mut rf, out) else {
        return Ok(Analysis {
            saved,
            layout,
            pre: None,
            current: BTreeMap::new(),
        });
    };
    let legacy: Option<BTreeSet<u64>> = elem_table::declared_element_ids(&mut rf)
        .ok()
        .map(|ids| ids.into_iter().map(u64::from).collect());
    let declared: BTreeSet<u64> = match (&pre.index, &legacy) {
        (Some(index), other) => {
            let ids: BTreeSet<u64> = index.identities.keys().copied().collect();
            if let Some(other) = other {
                out.line(format!(
                    "declared ids: index={} legacy_parser={} equal={}",
                    ids.len(),
                    other.len(),
                    ids == *other
                ));
            }
            ids
        }
        (None, Some(other)) => {
            out.line(format!("declared ids: legacy_parser={}", other.len()));
            other.clone()
        }
        (None, None) => {
            out.line("declared ids: none read");
            BTreeSet::new()
        }
    };
    let mut names: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|n| n.starts_with("Partitions/"))
        .collect();
    names.sort();
    let defs = native_parameter_definitions::Registry::default();
    let ctx = Ctx {
        registry: &pre.registry,
        es: pre.es.as_ref(),
        defs: &defs,
        declared: &declared,
        expected_width: if layout <= 2023 { 4 } else { 8 },
        decode,
        compare,
    };
    let mut walk = Walk::default();
    let mut totals = native_segments::Statistics::default();
    let mut failures = Vec::new();
    for stream in &names {
        let Some(partition) = stream
            .strip_prefix("Partitions/")
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let stored = match rf.read_stream_with_limit(stream, MAX_STREAM_BYTES) {
            Ok(stored) => stored,
            Err(error) => {
                failures.push(format!(
                    "{stream} unreadable: {}",
                    norm_msg(&error.to_string())
                ));
                continue;
            }
        };
        let prepared = prepare_stream_for_inflate(stream, &stored);
        let inflated = rf.inflated_partition(stream).ok();
        let result = native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            ctx.registry,
            MAX_GROUP_BYTES,
            |source, bytes| {
                walk.group(&ctx, out, partition, source, bytes);
                Ok(())
            },
        );
        match result {
            Ok(stats) => {
                totals.segments += stats.segments;
                totals.groups += stats.groups;
                totals.content_sections += stats.content_sections;
                totals.inflated_bytes += stats.inflated_bytes;
                totals.opaque_terminal_bytes += stats.opaque_terminal_bytes;
            }
            Err(error) => failures.push(format!(
                "{stream} walk failed: {}",
                norm_msg(&format!("{error:#}"))
            )),
        }
    }
    out.line(format!(
        "walk partitions={} failed={} segments={} groups={} content_sections={} inflated_bytes={} opaque_terminal_bytes={}",
        names.len(),
        failures.len(),
        totals.segments,
        totals.groups,
        totals.content_sections,
        totals.inflated_bytes,
        totals.opaque_terminal_bytes
    ));
    for failure in failures.iter().take(TOP) {
        out.line(format!("walk {failure}"));
    }
    report_framing(out, &walk, ctx.expected_width);
    let declared_without_record = declared
        .iter()
        .filter(|id| !walk.seen.contains_key(id))
        .count();
    out.line(format!(
        "records plain channel 102: framed={} distinct_ids={} ids_with_copies={} declared_ids={} declared_without_record={} undeclared_records={} unresolved_classes={}",
        walk.seen.values().sum::<usize>(),
        walk.seen.len(),
        walk.seen.values().filter(|n| **n > 1).count(),
        declared.len(),
        declared_without_record,
        walk.undeclared.values().map(|c| c.n).sum::<usize>(),
        walk.unresolved
    ));
    print_top(out, "classes", &walk.classes);
    print_top(out, "undeclared", &walk.undeclared);
    print_examples(out, "undeclared", &walk.undeclared);
    let copies = std::mem::take(&mut walk.copies);
    let (current, by_route, by_latest) = current_records(&mut rf, copies);
    if decode {
        report_records(out, &current, by_route, by_latest, pre.complete);
    }
    Ok(Analysis {
        saved,
        layout,
        pre: Some(pre),
        current,
    })
}

fn report_framing(out: &Out, walk: &Walk, expected_width: usize) {
    for ((keyed, channel), stats) in &walk.channels {
        let kind = if *keyed { "keyed" } else { "plain" };
        let mut layouts = Vec::new();
        for (layout, hits) in &stats.layouts {
            layouts.push(format!(
                "{}={}/{}/{}/{}",
                layout.name(),
                hits.framed,
                hits.declared,
                hits.counted,
                hits.trailer
            ));
        }
        let kinds: Vec<String> = stats
            .kinds
            .iter()
            .map(|(k, n)| format!("{k}={n}"))
            .collect();
        out.line(format!(
            "framing channel={channel} {kind} expected_id_width={expected_width} groups={} records={} bytes={} both_widths={} choice {} layouts framed/declared/count_agrees/body_plus_4_per_record_agrees {}",
            stats.groups,
            stats.records,
            stats.bytes,
            stats.both_widths,
            kinds.join(","),
            layouts.join(" ")
        ));
        if !stats.extras.is_empty() {
            let mut extras: Vec<_> = stats.extras.iter().collect();
            extras.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
            let shown: Vec<String> = extras
                .iter()
                .take(6)
                .map(|(word, n)| format!("{word:08x}={n}"))
                .collect();
            out.line(format!(
                "extra word channel={channel} {kind} distinct={} top {}",
                stats.extras.len(),
                shown.join(",")
            ));
        }
    }
}

#[derive(Default)]
struct Summary {
    records: usize,
    graph_ok: usize,
    id_agrees: usize,
    id_disagrees: usize,
    base_ok: usize,
    base_id_agrees: usize,
    contract_ok: usize,
    phased: usize,
    leveled: usize,
    contract_errors: Tally,
    metadata_ok: usize,
    parameter_records: usize,
    parameters: usize,
    graph_errors: Tally,
    graph_error_classes: Tally,
    metadata_errors: Tally,
}

fn report_records(
    out: &Out,
    current: &BTreeMap<u64, Rec>,
    by_route: usize,
    by_latest: usize,
    preamble_complete: bool,
) {
    let mut sum = Summary::default();
    for (id, rec) in current {
        sum.records += 1;
        match &rec.graph_error {
            None => {
                sum.graph_ok += 1;
                match rec.id_agrees {
                    Some(true) => sum.id_agrees += 1,
                    Some(false) => sum.id_disagrees += 1,
                    None => {}
                }
            }
            Some(message) => {
                bump(&mut sum.graph_errors, message.clone(), || {
                    format!(
                        "id {id} {}: {}",
                        rec.class,
                        rec.raw_graph_error.as_deref().unwrap_or_default()
                    )
                });
                bump(&mut sum.graph_error_classes, rec.class.clone(), || {
                    format!("id {id} {message}")
                });
            }
        }
        match &rec.contract_error {
            None => sum.contract_ok += 1,
            Some(message) => bump(&mut sum.contract_errors, message.clone(), || {
                format!("id {id} {}", rec.class)
            }),
        }
        if let Ok(base) = &rec.base {
            sum.base_ok += 1;
            if u64::try_from(base.id).ok() == Some(*id) {
                sum.base_id_agrees += 1;
            }
            sum.phased += usize::from(base.ids[4] != -1);
            sum.leveled += usize::from(base.ids[0] != -1);
        }
        if rec.graph_error.is_none() {
            match &rec.metadata_error {
                None => {
                    sum.metadata_ok += 1;
                    if !rec.params.is_empty() {
                        sum.parameter_records += 1;
                        sum.parameters += rec.params.len();
                    }
                }
                Some(message) => bump(&mut sum.metadata_errors, message.clone(), || {
                    format!("id {id} {}", rec.class)
                }),
            }
        }
    }
    out.line(format!(
        "current copies: records={} by_route={by_route} by_latest_partition={by_latest} routing_read={preamble_complete}",
        sum.records
    ));
    out.line(format!(
        "graph: ok={} refused={} owner_id_agrees={} owner_id_disagrees={}",
        sum.graph_ok,
        sum.records - sum.graph_ok,
        sum.id_agrees,
        sum.id_disagrees
    ));
    out.line(format!(
        "element base contract (native_element::decode_base): ok={} refused={}",
        sum.contract_ok,
        sum.records - sum.contract_ok
    ));
    out.line(format!(
        "element base from the graph: ok={} id_agrees={} created_phase_set={} associated_level_set={}",
        sum.base_ok, sum.base_id_agrees, sum.phased, sum.leveled
    ));
    out.line(format!(
        "saved parameters: metadata_ok={} metadata_refused={} records_with_parameters={} parameters={}",
        sum.metadata_ok,
        sum.graph_ok - sum.metadata_ok,
        sum.parameter_records,
        sum.parameters
    ));
    print_top(out, "graph refused", &sum.graph_errors);
    print_top(out, "graph refused class", &sum.graph_error_classes);
    print_top(out, "element base contract refused", &sum.contract_errors);
    print_top(out, "saved parameters refused", &sum.metadata_errors);
    print_examples(out, "graph refused", &sum.graph_errors);
    print_examples(out, "element base contract refused", &sum.contract_errors);
    print_examples(out, "saved parameters refused", &sum.metadata_errors);
}

fn class_signature(registry: &Registry, class: &Class) -> String {
    let fields: Vec<String> = class
        .fields
        .iter()
        .map(|f| describe_field(registry, f))
        .collect();
    let parent = if class.parent_reference.tag >= 12 {
        registry
            .class(class.parent_reference.tag)
            .map_or("?", |c| c.name.as_str())
    } else {
        "none"
    };
    format!("{parent}|{}", fields.join(","))
}

fn compare_schemas(out: &Out, old: &Registry, new: &Registry) {
    let by_name: HashMap<&str, &Class> = new.classes.iter().map(|c| (c.name.as_str(), c)).collect();
    let (mut same, mut changed, mut only_old) = (0usize, Vec::new(), Vec::new());
    for class in &old.classes {
        match by_name.get(class.name.as_str()) {
            Some(other) => {
                if class_signature(old, class) == class_signature(new, other) {
                    same += 1;
                } else {
                    changed.push(class.name.clone());
                }
            }
            None => only_old.push(class.name.clone()),
        }
    }
    let old_names: BTreeSet<&str> = old.classes.iter().map(|c| c.name.as_str()).collect();
    let only_new: Vec<&str> = new
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .filter(|name| !old_names.contains(name))
        .collect();
    out.line(format!(
        "schema classes old={} new={} same_fields={same} changed_fields={} only_old={} only_new={}",
        old.classes.len(),
        new.classes.len(),
        changed.len(),
        only_old.len(),
        only_new.len()
    ));
    for name in SHAPES {
        if let (Some(a), Some(b)) = (old.named(name), new.named(name)) {
            out.line(format!(
                "schema {name} same_shape={}",
                class_signature(old, a) == class_signature(new, b)
            ));
        }
    }
    changed.sort();
    out.line(format!(
        "schema changed classes {}",
        changed
            .iter()
            .take(60)
            .cloned()
            .collect::<Vec<_>>()
            .join(",")
    ));
}

#[derive(Default)]
struct Pair {
    both: usize,
    only_old: usize,
    only_new: usize,
    class_equal: usize,
    class_differs: Tally,
    both_decoded: usize,
    old_refused: usize,
    new_refused: usize,
    both_refused: usize,
    shape_differs: usize,
    identical: usize,
    differing: usize,
    agree: usize,
    differ: usize,
    field_only_old: usize,
    field_only_new: usize,
    differ_fields: Tally,
    only_old_fields: Tally,
    only_new_fields: Tally,
    base_compared: usize,
    base_equal: usize,
    base_fields: Tally,
    param_records: usize,
    param_identical: usize,
    param_agree: usize,
    param_differ: usize,
    param_only_old: usize,
    param_only_new: usize,
    differ_params: Tally,
}

fn compare_objects(pair: &mut Pair, id: u64, old: &Rec, new: &Rec) {
    let same_shape = old.objects.len() == new.objects.len()
        && old
            .objects
            .iter()
            .zip(&new.objects)
            .all(|(a, b)| a.class == b.class);
    if !same_shape {
        pair.shape_differs += 1;
    }
    let compared = if same_shape { old.objects.len() } else { 1 };
    let mut identical = same_shape;
    for (a, b) in old.objects.iter().zip(&new.objects).take(compared) {
        for (name, print) in &a.fields {
            match b.fields.get(name) {
                Some(other) if other.hash == print.hash => pair.agree += 1,
                Some(other) => {
                    identical = false;
                    pair.differ += 1;
                    bump(
                        &mut pair.differ_fields,
                        format!("{}.{name}", a.class),
                        || format!("id {id} old {} new {}", print.text, other.text),
                    );
                }
                None => {
                    identical = false;
                    pair.field_only_old += 1;
                    bump(
                        &mut pair.only_old_fields,
                        format!("{}.{name}", a.class),
                        || format!("id {id} old {}", print.text),
                    );
                }
            }
        }
        for (name, print) in &b.fields {
            if !a.fields.contains_key(name) {
                identical = false;
                pair.field_only_new += 1;
                bump(
                    &mut pair.only_new_fields,
                    format!("{}.{name}", b.class),
                    || format!("id {id} new {}", print.text),
                );
            }
        }
    }
    if identical {
        pair.identical += 1;
    } else {
        pair.differing += 1;
    }
}

/// A record's parameters by id and storage type: the value's hash and text.
type ParamMap = BTreeMap<(i64, String), (u64, String)>;

fn compare_parameters(pair: &mut Pair, id: u64, old: &Rec, new: &Rec) {
    if old.params.is_empty() && new.params.is_empty() {
        return;
    }
    pair.param_records += 1;
    let keyed = |params: &[Param]| -> ParamMap {
        let mut map = BTreeMap::new();
        for p in params {
            map.entry((p.id, p.storage.clone()))
                .or_insert((p.hash, p.text.clone()));
        }
        map
    };
    let (a, b) = (keyed(&old.params), keyed(&new.params));
    let mut identical = true;
    for (key, (hash, text)) in &a {
        match b.get(key) {
            Some((other, _)) if other == hash => pair.param_agree += 1,
            Some((_, other_text)) => {
                identical = false;
                pair.param_differ += 1;
                bump(
                    &mut pair.differ_params,
                    format!("{} {}", key.0, key.1),
                    || format!("id {id} old {text} new {other_text}"),
                );
            }
            None => {
                identical = false;
                pair.param_only_old += 1;
            }
        }
    }
    for key in b.keys() {
        if !a.contains_key(key) {
            identical = false;
            pair.param_only_new += 1;
        }
    }
    if identical {
        pair.param_identical += 1;
    }
}

fn compare(out: &Out, old: &Analysis, new: &Analysis) {
    out.line(format!(
        "compare old saved={} layout={} new saved={} layout={}",
        old.saved, old.layout, new.saved, new.layout
    ));
    let (Some(old_pre), Some(new_pre)) = (&old.pre, &new.pre) else {
        out.line("compare skipped: a schema did not parse");
        return;
    };
    compare_schemas(out, &old_pre.registry, &new_pre.registry);
    if let (Some(a), Some(b)) = (&old_pre.index, &new_pre.index) {
        let (mut both, mut unique, mut original, mut episode, mut owner, mut partition) =
            (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        for (id, x) in &a.identities {
            let Some(y) = b.identities.get(id) else {
                continue;
            };
            both += 1;
            unique += usize::from(x.unique_id == y.unique_id);
            original += usize::from(x.original_id_suffix == y.original_id_suffix);
            episode += usize::from(x.creation_episode == y.creation_episode);
            owner += usize::from(x.owning_element_id == y.owning_element_id);
            partition += usize::from(x.partition_id == y.partition_id);
        }
        out.line(format!(
            "index: old={} new={} both={both} unique_id_equal={unique} original_suffix_equal={original} creation_episode_equal={episode} owner_equal={owner} partition_id_equal={partition}",
            a.identities.len(),
            b.identities.len()
        ));
    }
    let mut pair = Pair::default();
    for (id, a) in &old.current {
        let Some(b) = new.current.get(id) else {
            pair.only_old += 1;
            continue;
        };
        pair.both += 1;
        if a.class != b.class {
            bump(
                &mut pair.class_differs,
                format!("{}->{}", a.class, b.class),
                || format!("id {id}"),
            );
            continue;
        }
        pair.class_equal += 1;
        if let (Ok(x), Ok(y)) = (&a.base, &b.base) {
            pair.base_compared += 1;
            if x == y {
                pair.base_equal += 1;
            } else {
                let (vx, vy) = (x.values(), y.values());
                for (at, name) in BASE_NAMES.iter().enumerate() {
                    if vx[at] != vy[at] {
                        bump(&mut pair.base_fields, (*name).to_string(), || {
                            format!("id {id} old {} new {}", vx[at], vy[at])
                        });
                    }
                }
            }
        }
        match (&a.graph_error, &b.graph_error) {
            (None, None) => {
                pair.both_decoded += 1;
                compare_objects(&mut pair, *id, a, b);
                compare_parameters(&mut pair, *id, a, b);
            }
            (Some(_), None) => pair.old_refused += 1,
            (None, Some(_)) => pair.new_refused += 1,
            (Some(_), Some(_)) => pair.both_refused += 1,
        }
    }
    pair.only_new = new
        .current
        .keys()
        .filter(|id| !old.current.contains_key(id))
        .count();
    out.line(format!(
        "elements: both={} only_old={} only_new={} class_equal={} class_differs={}",
        pair.both,
        pair.only_old,
        pair.only_new,
        pair.class_equal,
        pair.both - pair.class_equal
    ));
    out.line(format!(
        "graphs: both_decoded={} old_refused_only={} new_refused_only={} both_refused={} object_lists_differ={} records_identical={} records_differing={}",
        pair.both_decoded,
        pair.old_refused,
        pair.new_refused,
        pair.both_refused,
        pair.shape_differs,
        pair.identical,
        pair.differing
    ));
    out.line(format!(
        "fields: agree={} differ={} only_old={} only_new={}",
        pair.agree, pair.differ, pair.field_only_old, pair.field_only_new
    ));
    out.line(format!(
        "element base: compared={} equal={} differ={}",
        pair.base_compared,
        pair.base_equal,
        pair.base_compared - pair.base_equal
    ));
    out.line(format!(
        "parameters by id: records={} identical={} values_agree={} differ={} only_old={} only_new={}",
        pair.param_records,
        pair.param_identical,
        pair.param_agree,
        pair.param_differ,
        pair.param_only_old,
        pair.param_only_new
    ));
    print_top(out, "class differs", &pair.class_differs);
    print_top(out, "base field differs", &pair.base_fields);
    print_top(out, "field differs", &pair.differ_fields);
    print_top(out, "field only old", &pair.only_old_fields);
    print_top(out, "field only new", &pair.only_new_fields);
    print_top(out, "parameter differs", &pair.differ_params);
    print_examples(out, "class differs", &pair.class_differs);
    print_examples(out, "base field differs", &pair.base_fields);
    print_examples(out, "field differs", &pair.differ_fields);
    print_examples(out, "field only old", &pair.only_old_fields);
    print_examples(out, "field only new", &pair.only_new_fields);
    print_examples(out, "parameter differs", &pair.differ_params);
}

fn guarded(label: &str, body: impl FnOnce() -> Result<()>) {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(Ok(())) => {}
        Ok(Err(error)) => println!(
            "native framing: {label} failed: {}",
            scrub(&norm_msg(&format!("{error:#}")))
        ),
        Err(_) => println!("native framing: {label} panicked"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let light = args.iter().any(|arg| arg == "--records");
    let paths: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();
    match paths.as_slice() {
        [one] => guarded("probe", || {
            analyze(one, &Out { tag: "" }, !light, false).map(|_| ())
        }),
        [first, second] => guarded("probe", || {
            let a = analyze(first, &Out { tag: "first " }, true, true)?;
            let b = analyze(second, &Out { tag: "second " }, true, true)?;
            let (old, new) = if a.saved <= b.saved {
                (&a, &b)
            } else {
                (&b, &a)
            };
            compare(&Out { tag: "pair " }, old, new);
            Ok(())
        }),
        _ => {
            eprintln!(
                "usage: probe_re280_native_framing_pre2023 FILE.rvt [--records] | OLDER.rvt NEWER.rvt"
            );
            std::process::exit(2);
        }
    }
}
