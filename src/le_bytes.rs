//! Little-endian reads at a byte offset into an untrusted buffer.
//!
//! Each returns `None` when the value would run past the end of `buf`,
//! including for an offset near `usize::MAX`, and never panics: the
//! parsers read offsets out of the file itself. The float reads also return
//! `None` for a number no model holds ([`physical`]), as for one that is not
//! there: every reader of a stored float takes it through them.

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
    Some(f32::from_le_bytes(*buf.get(at..)?.first_chunk()?)).filter(|v| physical(f64::from(*v)))
}

pub(crate) fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(*buf.get(at..)?.first_chunk()?)).filter(|v| physical(*v))
}

/// The largest magnitude a stored double may have and still be read as a
/// number. Revit keeps model geometry within 20 miles (105,600 ft) of its
/// internal origin, and survey coordinates are of the order of 1e7 ft; this
/// is twenty orders above both. Below it, a sum of a few values stays finite
/// and each one fits an `f32`, so a crafted value cannot turn into an
/// infinity on its way to the IFC, glTF or plan SVG.
pub(crate) const MAX_MAGNITUDE: f64 = 1e30;

/// Whether a stored double is a number a model can hold: finite and within
/// [`MAX_MAGNITUDE`]. Every decode of a stored float goes through it: the
/// reads here ([`f64_at`], [`f32_at`]), which every partition scanner uses,
/// the walker's floats and the native graphs' numbers (`Fv::from_f64`).
pub(crate) fn physical(value: f64) -> bool {
    value.abs() <= MAX_MAGNITUDE
}

pub(crate) fn i32_at(buf: &[u8], at: usize) -> Option<i32> {
    Some(i32::from_le_bytes(*buf.get(at..)?.first_chunk()?))
}

/// The little-endian `u16` code units of `bytes`, two bytes each; a last odd
/// byte is dropped.
pub(crate) fn u16_units(bytes: &[u8]) -> impl Iterator<Item = u16> {
    bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
}
