//! A compact value for a decoded object graph's fields (#1694).
//!
//! FACT it rests on: after the saved-mesh pass's other levers, 75% to 87% of
//! its peak is decoded graph nodes, and what makes them large is the container
//! and not the content. A `serde_json::Value::Object` is a `BTreeMap<String,
//! Value>`: its 11-slot leaf is 631 bytes however few fields it holds, every key
//! is a heap `String` of its own, and a pointer, an object of three numbers,
//! costs about 730 bytes (heap profiles on `rac_basic` 2023, #1694).
//! [`Fv`] holds the same content with an object's values in one slice and its
//! names once for every object that has them: on 14 files it comes to 0.157 to
//! 0.185 of the heap bytes of the `Value` it replaces, and 4,276,161 objects
//! went to it and back unchanged (the probe of #1694).
//!
//! The conversions, [`Fv::from_json`] and [`Fv::to_json`], are exact: the value
//! that comes back is the one that went in, number kinds (`1` and `1.0` differ),
//! key order and all. Reading is by `serde_json::Value`'s own methods under the
//! same names (`get`, indexing, `as_u64`, `as_i64`, `as_f64`, `as_str`,
//! `as_bool`, `as_array`, `as_object`, `pointer`, the `is_*` tests and
//! comparison with a bool, a string or a number), so a reader written for a
//! `Value` reads an `Fv` as it did, with one difference: `as_array` is a slice
//! and `as_object` an [`Obj`]. `Serialize` writes the JSON of the `Value` it
//! stands for, keys sorted, byte for byte.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Index;
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, SerializeMap, SerializeSeq, Serializer};
use serde_json::{Map, Number, Value};

/// The names of an object's fields in the order `serde_json` iterates them,
/// shared by every object that has exactly these.
pub type Names = Arc<[Box<str>]>;

/// A graph value: the content of a `serde_json::Value`, laid out to cost little.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Fv {
    #[default]
    Null,
    Bool(bool),
    /// A number `serde_json` holds as a non-negative integer.
    U64(u64),
    /// A number `serde_json` holds as a negative integer.
    I64(i64),
    /// A number `serde_json` holds as a float.
    F64(f64),
    Str(Box<str>),
    Array(Box<[Fv]>),
    Object(Box<Obj>),
}

/// An object: its names (shared) and its values, in the same order.
#[derive(Debug, Clone, PartialEq)]
pub struct Obj {
    names: Names,
    values: Box<[Fv]>,
}

impl Obj {
    /// An object of `values` named `names`, one name for each value.
    pub fn from_parts(names: Names, values: Box<[Fv]>) -> Obj {
        debug_assert_eq!(names.len(), values.len());
        Obj { names, values }
    }

    /// The names of the fields, in order.
    pub fn names(&self) -> &[Box<str>] {
        &self.names
    }

    /// The values of the fields, in the order of [`Self::names`].
    pub fn values(&self) -> &[Fv] {
        &self.values
    }

    /// The field `name`, or none where there is no such field.
    pub fn get(&self, name: &str) -> Option<&Fv> {
        self.names
            .iter()
            .position(|candidate| &**candidate == name)
            .map(|index| &self.values[index])
    }

    /// Whether there is a field `name`.
    pub fn contains_key(&self, name: &str) -> bool {
        self.names.iter().any(|candidate| &**candidate == name)
    }

    /// The fields, name and value, in order.
    pub fn iter(&self) -> ObjIter<'_> {
        ObjIter {
            names: self.names.iter(),
            values: self.values.iter(),
        }
    }

    /// The names of the fields, in order.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.names.iter().map(|name| &**name)
    }

    /// How many fields there are.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether there is no field.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

/// The fields of an [`Obj`], name and value, in order.
#[derive(Debug, Clone)]
pub struct ObjIter<'a> {
    names: std::slice::Iter<'a, Box<str>>,
    values: std::slice::Iter<'a, Fv>,
}

impl<'a> Iterator for ObjIter<'a> {
    type Item = (&'a str, &'a Fv);

    fn next(&mut self) -> Option<Self::Item> {
        Some((&**self.names.next()?, self.values.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.names.size_hint()
    }
}

impl DoubleEndedIterator for ObjIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some((&**self.names.next_back()?, self.values.next_back()?))
    }
}

impl ExactSizeIterator for ObjIter<'_> {}

