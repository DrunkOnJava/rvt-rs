- **`rvt-capabilities` no longer lists relation domains, and
  `ifc::NotYetImplemented` is gone (the maintainer, 2026-10-30).** The
  capability manifest's `relation_domains` section and its
  `relations.typed_domains` row described the typed-relation module that
  0.11.0 removed (#1131): the row claimed a registry and "SCC/condensation/
  quarantine stubs exist", which was no longer true. The section leaves
  `rvt-capabilities` (text and `--json`) and
  `docs/schemas/capability-manifest.schema.json`, whose `schema_version` is
  now 2 (a consumer that reads version 1 sees the section is gone), and
  `rvt::capability::RelationDomain`, `RelationDomainRegistry`,
  `RelationDomainEntry` and `DomainStatus` leave the API. Nothing replaces
  them: no code read or wrote a relation domain. `ifc::NotYetImplemented`,
  an error no exporter returned, goes with them. Two remaining rows of the
  manifest say in plain words what they meant: nothing reads the compound
  ArcWall variant's openings, and remapping Extensible Storage ElementIds
  needs Revit API output the project does not have.
