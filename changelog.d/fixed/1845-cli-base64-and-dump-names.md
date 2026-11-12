- **`rvt-write` decodes a manifest's unpadded base64 to the bytes it holds
  (#1845).** A `new_decompressed_base64` value whose last group had no `=`
  padding decoded to three bytes whatever the group's length, and an `=`
  inside the text decoded as zeros, so the stream was patched with bytes the
  manifest did not give. Both now decode as standard base64 does, and text
  ending in a lone character is refused.
- **`rvt-dump` writes every stream on Windows (#1845).** A stream whose name
  holds a character Windows refuses in a file name (`<>"|?*` or a control
  character) failed its write and stopped the dump; the character is now
  `_` in the file name, as `/` is.
