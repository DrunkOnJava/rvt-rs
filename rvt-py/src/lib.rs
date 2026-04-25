//! Python bindings via pyo3. Gated behind the `python` Cargo feature
//! so the default Rust build is unaffected.
//!
//! Usage from Python (after installing the wheel that `maturin build
//! --features python` produces):
//!
//! ```python
//! import rvt
//!
//! f = rvt.RevitFile("sample.rfa")
//! print(f.version)              # 2024
//! print(f.part_atom_title)      # "Kitchen Cabinet — Base"
//! print(f.stream_names())       # ["BasicFileInfo", "Contents", ...]
//!
//! doc = f.read_adocument()      # dict | None
//! if doc is not None:
//!     for field in doc["fields"]:
//!         print(field["name"], field["kind"], field.get("id"))
//!
//! ifc_text = f.write_ifc()      # spec-valid IFC4 STEP as str
//! ```
//!
//! Design principle: expose only the stable high-level surface —
//! metadata, walker-read ADocument, IFC export. The low-level
//! byte-pattern / FieldType machinery stays in Rust; Python callers
//! get dicts and strings, no wrapper types to learn.

#![allow(non_local_definitions)]
// pyo3's #[pyclass]/#[pymethods]/#[pyfunction] macros expand into code
// that calls the pyo3 runtime's unsafe helpers inside otherwise-safe
// user function signatures. The Rust 2024 `unsafe_op_in_unsafe_fn` lint
// flags every one of those calls; pyo3 0.22 does not yet wrap them in
// explicit `unsafe {}` blocks. Similarly the macros generate `.into()`
// calls on already-PyErr values (clippy::useless_conversion). Both are
// tracked upstream (pyo3#4382, pyo3#4448) — silence them at the module
// level so the main Rust build stays clean with `-D warnings`.
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::useless_conversion)]

use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

use rvt::{RevitFile as RustRevitFile, elem_table, ifc, walker};

fn to_py_io<E: std::fmt::Display>(e: E) -> PyErr {
    PyIOError::new_err(e.to_string())
}
fn to_py_val<E: std::fmt::Display>(e: E) -> PyErr {
    PyValueError::new_err(e.to_string())
}

fn parse_export_quality_mode(mode: &str) -> PyResult<ifc::ExportQualityMode> {
    ifc::ExportQualityMode::parse(mode).map_err(to_py_val)
}

fn write_ifc_with_quality_mode(
    rf: &mut RustRevitFile,
    mode: ifc::ExportQualityMode,
) -> PyResult<String> {
    let result = ifc::RvtDocExporter
        .export_with_diagnostics(rf)
        .map_err(to_py_val)?;
    mode.validate(&result.diagnostics).map_err(to_py_val)?;
    Ok(ifc::write_step(&result.model))
}

/// Shared serialiser: ADocumentInstance → Python dict. Used by the
/// three ADocument accessors (plain / strict / lossy) so they all
/// return the same shape.
fn instance_to_dict<'py>(
    py: Python<'py>,
    inst: &walker::ADocumentInstance,
) -> PyResult<Bound<'py, PyDict>> {
    let d = PyDict::new(py);
    d.set_item("entry_offset", inst.entry_offset)?;
    d.set_item("version", inst.version)?;

    let fields = PyList::empty(py);
    for (name, value) in &inst.fields {
        let fd = PyDict::new(py);
        fd.set_item("name", name)?;
        match value {
            walker::InstanceField::Pointer { raw } => {
                fd.set_item("kind", "pointer")?;
                fd.set_item("slot_a", raw[0])?;
                fd.set_item("slot_b", raw[1])?;
            }
            walker::InstanceField::ElementId { tag, id } => {
                fd.set_item("kind", "element_id")?;
                fd.set_item("tag", *tag)?;
                fd.set_item("id", *id)?;
            }
            walker::InstanceField::RefContainer { col_a, col_b } => {
                fd.set_item("kind", "ref_container")?;
                fd.set_item("count", col_a.len())?;
                fd.set_item("col_a", col_a.clone())?;
                fd.set_item("col_b", col_b.clone())?;
            }
            walker::InstanceField::Integer {
                value,
                signed,
                size,
            } => {
                fd.set_item("kind", "integer")?;
                fd.set_item("value", *value)?;
                fd.set_item("signed", *signed)?;
                fd.set_item("size", *size)?;
            }
            walker::InstanceField::Float { value, size } => {
                fd.set_item("kind", "float")?;
                fd.set_item("value", *value)?;
                fd.set_item("size", *size)?;
            }
            walker::InstanceField::Bool(v) => {
                fd.set_item("kind", "bool")?;
                fd.set_item("value", *v)?;
            }
            walker::InstanceField::Guid(bytes) => {
                fd.set_item("kind", "guid")?;
                fd.set_item("bytes", bytes.to_vec())?;
            }
            walker::InstanceField::String(s) => {
                fd.set_item("kind", "string")?;
                fd.set_item("value", s.as_str())?;
            }
            walker::InstanceField::Vector(items) => {
                fd.set_item("kind", "vector")?;
                fd.set_item("len", items.len())?;
            }
            walker::InstanceField::Bytes(b) => {
                fd.set_item("kind", "bytes")?;
                fd.set_item("len", b.len())?;
            }
        }
        fields.append(fd)?;
    }
    d.set_item("fields", fields)?;
    Ok(d)
}

