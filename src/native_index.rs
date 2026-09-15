//! Native project identities and creation-episode UniqueIds.
//! Explicit four-field (2023) and five-field (2024/2027) ElemTable contracts.
//! Original identity suffixes are retained; they are not current record locators.
use crate::{native_parameters, schema_registry::Registry};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Increment {
    pub greatest_episode: i64,
    pub total_episodes: u64,
    pub stored_elements: u64,
    /// Chronological (causing increment, remaining element count) versions.
    /// Only the terminal zero-count version redirects an empty increment.
    pub redirects: Vec<(i64, u64)>,
    /// Raw sparse episode pairs; direction/history semantics are not inferred.
    /// Nonempty pairs are accepted only under the single-increment routing invariant.
    pub episode_permutation: Vec<(i64, i64)>,
}
/// The increment ordinal is distinct from its greatest modification episode.
pub fn storage_increments(bytes: &[u8], registry: &Registry) -> Result<Vec<Increment>> {
    ensure!(bytes.len() >= 2, "truncated increment class");
    let tag = u16::from_le_bytes(bytes[..2].try_into()?);
    ensure!(
        registry
            .class(tag)
            .is_some_and(|c| c.name == "DocumentIncrementTable"),
        "unsupported increment root class"
    );
    let root = native_parameters::decode_object_fields(bytes, registry, 2, tag)?;
    ensure!(
        bytes.get(root.end..) == Some(&[0; 4]),
        "unsupported increment table terminal"
    );
    let mut increments = reconcile_increment_rows(
        &root.fields["m_increments"],
        &root.fields["m_localIncrements"],
    )?;
    qualify_single_increment_permutation(&mut increments, &root.fields["m_permutation"])?;
    Ok(increments)
}

/// This does not interpret the direction of a sparse episode permutation.
/// With one stored increment and no redirect, all in-range episodes have the
/// same physical route. More general tables remain explicitly unsupported.
fn qualify_single_increment_permutation(
    increments: &mut [Increment],
    value: &serde_json::Value,
) -> Result<()> {
    let pairs = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("increment permutation absent"))?;
    if pairs.is_empty() {
        return Ok(());
    }
    ensure!(
        increments.len() == 1,
        "multi-increment permutation routing is unsupported"
    );
    let row = &mut increments[0];
    ensure!(
        row.stored_elements > 0 && row.redirects.iter().all(|(target, _)| *target == -1),
        "permuted increment must be stored without redirects"
    );
    let mut keys = BTreeSet::new();
    let mut evidence = Vec::new();
    for pair in pairs {
        let source = pair["first"]["m_id"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("permutation source episode absent"))?;
        let target = pair["second"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("permutation target episode absent"))?;
        ensure!(
            source >= 0
                && source <= row.greatest_episode
                && target >= 0
                && target <= row.greatest_episode,
            "permutation endpoint outside sole increment episode range"
        );
        ensure!(keys.insert(source), "duplicate permutation source episode");
        evidence.push((source, target));
    }
    row.episode_permutation = evidence;
    Ok(())
}

fn reconcile_increment_rows(
    global: &serde_json::Value,
    local: &serde_json::Value,
) -> Result<Vec<Increment>> {
    // Global and local stream bookkeeping revisions can differ after a save.
    // Equality is required for every field that determines physical routing;
    // unrelated stream revision counters are not routing equivalence evidence.
    let increments = decode_increment_rows(global)?;
    let local_increments = decode_increment_rows(local)?;
    ensure!(
        increments == local_increments,
        "global/local increment routing tables differ"
    );
    Ok(increments)
}

