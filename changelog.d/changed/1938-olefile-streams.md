- **CI reads every corpus file's container with a second reader, olefile, and
  holds rvt-rs's to it (#1938).** `tools/ci/olefile_streams.py` lists each
  `.rvt` and `.rfa` of the family corpus, the project corpus and
  `tests/fixtures/families/empty.rfa` with olefile, runs `rvt-dump --raw` on
  the same file and fails on any difference in the stream names, in each
  stream's stored size, or in the stored bytes. It prints how many files and
  streams it compared, and a directory with no Revit file or a run that
  compared no stream fails. `cfb-container-open` stays `partial`: a stream-list
  agreement checks the container and is not yet one of the two independent
  witnesses `docs/verification-protocol.md` asks for. No export changes.
