//! Readers for the JSON shapes the native record modules share: an ElementId
//! field, a finite point, and an id-keyed value that every copy must agree on.

use serde_json::Value;
use std::collections::BTreeMap;

/// `{"m_id":{"m_id64":n}}` as `n`.
pub(crate) fn element_id(value: &Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(Value::as_i64)
        .and_then(|n| u64::try_from(n).ok())
}

/// `{"m_id":{"m_id64":n}}` as `n`, signed.
pub(crate) fn element_id_i64(value: &Value) -> Option<i64> {
    value.pointer("/m_id/m_id64").and_then(Value::as_i64)
}

/// A three-number array as a point, `None` unless every coordinate is finite.
pub(crate) fn finite_triple(value: &Value) -> Option<[f64; 3]> {
    let [x, y, z] = value.as_array()?.as_slice() else {
        return None;
    };
    let p = [x.as_f64()?, y.as_f64()?, z.as_f64()?];
    p.iter().all(|v| v.is_finite()).then_some(p)
}

/// Record `value` for `id`, or `None` once two copies disagree or one has
/// none.
pub(crate) fn agree<T: PartialEq>(map: &mut BTreeMap<u64, Option<T>>, id: u64, value: Option<T>) {
    match map.get_mut(&id) {
        None => {
            map.insert(id, value);
        }
        Some(held) => {
            if *held != value {
                *held = None;
            }
        }
    }
}