impl<'a> IntoIterator for &'a Obj {
    type Item = (&'a str, &'a Fv);
    type IntoIter = ObjIter<'a>;

    fn into_iter(self) -> ObjIter<'a> {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a Box<Obj> {
    type Item = (&'a str, &'a Fv);
    type IntoIter = ObjIter<'a>;

    fn into_iter(self) -> ObjIter<'a> {
        self.iter()
    }
}

/// The lists of names the objects of a graph (or of a whole file) share.
#[derive(Debug, Default)]
pub struct NameTable {
    lists: HashMap<u64, Vec<Names>>,
}

impl NameTable {
    /// How many distinct lists of names are held.
    pub fn len(&self) -> usize {
        self.lists.values().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.lists.is_empty()
    }

    /// The shared list of `keys`, which must be in the order a `serde_json` map
    /// iterates them (sorted).
    pub fn intern(&mut self, keys: &[&str]) -> Names {
        let mut hasher = DefaultHasher::new();
        keys.hash(&mut hasher);
        let bucket = self.lists.entry(hasher.finish()).or_default();
        if let Some(found) = bucket.iter().find(|names| {
            names.len() == keys.len() && names.iter().map(|name| &**name).eq(keys.iter().copied())
        }) {
            return Arc::clone(found);
        }
        let names: Names = keys.iter().map(|key| Box::<str>::from(*key)).collect();
        bucket.push(Arc::clone(&names));
        names
    }
}

/// The table every decoded graph of the process shares, so that an object of a
/// class shares its names with every other object that has them.
fn shared_names() -> &'static Mutex<NameTable> {
    static TABLE: OnceLock<Mutex<NameTable>> = OnceLock::new();
    TABLE.get_or_init(|| Mutex::new(NameTable::default()))
}

/// The process-wide shared list of `keys` (sorted).
pub fn intern_names(keys: &[&str]) -> Names {
    shared_names()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .intern(keys)
}

fn number(n: &Number) -> Fv {
    if let Some(unsigned) = n.as_u64() {
        Fv::U64(unsigned)
    } else if let Some(signed) = n.as_i64() {
        Fv::I64(signed)
    } else {
        // Without serde_json's `arbitrary_precision` a number that is neither is a float.
        Fv::F64(n.as_f64().unwrap_or(f64::NAN))
    }
}

static NULL: Fv = Fv::Null;

mod private {
    pub trait Sealed {}
    impl Sealed for usize {}
    impl Sealed for str {}
    impl Sealed for String {}
    impl<T: ?Sized + Sealed> Sealed for &T {}
}

/// What [`Fv::get`] and indexing take: a field name or an array position.
pub trait FvIndex: private::Sealed {
    #[doc(hidden)]
    fn index_into<'v>(&self, value: &'v Fv) -> Option<&'v Fv>;
}

impl FvIndex for usize {
    fn index_into<'v>(&self, value: &'v Fv) -> Option<&'v Fv> {
        match value {
            Fv::Array(items) => items.get(*self),
            _ => None,
        }
    }
}

impl FvIndex for str {
    fn index_into<'v>(&self, value: &'v Fv) -> Option<&'v Fv> {
        match value {
            Fv::Object(object) => object.get(self),
            _ => None,
        }
    }
}

impl FvIndex for String {
    fn index_into<'v>(&self, value: &'v Fv) -> Option<&'v Fv> {
        self[..].index_into(value)
    }
}

impl<T: ?Sized + FvIndex> FvIndex for &T {
    fn index_into<'v>(&self, value: &'v Fv) -> Option<&'v Fv> {
        (**self).index_into(value)
    }
}

impl<I: FvIndex> Index<I> for Fv {
    type Output = Fv;

    /// The field or item, or `Null` where there is none, as `serde_json` does.
    fn index(&self, index: I) -> &Fv {
        index.index_into(self).unwrap_or(&NULL)
    }
}

fn parse_index(token: &str) -> Option<usize> {
    if token.starts_with('+') || (token.starts_with('0') && token.len() != 1) {
        return None;
    }
    token.parse().ok()
}

