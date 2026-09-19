//! Bounded numeric readers for the validated geometry profile.
//! Identity and increment contracts live exclusively in native_index.
use anyhow::Result;

pub fn u32_at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .ok_or_else(|| anyhow::anyhow!("truncated u32"))?
            .try_into()?,
    ))
}
pub fn u64_at(b: &[u8], p: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(p..p + 8)
            .ok_or_else(|| anyhow::anyhow!("truncated u64"))?
            .try_into()?,
    ))
}
