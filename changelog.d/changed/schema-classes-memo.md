- **One parse of the schema per file (#1032).** A dozen of the partition
  readers an export runs each read and parsed `Formats/Latest` again for a
  few class tags; the file handle now keeps the parsed classes for all of
  them, with every export byte for byte as before.