/// Opened Revit file — the primary Python entry point.
///
/// Constructed with a filesystem path. Raises `IOError` on missing
/// files, non-CFB input, or read errors.
#[pyclass(name = "RevitFile", module = "rvt")]
struct PyRevitFile {
    inner: RustRevitFile,
}

#[pymethods]
impl PyRevitFile {
    /// Open a Revit file with optional size limits.
    ///
    /// `max_file_bytes`, `max_stream_bytes`, and `max_inflate_bytes`
    /// cap the resources the reader will use. Each defaults to the
    /// Rust-side `OpenLimits::default()` values (2 GiB file, 256 MiB
    /// stream, 256 MiB inflate). Hostile input that would otherwise
    /// force multi-GB allocations is rejected up-front.
    #[new]
    #[pyo3(signature = (path, max_file_bytes=None, max_stream_bytes=None, max_inflate_bytes=None))]
    fn new(
        path: &str,
        max_file_bytes: Option<u64>,
        max_stream_bytes: Option<u64>,
        max_inflate_bytes: Option<usize>,
    ) -> PyResult<Self> {
        let default = rvt::reader::OpenLimits::default();
        let limits = rvt::reader::OpenLimits {
            max_file_bytes: max_file_bytes.unwrap_or(default.max_file_bytes),
            max_stream_bytes: max_stream_bytes.unwrap_or(default.max_stream_bytes),
            inflate_limits: rvt::compression::InflateLimits {
                max_output_bytes: max_inflate_bytes
                    .unwrap_or(default.inflate_limits.max_output_bytes),
            },
        };
        let inner = RustRevitFile::open_with_limits(path, limits).map_err(to_py_io)?;
        Ok(Self { inner })
    }

    /// Revit release year (e.g. 2024), or `None` if `BasicFileInfo`
    /// can't be parsed.
    #[getter]
    fn version(&mut self) -> Option<u32> {
        self.inner.basic_file_info().ok().map(|b| b.version)
    }

    /// Original file path recorded at save time on the creator's
    /// machine. Often contains Windows-style paths; use `--redact`
    /// in the CLI equivalent if you're surfacing to end users.
    #[getter]
    fn original_path(&mut self) -> Option<String> {
        self.inner
            .basic_file_info()
            .ok()
            .and_then(|b| b.original_path)
    }

    /// Revit build tag (e.g. "20230308_1635(x64)"), if present.
    #[getter]
    fn build(&mut self) -> Option<String> {
        self.inner.basic_file_info().ok().and_then(|b| b.build)
    }

    /// Document GUID from `BasicFileInfo`, if present.
    #[getter]
    fn guid(&mut self) -> Option<String> {
        self.inner.basic_file_info().ok().and_then(|b| b.guid)
    }

    /// PartAtom document title, if the file carries one.
    #[getter]
    fn part_atom_title(&mut self) -> Option<String> {
        self.inner.part_atom().ok().and_then(|p| p.title)
    }

    /// All OLE stream paths (sorted, `/`-separated on every OS).
    fn stream_names(&self) -> Vec<String> {
        self.inner.stream_names()
    }

