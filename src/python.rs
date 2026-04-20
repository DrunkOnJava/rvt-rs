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

use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};

use crate::{RevitFile as RustRevitFile, ifc, walker};

fn to_py_io<E: std::fmt::Display>(e: E) -> PyErr {
    PyIOError::new_err(e.to_string())
}
fn to_py_val<E: std::fmt::Display>(e: E) -> PyErr {
    PyValueError::new_err(e.to_string())
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
    #[new]
    fn new(path: &str) -> PyResult<Self> {
        let inner = RustRevitFile::open(path).map_err(to_py_io)?;
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
    /// it should use the Rust library directly.
    fn schema_summary<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Bound<'py, PyDict>> {
        let schema = self.inner.schema().map_err(to_py_val)?;
        let d = PyDict::new_bound(py);
        let total_fields: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
        d.set_item("classes", schema.classes.len())?;
        d.set_item("fields", total_fields)?;
        d.set_item("cpp_types", schema.cpp_types.len())?;
        Ok(d)
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
    fn read_adocument<'py>(
        &mut self,
        py: Python<'py>,
    ) -> PyResult<Option<Bound<'py, PyDict>>> {
        let Some(inst) = walker::read_adocument(&mut self.inner).map_err(to_py_io)? else {
            return Ok(None);
        };
        let d = PyDict::new_bound(py);
        d.set_item("entry_offset", inst.entry_offset)?;
        d.set_item("version", inst.version)?;

        let fields = PyList::empty_bound(py);
        for (name, value) in &inst.fields {
            let fd = PyDict::new_bound(py);
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
                walker::InstanceField::Bytes(b) => {
                    fd.set_item("kind", "bytes")?;
                    fd.set_item("len", b.len())?;
                }
            }
            fields.append(fd)?;
        }
        d.set_item("fields", fields)?;
        Ok(Some(d))
    }

    /// Produce an IFC4 STEP string for this Revit file via
    /// `ifc::RvtDocExporter`. Document-level export — project name,
    /// description, units, classifications. Raises `ValueError` if
    /// the file can't be parsed far enough to produce a model.
    fn write_ifc(&mut self) -> PyResult<String> {
        let model = ifc::Exporter::export(&ifc::RvtDocExporter, &mut self.inner).map_err(to_py_val)?;
        Ok(ifc::write_step(&model))
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
fn rvt_to_ifc(path: &str) -> PyResult<String> {
    let mut rf = RustRevitFile::open(path).map_err(to_py_io)?;
    let model = ifc::Exporter::export(&ifc::RvtDocExporter, &mut rf).map_err(to_py_val)?;
    Ok(ifc::write_step(&model))
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
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
