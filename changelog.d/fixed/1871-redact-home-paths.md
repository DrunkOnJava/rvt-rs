- **`--redact` scrubs the user name from a Linux home folder, and from every
  path of a message instead of the first of each kind (#1871).** The scrubber
  that each CLI's `--redact` runs its text through knew `\Users\<name>\` and
  `/Users/<name>/` only, so a file under `/home/<name>/`, or an error that
  names one, kept the name. It also scrubbed only the first path of its kind
  in a string, so a message naming two files kept the second one's name. A name
  that nothing follows, because the string ends at it, is left as it was, as
  before. No committed output holds a `/home/` path, so none of it changes.
