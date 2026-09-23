//! RE-36: score every placed-instance element record against a VIM export
//! of the same model.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 structural sample, every
//! ElementId the record decode yields for a placed instance (the `+0x00`
//! id, or its enclosing partition record's, RE-35) that the VIM
//! export carries has the `BuiltInCategory` the record carries. None lands
//! on an element of another category.
//!
//! A VIM file (`vimaec/vim-format`, MIT) is a BFAST container. Its
//! `Vim.Element` table lists each element's Revit ElementId (`long:Id`),
//! its category and its source document, so it is an oracle for models
//! that have no Revit IFC export. A VIM made from a later edition of the
//! sample leaves out elements that edition deleted, so an id "absent from
//! the VIM" is not by itself a wrong pick; the probe also reports how many
//! VIM ids this file does not declare, which measures that drift.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re36_vim_oracle -- \
//!     MODEL.rvt MODEL.vim "Document Title"

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE, PLACEMENT_KIND_OFFSET, bbox_marker,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Named buffers of the BFAST container starting at `base`.
fn bfast(buf: &[u8], base: usize) -> Option<BTreeMap<String, (usize, usize)>> {
    let count = usize::try_from(u64_at(buf, base + 24)?).ok()?;
    let range = |index: usize| -> Option<(usize, usize)> {
        let begin = usize::try_from(u64_at(buf, base + 32 + 16 * index)?).ok()?;
        let end = usize::try_from(u64_at(buf, base + 40 + 16 * index)?).ok()?;
        Some((base + begin, base + end))
    };
    let (begin, end) = range(0)?;
    let names: Vec<&[u8]> = buf.get(begin..end)?.split(|&c| c == 0).collect();
    let mut out = BTreeMap::new();
    for index in 1..count {
        let name = String::from_utf8_lossy(names.get(index - 1)?).into_owned();
        out.insert(name, range(index)?);
    }
    Some(out)
}

struct Vim {
    data: Vec<u8>,
    entities: BTreeMap<String, (usize, usize)>,
    strings: Vec<String>,
}

impl Vim {
    fn open(path: &PathBuf) -> Option<Self> {
        let data = std::fs::read(path).ok()?;
        let top = bfast(&data, 0)?;
        let entities = bfast(&data, top.get("entities")?.0)?;
        let (begin, end) = *top.get("strings")?;
        let strings = data
            .get(begin..end)?
            .split(|&c| c == 0)
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect();
        Some(Vim {
            data,
            entities,
            strings,
        })
    }

    fn raw(&self, table: &str, column: &str) -> Option<&[u8]> {
        let columns = bfast(&self.data, self.entities.get(table)?.0)?;
        let (begin, end) = *columns.get(column)?;
        self.data.get(begin..end)
    }

    fn i64s(&self, table: &str, column: &str) -> Option<Vec<i64>> {
        Some(
            self.raw(table, column)?
                .chunks_exact(8)
                .map(|c| i64::from_le_bytes(c.try_into().expect("8 bytes")))
                .collect(),
        )
    }

    fn i32s(&self, table: &str, column: &str) -> Option<Vec<i32>> {
        Some(
            self.raw(table, column)?
                .chunks_exact(4)
                .map(|c| i32::from_le_bytes(c.try_into().expect("4 bytes")))
                .collect(),
        )
    }

    fn strings(&self, table: &str, column: &str) -> Option<Vec<String>> {
        Some(
            self.i32s(table, column)?
                .into_iter()
                .map(|i| {
                    usize::try_from(i)
                        .ok()
                        .and_then(|i| self.strings.get(i))
                        .cloned()
                        .unwrap_or_default()
                })
                .collect(),
        )
    }
}

