- **A partition may inflate to 16 times its stored size at the default
  limits, at least 1 GiB and at most 4 GiB on a 64-bit build.** It was
  1 GiB whatever its size. A 202 MB Revit 2024 model whose 141 MiB partition
  inflates past 1 GiB, which 0.6.1 read by cutting the partition short
  without a word, has needed `--large-model` since 0.11.0; it now reads whole
  at the defaults. A small file still cannot make a reader hold more than
  1 GiB of one stream, and a 32-bit build and the browser viewer keep 1 GiB
  (#1075).
