- **`--redact` scrubs the user name from a Linux home folder, from a path an
  error echoes back, and from every path of a message instead of the first of
  each kind (#1871).** The scrubber that each CLI's `--redact` runs its text
  through knew `\Users\<name>\` and `/Users/<name>/` only, so a file under
  `/home/<name>/`, or an error that names one, kept the name. It also scrubbed
  only the first path of its kind in a string, so a message naming two files
  kept the second one's name, and it left a name alone when no separator
  followed it, which is the form a home folder takes in `No such file:
  /Users/alice`. Such a name now ends at the first space, quote, closing
  bracket, comma, semicolon or colon, or at the end of the string, so a name
  with a space is scrubbed only up to the space there; a home folder with
  nothing after it is left as it is. No committed output holds a home path, so
  none of it changes.