impl Fv {
    /// The compact form of `value`; `names` holds the lists its objects share.
    pub fn from_json(value: &Value, names: &mut NameTable) -> Fv {
        match value {
            Value::Null => Fv::Null,
            Value::Bool(b) => Fv::Bool(*b),
            Value::Number(n) => number(n),
            Value::String(s) => Fv::Str(s.as_str().into()),
            Value::Array(items) => Fv::Array(
                items
                    .iter()
                    .map(|item| Fv::from_json(item, names))
                    .collect(),
            ),
            Value::Object(map) => {
                let keys: Vec<&str> = map.keys().map(String::as_str).collect();
                let interned = names.intern(&keys);
                Fv::Object(Box::new(Obj {
                    names: interned,
                    values: map
                        .values()
                        .map(|item| Fv::from_json(item, names))
                        .collect(),
                }))
            }
        }
    }

    /// [`Self::from_json`] with the process-wide table of names.
    pub fn from_value(value: &Value) -> Fv {
        let mut table = shared_names()
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        Fv::from_json(value, &mut table)
    }

    /// The `serde_json::Value` this was made from.
    pub fn to_json(&self) -> Value {
        match self {
            Fv::Null => Value::Null,
            Fv::Bool(b) => Value::Bool(*b),
            Fv::U64(n) => Value::Number((*n).into()),
            Fv::I64(n) => Value::Number((*n).into()),
            Fv::F64(n) => Number::from_f64(*n).map_or(Value::Null, Value::Number),
            Fv::Str(s) => Value::String(s.to_string()),
            Fv::Array(items) => Value::Array(items.iter().map(Fv::to_json).collect()),
            Fv::Object(object) => Value::Object(
                object
                    .names
                    .iter()
                    .zip(object.values.iter())
                    .map(|(name, item)| (name.to_string(), item.to_json()))
                    .collect::<Map<String, Value>>(),
            ),
        }
    }

    /// A number as `json!` makes it of a float: `Null` where it is not finite.
    pub fn from_f64(n: f64) -> Fv {
        if n.is_finite() { Fv::F64(n) } else { Fv::Null }
    }

    /// A number as `json!` makes it of a signed integer.
    pub fn from_i64(n: i64) -> Fv {
        if n < 0 { Fv::I64(n) } else { Fv::U64(n as u64) }
    }

    /// An object of the `fields` given, whatever their order.
    pub fn object<const N: usize>(mut fields: [(&str, Fv); N]) -> Fv {
        fields.sort_by(|a, b| a.0.cmp(b.0));
        let keys: Vec<&str> = fields.iter().map(|field| field.0).collect();
        let names = intern_names(&keys);
        let values: Box<[Fv]> = fields.into_iter().map(|field| field.1).collect();
        Fv::Object(Box::new(Obj { names, values }))
    }

    /// The value of a pointer field: `{"class_tag", "offset", "pointer_token"}`.
    pub fn pointer_value(token: u32, class_tag: Option<u16>, offset: usize) -> Fv {
        static NAMES: OnceLock<Names> = OnceLock::new();
        let names = NAMES.get_or_init(|| intern_names(&["class_tag", "offset", "pointer_token"]));
        Fv::Object(Box::new(Obj {
            names: Arc::clone(names),
            values: Box::new([
                class_tag.map_or(Fv::Null, |tag| Fv::U64(u64::from(tag))),
                Fv::U64(offset as u64),
                Fv::U64(u64::from(token)),
            ]),
        }))
    }

    /// The field `index` names of an object, or the item it names of an array.
    pub fn get<I: FvIndex>(&self, index: I) -> Option<&Fv> {
        index.index_into(self)
    }