fn decode_increment_rows(value: &serde_json::Value) -> Result<Vec<Increment>> {
    let rows = value
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("increment rows absent"))?;
    let mut result = Vec::new();
    let mut previous = -1;
    for row in rows {
        let greatest_episode = row["m_greatest"]["m_id"]
            .as_i64()
            .ok_or_else(|| anyhow::anyhow!("greatest episode absent"))?;
        let total_episodes = row["m_totalEpisodes"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("total episode count absent"))?;
        let stored_elements = row["m_totalElements"]
            .as_u64()
            .ok_or_else(|| anyhow::anyhow!("stored element count absent"))?;
        ensure!(
            greatest_episode >= previous
                && greatest_episode >= -1
                && total_episodes >= u64::try_from(greatest_episode + 1)?,
            "invalid increment episode range"
        );
        previous = greatest_episode;
        let mut redirects = Vec::new();
        for r in row["m_incrementVersions"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("increment versions absent"))?
        {
            redirects.push((
                r["m_causedByIncrementNumber"]
                    .as_i64()
                    .ok_or_else(|| anyhow::anyhow!("increment redirect absent"))?,
                r["m_totalElements"]
                    .as_u64()
                    .ok_or_else(|| anyhow::anyhow!("redirect element count absent"))?,
            ));
        }
        result.push(Increment {
            greatest_episode,
            total_episodes,
            stored_elements,
            redirects,
            episode_permutation: Vec::new(),
        });
    }
    Ok(result)
}
pub fn route_episode(
    episode: u32,
    increments: &[Increment],
    present: &BTreeSet<u32>,
) -> Result<u32> {
    let initial = increments
        .iter()
        .position(|r| r.greatest_episode >= i64::from(episode))
        .ok_or_else(|| anyhow::anyhow!("modification episode outside increment ranges"))?;
    let mut index = initial;
    for _ in 0..=128 {
        let row = increments
            .get(index)
            .ok_or_else(|| anyhow::anyhow!("increment redirect outside table"))?;
        let mut previous_cause = -2;
        let mut previous_count = u64::MAX;
        for &(cause, count) in &row.redirects {
            ensure!(
                cause >= -1
                    && cause > previous_cause
                    && (cause == -1
                        || (cause as usize > index && (cause as usize) < increments.len()))
                    && count <= previous_count,
                "invalid chronological increment version history"
            );
            previous_cause = cause;
            previous_count = count;
        }
        if let Some(&(_, terminal_count)) = row.redirects.last() {
            ensure!(
                terminal_count == row.stored_elements,
                "increment terminal version count differs from current storage"
            );
        }
        if row.stored_elements > 0 {
            ensure!(
                present.contains(&(index as u32)),
                "active physical increment absent"
            );
            return Ok(index as u32);
        }
        // Earlier nonzero counts describe historical residual populations. They
        // are not current routes. A compaction appends the new increment with
        // count zero; its ordinal is the terminal version's causing increment.
        let &(target, count) = row
            .redirects
            .last()
            .ok_or_else(|| anyhow::anyhow!("unresolved empty increment route"))?;
        ensure!(
            count == 0 && target >= 0 && target as usize > index,
            "unsupported terminal increment redirect relation"
        );
        index = target as usize;
    }
    anyhow::bail!("increment redirect recursion budget exceeded")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub element_id: u64,
    pub original_id_suffix: u64,
    pub creation_episode: u32,
    pub stored_revision: u32,
    pub other_revision: u32,
    pub row_offset: usize,
    pub raw_fields: Vec<u32>,
    pub owning_element_id: i64,
    pub partition_id: i64,
    pub unique_id: String,
}
fn guid_string(b: &[u8; 16]) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes(b[..4].try_into().unwrap()),
        u16::from_le_bytes(b[4..6].try_into().unwrap()),
        u16::from_le_bytes(b[6..8].try_into().unwrap()),
        b[8],
        b[9],
        b[10],
        b[11],
        b[12],
        b[13],
        b[14],
        b[15]
    )
}
/// Decode the measured post-root episode section. Its start is determined by
/// schema field traversal; it is not a fixed byte offset or byte-pattern scan.
pub fn creation_episodes(history: &[u8], registry: &Registry) -> Result<Vec<String>> {
    ensure!(history.len() >= 2, "truncated history tag");
    let tag = u16::from_le_bytes(history[..2].try_into()?);
    ensure!(
        registry
            .class(tag)
            .is_some_and(|c| c.name == "DocumentHistory"),
        "unsupported history class"
    );
    let root = native_parameters::decode_object_fields(history, registry, 2, tag)?;
    ensure!(
        root.fields["m_episodeList"]["m_oEpisodes"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "inline history episodes require a different contract"
    );
    let count_bytes = history
        .get(root.end..root.end + 4)
        .ok_or_else(|| anyhow::anyhow!("history episode count absent"))?;
    let count = u32::from_le_bytes(count_bytes.try_into()?) as usize;
    ensure!(count <= 1_000_000, "history episode count budget exceeded");
    let episode = registry
        .named("Episode")
        .ok_or_else(|| anyhow::anyhow!("Episode schema absent"))?;
    let mut pos = root.end + 4;
    let mut guids = Vec::new();
    for _ in 0..count {
        let decoded = native_parameters::decode_object_fields(history, registry, pos, episode.tag)?;
        let raw = &decoded.fields["m_eGUID"]["m_guid"]["m_guid"]["guid_bytes"];
        let values = raw
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("unsupported Episode GUID shape"))?;
        ensure!(values.len() == 16, "invalid Episode GUID width");
        let mut bytes = [0u8; 16];
        for (i, v) in values.iter().enumerate() {
            bytes[i] = u8::try_from(
                v.as_u64()
                    .ok_or_else(|| anyhow::anyhow!("invalid GUID byte"))?,
            )?;
        }
        guids.push(guid_string(&bytes));
        pos = decoded.end;
    }
    ensure!(
        history.get(pos..) == Some(&[0; 4]),
        "unsupported history episode trailer"
    );
    Ok(guids)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectIndex {
    pub identities: BTreeMap<u64, Identity>,
    pub graveyard_rows: Vec<serde_json::Value>,
    pub row_stride: usize,
    pub source_fields: serde_json::Value,
}
#[derive(Clone)]
struct Leaf {
    path: String,
    offset: usize,
    size: usize,
}
fn fixed_layout(registry: &Registry, tag: u16) -> Result<(usize, Vec<Leaf>)> {
    fn visit(
        registry: &Registry,
        tag: u16,
        prefix: &str,
        pos: &mut usize,
        leaves: &mut Vec<Leaf>,
        depth: usize,
    ) -> Result<()> {
        ensure!(depth < 128, "index layout recursion budget");
        let c = registry
            .class(tag)
            .ok_or_else(|| anyhow::anyhow!("index layout class absent"))?;
        if c.parent_reference.tag >= 12 {
            visit(
                registry,
                c.parent_reference.tag,
                prefix,
                pos,
                leaves,
                depth + 1,
            )?;
        }
        for f in &c.fields {
            if f.uninterpreted_flags & 2 != 0 {
                continue;
            }
            ensure!(
                f.uninterpreted_flags == 0 && f.modifier == 0,
                "nonfixed index schema field {}.{}",
                c.name,
                f.name
            );
            let path = if prefix.is_empty() {
                f.name.clone()
            } else {
                format!("{prefix}.{}", f.name)
            };
            if f.base == 14 {
                ensure!(f.references.len() == 1, "index reference field schema");
                visit(registry, f.references[0].tag, &path, pos, leaves, depth + 1)?;
            } else {
                let size = match f.base {
                    1 | 2 => 1,
                    3 => 2,
                    4..=6 => 4,
                    7 | 11 => 8,
                    9 => 16,
                    _ => anyhow::bail!("unsupported index primitive"),
                };
                leaves.push(Leaf {
                    path,
                    offset: *pos,
                    size,
                });
                *pos += size;
            }
        }
        Ok(())
    }
    let mut size = 0;
    let mut leaves = Vec::new();
    visit(registry, tag, "", &mut size, &mut leaves, 0)?;
    Ok((size, leaves))
}
fn leaf_value(row: &[u8], layout: &[Leaf], prefix: &str) -> Result<(u64, usize)> {
    let candidates: Vec<_> = layout
        .iter()
        .filter(|f| f.path.starts_with(prefix))
        .collect();
    ensure!(
        candidates.len() == 1,
        "ambiguous/missing index field {prefix}"
    );
    let f = candidates[0];
    let bytes = row
        .get(f.offset..f.offset + f.size)
        .ok_or_else(|| anyhow::anyhow!("truncated index field"))?;
    Ok((
        match f.size {
            4 => u64::from(u32::from_le_bytes(bytes.try_into()?)),
            8 => u64::from_le_bytes(bytes.try_into()?),
            _ => anyhow::bail!("unsupported index identity width"),
        },
        f.size,
    ))
}
/// Decode the schema-defined ElemRec array from its true root cursor, followed
/// by graveyard records, flags, deferred IdentifierSource and exact terminator.
/// Original suffixes may repeat; current ElementIds must be unique.
pub fn parse(index: &[u8], registry: &Registry, episodes: &[String]) -> Result<ProjectIndex> {
    ensure!(index.len() >= 6, "short project element index");
    let tag = u16::from_le_bytes(index[..2].try_into()?);
    let table = registry
        .class(tag)
        .ok_or_else(|| anyhow::anyhow!("index root absent"))?;
    ensure!(
        table.name == "ElemTable" && matches!(table.fields.len(), 4 | 5),
        "unsupported index root schema"
    );
    let array = &table.fields[0];
    ensure!(
        array.name == "m_elemArr" && array.raw_descriptor == 0x500e && array.references.len() == 1,
        "unsupported index array"
    );
    let (stride, layout) = fixed_layout(registry, array.references[0].tag)?;
    ensure!(stride > 0, "empty index layout");
    let count = u32::from_le_bytes(index[2..6].try_into()?) as usize;
    let end = count
        .checked_mul(stride)
        .and_then(|n| n.checked_add(6))
        .ok_or_else(|| anyhow::anyhow!("index array overflow"))?;
    let rows = index
        .get(6..end)
        .ok_or_else(|| anyhow::anyhow!("truncated index array"))?;
    let mut result = BTreeMap::new();
    for (i, row) in rows.chunks_exact(stride).enumerate() {
        let element_id = leaf_value(row, &layout, "m_id.")?.0;
        let original_id_suffix = leaf_value(row, &layout, "m_history.m_originalElementId.")?.0;
        let creation_episode =
            u32::try_from(leaf_value(row, &layout, "m_history.m_creationDate.")?.0)?;
        let stored_revision =
            u32::try_from(leaf_value(row, &layout, "m_history.m_lastModificationDate.")?.0)?;
        let other_revision =
            u32::try_from(leaf_value(row, &layout, "m_history.m_lastUserModificationDate.")?.0)?;
        let (owner, width) = leaf_value(row, &layout, "m_OwningElementId.")?;
        let owning_element_id = if width == 4 {
            i64::from(owner as u32 as i32)
        } else {
            owner as i64
        };
        let partition_id = i64::from(leaf_value(row, &layout, "m_partitionId.")?.0 as u32 as i32);
        let reverse = episodes
            .len()
            .checked_sub(creation_episode as usize + 1)
            .ok_or_else(|| anyhow::anyhow!("creation episode outside history for {element_id}"))?;
        let unique_id = format!("{}-{original_id_suffix:08x}", episodes[reverse]);
        let raw_fields = row
            .chunks_exact(4)
            .map(|v| u32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        let entry = Identity {
            element_id,
            original_id_suffix,
            creation_episode,
            stored_revision,
            other_revision,
            row_offset: 6 + i * stride,
            raw_fields,
            owning_element_id,
            partition_id,
            unique_id,
        };
        ensure!(
            result.insert(element_id, entry).is_none(),
            "duplicate current ElementId {element_id}"
        );
    }
    let mut pos = end;
    let read_u32 = |pos: &mut usize| -> Result<u32> {
        let b = index
            .get(*pos..*pos + 4)
            .ok_or_else(|| anyhow::anyhow!("truncated index trailer"))?;
        *pos += 4;
        Ok(u32::from_le_bytes(b.try_into()?))
    };
    let grave_count = read_u32(&mut pos)? as usize;
    ensure!(grave_count <= 1_000_000, "graveyard count budget");
    let field = &table.fields[1];
    ensure!(
        field.name == "m_graveyardRecs"
            && field.raw_descriptor == 0x500e
            && field.references.len() == 1,
        "unsupported graveyard schema"
    );
    let mut graveyard_rows = Vec::new();
    for _ in 0..grave_count {
        let decoded =
            native_parameters::decode_object_fields(index, registry, pos, field.references[0].tag)?;
        pos = decoded.end;
        graveyard_rows.push(decoded.fields);
    }
    ensure!(
        table.fields[2].name == "m_pSource" && table.fields[2].raw_descriptor == 0x20e,
        "unsupported index source field"
    );
    ensure!(
        read_u32(&mut pos)? == u32::MAX,
        "unsupported identifier source pointer"
    );
    let source_tag = u16::from_le_bytes(
        index
            .get(pos..pos + 2)
            .ok_or_else(|| anyhow::anyhow!("source class absent"))?
            .try_into()?,
    );
    pos += 2;
    // Revit 2023 has only ExpandAllOnLoad. The later contract appends
    // LastElementIdOverride before the deferred source body; consuming it in
    // the older contract would shift m_last and the exact terminal by a byte.
    let flag_names: &[&str] = match table.fields.len() {
        4 => &["m_bExpandAllOnLoad"],
        5 => &["m_bExpandAllOnLoad", "m_bLastElementIdOverride"],
        _ => unreachable!("root field count checked above"),
    };
    for (i, name) in flag_names.iter().enumerate() {
        ensure!(
            table.fields[i + 3].name == *name && table.fields[i + 3].raw_descriptor == 1,
            "unsupported index flag schema"
        );
        ensure!(
            index.get(pos).is_some_and(|v| *v <= 1),
            "invalid index flag"
        );
        pos += 1;
    }
    ensure!(
        registry
            .class(source_tag)
            .is_some_and(|c| c.name == "IdentifierSource"),
        "unsupported identifier source class"
    );
    let source = native_parameters::decode_object_fields(index, registry, pos, source_tag)?;
    pos = source.end;
    ensure!(
        index.get(pos..) == Some(&[0; 4]),
        "unsupported index terminator"
    );
    Ok(ProjectIndex {
        identities: result,
        graveyard_rows,
        row_stride: stride,
        source_fields: source.fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_registry::{Class, Field, Reference};

    fn reference(tag: u16) -> Reference {
        Reference {
            offset: 0,
            tag,
            introduces_definition: false,
        }
    }
    fn field(name: &str, descriptor: u32, tag: Option<u16>) -> Field {
        Field {
            offset: 0,
            name: name.into(),
            descriptor_offset: 0,
            raw_descriptor: descriptor,
            base: descriptor as u8,
            modifier: (descriptor >> 8) as u8,
            uninterpreted_flags: (descriptor >> 16) as u16,
            array_count: None,
            references: tag.into_iter().map(reference).collect(),
            nested_descriptor: None,
            end: 0,
        }
    }
    fn class(tag: u16, name: &str, fields: Vec<Field>) -> Class {
        Class {
            tag,
            name: name.into(),
            offset: 0,
            parent_reference: reference(0),
            version_like_word: 0,
            fields,
            opaque_16byte_entry_count: 0,
            opaque_entries_offset: 0,
            opaque_entries_sha256: String::new(),
            end: 0,
        }
    }
    // Synthetic schema preserves the independently measured ordering differences,
    // without distributing model-derived fixture bytes.
    fn fixture(older: bool) -> (Registry, Vec<u8>) {
        let id = |name| field(name, 14, Some(12));
        let episode = |name| field(name, 14, Some(13));
        let history = field("m_history", 14, Some(14));
        let row_fields = if older {
            vec![
                id("m_id"),
                history,
                episode("m_partitionId"),
                id("m_OwningElementId"),
            ]
        } else {
            vec![
                history,
                id("m_id"),
                id("m_OwningElementId"),
                episode("m_partitionId"),
            ]
        };
        let mut table_fields = vec![
            field("m_elemArr", 0x500e, Some(15)),
            field("m_graveyardRecs", 0x500e, Some(16)),
            field("m_pSource", 0x20e, Some(17)),
            field("m_bExpandAllOnLoad", 1, None),
        ];
        if !older {
            table_fields.push(field("m_bLastElementIdOverride", 1, None));
        }
        let registry = Registry {
            source_sha256: String::new(),
            consumed_bytes: 0,
            reference_count: 0,
            terminator_offset: 0,
            classes: vec![
                class(
                    12,
                    "ElementId",
                    vec![field("m_id", if older { 4 } else { 11 }, None)],
                ),
                class(13, "EpisodeId", vec![field("m_id", 4, None)]),
                class(
                    14,
                    "ElementHistory",
                    vec![
                        id("m_originalElementId"),
                        episode("m_creationDate"),
                        episode("m_lastModificationDate"),
                        episode("m_lastUserModificationDate"),
                    ],
                ),
                class(15, "ElemRec", row_fields),
                class(16, "GraveyardRec", vec![id("m_id")]),
                class(17, "IdentifierSource", vec![id("m_last")]),
                class(18, "ElemTable", table_fields),
            ],
        };
        let mut bytes = 18u16.to_le_bytes().to_vec();
        bytes.extend(1u32.to_le_bytes());
        let words: Vec<u32> = if older {
            vec![42, 19, 0, 2, 1, 3, u32::MAX]
        } else {
            vec![19, 0, 0, 2, 1, 42, 0, u32::MAX, u32::MAX, 3]
        };
        for word in words {
            bytes.extend(word.to_le_bytes());
        }
        bytes.extend(1u32.to_le_bytes()); // nonempty graveyard exercises cursor
        bytes.extend(99u32.to_le_bytes());
        if !older {
            bytes.extend(0u32.to_le_bytes());
        }
        bytes.extend(u32::MAX.to_le_bytes());
        bytes.extend(17u16.to_le_bytes());
        bytes.push(0);
        if !older {
            bytes.push(1);
        }
        bytes.extend(123u32.to_le_bytes());
        if !older {
            bytes.extend(0u32.to_le_bytes());
        }
        bytes.extend([0; 4]);
        (registry, bytes)
    }

    #[test]
    fn modification_episode_routes_through_compaction_not_as_ordinal() {
        let rows = vec![
            Increment {
                greatest_episode: 5721,
                total_episodes: 5722,
                stored_elements: 0,
                redirects: vec![(1, 0)],
                episode_permutation: Vec::new(),
            },
            Increment {
                greatest_episode: 5723,
                total_episodes: 5724,
                stored_elements: 20,
                redirects: vec![(-1, 20)],
                episode_permutation: Vec::new(),
            },
        ];
        let present = BTreeSet::from([1]);
        for episode in [0, 5721, 5722, 5723] {
            assert_eq!(route_episode(episode, &rows, &present).unwrap(), 1);
        }
        assert!(route_episode(5724, &rows, &present).is_err());
        assert!(route_episode(5721, &rows, &BTreeSet::from([2])).is_err());
        let mut cycle = rows.clone();
        cycle[0].redirects = vec![(0, 0)];
        assert!(route_episode(5721, &cycle, &present).is_err());
    }

    #[test]
    fn historical_residual_versions_do_not_form_current_redirect_branches() {
        // Core Interior 2024 retains partial reductions before final compaction.
        let rows = vec![
            Increment {
                greatest_episode: 0,
                total_episodes: 1,
                stored_elements: 0,
                redirects: vec![(-1, 745), (1, 98), (2, 26), (3, 0)],
                episode_permutation: vec![],
            },
            Increment {
                greatest_episode: 1,
                total_episodes: 2,
                stored_elements: 0,
                redirects: vec![(-1, 100), (3, 0)],
                episode_permutation: vec![],
            },
            Increment {
                greatest_episode: 2,
                total_episodes: 3,
                stored_elements: 0,
                redirects: vec![(-1, 50), (3, 0)],
                episode_permutation: vec![],
            },
            Increment {
                greatest_episode: 3,
                total_episodes: 4,
                stored_elements: 42,
                redirects: vec![(-1, 42)],
                episode_permutation: vec![],
            },
        ];
        assert_eq!(route_episode(0, &rows, &BTreeSet::from([3])).unwrap(), 3);
        let mut invalid = rows.clone();
        invalid[0].redirects.swap(1, 2);
        assert!(route_episode(0, &invalid, &BTreeSet::from([3])).is_err());
        let mut invalid = rows.clone();
        invalid[0].redirects[2].1 = 99;
        assert!(route_episode(0, &invalid, &BTreeSet::from([3])).is_err());
        let mut invalid = rows.clone();
        invalid[0].redirects.pop();
        assert!(route_episode(0, &invalid, &BTreeSet::from([3])).is_err());
    }

    #[test]
    fn partially_reduced_increment_still_routes_to_its_own_current_storage() {
        let rows = vec![
            Increment {
                greatest_episode: 0,
                total_episodes: 1,
                stored_elements: 13,
                redirects: vec![(-1, 32), (1, 13)],
                episode_permutation: vec![],
            },
            Increment {
                greatest_episode: 1,
                total_episodes: 2,
                stored_elements: 20,
                redirects: vec![(-1, 20)],
                episode_permutation: vec![],
            },
        ];
        assert_eq!(route_episode(0, &rows, &BTreeSet::from([0, 1])).unwrap(), 0);
        assert!(route_episode(0, &rows, &BTreeSet::from([1])).is_err());
    }

    #[test]
    fn sole_increment_permutation_preserves_evidence_and_route() {
        let mut rows = vec![Increment {
            greatest_episode: 14775,
            total_episodes: 14776,
            stored_elements: 1918002,
            redirects: vec![(-1, 1918002)],
            episode_permutation: Vec::new(),
        }];
        let pairs = serde_json::json!([{"first":{"m_id":14775},"second":14762}]);
        qualify_single_increment_permutation(&mut rows, &pairs).unwrap();
        assert_eq!(rows[0].episode_permutation, vec![(14775, 14762)]);
        for episode in [0, 14762, 14775] {
            assert_eq!(
                route_episode(episode, &rows, &BTreeSet::from([0])).unwrap(),
                0
            );
        }
        assert!(route_episode(14775, &rows, &BTreeSet::new()).is_err());
        for invalid in [
            serde_json::json!([{"first":{"m_id":14776},"second":0}]),
            serde_json::json!([{"first":{"m_id":0},"second":-1}]),
            serde_json::json!([{"first":{"m_id":0},"second":14776}]),
            serde_json::json!([{"first":0,"second":0}]),
            serde_json::json!([{"first":{"m_id":1},"second":0},{"first":{"m_id":1},"second":2}]),
            serde_json::Value::Null,
        ] {
            assert!(qualify_single_increment_permutation(&mut rows.clone(), &invalid).is_err());
        }
        let mut multiple = vec![rows[0].clone(), rows[0].clone()];
        assert!(qualify_single_increment_permutation(&mut multiple, &pairs).is_err());
        let mut redirected = rows.clone();
        redirected[0].redirects = vec![(1, 0)];
        assert!(qualify_single_increment_permutation(&mut redirected, &pairs).is_err());
        let mut empty = rows.clone();
        empty[0].stored_elements = 0;
        assert!(qualify_single_increment_permutation(&mut empty, &pairs).is_err());
    }

    #[test]
    fn increment_routing_equivalence_excludes_stream_bookkeeping() {
        let global = serde_json::json!([{
            "m_greatest": {"m_id": 10}, "m_totalEpisodes": 11,
            "m_totalElements": 20,
            "m_incrementVersions": [{"m_causedByIncrementNumber": -1, "m_totalElements": 0}],
            "m_basicFileInfoStreamRevision": 4539,
            "m_incrementTableStreamRevision": 4539
        }]);
        let mut local = global.clone();
        local[0]["m_basicFileInfoStreamRevision"] = 4538.into();
        local[0]["m_incrementTableStreamRevision"] = 4538.into();
        assert_eq!(reconcile_increment_rows(&global, &local).unwrap().len(), 1);
        for field in ["m_totalEpisodes", "m_totalElements"] {
            let mut bad = local.clone();
            bad[0][field] = 21.into();
            assert!(reconcile_increment_rows(&global, &bad).is_err());
        }
        let mut bad = local.clone();
        bad[0]["m_greatest"]["m_id"] = 9.into();
        assert!(reconcile_increment_rows(&global, &bad).is_err());
        let mut bad = local.clone();
        bad[0]["m_incrementVersions"][0]["m_causedByIncrementNumber"] = 1.into();
        assert!(reconcile_increment_rows(&global, &bad).is_err());
        let mut bad = local.clone();
        bad[0]["m_incrementVersions"][0]["m_totalElements"] = 1.into();
        assert!(reconcile_increment_rows(&global, &bad).is_err());
        local[0].as_object_mut().unwrap().remove("m_totalEpisodes");
        assert!(reconcile_increment_rows(&global, &local).is_err());
    }

    #[test]
    fn explicit_index_variants_preserve_identity_and_tail_alignment() {
        for older in [true, false] {
            let (registry, bytes) = fixture(older);
            let result = parse(&bytes, &registry, &["episode".into()]).unwrap();
            assert_eq!(result.row_stride, if older { 28 } else { 40 });
            let id = &result.identities[&42];
            assert_eq!(id.original_id_suffix, 19);
            assert_eq!(id.unique_id, "episode-00000013");
            assert_eq!(id.owning_element_id, -1);
            assert_eq!(id.partition_id, 3);
            assert_eq!(id.row_offset, 6);
            assert_eq!(id.stored_revision, 2);
            assert_eq!(result.graveyard_rows.len(), 1);
            assert_eq!(result.source_fields["m_last"]["m_id"], 123);
        }
    }

    #[test]
    fn index_variants_reject_mismatched_roots_and_corrupt_tails() {
        for older in [true, false] {
            let (registry, bytes) = fixture(older);
            let episodes = ["episode".into()];
            for n in 1..=6 {
                assert!(parse(&bytes[..bytes.len() - n], &registry, &episodes).is_err());
            }
            let mut bad = bytes.clone();
            *bad.last_mut().unwrap() = 1;
            assert!(parse(&bad, &registry, &episodes).is_err());
            let mut invalid_flag = bytes.clone();
            let flag_offset = bytes.len() - 4 - if older { 4 + 1 } else { 8 + 2 };
            invalid_flag[flag_offset] = 2;
            assert!(parse(&invalid_flag, &registry, &episodes).is_err());
            let stride = if older { 28 } else { 40 };
            let mut duplicate = bytes[..6 + stride].to_vec();
            duplicate[2..6].copy_from_slice(&2u32.to_le_bytes());
            duplicate.extend_from_slice(&bytes[6..6 + stride]);
            duplicate.extend_from_slice(&bytes[6 + stride..]);
            assert!(parse(&duplicate, &registry, &episodes).is_err());
            let mut wrong = registry.clone();
            wrong.classes.last_mut().unwrap().fields[3].name = "unknown_flag".into();
            assert!(parse(&bytes, &wrong, &episodes).is_err());
            let mut wrong = registry.clone();
            wrong.classes[5].name = "UnrelatedDeferredObject".into();
            assert!(parse(&bytes, &wrong, &episodes).is_err());
            let (other_registry, _) = fixture(!older);
            assert!(parse(&bytes, &other_registry, &episodes).is_err());
        }
    }

    #[test]
    #[ignore = "requires RVT_NATIVE_INDEX_MODEL with a private or downloaded real model"]
    fn real_model_index_contract() {
        let path = std::env::var("RVT_NATIVE_INDEX_MODEL").expect("set RVT_NATIVE_INDEX_MODEL");
        let mut file = crate::RevitFile::open(path).unwrap();
        let registry = crate::schema_registry::parse(
            &crate::native_document::read_single(&mut file, "Formats/Latest").unwrap(),
        )
        .unwrap();
        let history = creation_episodes(
            &crate::native_document::read_single(&mut file, "Global/History").unwrap(),
            &registry,
        )
        .unwrap();
        let index = parse(
            &crate::native_document::read_single(&mut file, "Global/ElemTable").unwrap(),
            &registry,
            &history,
        )
        .unwrap();
        let increments = storage_increments(
            &crate::native_document::read_single(&mut file, "Global/DocumentIncrementTable")
                .unwrap(),
            &registry,
        )
        .unwrap();
        let present: BTreeSet<u32> = file
            .stream_names()
            .iter()
            .filter_map(|n| n.strip_prefix("Partitions/"))
            .map(|n| n.parse().unwrap())
            .collect();
        let mut routes = BTreeMap::<u32, usize>::new();
        for identity in index.identities.values() {
            *routes
                .entry(route_episode(identity.stored_revision, &increments, &present).unwrap())
                .or_default() += 1;
        }
        if let Ok(output) = std::env::var("RVT_NATIVE_INDEX_PROOF") {
            let selected: BTreeSet<u64> = std::env::var("RVT_NATIVE_INDEX_IDS")
                .unwrap_or_default()
                .split(',')
                .filter(|v| !v.is_empty())
                .map(|v| v.parse().unwrap())
                .collect();
            let identities: Vec<_> = index
                .identities
                .values()
                .filter(|i| selected.contains(&i.element_id))
                .collect();
            let mut physical = Vec::new();
            let names: Vec<_> = file
                .stream_names()
                .iter()
                .filter(|n| n.starts_with("Partitions/"))
                .cloned()
                .collect();
            for name in names {
                let stored = file.read_stream(&name).unwrap();
                let prepared = crate::compression::prepare_stream_for_inflate(&name, &stored);
                crate::native_segments::walk(&prepared,&registry,268435456,|source,bytes| {
                    if source.content_key.is_some(){return Ok(());}
                    let id_bytes=if index.row_stride==28 {4}else{8};
                    let header=match source.channel{101=>id_bytes+4,102|103=>id_bytes+8,_=>anyhow::bail!("unknown channel")};
                    let mut pos=0;let mut count=0;let mut body_sum=0;
                    while pos<bytes.len(){
                        ensure!(bytes.len()-pos>=header+4,"short physical record");
                        let id=if id_bytes==4 {u32::from_le_bytes(bytes[pos..pos+4].try_into()?)as u64}else{u64::from_le_bytes(bytes[pos..pos+8].try_into()?)};
                        let len=u32::from_le_bytes(bytes[pos+header-4..pos+header].try_into()?)as usize;
                        let start=pos+header;let end=start.checked_add(len).ok_or_else(||anyhow::anyhow!("physical size overflow"))?;
                        ensure!(end+4<=bytes.len()&&u32::from_le_bytes(bytes[end..end+4].try_into()?)as usize==len,"physical dual lengths");
                        if selected.contains(&id){
                            use sha2::{Digest,Sha256};
                            let tag=bytes.get(start..start+2).map(|b|u16::from_le_bytes(b.try_into().unwrap()));
                            let decoded=crate::native_parameters::decode_graph(&bytes[start..end],&registry);
                            physical.push(serde_json::json!({"element_id":id,"stream":name,"group":source,"group_record_offset":pos,"header_u32":bytes[pos..start].chunks_exact(4).map(|b|u32::from_le_bytes(b.try_into().unwrap())).collect::<Vec<_>>(),"body_bytes":len,"body_sha256":format!("{:x}",Sha256::digest(&bytes[start..end])),"class_name":tag.and_then(|t|registry.class(t)).map(|c|&c.name),"graph":decoded.as_ref().ok(),"graph_error":decoded.as_ref().err().map(|e|e.to_string())}));
                        }
                        count+=1;body_sum+=len as u64;pos=end+4;
                    }
                    ensure!(count==source.declared_objects&&body_sum==source.declared_body_bytes,"physical group framing totals");
                    Ok(())
                }).unwrap();
            }

            let proof = serde_json::json!({"indexed_owners":index.identities.len(),"graveyard_rows":index.graveyard_rows.len(),"route_counts":routes,"present_partitions":present,"increments":increments,"selected_identities":identities,"physical_records_for_selected_ids":physical});
            std::fs::write(output, serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
        }
        assert!(!index.identities.is_empty());
        println!(
            "index rows={} graveyard={} stride={} episodes={}",
            index.identities.len(),
            index.graveyard_rows.len(),
            index.row_stride,
            history.len()
        );
    }
    #[test]
    fn fixed_layout_short_has_two_byte_width_and_aligned_successor() {
        let (mut registry, _) = fixture(false);
        registry.classes[0] = class(
            12,
            "ShortThenInt",
            vec![field("short", 3, None), field("integer", 4, None)],
        );
        let (size, leaves) = fixed_layout(&registry, 12).unwrap();
        assert_eq!(size, 6);
        assert_eq!(leaves[0].size, 2);
        assert_eq!(leaves[1].offset, 2);
        assert_eq!(leaves[1].size, 4);
    }
}
