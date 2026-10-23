//! Little-endian reads at a byte offset into an untrusted buffer.
//!
//! Each returns `None` when the value would run past the end of `buf`,
//! including for an offset near `usize::MAX`, and never panics: the
//! parsers read offsets out of the file itself.

pub(crate) fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}

pub(crate) fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}

pub(crate) fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}

pub(crate) fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}

pub(crate) fn f32_at(buf: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}

pub(crate) fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}
