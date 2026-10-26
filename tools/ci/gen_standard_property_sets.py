#!/usr/bin/env python3
"""Generate src/ifc/standard_property_sets/table.rs, IFC4's property sets for #817.

The source is buildingSMART's own list of IFC4's property set definitions, the
one its validation service checks rule PSE001 against:
features/resources/IFC4/pset_definitions.csv of
https://github.com/buildingSMART/ifc-gherkin-rules (MIT), at SOURCE_COMMIT,
with the sha256 SOURCE_SHA256 (CLEANROOM.md source 4). IFC4's entity
hierarchy comes from IfcOpenShell's IFC4 schema, at the version
tools/ci/requirements-ifc.txt pins.

The table keeps, of each set that applies to an entity, the single-value
properties of the kinds a parameter's value is read as (RE-194), the names
of its other properties (RE-235), and the
supertype of every IFC4 entity, so that a set applies to its entities'
subtypes. A property the CSV gives no type takes the type most of the other
sets give a property of that name: `Reference` in four sets, `IfcIdentifier`
in 103 sets and `IfcLabel` in 6. Sets of performance history and
sets without an entity are left out.

Usage:
    python3 tools/ci/gen_standard_property_sets.py [PSET_DEFINITIONS.csv] > src/ifc/standard_property_sets/table.rs
    rustfmt --edition 2024 src/ifc/standard_property_sets/table.rs

Without a path the file is read from GitHub at SOURCE_COMMIT. Either way its
sha256 must be SOURCE_SHA256.
"""

import ast
import collections
import csv
import hashlib
import io
import sys
import time
import urllib.error
import urllib.request

import ifcopenshell.ifcopenshell_wrapper as wrapper

SOURCE_REPOSITORY = "buildingSMART/ifc-gherkin-rules"
SOURCE_COMMIT = "bb1f0af99aa11d6ffe62b18a7c8d238b09616114"
SOURCE_PATH = "features/resources/IFC4/pset_definitions.csv"
SOURCE_SHA256 = "7575131470b6c1d8b3faf9e73f2aa24d167af50ad21cd055497c02a8423470b4"

# IFC type of a single value -> the StandardPropertyKind it is read as.
KINDS = {
    "IfcBoolean": "Boolean",
    "IfcIdentifier": "Identifier",
    "IfcLabel": "Label",
    "IfcText": "Text",
    "IfcCountMeasure": "Count",
    "IfcInteger": "Integer",
    "IfcLengthMeasure": "Length",
    "IfcPositiveLengthMeasure": "PositiveLength",
    "IfcThermalTransmittanceMeasure": "ThermalTransmittance",
}


def source_text(path):
    if path:
        data = open(path, "rb").read()
    else:
        url = f"https://raw.githubusercontent.com/{SOURCE_REPOSITORY}/{SOURCE_COMMIT}/{SOURCE_PATH}"
        # A transient network failure is retried, so it does not fail CI;
        # the pinned sha256 still decides what is read.
        for attempt in range(1, 4):
            try:
                data = urllib.request.urlopen(url, timeout=120).read()
                break
            except (urllib.error.URLError, TimeoutError) as error:
                if attempt == 3:
                    raise
                print(f"retrying {url} after: {error}", file=sys.stderr)
                time.sleep(15 * attempt)
    digest = hashlib.sha256(data).hexdigest()
    if digest != SOURCE_SHA256:
        sys.exit(f"error: {SOURCE_PATH} has sha256 {digest}, pinned {SOURCE_SHA256}")
    return data.decode("utf-8")


def main(argv):
    csv.field_size_limit(sys.maxsize)
    rows = list(csv.DictReader(io.StringIO(source_text(argv[1] if len(argv) > 1 else None))))

    def definitions(row):
        return ast.literal_eval(row["property_definitions"])

    def data_type(prop):
        value = prop.get("data_type")
        return value.get("instance") if isinstance(value, dict) else None

    # A property's type where the CSV gives none: the one most sets give it.
    named = collections.defaultdict(collections.Counter)
    for row in rows:
        for prop in definitions(row):
            if prop["property_type"] == "IfcPropertySingleValue" and data_type(prop):
                named[prop["property_name"]][data_type(prop)] += 1

    schema = wrapper.schema_by_name("IFC4")
    supertypes = {}
    for entity in schema.entities():
        parent = entity.supertype()
        if parent is not None:
            supertypes[entity.name().upper()] = parent.name().upper()

    sets = []
    for row in sorted(rows, key=lambda row: row["Name"]):
        if row["template_type"] == "PSET_PERFORMANCEDRIVEN":
            continue
        applies = []
        for applicable in ast.literal_eval(row["applicable_entities"]):
            entity, _, predefined = applicable.partition("/")
            entity = entity.strip()
            if schema.declaration_by_name(entity) is None:
                sys.exit(f"error: {row['Name']} applies to {entity}, not an IFC4 entity")
            applies.append((entity.upper(), predefined.strip() or None))
        if not applies:
            continue
        properties = []
        for prop in definitions(row):
            if prop["property_type"] != "IfcPropertySingleValue":
                continue
            kind = data_type(prop)
            types = named[prop["property_name"]]
            if kind is None and types:
                ((common, count),) = types.most_common(1)
                if 2 * count > sum(types.values()):
                    kind = common
            if kind in KINDS:
                properties.append((prop["property_name"], KINDS[kind]))
        listed = {name for name, _ in properties}
        others = []
        for prop in definitions(row):
            if prop["property_name"] not in listed and prop["property_name"] not in others:
                others.append(prop["property_name"])
        if properties:
            sets.append(
                (row["Name"], sorted(set(applies), key=lambda a: (a[0], a[1] or "")), properties, others)
            )

    out = sys.stdout
    out.write(
        "// @generated by tools/ci/gen_standard_property_sets.py; do not edit.\n"
        f"// Source: {SOURCE_REPOSITORY} {SOURCE_COMMIT} {SOURCE_PATH}\n"
        f"// (sha256 {SOURCE_SHA256}), and IfcOpenShell's IFC4 schema.\n\n"
        "use super::StandardPropertyKind::{\n"
        "    Boolean, Count, Identifier, Integer, Label, Length, PositiveLength, Text, ThermalTransmittance,\n"
        "};\n"
        "use super::StandardPropertySet;\n\n"
        "/// Each IFC4 entity's supertype, by entity, sorted.\n"
        "pub(super) const SUPERTYPES: &[(&str, &str)] = &[\n"
    )
    for entity in sorted(supertypes):
        out.write(f'    ("{entity}", "{supertypes[entity]}"),\n')
    out.write("];\n\n")
    out.write(
        "/// IFC4's property sets that apply to an entity, by name, with their\n"
        "/// single-value properties of the kinds a parameter's value is read as\n"
        "/// and the names of their other properties.\n"
        "pub(super) const SETS: &[StandardPropertySet] = &[\n"
    )
    for name, applies, properties, others in sets:
        out.write("    StandardPropertySet {\n")
        out.write(f'        name: "{name}",\n')
        out.write("        applies: &[\n")
        for entity, predefined in applies:
            value = f'Some("{predefined}")' if predefined else "None"
            out.write(f'            ("{entity}", {value}),\n')
        out.write("        ],\n")
        out.write("        properties: &[\n")
        for prop, kind in properties:
            out.write(f'            ("{prop}", {kind}),\n')
        out.write("        ],\n")
        out.write("        other_properties: &[\n")
        for prop in others:
            out.write(f'            "{prop}",\n')
        out.write("        ],\n")
        out.write("    },\n")
    out.write("];\n")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
