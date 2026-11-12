//! RE-325 (#1196): how many bytes do the schema's base-2 and base-3 scalars
//! occupy in an instance?
//!
//! FACT under test: a `Formats/Latest` field whose descriptor base byte is 2
//! occupies ONE byte in instance data (the C++ `char`) and one whose base byte
//! is 3 occupies TWO (`short`). `FieldType::decode` gave `Primitive { kind:
//! 0x02 }` a size of 2 and `Primitive { kind: 0x03 }` a size of 4, and the
//! native graph decoder reads them as 1 and 2.
//!
//! The evidence, in the order the probe prints it:
//!   1. the schema's own C++ type names: in every `std::pair< A, B >` class
//!      the `first` and `second` fields name their type, and the probe tallies
//!      which base byte each type name carries (`pair-slot`);
//!   2. the census of every base-2 and base-3 field of the grammar
//!      (`census`, `census-shape`), and of the old scan's reading of them
//!      (`scan-census`, `scan-field`, `scan-only`, `grammar-only`);
//!   3. the instances in complete native graphs (`field-width`): value
//!      ranges, and for byte containers the declared count against the
//!      payload (a PNG that starts with its signature and ends with its IEND
//!      chunk, a zip that starts with its local header);
//!   4. the counter-readings: each record that holds an instance is decoded
//!      again from its bytes with the schema's base bytes rewritten so the
//!      decoder reads another width, and the decodes that consume the whole
//!      body are counted. `hyp` rewrites every field at once, `field-alt`
//!      one field at a time (the control is the schema as it is).
//!
//! Output, one record per line:
//!   file <path> release <layout release> stream-bytes <Formats/Latest bytes>
//!   census <Class>::<field> base .. mod .. flags .. count .. nested .. offset ..
//!   census-shape base .. mod .. flags .. nested .. fields <n>
//!   pair-slot base <b> type <C++ type> fields <n>
//!   scan-census <label> fields <n> / scan-field <Class>::<field> <label>
//!   grammar-scalars <n> scan-scalars <n> grammar-only <n> scan-only <n> ...
//!   summary release .. indexed .. emitted .. complete .. unsupported ..
//!   native-records seen .. complete .. with-instances .. control-tested ..
//!     control-failed .. no-body .. failed-records-tested ..
//!   field-width <Class>::<field> base .. mod .. instances .. items .. records
//!     .. values .. blobs .. png .. png-end .. zip .. signatures ..
//!   field-alt <Class>::<field> base .. at <bytes> tested .. complete .. diag ..
//!   hyp char <bytes> short <bytes> tested .. complete .. failed-records-tested
//!     .. failed-records-complete .. diag ..
//!   unsupported <diagnostic> <records>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re325_scalar_widths -- FILE [--records]
use anyhow::Result;
use rvt::RevitFile;
use rvt::formats::FieldType;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Options};
use rvt::research::native_parameters;
use rvt::research::schema_registry::{self, Class, Field, Registry};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

const BUDGET: Duration = Duration::from_secs(14 * 60);
const MAX_GLOBAL_TESTS: usize = 60_000;
const MAX_FIELD_TESTS: usize = 2_000;
const MAX_FAILED_TESTS: usize = 6_000;

const PNG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const PNG_END: [u8; 8] = [0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82];
const ZIP: [u8; 4] = [0x50, 0x4b, 0x03, 0x04];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn normalise(text: &str) -> String {
    let mut out = String::new();
    let mut digits = false;
    for c in text.chars() {
        if c.is_ascii_digit() {
            if !digits {
                out.push('#');
            }
            digits = true;
        } else {
            digits = false;
            out.push(c);
        }
    }
    out.chars().take(140).collect()
}

fn top(map: &BTreeMap<String, usize>, count: usize) -> String {
    let mut ranked: Vec<_> = map.iter().collect();
    ranked.sort_by_key(|entry| std::cmp::Reverse(*entry.1));
    ranked
        .into_iter()
        .take(count)
        .map(|(key, n)| format!("{key}x{n}"))
        .collect::<Vec<_>>()
        .join(" ; ")
}

fn label(name: &str) -> &str {
    if name.is_empty() { "-" } else { name }
}

fn lineage(registry: &Registry, tag: u16) -> Vec<&Class> {
    let mut out = Vec::new();
    let mut current = tag;
    for _ in 0..256 {
        let Some(class) = registry.class(current) else {
            break;
        };
        current = class.parent_reference.tag;
        out.push(class);
        if current < 12 {
            break;
        }
    }
    out
}

