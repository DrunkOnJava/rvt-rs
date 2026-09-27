//! Forward-only physical partition markers and continuation groups.
//! Class tags come from the structural registry, never gzip/name searching.
use crate::schema_registry::Registry;
use anyhow::{Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;

#[derive(Debug, Clone, Serialize)]
pub struct GroupSource {
    pub content_key: Option<[u8; 16]>,
    pub channel: u64,
    pub first_marker_offset: usize,
    pub segment_count: usize,
    pub declared_objects: u64,
    pub declared_body_bytes: u64,
}
#[derive(Debug, Default, Clone, Serialize)]
pub struct Statistics {
    pub segments: usize,
    pub groups: usize,
    pub content_sections: usize,
    pub inflated_bytes: u64,
    pub trailing_padding_bytes: usize,
    pub opaque_terminal_bytes: usize,
    pub opaque_terminal_sha256: String,
}
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or_else(|| anyhow::anyhow!("partition marker overflow"))?;
        let b = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| anyhow::anyhow!("truncated partition marker at {}", self.pos))?;
        self.pos = end;
        Ok(b)
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into()?))
    }
}
/// Walk a checksum-prepared partition. Each callback receives a complete
/// continuation group, not a claimed live element. Maximum reassembled group
/// size is an explicit caller budget. Gzip CRC, ISIZE, boundary, checkback and
/// continuation flags are checked before a group is exposed.
pub fn walk(
    prepared: &[u8],
    registry: &Registry,
    max_group_bytes: usize,
    mut callback: impl FnMut(&GroupSource, &[u8]) -> Result<()>,
) -> Result<Statistics> {
    ensure!(max_group_bytes > 0, "zero partition group budget");
    let tag = |name: &str| {
        registry
            .named(name)
            .map(|c| c.tag)
            .ok_or_else(|| anyhow::anyhow!("partition schema class absent: {name}"))
    };
    let content_tag = tag("ContentMarker")?;
    let content_key_tag = tag("ContentKey")?;
    let segment_tag = tag("SegmentMarker")?;
    let checkback_tag = tag("SegmentCheckback")?;
    let signature_tag = tag("SignatureMarker")?;
    let mut r = Reader {
        bytes: prepared,
        pos: 0,
    };
    r.take(8)?;
    let mut stats = Statistics::default();
    let mut content_key = None;
    let mut last_size = None;
    let mut active: Option<(GroupSource, Vec<u8>)> = None;
    loop {
        let offset = r.pos;
        let next = r.u16()?;
        if next == content_tag {
            ensure!(
                active.is_none(),
                "content boundary interrupts continued segment"
            );
            let token = r.u32()?;
            let count = if token == 0 {
                content_key = None;
                r.u32()?
            } else {
                ensure!(
                    token == u32::MAX && r.u16()? == content_key_tag,
                    "unsupported content key pointer"
                );
                let count = r.u32()?;
                content_key = Some(r.take(16)?.try_into()?);
                count
            };
            if token == 0 && count == u32::MAX {
                let tail = &prepared[r.pos..];
                ensure!(
                    tail.len() <= crate::compression::REVIT_STORED_PAGE_BYTES,
                    "partition opaque terminal exceeds one storage page"
                );
                stats.trailing_padding_bytes = tail.iter().take_while(|&&v| v == 0).count();
                stats.opaque_terminal_bytes = tail.len();
                stats.opaque_terminal_sha256 = format!("{:x}", Sha256::digest(tail));
                return Ok(stats);
            }
            stats.content_sections += 1;
        } else if next == segment_tag {
            let flags = r.u32()?;
            let count = r.u32()?;
            let size = r.u32()?;
            let raw = r.u32()?;
            if size == 0 {
                ensure!(
                    flags == 0 && count == 0 && raw == 0 && active.is_none(),
                    "invalid empty segment marker"
                );
                continue;
            }
            ensure!(
                (4..=7).contains(&flags) && size >= 18,
                "unsupported segment flags/size at {offset}"
            );
            let channel = r.u64()?;
            let compressed = r.take(size as usize - 8)?;
            let mut decoder = flate2::bufread::GzDecoder::new(compressed);
            let mut bytes = Vec::new();
            (&mut decoder)
                .take(max_group_bytes as u64 + 1)
                .read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() <= max_group_bytes,
                "inflated segment group budget exceeded"
            );
            ensure!(
                decoder.get_ref().is_empty(),
                "gzip does not consume exact declared segment boundary"
            );
            stats.segments += 1;
            stats.inflated_bytes += bytes.len() as u64;
            last_size = Some(size);
            let from_previous = flags & 1 != 0;
            let to_next = flags & 2 != 0;
            ensure!(
                from_previous == active.is_some(),
                "segment continuation state mismatch at {offset}"
            );
            if let Some((source, data)) = &mut active {
                ensure!(
                    source.channel == channel && source.content_key == content_key,
                    "continuation channel/content mismatch"
                );
                ensure!(
                    data.len()
                        .checked_add(bytes.len())
                        .is_some_and(|n| n <= max_group_bytes),
                    "reassembled group budget exceeded"
                );
                data.extend(bytes);
                source.segment_count += 1;
                source.declared_objects += u64::from(count);
                source.declared_body_bytes += u64::from(raw);
            } else {
                active = Some((
                    GroupSource {
                        content_key,
                        channel,
                        first_marker_offset: offset,
                        segment_count: 1,
                        declared_objects: u64::from(count),
                        declared_body_bytes: u64::from(raw),
                    },
                    bytes,
                ));
            }
            if !to_next {
                let (source, bytes) = active.take().unwrap();
                callback(&source, &bytes)?;
                stats.groups += 1;
            }
        } else if next == checkback_tag {
            ensure!(
                last_size == Some(r.u32()?),
                "segment checkback mismatch at {offset}"
            );
            last_size = None;
        } else if next == signature_tag {
            ensure!(active.is_none(), "signature interrupts continued segment");
            let count = r.u32()? as usize;
            ensure!(count <= 1_000_000, "partition signature budget exceeded");
            r.take(count)?;
            let units = r.u32()? as usize;
            ensure!(
                units <= 1_000_000,
                "partition signature string budget exceeded"
            );
            let text = r
                .take(units * 2)?
                .chunks_exact(2)
                .map(|s| u16::from_le_bytes([s[0], s[1]]))
                .collect::<Vec<_>>();
            String::from_utf16(&text)?;
        } else {
            anyhow::bail!("unsupported partition marker {next} at {offset}");
        }
    }
}