    /// Raw bytes of the named OLE stream. Accepts either a
    /// leading-slash path (`"/Formats/Latest"`) or the bare name
    /// (`"Formats/Latest"`) — both resolve the same way. Returns the
    /// raw, *compressed* bytes on streams that use truncated-gzip
    /// framing (most of them); callers wanting decompressed content
    /// should pipe through the Rust `compression::inflate_at`
    /// equivalent (pending its own Python binding).
    ///
    /// Raises `IOError` when the stream name doesn't exist. Use
    /// `stream_names()` first to enumerate what's readable.
    fn read_stream<'py>(&mut self, py: Python<'py>, name: &str) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.inner.read_stream(name).map_err(to_py_io)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Required streams that are absent — empty list means the file
    /// looks like a Revit file. Useful for validation in scripts.
    fn missing_required_streams(&self) -> Vec<String> {
        self.inner
            .missing_required_streams()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }

    /// Decode the schema from `Formats/Latest` and return a count of
    /// classes + fields. The full schema is large; callers that want
    /// it should use `schema_json()`.
    fn schema_summary<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let schema = self.inner.schema().map_err(to_py_val)?;
        let d = PyDict::new(py);
        let total_fields: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
        d.set_item("classes", schema.classes.len())?;
        d.set_item("fields", total_fields)?;
        d.set_item("cpp_types", schema.cpp_types.len())?;
        Ok(d)
    }

    /// Full `BasicFileInfo` as a JSON string. Single-call equivalent
    /// of the four individual getters (`version`, `original_path`,
    /// `build`, `guid`) plus any future fields added to the Rust
    /// `BasicFileInfo` struct. Parse with `json.loads()` in Python.
    ///
    /// Returns `None` if the `BasicFileInfo` stream can't be parsed.
    fn basic_file_info_json(&mut self) -> PyResult<Option<String>> {
        let Some(bfi) = self.inner.basic_file_info().ok() else {
            return Ok(None);
        };
        Ok(Some(serde_json::to_string(&bfi).map_err(to_py_val)?))
    }

    /// Strict variant of [`basic_file_info_json`] (API-14). Raises
    /// `ValueError` if the stream is missing or parse fails, instead
    /// of returning `None`. Use when downstream Python code needs
    /// to fail loud on malformed input.
    ///
    /// Python:
    /// ```python
    /// try:
    ///     bfi = json.loads(rf.basic_file_info_json_strict())
    /// except ValueError as e:
    ///     # stream missing or parse failure
    ///     ...
    /// ```
    fn basic_file_info_json_strict(&mut self) -> PyResult<String> {
        let bfi = self.inner.basic_file_info().map_err(to_py_val)?;
        serde_json::to_string(&bfi).map_err(to_py_val)
    }

    /// Full `PartAtom` as a JSON string. Superset of the
    /// `part_atom_title` getter — also includes `id`, `updated`,
    /// `taxonomies`, `categories`, `omniclass`, and `raw_xml` (the
    /// original XML bytes for lossless downstream reuse). Parse with
    /// `json.loads()` in Python.
    ///
    /// Returns `None` if the file has no PartAtom stream (common on
    /// project `.rvt` files; family `.rfa` files almost always carry
    /// one).
    fn part_atom_json(&mut self) -> PyResult<Option<String>> {
        let Some(pa) = self.inner.part_atom().ok() else {
            return Ok(None);
        };
        Ok(Some(serde_json::to_string(&pa).map_err(to_py_val)?))
    }

    /// Full schema as a JSON string. The Rust-side `SchemaTable` type
    /// already derives `Serialize`, so this is zero-copy relative to
    /// the in-memory schema. Parse with `json.loads()` in Python to
    /// get a structured dict equivalent to `rvt::formats::SchemaTable`.
    ///
    /// Return shape (after `json.loads`):
    ///
    /// ```python
    /// {
    ///     "classes": [
    ///         {
    ///             "name": "ADocument",
    ///             "offset": 123,
    ///             "fields": [
    ///                 {"name": "m_elemTable", "cpp_type": "...",
    ///                  "field_type": {"ElementId": null}},
    ///                 ...
    ///             ],
    ///             "tag": 4,
    ///             "parent": null,
    ///             "declared_field_count": 13,
    ///             "was_parent_only": false,
    ///             "ancestor_tag": null,
    ///         },
    ///         ...
    ///     ],
    ///     "cpp_types": ["ElementId", "std::pair< ElementId, double >", ...],
    ///     "skipped_records": 0,
    /// }
    /// ```
    ///
    /// Typical 11-release corpus schema is ~395 classes / 13,570
    /// fields — the JSON string is on the order of 1-2 MB. For just
    /// counts, prefer `schema_summary()`.
    fn schema_json(&mut self) -> PyResult<String> {
        let schema = self.inner.schema().map_err(to_py_val)?;
        serde_json::to_string(&schema).map_err(to_py_val)
    }

    /// Run the Layer-5a walker and return ADocument's instance fields
    /// as a Python dict, or `None` if the entry-point detector can't
    /// confidently locate an ADocument record in this file.
    ///
    /// Return shape when present:
    ///
    /// ```python
    /// {
    ///     "entry_offset": int,     # byte offset in decompressed Global/Latest
    ///     "version": int,          # Revit release year
    ///     "fields": [
    ///         {"name": "m_elemTable", "kind": "pointer", "slot_a": 0, "slot_b": 0},
    ///         {"name": "m_appInfoArr", "kind": "ref_container", "count": 12, ...},
    ///         {"name": "m_ownerFamilyId", "kind": "element_id", "tag": 0, "id": 27},
    ///         ...
    ///     ],
    /// }
    /// ```
    fn read_adocument<'py>(&mut self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyDict>>> {
        let Some(inst) = walker::read_adocument(&mut self.inner).map_err(to_py_io)? else {
            return Ok(None);
        };
        instance_to_dict(py, &inst).map(Some)
    }

    /// Strict variant of [`read_adocument`] (API-15). Raises
    /// `ValueError` if the entry-point detector couldn't confidently
    /// locate the record, OR if any field fell back to raw bytes.
    /// Contract: success means every field decoded cleanly — mirrors
    /// the Rust `walker::read_adocument_strict` bar.
    fn read_adocument_strict<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let inst = walker::read_adocument_strict(&mut self.inner).map_err(to_py_val)?;
        instance_to_dict(py, &inst)
    }

    /// Lossy variant of [`read_adocument`] with a diagnostics
    /// accumulator exposed to Python (API-16). Returns a dict with
    /// `value` (the ADocument dict), `complete` (bool),
    /// `partial_fields` (list of field names that fell back to raw
    /// bytes), `failed_streams` (list), and `confidence` (float or
    /// None — ratio of typed fields).
    ///
    /// Raises `OSError` for stream-level failures (BFI unreadable,
    /// Global/Latest inflate failure, schema parse failure) — same
    /// hard-error cases as the Rust equivalent.
    ///
    /// Python:
    /// ```python
    /// d = rf.read_adocument_lossy()
    /// if d["complete"]:
    ///     print("clean decode", d["value"])
    /// else:
    ///     print(f"partial: {d['confidence']:.0%} typed")
    ///     print(d["partial_fields"])
    /// ```
    fn read_adocument_lossy<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let decoded = walker::read_adocument_lossy(&mut self.inner).map_err(to_py_io)?;
        let out = PyDict::new(py);
        let value = instance_to_dict(py, &decoded.value)?;
        out.set_item("value", value)?;
        out.set_item("complete", decoded.complete)?;
        out.set_item("partial_fields", decoded.diagnostics.partial_fields.clone())?;
        out.set_item("failed_streams", decoded.diagnostics.failed_streams.clone())?;
        match decoded.diagnostics.confidence {
            Some(c) => out.set_item("confidence", c as f64)?,
            None => out.set_item("confidence", py.None())?,
        }
        Ok(out)
    }

    /// Schema diagnostics as a dict (API-13 Python surface for
    /// `SchemaTable::diagnostics`). Returns class_count,
    /// parsed_field_count, declared_field_count_sum,
    /// field_count_mismatches, tagged_class_count,
    /// parent_only_class_count, ancestor_tag_count, skipped_records,
    /// cpp_type_count — one dict, all integers.
    ///
    /// Raises `ValueError` if the schema can't be parsed (strict).
    fn schema_diagnostics<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let schema = self.inner.schema().map_err(to_py_val)?;
        let d = schema.diagnostics();
        let out = PyDict::new(py);
        out.set_item("class_count", d.class_count)?;
        out.set_item("parsed_field_count", d.parsed_field_count)?;
        out.set_item("declared_field_count_sum", d.declared_field_count_sum)?;
        out.set_item("field_count_mismatches", d.field_count_mismatches)?;
        out.set_item("tagged_class_count", d.tagged_class_count)?;
        out.set_item("parent_only_class_count", d.parent_only_class_count)?;
        out.set_item("ancestor_tag_count", d.ancestor_tag_count)?;
        out.set_item("skipped_records", d.skipped_records)?;
        out.set_item("cpp_type_count", d.cpp_type_count)?;
        Ok(out)
    }

    /// Produce an IFC4 STEP string for this Revit file via
    /// `ifc::RvtDocExporter`. Document-level export — project name,
    /// description, units, classifications. Raises `ValueError` if
    /// the file can't be parsed far enough to produce a model.
    #[pyo3(signature = (mode = "scaffold"))]
    fn write_ifc(&mut self, mode: &str) -> PyResult<String> {
        let mode = parse_export_quality_mode(mode)?;
        write_ifc_with_quality_mode(&mut self.inner, mode)
    }

    /// Produce the JSON diagnostics sidecar for the default IFC export.
    ///
    /// The returned string matches `rvt-ifc --diagnostics` and is intended
    /// for bug reports, support bundles, and automated readiness checks.
    fn export_diagnostics_json(&mut self) -> PyResult<String> {
        let result = ifc::RvtDocExporter
            .export_with_diagnostics(&mut self.inner)
            .map_err(to_py_val)?;
        serde_json::to_string(&result.diagnostics).map_err(to_py_val)
    }

    /// Parse Global/ElemTable header. Returns a dict with
    /// `{element_count, record_count, header_flag, decompressed_bytes}`.
    /// See `docs/elem-table-record-layout-2026-04-21.md`.
    fn elem_table_header<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let header = elem_table::parse_header(&mut self.inner).map_err(to_py_val)?;
        let d = PyDict::new(py);
        d.set_item("element_count", header.element_count)?;
        d.set_item("record_count", header.record_count)?;
        d.set_item("header_flag", header.header_flag)?;
        d.set_item("decompressed_bytes", header.decompressed_bytes)?;
        Ok(d)
    }

    /// Parse Global/ElemTable records. Returns a list of dicts with
    /// `{offset, id_primary, id_secondary}`. Handles the three layout
    /// variants automatically (family 12 B, project 2023 28 B,
    /// project 2024 40 B). On a 34 MB project this returns all
    /// 26,425 records; on a family file, all declared records.
    fn elem_table_records<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyList>> {
        let records = elem_table::parse_records(&mut self.inner).map_err(to_py_val)?;
        let list = PyList::empty(py);
        for r in records {
            let d = PyDict::new(py);
            d.set_item("offset", r.offset)?;
            d.set_item("id_primary", r.id_primary)?;
            d.set_item("id_secondary", r.id_secondary)?;
            list.append(d)?;
        }
        Ok(list)
    }

    /// Return the sorted, deduplicated list of ElementIds declared by
    /// Global/ElemTable. Useful for walker coverage validation —
    /// diff the walker's HandleIndex against this set to find
    /// "declared but not located" elements.
    fn declared_element_ids(&mut self) -> PyResult<Vec<u32>> {
        elem_table::declared_element_ids(&mut self.inner).map_err(to_py_val)
    }

    fn __repr__(&mut self) -> String {
        let v = self.inner.basic_file_info().ok().map(|b| b.version);
        match v {
            Some(v) => format!("RevitFile(version={v})"),
            None => "RevitFile(version=?)".into(),
        }
    }
}