fn pair_args(name: &str) -> Option<(String, String)> {
    let inner = name.strip_prefix("std::pair<")?.strip_suffix('>')?;
    let mut depth = 0i32;
    for (at, c) in inner.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                return Some((
                    inner[..at].trim().to_string(),
                    inner[at + 1..].trim().to_string(),
                ));
            }
            _ => {}
        }
    }
    None
}

fn census(registry: &Registry) {
    let mut shapes: BTreeMap<(u8, u8, u16, bool), usize> = BTreeMap::new();
    let mut slots: BTreeMap<(u8, String), usize> = BTreeMap::new();
    for class in &registry.classes {
        for field in &class.fields {
            let mut current = Some(field);
            let mut nested = false;
            while let Some(f) = current {
                if matches!(f.base, 2 | 3) {
                    *shapes
                        .entry((f.base, f.modifier, f.uninterpreted_flags, nested))
                        .or_default() += 1;
                    println!(
                        "census {}::{} base {} mod {:02x} flags {:04x} count {:?} nested {nested} offset {}",
                        class.name,
                        label(&f.name),
                        f.base,
                        f.modifier,
                        f.uninterpreted_flags,
                        f.array_count,
                        f.offset
                    );
                }
                current = f.nested_descriptor.as_deref();
                nested = true;
            }
        }
        if let Some((first, second)) = pair_args(&class.name) {
            for field in &class.fields {
                let slot = match field.name.as_str() {
                    "first" => &first,
                    "second" => &second,
                    _ => continue,
                };
                if field.modifier == 0 && !matches!(field.base, 13 | 14) {
                    *slots.entry((field.base, slot.clone())).or_default() += 1;
                }
            }
        }
    }
    for ((base, modifier, flags, nested), n) in &shapes {
        println!(
            "census-shape base {base} mod {modifier:02x} flags {flags:04x} nested {nested} fields {n}"
        );
    }
    for ((base, slot), n) in &slots {
        println!("pair-slot base {base} type {slot} fields {n}");
    }
}

fn scan_census(rf: &mut RevitFile, registry: &Registry) {
    let table = match rf.schema() {
        Ok(table) => table,
        Err(error) => {
            println!("scan: {error}");
            return;
        }
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut scan_scalars: BTreeSet<(String, String)> = BTreeSet::new();
    for class in &table.classes {
        for field in &class.fields {
            let found = match &field.field_type {
                Some(FieldType::Primitive { kind, size }) if matches!(*kind, 2 | 3) => {
                    scan_scalars.insert((class.name.clone(), field.name.clone()));
                    format!("primitive-{kind}-size-{size}")
                }
                Some(FieldType::Vector { kind, .. }) if matches!(*kind, 2 | 3) => {
                    format!("vector-{kind}")
                }
                Some(FieldType::Container { kind, .. }) if matches!(*kind, 2 | 3) => {
                    format!("container-{kind}")
                }
                _ => continue,
            };
            println!("scan-field {}::{} {found}", class.name, field.name);
            *counts.entry(found).or_default() += 1;
        }
    }
    for (found, n) in &counts {
        println!("scan-census {found} fields {n}");
    }
    let mut grammar_scalars: BTreeSet<(String, String)> = BTreeSet::new();
    for class in &registry.classes {
        for field in &class.fields {
            if matches!(field.base, 2 | 3) && field.modifier == 0 && !field.name.is_empty() {
                grammar_scalars.insert((class.name.clone(), field.name.clone()));
            }
        }
    }
    let grammar_only: Vec<_> = grammar_scalars.difference(&scan_scalars).collect();
    let scan_only: Vec<_> = scan_scalars.difference(&grammar_scalars).collect();
    println!(
        "grammar-scalars {} scan-scalars {} grammar-only {} scan-only {}",
        grammar_scalars.len(),
        scan_scalars.len(),
        grammar_only.len(),
        scan_only.len()
    );
    for (class, field) in grammar_only.iter().take(40) {
        println!("grammar-only {class}::{field}");
    }
    for (class, field) in scan_only.iter().take(40) {
        println!("scan-only {class}::{field}");
    }
}

#[derive(Default)]
struct Stat {
    name: String,
    base: u8,
    modifier: u8,
    instances: usize,
    items: usize,
    records: BTreeSet<u64>,
    low: Option<i64>,
    high: Option<i64>,
    values: BTreeMap<String, usize>,
    signatures: BTreeMap<String, usize>,
    blobs: usize,
    png: usize,
    png_end: usize,
    zip: usize,
    non_numeric: usize,
}

struct Walk<'a> {
    registry: &'a Registry,
    fields: BTreeMap<usize, Stat>,
    record: u64,
    touched: BTreeSet<usize>,
}

