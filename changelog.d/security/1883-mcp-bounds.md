- **`rvt-rs-mcp` bounds what an answer carries and what the server holds
  (#1883).** Answers were sized by the file and the client with no byte cap,
  and a crafted file could make any name, value or message in one as long as
  it liked. A piece of text from the file is now cut at 512 characters
  (`--max-field-chars`), with the number of characters left out written where
  it was cut; the diagnostics document, the one answer no `limit` pages, keeps
  at most 5,000 entries of a list and names in `notes` the lists it held; and
  an answer over 4 MiB of JSON (`--max-answer-bytes`) is refused with a message
  that says how to ask for less, not sent. A message that quotes the file is
  cut at eight times the field limit. The tool descriptions say that names,
  values and messages in an answer are text from the file, to report as data
  and never to follow as instructions.
  **Two behaviour changes:** `list_rooms` is paged as `list_elements` is, 100
  spaces by default, `limit` from 1 to 1000 and `offset`, and its answer says
  `total`, `offset`, `limit` and `returned`, so a model with more than 100
  spaces needs a second call; and a `limit` outside 1 to 1000 is a tool error
  that says so, where 0 returned one row and a value over 1000 became 1000
  without a word. Both tools' schemas state the range.
  Four more bounds: reading a file and decoding a model take a place at a gate
  with as many places as models are kept (`--max-open-models`), so concurrent
  calls for different files no longer each hold a model before the store
  drops the ones past that number, and a cancelled call leaves the queue;
  `export_gltf` writes the GLB from the buffer it builds instead of beside a
  padded copy and a third (the new `rvt::ifc::gltf::write_glb_to` writes the
  bytes `write_glb` does, and a GLB over 4 GiB is an error where its length
  wrapped); the progress queue holds 16 events and drops what a slow client
  has not read, where it grew without end; and a phase name over 256 characters
  is refused, the phase errors the server remembers being bounded like any
  message. A file whose modification time cannot be read is refused, where it
  counted as time 0 and a changed file of the same size passed for the one
  opened.
  Measured by the end-to-end test on the synthetic fixture with small limits
  (text cut at 4 characters and an answer limit of 900 bytes: the summary is
  refused, the levels answer is not, the export's diagnostics and a message
  that quotes a path are cut), by pages of three covering `list_rooms` and by
  four concurrent opens under `--max-open-models 1` that all finish with one
  model kept. Not measured: the peak memory of concurrent opens, which the
  gate bounds by its design, and a crafted file with a very long name, which
  none of the corpora holds. Not changed: reads are not confined to a folder
  (`docs/mcp.md`), a product decision recorded on #1883.