    /// The value a JSON pointer (`/a/b/0`) leads to, as `serde_json` reads one.
    pub fn pointer(&self, pointer: &str) -> Option<&Fv> {
        if pointer.is_empty() {
            return Some(self);
        }
        if !pointer.starts_with('/') {
            return None;
        }
        pointer
            .split('/')
            .skip(1)
            .map(|token| token.replace("~1", "/").replace("~0", "~"))
            .try_fold(self, |target, token| match target {
                Fv::Object(object) => object.get(&token),
                Fv::Array(items) => parse_index(&token).and_then(|index| items.get(index)),
                _ => None,
            })
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Fv::Null)
    }

    pub fn is_boolean(&self) -> bool {
        matches!(self, Fv::Bool(_))
    }

    pub fn is_number(&self) -> bool {
        matches!(self, Fv::U64(_) | Fv::I64(_) | Fv::F64(_))
    }

    /// Whether the number is an integer that fits `i64`, as `serde_json` tests it.
    pub fn is_i64(&self) -> bool {
        self.as_i64().is_some()
    }

    /// Whether the number is a non-negative integer.
    pub fn is_u64(&self) -> bool {
        matches!(self, Fv::U64(_))
    }

    /// Whether the number is a float.
    pub fn is_f64(&self) -> bool {
        matches!(self, Fv::F64(_))
    }

    pub fn is_string(&self) -> bool {
        matches!(self, Fv::Str(_))
    }

    pub fn is_array(&self) -> bool {
        matches!(self, Fv::Array(_))
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Fv::Object(_))
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Fv::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The number as `u64` where `serde_json` would give one: a non-negative integer.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Fv::U64(n) => Some(*n),
            _ => None,
        }
    }

    /// The number as `i64` where `serde_json` would give one: an integer that fits.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Fv::I64(n) => Some(*n),
            Fv::U64(n) => i64::try_from(*n).ok(),
            _ => None,
        }
    }

    /// Any number as `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Fv::F64(n) => Some(*n),
            Fv::U64(n) => Some(*n as f64),
            Fv::I64(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Fv::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Fv]> {
        match self {
            Fv::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&Obj> {
        match self {
            Fv::Object(object) => Some(object),
            _ => None,
        }
    }

    /// How many values this holds, itself and everything inside it.
    pub fn node_count(&self) -> usize {
        1 + match self {
            Fv::Array(items) => items.iter().map(Fv::node_count).sum(),
            Fv::Object(object) => object.values.iter().map(Fv::node_count).sum(),
            _ => 0,
        }
    }
}

impl From<bool> for Fv {
    fn from(b: bool) -> Fv {
        Fv::Bool(b)
    }
}

macro_rules! from_unsigned {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Fv {
            fn from(n: $ty) -> Fv {
                Fv::U64(n as u64)
            }
        }
    )*};
}

macro_rules! from_signed {
    ($($ty:ty),*) => {$(
        impl From<$ty> for Fv {
            fn from(n: $ty) -> Fv {
                Fv::from_i64(n as i64)
            }
        }
    )*};
}

from_unsigned!(u8, u16, u32, u64, usize);
from_signed!(i8, i16, i32, i64, isize);

impl From<f64> for Fv {
    fn from(n: f64) -> Fv {
        Fv::from_f64(n)
    }
}

impl From<f32> for Fv {
    fn from(n: f32) -> Fv {
        Fv::from_f64(f64::from(n))
    }
}

impl From<&str> for Fv {
    fn from(s: &str) -> Fv {
        Fv::Str(s.into())
    }
}

impl From<String> for Fv {
    fn from(s: String) -> Fv {
        Fv::Str(s.into_boxed_str())
    }
}

impl fmt::Display for Fv {
    /// The compact JSON of the value, as `serde_json::Value` prints it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.to_json(), f)
    }
}

impl Serialize for Fv {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Fv::Null => serializer.serialize_unit(),
            Fv::Bool(b) => serializer.serialize_bool(*b),
            Fv::U64(n) => serializer.serialize_u64(*n),
            Fv::I64(n) => serializer.serialize_i64(*n),
            Fv::F64(n) => serializer.serialize_f64(*n),
            Fv::Str(s) => serializer.serialize_str(s),
            Fv::Array(items) => {
                let mut seq = serializer.serialize_seq(Some(items.len()))?;
                for item in items.iter() {
                    seq.serialize_element(item)?;
                }
                seq.end()
            }
            Fv::Object(object) => {
                let mut map = serializer.serialize_map(Some(object.len()))?;
                for (name, item) in object.iter() {
                    map.serialize_entry(name, item)?;
                }
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Fv {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Fv, D::Error> {
        Ok(Fv::from_value(&Value::deserialize(deserializer)?))
    }
}

impl PartialEq<bool> for Fv {
    fn eq(&self, other: &bool) -> bool {
        self.as_bool() == Some(*other)
    }
}

impl PartialEq<str> for Fv {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == Some(other)
    }
}

impl PartialEq<&str> for Fv {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == Some(*other)
    }
}

impl PartialEq<String> for Fv {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == Some(other.as_str())
    }
}

macro_rules! equals_signed {
    ($($ty:ty),*) => {$(
        impl PartialEq<$ty> for Fv {
            fn eq(&self, other: &$ty) -> bool {
                self.as_i64() == i64::try_from(*other).ok()
            }
        }
    )*};
}

