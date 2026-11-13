- **`rvt-info -f csv` writes a file's text as text, and `rvt-dump` prints a
  stream's name without its control characters (#1885).** A title, a user
  name, a path or a name of a file's own reached a CSV cell as it stood, so
  one that starts with `=`, `+`, `-` or `@` ran as a formula when the CSV was
  opened in a spreadsheet. Such a cell is now led by `'`, as `rvt-schedule`'s
  cells have been, from one shared rule (`rvt::cli::csv_text`). A crafted
  file can also name a stream with control characters, which `rvt-dump`
  printed to the terminal as they were; it prints the name escaped. The file
  name `rvt-dump` writes a stream to was already sanitised. No committed
  output holds either, so none of it changes.
