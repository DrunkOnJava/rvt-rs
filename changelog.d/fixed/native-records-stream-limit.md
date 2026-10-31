- **A partition over 512 MiB no longer stops every reader of native records
  ([#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075)).** The readers
  of native records (types, materials, stairs, phases, saved parameters and
  others) kept a stream cap of their own, 512 MiB, that `--large-model` did
  not raise. A model with a larger partition got none of their values, not
  only that partition's, and the diagnostics did not say so. They now read under the file's own
  limits: `--large-model` raises theirs too, and a partition over the limit is
  named in the diagnostics and skipped while the rest of the file is read.
  An element whose current record sits in that partition is left out rather
  than read from an older copy in another partition. At
  the default limits they now stop at 256 MiB, as the rest of the reader
  does, where they read up to 512 MiB before. Not yet measured on a model
  over the limits: a contributor's model with a 1.33 GB partition (#1117) is
  the first such file.
