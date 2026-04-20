"""Type stubs for the rvt Python bindings.

Hand-maintained. Source of truth for the runtime API is
`src/python.rs` — if you add a method there, add it here too.
PEP 561 marker (`py.typed`) accompanies this file so mypy and
pyright pick it up.
"""
from __future__ import annotations

from typing import Final, Optional, Union

__version__: Final[str]

TypedSchemaSummary = dict[str, int]
"""Return shape of `RevitFile.schema_summary()`.

Keys: ``classes``, ``fields``, ``cpp_types`` — all ``int``.
"""

TypedField = dict[str, Union[str, int, list[int]]]
"""Return shape of an entry in `RevitFile.read_adocument()["fields"]`.

All entries have ``name`` (str) and ``kind`` (str). Additional keys
depend on the kind:

- ``kind == "pointer"``: ``slot_a`` (int), ``slot_b`` (int)
- ``kind == "element_id"``: ``tag`` (int), ``id`` (int)
- ``kind == "ref_container"``: ``count`` (int), ``col_a`` (list[int]), ``col_b`` (list[int])
- ``kind == "bytes"``: ``len`` (int)
"""

TypedADocument = dict[str, Union[int, list[TypedField]]]
"""Return shape of `RevitFile.read_adocument()`.

Keys:
- ``entry_offset`` (int): byte offset in decompressed Global/Latest.
- ``version`` (int): Revit release year.
- ``fields`` (list[TypedField]): one entry per schema-declared field.
"""


class RevitFile:
    """Opened Revit file. Entry point for all Python bindings.

    Construct with a filesystem path to a ``.rvt`` / ``.rfa`` /
    ``.rte`` / ``.rft`` file. Raises ``IOError`` on missing files,
    non-CFB input, or read failures.
    """

    def __init__(self, path: str) -> None: ...
    def __repr__(self) -> str: ...

    @property
    def version(self) -> Optional[int]:
        """Revit release year (e.g. ``2024``), or ``None`` if
        ``BasicFileInfo`` can't be parsed.
        """

    @property
    def original_path(self) -> Optional[str]:
        """Original file path recorded at save time on the creator's
        machine (may be a Windows-style path). Use ``--redact`` in
        the CLI equivalent when surfacing to end users.
        """

    @property
    def build(self) -> Optional[str]:
        """Revit build tag (e.g. ``"20230308_1635(x64)"``), if
        recorded in ``BasicFileInfo``.
        """

    @property
    def guid(self) -> Optional[str]:
        """Document GUID from ``BasicFileInfo``, if present."""

    @property
    def part_atom_title(self) -> Optional[str]:
        """Document title from the ``PartAtom`` XML stream (family
        files carry one; some project files don't).
        """

    def stream_names(self) -> list[str]:
        """All OLE stream paths in the file, sorted, `/`-separated
        regardless of host OS.
        """

    def missing_required_streams(self) -> list[str]:
        """List of required Revit stream names the file doesn't
        contain. Empty list on a valid Revit file. Useful for
        pre-validating an input before running heavy extractors.
        """

    def basic_file_info_json(self) -> Optional[str]:
        """Full ``BasicFileInfo`` as a JSON string (parseable via
        ``json.loads``). Single-call equivalent of the four individual
        getters (``version``, ``original_path``, ``build``, ``guid``)
        plus any future fields added to the Rust ``BasicFileInfo``
        struct. Returns ``None`` if the ``BasicFileInfo`` stream
        can't be parsed.
        """

    def part_atom_json(self) -> Optional[str]:
        """Full ``PartAtom`` as a JSON string (parseable via
        ``json.loads``). Superset of the ``part_atom_title`` getter —
        also includes ``id``, ``updated``, ``taxonomies``,
        ``categories``, ``omniclass``, and ``raw_xml``. Returns
        ``None`` if the file has no PartAtom stream (common on
        project ``.rvt`` files).
        """

    def schema_summary(self) -> TypedSchemaSummary:
        """Decoded schema counts. Cheap. Returns a dict with keys
        ``classes``, ``fields``, ``cpp_types``.
        """

    def schema_json(self) -> str:
        """Full schema as a JSON string (parseable via ``json.loads``).

        Return shape: ``{"classes": [...], "cpp_types": [...],
        "skipped_records": int}``. Each class has ``name``, ``offset``,
        ``fields``, ``tag``, ``parent``, ``declared_field_count``,
        ``was_parent_only``, ``ancestor_tag``. Each field has
        ``name``, ``cpp_type``, ``field_type`` (a tagged enum).

        The string is on the order of 1-2 MB for a typical Revit
        family (~395 classes, ~13,570 fields). For just counts, use
        ``schema_summary()``.
        """

    def read_adocument(self) -> Optional[TypedADocument]:
        """Run the Layer-5a walker and return ADocument's instance
        as a dict, or ``None`` if the entry-point detector can't
        confidently locate ADocument in the file.

        See module docs for the field-kind schema.
        """

    def write_ifc(self) -> str:
        """Produce an IFC4 STEP string for this file via the
        ``RvtDocExporter``. Raises ``ValueError`` if the file can't
        be parsed far enough to produce a minimal model.
        """


def rvt_to_ifc(path: str) -> str:
    """One-shot: open ``path``, run ``RvtDocExporter``, return the
    IFC4 STEP text. Equivalent to
    ``RevitFile(path).write_ifc()``.
    """