#[derive(Default)]
struct Score {
    frames: usize,
    first: usize,
    inferred: usize,
    unassigned: usize,
    same_category: usize,
    other_category: BTreeMap<String, usize>,
    absent: usize,
}

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: MODEL.rvt MODEL.vim \"Document Title\"";
    let rvt_path = PathBuf::from(args.next().expect(usage));
    let vim_path = PathBuf::from(args.next().expect(usage));
    let title = args.next().expect(usage);

    let vim = Vim::open(&vim_path).expect("readable VIM file");
    let titles = vim
        .strings("Vim.BimDocument", "string:Title")
        .expect("Vim.BimDocument titles");
    let Some(document) = titles.iter().position(|t| *t == title) else {
        println!("no document titled {title:?}; documents: {titles:?}");
        return Ok(());
    };
    let ids = vim.i64s("Vim.Element", "long:Id").expect("element ids");
    let documents = vim
        .i32s("Vim.Element", "index:Vim.BimDocument:BimDocument")
        .expect("element documents");
    let categories = vim
        .i32s("Vim.Element", "index:Vim.Category:Category")
        .expect("element categories");
    let category_names = vim
        .strings("Vim.Category", "string:BuiltInCategory")
        .expect("category names");
    let mut oracle: BTreeMap<u32, String> = BTreeMap::new();
    for ((&id, &doc), &category) in ids.iter().zip(&documents).zip(&categories) {
        if usize::try_from(doc).ok() != Some(document) {
            continue;
        }
        let Ok(id) = u32::try_from(id) else { continue };
        let name = usize::try_from(category)
            .ok()
            .and_then(|c| category_names.get(c))
            .cloned()
            .unwrap_or_default();
        oracle.insert(id, name);
    }

    let mut rf = RevitFile::open(&rvt_path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version) else {
        println!("release {version}: no known bbox marker");
        return Ok(());
    };
    let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();
    let undeclared = oracle.keys().filter(|id| !declared.contains(id)).count();
    println!(
        "release {version}; VIM document {title:?}: {} elements, {undeclared} not declared in this file",
        oracle.len()
    );

    let inferred_ids = rf.second_prologue_ids();
    let mut score: BTreeMap<i64, Score> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let inferred = inferred_ids.get(&stream);
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let Some(category) = u64_at(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
                continue;
            };
            if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&category) {
                continue;
            }
            let placed = u64_at(buf, offset + CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && u64_at(buf, offset + PLACEMENT_KIND_OFFSET)
                    .is_some_and(|v| (v & 0xffff_ffff) as u32 == PLACEMENT_KIND_INSTANCE);
            if !placed {
                continue;
            }
            let Some(head) = u64_at(buf, offset) else {
                continue;
            };
            let first = u32::try_from(head).ok().filter(|id| declared.contains(id));
            let second = head == 0 || head > u64::from(u32::MAX);
            if first.is_none() && !second {
                continue;
            }
            let entry = score.entry(category).or_default();
            entry.frames += 1;
            let id = match first {
                Some(id) => {
                    entry.first += 1;
                    id
                }
                None => match inferred.and_then(|m| m.get(&offset)) {
                    Some(&id) => {
                        entry.inferred += 1;
                        id
                    }
                    None => {
                        entry.unassigned += 1;
                        continue;
                    }
                },
            };
            match oracle.get(&id) {
                None => entry.absent += 1,
                Some(name) if same_category(category, name) => entry.same_category += 1,
                Some(name) => *entry.other_category.entry(name.clone()).or_default() += 1,
            }
        }
    }

    println!(
        "{:>9} {:<32} {:>6} {:>6} {:>8} {:>10} {:>6} {:>7} {:>6}",
        "category",
        "VIM name",
        "frames",
        "first",
        "inferred",
        "unassigned",
        "same",
        "absent",
        "other"
    );
    for (category, s) in &score {
        let other: usize = s.other_category.values().sum();
        println!(
            "{category:>9} {:<32} {:>6} {:>6} {:>8} {:>10} {:>6} {:>7} {:>6}{}",
            vim_name(*category),
            s.frames,
            s.first,
            s.inferred,
            s.unassigned,
            s.same_category,
            s.absent,
            other,
            if other > 0 {
                format!("  {:?}", s.other_category)
            } else {
                String::new()
            }
        );
    }
    let (same, other, absent) = score.values().fold((0, 0, 0), |(a, b, c), s| {
        (
            a + s.same_category,
            b + s.other_category.values().sum::<usize>(),
            c + s.absent,
        )
    });
    println!("total: {same} same category, {other} other category, {absent} absent from the VIM");
    Ok(())
}

/// `BuiltInCategory` names for the ids the probe reports, from the public
/// enum (the same names `vim-format` writes).
fn vim_name(category: i64) -> &'static str {
    match category {
        -2_000_011 => "OST_Walls",
        -2_000_014 => "OST_Windows",
        -2_000_023 => "OST_Doors",
        -2_000_032 => "OST_Floors",
        -2_000_038 => "OST_Ceilings",
        -2_000_045 => "OST_SketchLines",
        -2_000_051 => "OST_Lines",
        -2_000_079 => "OST_AreaSchemeLines",
        -2_000_080 => "OST_Furniture",
        -2_000_100 => "OST_Columns",
        -2_000_126 => "OST_StairsRailing",
        -2_000_151 => "OST_GenericModel",
        -2_000_160 => "OST_Rooms",
        -2_000_170 => "OST_CurtainWallPanels",
        -2_000_171 => "OST_CurtainWallMullions",
        -2_000_181 => "OST_Cornices",
        -2_000_831 => "OST_MEPSpaceSeparationLines",
        -2_000_977 => "OST_CoordinateSystem",
        -2_000_996 => "OST_ShaftOpening",
        -2_001_000 => "OST_Casework",
        -2_001_160 => "OST_PlumbingFixtures",
        -2_001_263 => "OST_BuildingPad",
        -2_001_271 => "OST_ProjectBasePoint",
        -2_001_272 => "OST_SharedBasePoint",
        -2_001_300 => "OST_StructuralFoundation",
        -2_001_320 => "OST_StructuralFraming",
        -2_001_327 => "OST_StructuralFramingSystem",
        -2_001_330 => "OST_StructuralColumns",
        -2_001_350 => "OST_SpecialityEquipment",
        -2_002_000 => "OST_DetailComponents",
        -2_008_000 => "OST_DuctCurves",
        -2_008_010 => "OST_DuctFitting",
        -2_008_044 => "OST_PipeCurves",
        -2_008_049 => "OST_PipeFitting",
        -2_009_000 => "OST_Rebar",
        -2_009_030 => "OST_StructConnections",
        _ => "",
    }
}

/// Whether the VIM's category `name` is the record's `category`. For an id
/// with no name in [`vim_name`] every VIM category counts as "other", so
/// an unnamed category never scores as agreeing.
fn same_category(category: i64, name: &str) -> bool {
    let expected = vim_name(category);
    !expected.is_empty() && expected == name
}
