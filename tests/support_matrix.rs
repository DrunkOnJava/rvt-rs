//! Executable support-matrix contracts (Unified Repository Audit A3).
//!
//! COR-001 — matrix statuses are honest ceilings, not aspirational claims.
//! TEST-001 — schema validation + honesty invariants run in CI.
//! DOC-001 — docs/status.md references the checked-in matrix.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn load_json(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|err| panic!("parse {}: {err}", path.display()))
}

fn status_rank(status: &str) -> u8 {
    match status {
        "unsupported" => 0,
        "unknown" => 1,
        "experimental" => 2,
        "partial" => 3,
        "verified" => 4,
        other => panic!("unexpected status vocabulary entry: {other}"),
    }
}

fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    validate_at(schema, schema, value, "$")
}

fn validate_at(root: &Value, schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let resolved = resolve_ref(root, reference)?;
        return validate_at(root, resolved, value, path);
    }

    if let Some(constant) = schema.get("const") {
        if value != constant {
            return Err(format!("{path}: expected const {constant}, got {value}"));
        }
    }

    if let Some(enumerants) = schema.get("enum").and_then(Value::as_array) {
        if !enumerants.iter().any(|candidate| candidate == value) {
            return Err(format!("{path}: value {value:?} is not in enum"));
        }
    }

    if let Some(pattern) = schema.get("pattern").and_then(Value::as_str) {
        let Some(text) = value.as_str() else {
            return Err(format!("{path}: pattern applies only to strings"));
        };
        let re = regex_lite_is_match(pattern, text);
        if !re {
            return Err(format!("{path}: {text:?} does not match pattern {pattern}"));
        }
    }

    if let Some(expected) = schema.get("type") {
        validate_type(expected, value, path)?;
    }

    if let Some(minimum) = schema.get("minimum").and_then(Value::as_f64) {
        if let Some(number) = value.as_f64() {
            if number < minimum {
                return Err(format!("{path}: {number} is below minimum {minimum}"));
            }
        }
    }

    if let Some(maximum) = schema.get("maximum").and_then(Value::as_f64) {
        if let Some(number) = value.as_f64() {
            if number > maximum {
                return Err(format!("{path}: {number} is above maximum {maximum}"));
            }
        }
    }

    if let Some(object) = value.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(key) {
                    return Err(format!("{path}: missing required property {key}"));
                }
            }
        }

        let properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        for (key, property_schema) in &properties {
            if let Some(child) = object.get(key) {
                validate_at(root, property_schema, child, &format!("{path}.{key}"))?;
            }
        }

        if let Some(additional) = schema.get("additionalProperties") {
            if additional.as_bool() == Some(false) {
                for key in object.keys() {
                    if !properties.contains_key(key) {
                        return Err(format!("{path}: unexpected property {key}"));
                    }
                }
            } else if additional.is_object() {
                for (key, child) in object {
                    if !properties.contains_key(key) {
                        validate_at(root, additional, child, &format!("{path}.{key}"))?;
                    }
                }
            }
        }
    }

    if let Some(array) = value.as_array() {
        if let Some(min_items) = schema.get("minItems").and_then(Value::as_u64) {
            if (array.len() as u64) < min_items {
                return Err(format!(
                    "{path}: array length {} is below minItems {min_items}",
                    array.len()
                ));
            }
        }
        if schema.get("uniqueItems").and_then(Value::as_bool) == Some(true) {
            let mut seen = BTreeSet::new();
            for item in array {
                let key = item.to_string();
                if !seen.insert(key) {
                    return Err(format!("{path}: array items are not unique"));
                }
            }
        }
        if let Some(item_schema) = schema.get("items") {
            for (idx, child) in array.iter().enumerate() {
                validate_at(root, item_schema, child, &format!("{path}[{idx}]"))?;
            }
        }
    }

    Ok(())
}

/// Minimal pattern checks used by the support-matrix schema (avoid a new crate dep).
fn regex_lite_is_match(pattern: &str, text: &str) -> bool {
    match pattern {
        "^[0-9]{4}-[0-9]{2}-[0-9]{2}$" => {
            let bytes = text.as_bytes();
            bytes.len() == 10
                && bytes[0..4].iter().all(u8::is_ascii_digit)
                && bytes[4] == b'-'
                && bytes[5..7].iter().all(u8::is_ascii_digit)
                && bytes[7] == b'-'
                && bytes[8..10].iter().all(u8::is_ascii_digit)
        }
        "^[a-z][a-z0-9-]*$" => {
            let mut chars = text.chars();
            match chars.next() {
                Some(c) if c.is_ascii_lowercase() => {
                    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                }
                _ => false,
            }
        }
        _ => panic!("support_matrix test: unsupported pattern {pattern}"),
    }
}

