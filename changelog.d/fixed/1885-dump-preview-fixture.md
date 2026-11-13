- **`rvt-dump` takes the resource-limit options and goes on past a stream it
  cannot read, `rvt-info --extract-preview` refuses a preview that is not a
  whole PNG, and `gen-fixture` refuses a name with a path in it (#1885).**
  `rvt-dump` opened every file under the default limits with no option to
  change them, and a stream over the 256 MiB default stopped the whole dump
  with an error. It now has `--max-stream-bytes`, `--max-inflate-bytes` and
  `--large-model` as `rvt-gltf` and `rvt-ifc` do; a stream over the limit is
  listed as not read, the rest of the file is dumped, and the streams a limit
  left unread or cut short are listed on stderr with the way to read them, with
  exit status 3 (`--allow-partial` makes it 0). It also drops each inflated
  partition from the reader's memo once it has been written, where the memo
  held every one (up to 16 GiB) to the end of the dump; the memory this
  saves is not measured. `rvt-info --extract-preview` wrote what the stream
  held from the PNG signature on, and said "preview PNG written", when a
  preview cut short had no IEND chunk and no viewer opens it; it now writes
  nothing, exits 1 and says the preview is cut short (`rvt-dump --raw` writes
  the stream as it is). `gen-fixture NAME` with no `--output` wrote
  `tests/fixtures/synthetic-NAME.rvt`, so a NAME with a `/` or `\` in it
  wrote elsewhere; it is refused, and `--output` names another place.
  `RevitFile::release_inflated_partitions` is now public.
