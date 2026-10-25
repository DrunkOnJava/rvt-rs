- **`ifc::RevitGlobalIds` gains `project`, `site` and `building` (#878).**
  They carry the GlobalIds a file stores for the project, site and
  building (RE-220). Code that builds a `RevitGlobalIds` with a struct
  literal needs the new fields or `..Default::default()`; they deserialise
  as `None` when absent.