fn validate_type(expected: &Value, value: &Value, path: &str) -> Result<(), String> {
    match expected {
        Value::String(kind) => validate_type_name(kind, value, path),
        Value::Array(kinds) => {
            if kinds
                .iter()
                .filter_map(Value::as_str)
                .any(|kind| validate_type_name(kind, value, path).is_ok())
            {
                Ok(())
            } else {
                Err(format!(
                    "{path}: value {value:?} did not match any allowed type"
                ))
            }
        }
        _ => Err(format!(
            "{path}: schema type must be a string or string array"
        )),
    }
}

fn validate_type_name(kind: &str, value: &Value, path: &str) -> Result<(), String> {
    let ok = match kind {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "number" => value.as_f64().is_some(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        other => return Err(format!("{path}: unsupported schema type {other}")),
    };
    if ok {
        Ok(())
    } else {
        Err(format!("{path}: expected {kind}, got {value:?}"))
    }
}

fn resolve_ref<'a>(root: &'a Value, reference: &str) -> Result<&'a Value, String> {
    let Some(pointer) = reference.strip_prefix('#') else {
        return Err(format!("unsupported non-local ref {reference}"));
    };
    root.pointer(pointer)
        .ok_or_else(|| format!("unresolved ref {reference}"))
}

#[test]
fn support_matrix_schema_is_valid_json_schema_document() {
    let schema = load_json("docs/schemas/support-matrix.schema.json");
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    assert!(schema["$id"].as_str().unwrap().contains("support-matrix"));
    assert_eq!(schema["title"], "rvt-rs executable support matrix");
    assert_eq!(schema["type"], "object");
}

#[test]
fn seeded_support_matrix_validates_and_respects_honesty_invariants() {
    let schema = load_json("docs/schemas/support-matrix.schema.json");
    let matrix = load_json("docs/support-matrix.json");

    validate(&schema, &matrix).expect("support-matrix.json must validate against its schema");

    let controls = matrix["audit_controls"]
        .as_array()
        .expect("audit_controls array");
    for required in ["COR-001", "TEST-001", "DOC-001", "A3"] {
        assert!(
            controls
                .iter()
                .any(|value| value.as_str() == Some(required)),
            "matrix must declare audit control {required}"
        );
    }

    let never_verified: Vec<&str> = matrix["honesty_invariants"]["never_verified_capability_ids"]
        .as_array()
        .expect("never_verified_capability_ids")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert!(
        never_verified.contains(&"converter-grade-rvt-ifc"),
        "COR-001: converter-grade claim must be listed as never-verified"
    );
    assert!(
        never_verified.contains(&"typed-project-elements"),
        "COR-001: generic typed project elements must be listed as never-verified"
    );

    let max_status: BTreeMap<&str, &str> =
        matrix["honesty_invariants"]["max_status_by_capability_id"]
            .as_object()
            .expect("max_status_by_capability_id")
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().expect("status string")))
            .collect();

    let capabilities = matrix["capabilities"]
        .as_array()
        .expect("capabilities array");
    let mut seen = BTreeMap::new();
    for capability in capabilities {
        let id = capability["id"].as_str().expect("capability id");
        let status = capability["status"].as_str().expect("capability status");
        assert!(
            seen.insert(id, status).is_none(),
            "duplicate capability id {id}"
        );

        if never_verified.contains(&id) {
            assert_ne!(
                status, "verified",
                "COR-001/TEST-001: capability {id} must not be verified"
            );
        }

        if let Some(ceiling) = max_status.get(id) {
            assert!(
                status_rank(status) <= status_rank(ceiling),
                "TEST-001: capability {id} status {status} exceeds seeded ceiling {ceiling}"
            );
        }

        let notes = capability["notes"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase();
        if notes.contains("converter-grade") || notes.contains("production revit ifc converter") {
            assert_ne!(
                status, "verified",
                "COR-001: notes that discuss converter-grade must not be verified for {id}"
            );
        }
    }

    assert_eq!(
        seen.get("converter-grade-rvt-ifc").copied(),
        Some("unsupported"),
        "converter-grade RVT-to-IFC must remain unsupported"
    );
    assert_eq!(
        seen.get("typed-door-window").copied(),
        Some("unsupported"),
        "typed Door/Window must remain unsupported"
    );
    assert_eq!(
        seen.get("schema-field-wall").copied(),
        Some("unsupported"),
        "schema-field Wall must remain unsupported"
    );
    assert_ne!(
        seen.get("typed-project-elements").copied(),
        Some("verified"),
        "typed project elements must not claim verified"
    );
    assert_eq!(
        seen.get("export-source-coverage").copied(),
        Some("partial"),
        "A10 coverage fractions are partially measured (fail-closed where unknown)"
    );
}

#[test]
fn status_docs_reference_executable_support_matrix() {
    let status =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/status.md"))
            .expect("read docs/status.md");
    assert!(
        status.contains("support-matrix.json"),
        "DOC-001: docs/status.md must reference docs/support-matrix.json"
    );
    assert!(
        status.contains("COR-001") || status.contains("executable support matrix"),
        "DOC-001: docs/status.md should point readers at the executable matrix / audit control"
    );
}