macro_rules! equals_unsigned {
    ($($ty:ty),*) => {$(
        impl PartialEq<$ty> for Fv {
            fn eq(&self, other: &$ty) -> bool {
                self.as_u64() == u64::try_from(*other).ok()
            }
        }
    )*};
}

macro_rules! equals_float {
    ($($ty:ty),*) => {$(
        impl PartialEq<$ty> for Fv {
            fn eq(&self, other: &$ty) -> bool {
                self.as_f64() == Some(f64::from(*other))
            }
        }
    )*};
}

equals_signed!(i8, i16, i32, i64, isize);
equals_unsigned!(u8, u16, u32, u64, usize);
equals_float!(f32, f64);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn round_trip(value: &Value) -> Value {
        Fv::from_json(value, &mut NameTable::default()).to_json()
    }

    // How a conversion can lose what a graph holds (#1694): a number changes
    // kind, so that `1` and `1.0` or a negative zero compare or print
    // differently; the key order changes, which every dump of a graph shows; two
    // lists of names are told apart by their concatenation; an object that
    // should share its names does not, and the saving is gone.

    #[test]
    fn a_number_keeps_the_kind_serde_json_held_it_in() {
        for value in [
            json!(0),
            json!(1),
            json!(-1),
            json!(u64::MAX),
            json!(i64::MIN),
            json!(1.0),
            json!(-0.0),
            json!(0.1),
            json!(1e300),
        ] {
            let back = round_trip(&value);
            assert_eq!(back, value);
            assert_eq!(back.to_string(), value.to_string());
        }
        assert_ne!(round_trip(&json!(1.0)), json!(1));
        assert_ne!(round_trip(&json!(1)), json!(1.0));
    }

    #[test]
    fn an_object_comes_back_with_serde_jsons_key_order() {
        let value = json!({"b": 1, "a": {"y": [1, 2], "x": null}, "ab": "s"});
        let back = round_trip(&value);
        assert_eq!(back.to_string(), value.to_string());
        assert_eq!(
            back.to_string(),
            r#"{"a":{"x":null,"y":[1,2]},"ab":"s","b":1}"#
        );
    }

    #[test]
    fn empty_and_odd_containers_and_strings_survive() {
        for value in [
            json!({}),
            json!([]),
            json!([[], {}, [[]]]),
            json!(""),
            json!("é\u{0}\u{1F600}\"\\"),
            json!({"": 1}),
            json!(null),
            json!(true),
        ] {
            assert_eq!(round_trip(&value), value);
        }
    }

    #[test]
    fn objects_with_the_same_names_share_one_list_and_others_do_not() {
        let mut table = NameTable::default();
        let first = Fv::from_json(&json!({"a": 1, "b": 2}), &mut table);
        let second = Fv::from_json(&json!({"a": 3, "b": "x"}), &mut table);
        let other = Fv::from_json(&json!({"a": 1, "c": 2}), &mut table);
        let (Fv::Object(first), Fv::Object(second), Fv::Object(other)) = (&first, &second, &other)
        else {
            panic!("objects");
        };
        assert!(Arc::ptr_eq(&first.names, &second.names));
        assert!(!Arc::ptr_eq(&first.names, &other.names));
        assert_eq!(table.len(), 2);
    }

    #[test]
    fn two_lists_with_the_same_letters_are_two_lists() {
        let mut table = NameTable::default();
        let one = Fv::from_json(&json!({"a": 1, "bc": 2}), &mut table);
        let two = Fv::from_json(&json!({"ab": 1, "c": 2}), &mut table);
        assert_eq!(table.len(), 2);
        assert_eq!(one.to_json(), json!({"a": 1, "bc": 2}));
        assert_eq!(two.to_json(), json!({"ab": 1, "c": 2}));
    }

    #[test]
    fn get_finds_a_field_and_node_count_counts_every_value() {
        let fv = Fv::from_json(
            &json!({"p": {"pointer_token": 7, "class_tag": 3}, "n": [1, 2]}),
            &mut NameTable::default(),
        );
        assert_eq!(fv.get("n").map(Fv::node_count), Some(3));
        assert_eq!(
            fv.get("p").and_then(|p| p.get("class_tag")),
            Some(&Fv::U64(3))
        );
        assert_eq!(fv.get("missing"), None);
        assert_eq!(Fv::U64(1).get("a"), None);
        assert_eq!(fv.node_count(), 1 + 3 + 3);
    }

    // How reading an `Fv` can differ from reading the `Value` it stands for: an
    // absent field panics or is not `Null`, an array position is read as a name,
    // a pointer is escaped or indexed another way, an integer is read through
    // the wrong sign, a comparison is true of another kind.

    #[test]
    fn indexing_gives_null_where_serde_json_does() {
        let fv = Fv::from_value(&json!({"a": [10, 20], "s": "x"}));
        assert_eq!(fv["a"][1], 20u64);
        assert!(fv["missing"].is_null());
        assert!(fv["a"][5].is_null());
        assert!(fv["a"]["name"].is_null());
        assert!(fv["s"][0].is_null());
        assert!(Fv::Null["a"].is_null());
        let name = String::from("s");
        assert_eq!(fv[&name], "x");
        assert_eq!(fv.get(&name[..]), Some(&Fv::Str("x".into())));
        assert_eq!(fv["a"].get(0), Some(&Fv::U64(10)));
    }

    #[test]
    fn a_pointer_unescapes_its_tokens_and_reads_array_positions_as_serde_json_does() {
        let value = json!({"a/b": {"c~d": [5, 6]}, "k": [[1], [2]], "01": 9});
        let fv = Fv::from_value(&value);
        for pointer in [
            "",
            "/a~1b",
            "/a~1b/c~0d",
            "/a~1b/c~0d/1",
            "/k/1/0",
            "/k/01",
            "/k/+1",
            "/k/2",
            "/01",
            "a",
            "/missing/x",
        ] {
            assert_eq!(
                fv.pointer(pointer).map(Fv::to_json).as_ref(),
                value.pointer(pointer),
                "pointer {pointer:?}"
            );
        }
    }

    #[test]
    fn numbers_read_through_the_sign_serde_json_reads_them_in() {
        let negative = Fv::from_i64(-3);
        let big = Fv::U64(u64::MAX);
        let small = Fv::U64(3);
        let float = Fv::F64(3.0);
        assert_eq!(negative.as_u64(), None);
        assert_eq!(negative.as_i64(), Some(-3));
        assert_eq!(big.as_i64(), None);
        assert_eq!(big.as_u64(), Some(u64::MAX));
        assert_eq!(small.as_i64(), Some(3));
        assert_eq!(float.as_i64(), None);
        assert_eq!(float.as_f64(), Some(3.0));
        assert_eq!(small.as_f64(), Some(3.0));
        assert_eq!(Fv::from_i64(7), Fv::U64(7));
        assert_eq!(Fv::from_f64(f64::NAN), Fv::Null);
        assert_eq!(Fv::from_f64(f64::INFINITY), Fv::Null);
    }

    #[test]
    fn a_comparison_is_true_only_of_the_kind_it_names() {
        assert!(Fv::Bool(false) == false);
        assert!(Fv::Null != false);
        assert!(Fv::U64(0) != false);
        assert!(Fv::U64(3) == 3);
        assert!(Fv::I64(-3) == -3);
        assert!(Fv::I64(-3) != 3u64);
        assert!(Fv::F64(2.5) == 2.5);
        assert!(Fv::U64(2) != 2.5);
        assert!(Fv::Str("x".into()) == "x");
        assert!(Fv::Str("x".into()) != "y");
        assert!(Fv::Null != "x");
    }

    #[test]
    fn serialisation_is_the_json_of_the_value_it_stands_for() {
        let value = json!({"b": [1, -2, 3.5, null, true, "s"], "a": {"z": {}, "y": []}});
        let fv = Fv::from_value(&value);
        assert_eq!(serde_json::to_string(&fv).unwrap(), value.to_string());
        assert_eq!(fv.to_string(), value.to_string());
        let again: Fv = serde_json::from_str(&value.to_string()).unwrap();
        assert_eq!(again, fv);
    }

    #[test]
    fn a_built_object_and_a_pointer_are_the_objects_json_makes() {
        let built = Fv::object([
            ("z", Fv::U64(1)),
            ("a", Fv::Str("x".into())),
            ("m", Fv::Null),
        ]);
        assert_eq!(built.to_json(), json!({"a": "x", "m": null, "z": 1}));
        assert_eq!(
            Fv::pointer_value(7, Some(3), 40).to_json(),
            json!({"pointer_token": 7, "class_tag": 3, "offset": 40})
        );
        assert_eq!(
            Fv::pointer_value(0, None, 0).to_json(),
            json!({"pointer_token": 0, "class_tag": null, "offset": 0})
        );
    }
}
