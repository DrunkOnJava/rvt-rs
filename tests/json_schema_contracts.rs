mod common;

use common::{sample_for_year, samples_dir};
use serde_json::Value;
use std::{path::Path, process::Command};

fn corpus_available() -> bool {
    sample_for_year(2024).exists()
        && sample_for_year(2023).exists()
        && sample_for_year(2022).exists()
}

fn load_schema(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("docs")
        .join("schemas")
        .join(name);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("read schema {}: {err}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|err| panic!("parse schema {}: {err}", path.display()))
}

fn command_json(command: &str, args: &[&std::ffi::OsStr]) -> Value {
    let output = Command::new(command)
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("run {command}: {err}"));
    assert!(
        output.status.success(),
        "{command} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("parse {command} JSON: {err}"))
}

fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    validate_at(schema, schema, value, "$")
}

fn validate_at(root: &Value, schema: &Value, value: &Value, path: &str) -> Result<(), String> {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let resolved = resolve_ref(root, reference)?;
        return validate_at(root, resolved, value, path);
    }

    if let Some(options) = schema.get("anyOf").and_then(Value::as_array) {
        if options
            .iter()
            .any(|option| validate_at(root, option, value, path).is_ok())
        {
            return Ok(());
        }
        return Err(format!("{path}: did not match any anyOf branch"));
    }

    if let Some(expected) = schema.get("type") {
        validate_type(expected, value, path)?;
    }

    if let Some(enumerants) = schema.get("enum").and_then(Value::as_array) {
        if !enumerants.iter().any(|candidate| candidate == value) {
            return Err(format!("{path}: value {value:?} is not in enum"));
        }
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

        if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
            for (key, property_schema) in properties {
                if let Some(child) = object.get(key) {
                    validate_at(root, property_schema, child, &format!("{path}.{key}"))?;
                }
            }
        }

        if let Some(additional) = schema.get("additionalProperties") {
            if additional.is_object() {
                let properties = schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                for (key, child) in object {
                    if !properties.contains_key(key) {
                        validate_at(root, additional, child, &format!("{path}.{key}"))?;
                    }
                }
            }
        }
    }

    if let Some(array) = value.as_array() {
        if let Some(item_schema) = schema.get("items") {
            for (idx, child) in array.iter().enumerate() {
                validate_at(root, item_schema, child, &format!("{path}[{idx}]"))?;
            }
        }
    }

    Ok(())
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
fn checked_in_schemas_are_valid_json_schema_documents() {
    for name in [
        "summary.schema.json",
        "schema-diagnostics.schema.json",
        "element-records.schema.json",
        "export-diagnostics.schema.json",
        "corpus-report.schema.json",
    ] {
        let schema = load_schema(name);
        assert_eq!(
            schema["$schema"], "https://json-schema.org/draft/2020-12/schema",
            "{name} should declare the draft it uses"
        );
        assert!(schema["$id"].is_string(), "{name} should declare an id");
        assert!(schema["title"].is_string(), "{name} should declare a title");
        assert_eq!(schema["type"], "object", "{name} should describe an object");
    }
}

#[test]
fn cli_json_outputs_validate_against_stable_schemas() {
    if !corpus_available() {
        eprintln!(
            "skipping JSON schema contract test: family corpus missing at {}",
            samples_dir().display()
        );
        return;
    }

    let sample_2024 = sample_for_year(2024);
    let sample_2023 = sample_for_year(2023);
    let sample_2022 = sample_for_year(2022);

    let summary = command_json(
        env!("CARGO_BIN_EXE_rvt-info"),
        &[
            sample_2024.as_os_str(),
            std::ffi::OsStr::new("-f"),
            std::ffi::OsStr::new("json"),
        ],
    );
    validate(&load_schema("summary.schema.json"), &summary).expect("summary schema validation");

    let schema_diagnostics = command_json(
        env!("CARGO_BIN_EXE_rvt-schema"),
        &[
            sample_2024.as_os_str(),
            std::ffi::OsStr::new("-f"),
            std::ffi::OsStr::new("json"),
            std::ffi::OsStr::new("--diagnostics"),
        ],
    );
    validate(
        &load_schema("schema-diagnostics.schema.json"),
        &schema_diagnostics,
    )
    .expect("schema diagnostics schema validation");

    let element_records = command_json(
        env!("CARGO_BIN_EXE_rvt-doc"),
        &[
            sample_2024.as_os_str(),
            std::ffi::OsStr::new("--json"),
            std::ffi::OsStr::new("--redact"),
        ],
    );
    validate(
        &load_schema("element-records.schema.json"),
        &element_records,
    )
    .expect("element records schema validation");

    let diagnostics_path = std::env::temp_dir().join(format!(
        "rvt-schema-contract-{}-diagnostics.json",
        std::process::id()
    ));
    let ifc_path = std::env::temp_dir().join(format!(
        "rvt-schema-contract-{}-out.ifc",
        std::process::id()
    ));
    let ifc_output = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .args([
            sample_2024.as_os_str(),
            std::ffi::OsStr::new("-o"),
            ifc_path.as_os_str(),
            std::ffi::OsStr::new("--diagnostics"),
            diagnostics_path.as_os_str(),
        ])
        .output()
        .expect("run rvt-ifc diagnostics");
    assert!(
        ifc_output.status.success(),
        "rvt-ifc failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&ifc_output.stdout),
        String::from_utf8_lossy(&ifc_output.stderr)
    );
    let export_diagnostics: Value = serde_json::from_str(
        &std::fs::read_to_string(&diagnostics_path).expect("read diagnostics JSON"),
    )
    .expect("parse diagnostics JSON");
    let _ = std::fs::remove_file(&diagnostics_path);
    let _ = std::fs::remove_file(&ifc_path);
    validate(
        &load_schema("export-diagnostics.schema.json"),
        &export_diagnostics,
    )
    .expect("export diagnostics schema validation");

    let corpus_report = command_json(
        env!("CARGO_BIN_EXE_rvt-corpus"),
        &[
            sample_2022.as_os_str(),
            sample_2023.as_os_str(),
            sample_2024.as_os_str(),
            std::ffi::OsStr::new("-f"),
            std::ffi::OsStr::new("json"),
        ],
    );
    validate(&load_schema("corpus-report.schema.json"), &corpus_report)
        .expect("corpus report schema validation");
}