impl Walk<'_> {
    fn begin(&mut self, record: u64) {
        self.record = record;
        self.touched.clear();
    }

    fn note(&mut self, owner: &str, field: &Field, value: &Fv) {
        let is_array = matches!(value, Fv::Array(_));
        let items: Vec<&Fv> = match value {
            Fv::Array(items) => items.iter().collect(),
            other => vec![other],
        };
        self.touched.insert(field.offset);
        let record = self.record;
        let stat = self.fields.entry(field.offset).or_insert_with(|| Stat {
            name: format!("{owner}::{}", label(&field.name)),
            base: field.base,
            modifier: field.modifier,
            ..Stat::default()
        });
        stat.instances += 1;
        stat.items += items.len();
        stat.records.insert(record);
        let mut bytes: Vec<u8> = Vec::new();
        for item in &items {
            let Some(number) = item.as_i64() else {
                stat.non_numeric += 1;
                continue;
            };
            stat.low = Some(stat.low.map_or(number, |low| low.min(number)));
            stat.high = Some(stat.high.map_or(number, |high| high.max(number)));
            if is_array && field.base == 2 {
                if let Ok(byte) = u8::try_from(number) {
                    bytes.push(byte);
                }
            }
            if !is_array || items.len() <= 16 {
                let key = number.to_string();
                if stat.values.len() < 40 || stat.values.contains_key(&key) {
                    *stat.values.entry(key).or_default() += 1;
                }
            }
        }
        if is_array && field.base == 2 && bytes.len() == items.len() {
            stat.blobs += 1;
            if bytes.len() >= 4 {
                let key = hex(&bytes[..4]);
                if stat.signatures.len() < 12 || stat.signatures.contains_key(&key) {
                    *stat.signatures.entry(key).or_default() += 1;
                }
            }
            stat.png += usize::from(bytes.starts_with(&PNG));
            stat.png_end += usize::from(bytes.starts_with(&PNG) && bytes.ends_with(&PNG_END));
            stat.zip += usize::from(bytes.starts_with(&ZIP));
        }
    }

    fn class(&mut self, tag: u16, value: &Fv, depth: usize) {
        if depth > 64 {
            return;
        }
        let registry = self.registry;
        let chain = lineage(registry, tag);
        let mut names: BTreeMap<&str, usize> = BTreeMap::new();
        for class in &chain {
            for field in &class.fields {
                if field.uninterpreted_flags & 2 == 0 {
                    *names.entry(field.name.as_str()).or_default() += 1;
                }
            }
        }
        for class in chain.iter().rev() {
            for field in &class.fields {
                if field.uninterpreted_flags & 2 != 0 {
                    continue;
                }
                let key = if names[field.name.as_str()] > 1 {
                    format!("{}::{}", class.name, field.name)
                } else {
                    field.name.clone()
                };
                let Some(found) = value.get(&key) else {
                    continue;
                };
                self.field(&class.name, field, found, depth + 1);
            }
        }
    }

    fn field(&mut self, owner: &str, field: &Field, value: &Fv, depth: usize) {
        if depth > 96 {
            return;
        }
        if matches!(field.base, 2 | 3) {
            self.note(owner, field, value);
            return;
        }
        match field.modifier {
            0 => match field.base {
                14 => {
                    if let Some(reference) = field.references.first() {
                        self.class(reference.tag, value, depth + 1);
                    }
                }
                13 => {
                    if let Some(nested) = &field.nested_descriptor {
                        self.field(owner, nested, value, depth + 1);
                    }
                }
                _ => {}
            },
            0x10..=0x13 | 0x50..=0x54 => {
                if let Fv::Array(items) = value {
                    let mut item = field.clone();
                    item.modifier &= 0xf;
                    item.array_count = None;
                    for found in items {
                        self.field(owner, &item, found, depth + 1);
                    }
                }
            }
            _ => {}
        }
    }
}

struct Loc {
    offset: usize,
    class: usize,
    field: usize,
    depth: usize,
    base: u8,
}

fn locate(registry: &Registry) -> Vec<Loc> {
    let mut out = Vec::new();
    for (class_at, class) in registry.classes.iter().enumerate() {
        for (field_at, field) in class.fields.iter().enumerate() {
            let mut current = Some(field);
            let mut depth = 0;
            while let Some(f) = current {
                if matches!(f.base, 2 | 3) {
                    out.push(Loc {
                        offset: f.offset,
                        class: class_at,
                        field: field_at,
                        depth,
                        base: f.base,
                    });
                }
                current = f.nested_descriptor.as_deref();
                depth += 1;
            }
        }
    }
    out
}

