- **`rvt-write --dry-run` checks what the write would refuse, and the write
  refuses a patch that frames a stream otherwise than it is stored (#1885).**
  The dry run its help describes was unreachable without `--patches`, and with
  one it exited 0 without looking at the streams. It now opens the source,
  reads the manifest if one is given and applies the writer's own refusals
  (`rvt::writer::validate_patches`, new): a stream the file does not have, a
  framing the stream is not stored in, and a patch that would span a stored
  page. A manifest alone chose a stream's framing on disk and the read-back
  used the same framing, so a `Formats/Latest` written as `Global/Latest` is
  framed passed its own verification; a patch framed `RawGzipFromZero` or
  `CustomPrefix8` is now refused unless the stream is stored that way
  (`Verbatim` is the caller's own framing and is not checked); the library's
  `rvt::writer::write_with_patches` returns `Error::WriteRefused` for it before
  it writes anything, where it wrote the file and verified it with the same
  wrong framing. A
  `new_decompressed_base64` that decodes to no bytes (`"="`, `""`) is refused
  where it emptied the stream; an empty `patches` array is a plain copy and
  exits 0, where it exited 1 with `0 stream(s) failed verification`; `--src`
  and `--dst` naming one file, spelled any way, leave a file with no patch as it
  is instead of rebuilding its container, and a copy that fails removes its
  temporary file.
