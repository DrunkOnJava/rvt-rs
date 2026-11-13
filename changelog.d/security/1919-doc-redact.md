- **`rvt-doc --redact` scrubs the error text, every part of the path and every
  string a field holds (#1919).** The flag scrubbed only the user name of a
  home folder in the path it printed, against its own help, which promised
  user names and Autodesk-internal paths. A file in an Autodesk OneDrive or
  project-id folder printed that folder as it stood, and the error `rvt-doc`
  prints when it cannot open a file, which names the path it was given, was not
  scrubbed at all. The path, the notes and the error text now go through the
  scrubber the other commands' `--redact` runs (`rvt::redact::redact_sensitive`),
  and the strings of the fields through the new
  `rvt::redact::redact_json_strings`. No field of the 11 family releases (2016
  to 2026) or the 6 projects measured decodes as a string, so the output of a
  file that opens changes only in its path.