fn field_mut<'a>(registry: &'a mut Registry, loc: &Loc) -> &'a mut Field {
    let mut field = &mut registry.classes[loc.class].fields[loc.field];
    for _ in 0..loc.depth {
        field = field
            .nested_descriptor
            .as_deref_mut()
            .expect("a located nested descriptor");
    }
    field
}

fn base_for_width(width: u8) -> u8 {
    match width {
        1 => 2,
        2 => 3,
        _ => 4,
    }
}

struct Hyp {
    char_width: u8,
    short_width: u8,
    tested: usize,
    complete: usize,
    diag: BTreeMap<String, usize>,
    failed_complete: usize,
}

#[derive(Default)]
struct Alt {
    tested: usize,
    complete: usize,
    diag: BTreeMap<String, usize>,
}

struct Lab {
    scratch: Registry,
    locs: Vec<Loc>,
    hyps: Vec<Hyp>,
    alts: BTreeMap<(usize, u8), Alt>,
    control_tested: usize,
    control_failed: usize,
    failed_tested: usize,
}

impl Lab {
    fn new(registry: &Registry) -> Self {
        let hyps = [(1, 2), (2, 4), (2, 2), (1, 4), (4, 2), (1, 1)]
            .into_iter()
            .map(|(char_width, short_width)| Hyp {
                char_width,
                short_width,
                tested: 0,
                complete: 0,
                diag: BTreeMap::new(),
                failed_complete: 0,
            })
            .collect();
        Self {
            scratch: registry.clone(),
            locs: locate(registry),
            hyps,
            alts: BTreeMap::new(),
            control_tested: 0,
            control_failed: 0,
            failed_tested: 0,
        }
    }

    fn set(&mut self, at: usize, base: u8) {
        let loc = &self.locs[at];
        field_mut(&mut self.scratch, loc).base = base;
    }

    fn apply(&mut self, char_width: u8, short_width: u8) {
        for at in 0..self.locs.len() {
            let width = if self.locs[at].base == 2 {
                char_width
            } else {
                short_width
            };
            self.set(at, base_for_width(width));
        }
    }

    fn restore(&mut self) {
        for at in 0..self.locs.len() {
            let base = self.locs[at].base;
            self.set(at, base);
        }
    }

    fn decode(&self, body: &[u8]) -> std::result::Result<(), String> {
        native_parameters::decode_graph(body, &self.scratch)
            .map(|_| ())
            .map_err(|error| normalise(&error.to_string()))
    }

    fn test_record(&mut self, body: &[u8], touched: &BTreeSet<usize>) {
        self.control_tested += 1;
        if self.decode(body).is_err() {
            self.control_failed += 1;
            return;
        }
        for h in 0..self.hyps.len() {
            if self.hyps[h].tested >= MAX_GLOBAL_TESTS {
                continue;
            }
            let (char_width, short_width) = (self.hyps[h].char_width, self.hyps[h].short_width);
            self.apply(char_width, short_width);
            let result = self.decode(body);
            self.restore();
            let hyp = &mut self.hyps[h];
            hyp.tested += 1;
            match result {
                Ok(()) => hyp.complete += 1,
                Err(error) => *hyp.diag.entry(error).or_default() += 1,
            }
        }
        for &offset in touched {
            let Some(at) = self.locs.iter().position(|loc| loc.offset == offset) else {
                continue;
            };
            let original = self.locs[at].base;
            let widths: [u8; 2] = if original == 2 { [2, 4] } else { [1, 4] };
            for width in widths {
                let done = self.alts.get(&(offset, width)).map_or(0, |alt| alt.tested);
                if done >= MAX_FIELD_TESTS {
                    continue;
                }
                self.set(at, base_for_width(width));
                let result = self.decode(body);
                self.set(at, original);
                let alt = self.alts.entry((offset, width)).or_default();
                alt.tested += 1;
                match result {
                    Ok(()) => alt.complete += 1,
                    Err(error) => *alt.diag.entry(error).or_default() += 1,
                }
            }
        }
    }