/// One-shot helper: open a Revit file, run the document-level IFC
/// exporter, return the STEP string. Equivalent to
/// `rvt.RevitFile(path).write_ifc()`.
#[pyfunction]
#[pyo3(signature = (path, mode = "scaffold"))]
fn rvt_to_ifc(path: &str, mode: &str) -> PyResult<String> {
    let mut rf = RustRevitFile::open(path).map_err(to_py_io)?;
    let mode = parse_export_quality_mode(mode)?;
    write_ifc_with_quality_mode(&mut rf, mode)
}

/// One-shot helper: open a Revit file, run the default IFC exporter,
/// and return the JSON diagnostics sidecar.
#[pyfunction]
fn rvt_to_ifc_diagnostics(path: &str) -> PyResult<String> {
    let mut rf = RustRevitFile::open(path).map_err(to_py_io)?;
    let result = ifc::RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .map_err(to_py_val)?;
    serde_json::to_string(&result.diagnostics).map_err(to_py_val)
}

/// Compiled Python submodule named `_rvt`. Sits under the
/// pure-Python `rvt` package in `python/`, whose `__init__.py`
/// re-exports everything here. That layout ships type stubs
/// (`__init__.pyi`) and a PEP-561 `py.typed` marker alongside the
/// extension so mypy and pyright pick them up.
#[pymodule]
fn _rvt(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyRevitFile>()?;
    m.add_function(wrap_pyfunction!(rvt_to_ifc, m)?)?;
    m.add_function(wrap_pyfunction!(rvt_to_ifc_diagnostics, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
