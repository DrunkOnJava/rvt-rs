- **The read of the parameters named for a property searches each record for
  the few definition ids it looks for, instead of scanning every four-byte
  window (#1335).** On a Revit 2015 to 2023 project that read looks at about
  5,600 records for six ids. It now searches each record's bytes for each id
  with the vectorised substring search, and keeps the window scan for more
  than 16 ids. The records it decodes are the same. No export changes. The
  figures are in the pull request.
