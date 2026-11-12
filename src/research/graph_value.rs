//! A compact value for a decoded object graph's fields (#1694).
//!
//! FACT it rests on: after the saved-mesh pass's other levers, 75% to 87% of
//! its peak is decoded graph nodes, and what makes them large is the container
//! and not the content. A `serde_json::Value::Object` is a `BTreeMap<String,
//! Value>`: its 11-slot leaf is 631 bytes however few fields it holds, every key
//! is a heap `String` of its own, and a pointer, an object of three numbers,
//! costs about 730 bytes (heap profiles on `rac_basic` 2023, #1694).
//! [`Fv`] holds the same content with an object's values in one slice and its
//! names once for every object that has them.
//!
//! This module is the type and its two conversions, `from_json` and `to_json`,
//! which are exact: the value that comes back is the one that went in, number
//! kinds (`1` and `1.0` differ), key order and all. Nothing reads a graph
//! through it yet; `probes/examples/probe_1694_compact_graph.rs` sizes it on
//! real graphs and checks the round trip on every object it reads.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde_json::{Map, Number, Value};

/// The names of an object's fields in the order `serde_json` iterates them,
/// shared by every object that has exactly these.
pub type Names = Arc<[Box<str>]>;

/// A graph value: the content of a `serde_json::Value`, laid out to cost little.
#[derive(Debug, Clone, PartialEq)]
pub enum Fv {
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
    /// The names of the fields, in order.
    pub fn names(&self) -> &[Box<str>] {
        &self.names
    }

    /// The values of the fields, in the order of [`Self::names`].
    pub fn values(&self) -> &[Fv] {
        &self.values
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

    fn intern(&mut self, keys: &[&str]) -> Names {
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

    /// The field `name` of an object, or none where this is not one or has no such field.
    pub fn get(&self, name: &str) -> Option<&Fv> {
        match self {
            Fv::Object(object) => object
                .names
                .iter()
                .position(|candidate| &**candidate == name)
                .map(|index| &object.values[index]),
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
}