    fn test_failed(&mut self, body: &[u8]) {
        if self.failed_tested >= MAX_FAILED_TESTS {
            return;
        }
        self.failed_tested += 1;
        for h in 0..self.hyps.len() {
            let (char_width, short_width) = (self.hyps[h].char_width, self.hyps[h].short_width);
            self.apply(char_width, short_width);
            let result = self.decode(body);
            self.restore();
            if result.is_ok() {
                self.hyps[h].failed_complete += 1;
            }
        }
    }
}

fn native(rf: &mut RevitFile, registry: &Registry, release: u32) {
    if !native_document::NATIVE_RELEASES.contains(&release) {
        println!("native: release {release} is outside the native readers' range");
        return;
    }
    let mut lab = Lab::new(registry);
    let mut walk = Walk {
        registry,
        fields: BTreeMap::new(),
        record: 0,
        touched: BTreeSet::new(),
    };
    let options = Options {
        releases: native_document::NATIVE_RELEASES,
        keep_body: true,
        ..Options::default()
    };
    let started = Instant::now();
    let mut seen = 0usize;
    let mut complete = 0usize;
    let mut with_instances = 0usize;
    let mut no_body = 0usize;
    let mut failures: BTreeMap<String, usize> = BTreeMap::new();
    let result = native_document::extract_graphs(rf, &options, |record| {
        seen += 1;
        if started.elapsed() > BUDGET {
            anyhow::bail!("time budget after {seen} records");
        }
        if let Some(graph) = &record.graph {
            complete += 1;
            walk.begin(record.identity.element_id);
            for object in &graph.objects {
                walk.class(object.class_tag, &object.fields, 0);
            }
            if !walk.touched.is_empty() {
                with_instances += 1;
                match &record.body {
                    Some(body) => lab.test_record(body, &walk.touched),
                    None => no_body += 1,
                }
            }
        } else {
            if let Some(diagnostic) = &record.diagnostic {
                *failures.entry(normalise(diagnostic)).or_default() += 1;
            }
            if let Some(body) = &record.body {
                lab.test_failed(body);
            }
        }
        Ok(())
    });
    match &result {
        Ok(summary) => println!(
            "summary release {} indexed {} emitted {} complete {} unsupported {} graveyard {}",
            summary.revit_version,
            summary.indexed_elements,
            summary.emitted_records,
            summary.complete_graph_records,
            summary.unsupported_graph_records,
            summary.graveyard_records,
        ),
        Err(error) => println!("native: {error:#} (after {seen} records)"),
    }
    println!(
        "native-records seen {seen} complete {complete} with-instances {with_instances} control-tested {} control-failed {} no-body {no_body} failed-records-tested {}",
        lab.control_tested, lab.control_failed, lab.failed_tested
    );
    let mut ranked: Vec<_> = failures.iter().collect();
    ranked.sort_by_key(|entry| std::cmp::Reverse(*entry.1));
    for (diagnostic, count) in ranked.into_iter().take(12) {
        println!("unsupported {diagnostic} {count}");
    }
    for stat in walk.fields.values() {
        println!(
            "field-width {} base {} mod {:02x} instances {} items {} records {} low {:?} high {:?} non-numeric {} values {} blobs {} png {} png-end {} zip {} signatures {}",
            stat.name,
            stat.base,
            stat.modifier,
            stat.instances,
            stat.items,
            stat.records.len(),
            stat.low,
            stat.high,
            stat.non_numeric,
            top(&stat.values, 8),
            stat.blobs,
            stat.png,
            stat.png_end,
            stat.zip,
            top(&stat.signatures, 6),
        );
    }
    for ((offset, width), alt) in &lab.alts {
        let (name, base) = walk
            .fields
            .get(offset)
            .map_or(("?", 0), |stat| (stat.name.as_str(), stat.base));
        println!(
            "field-alt {name} base {base} at {width} tested {} complete {} diag {}",
            alt.tested,
            alt.complete,
            top(&alt.diag, 3)
        );
    }
    for hyp in &lab.hyps {
        println!(
            "hyp char {} short {} tested {} complete {} failed-records-tested {} failed-records-complete {} diag {}",
            hyp.char_width,
            hyp.short_width,
            hyp.tested,
            hyp.complete,
            lab.failed_tested,
            hyp.failed_complete,
            top(&hyp.diag, 3)
        );
    }
}

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re325_scalar_widths FILE [--records]"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    let stream = native_document::read_single(&mut rf, "Formats/Latest")?;
    println!(
        "file {path} release {release} stream-bytes {}",
        stream.len()
    );
    let registry = schema_registry::parse(&stream)?;
    census(&registry);
    scan_census(&mut rf, &registry);
    native(&mut rf, &registry, release);
    Ok(())
}
