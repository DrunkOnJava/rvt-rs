- **`rvt-write` decodes a manifest's unpadded base64 to the bytes it holds
  (#1845).** A `new_decompressed_base64` value whose last group had no `=`
  padding decoded to three bytes whatever the group's length, and an `=`
  inside the text decoded as zeros, so the stream was patched with bytes the
  manifest did not give. Both now decode as standard base64 does, and text
  ending in a lone character is refused.
- **`rvt-dump` writes every stream, each to its own file (#1845).** A
  stream's name is the file's own. One holding a character Windows refuses
  in a file name (`<>"|?*` or a control character) failed its write and
  stopped the dump: the character is now `_`, as `/` is. A name Windows
  reserves for a device (`CON`, `NUL`, `COM1` and the like) takes a `_` in
  front. Two names that became one file name, the later stream replacing
  the earlier, now take `~2`, `~3` and so on.
