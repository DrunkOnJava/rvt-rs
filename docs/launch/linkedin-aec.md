Revit files have been closed to everyone outside the Autodesk runtime for two decades. If you wanted to read a .rvt programmatically, your options were Dynamo/pyRevit inside a running Revit, the Forge API in the cloud, or commercial SDKs with per-seat licensing. Nothing you could link into a Rust service or a batch job.

rvt-rs is an Apache-2.0 Rust library that opens .rvt and .rfa files directly. No Autodesk install, no network call, no seat license. It reads the OLE container, decodes the schema, walks the element graph, and exports IFC4 STEP that opens cleanly in BlenderBIM and IfcOpenShell. Revit 2016 through 2026. Works on the project file on your disk right now.

Where this starts to matter:

- BIM analytics across a portfolio without spinning up Revit on a server
- Model migration when you are leaving Autodesk or integrating a non-Autodesk toolchain
- Interop for the openBIM stack without going through the Revit API's "very limited" IFC export
- Research on BIM adoption patterns, schema evolution, format history
- Archival and preservation of legacy projects when the originating Revit seat is long gone

Honest scope: it reads and exports. It does not write .rvt files. 54 element classes decoded today, geometry helpers per-class, MEP decoders still pending. Early-stage, but working against real files.

If you work with .rvt outside Revit itself and hit walls, tell me what would help.

github.com/DrunkOnJava/rvt-rs — Apache-2.0
