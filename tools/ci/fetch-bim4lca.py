#!/usr/bin/env python3
"""Fetch the BIM4LCA models CI reads, by byte range from VTT's ARCH.zip (#766).

Usage: fetch-bim4lca.py <pins.tsv> <target_dir>
       fetch-bim4lca.py --pin <pins.tsv> <entry>...

The pins (research/bim4lca-models.tsv) give the archive's URL and byte count
and, per entry, its path in the archive and the sha256 and byte count of the
file it holds. Only those entries are read: the archive's central directory
gives each one's offset, and range requests fetch it, so a run downloads the
models and not the 1.58 GB archive. Each file lands in target_dir under its
own name and is checked against its pins; one already there with the right
hash is kept. `--pin` prints the pin rows of the named entries instead.

Exit 0 when every pinned file is in place and verified; 1 when the archive or
an entry no longer matches its pins (the host replaced it: re-pin); 2 when the
host could not be reached after retries, which says nothing about the models.
"""

from __future__ import annotations

import hashlib
import struct
import sys
import time
import urllib.error
import urllib.request
import zlib
from pathlib import Path

USER_AGENT = "rvt-rs-ci (https://github.com/DrunkOnJava/rvt-rs)"
TAIL = 1 << 20


class Stale(Exception):
    pass


class Unreachable(Exception):
    pass


def read_pins(path: Path) -> tuple[str, int, list[tuple[str, str, int]]]:
    archive, entries = None, []
    for line in path.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        kind, name, sha, size = line.split("\t")
        if kind == "archive":
            archive = (name, int(size))
        elif kind == "entry":
            entries.append((name, sha, int(size)))
    if archive is None:
        raise SystemExit(f"error: {path} names no archive")
    return archive[0], archive[1], entries


def fetch(url: str, start: int, end: int, total: int) -> bytes:
    """Bytes start..end of the archive, inclusive, as a 206 of the pinned total."""
    last = None
    for attempt in range(5):
        request = urllib.request.Request(
            url, headers={"Range": f"bytes={start}-{end}", "User-Agent": USER_AGENT}
        )
        try:
            with urllib.request.urlopen(request, timeout=300) as response:
                if response.status != 206:
                    raise Stale(f"the host answered {response.status} to a range request")
                served = int(response.headers.get("Content-Range", "/0").rsplit("/", 1)[1])
                if served != total:
                    raise Stale(f"the archive is {served} bytes, pinned {total}: it was replaced")
                body = response.read()
                if len(body) != end - start + 1:
                    raise ConnectionError(f"short read: {len(body)} of {end - start + 1} bytes")
                return body
        except Stale:
            raise
        except urllib.error.HTTPError as error:
            if error.code in (404, 410):
                raise Stale(f"the host answered {error.code}: the archive is gone")
            last = error
            time.sleep(10 * (attempt + 1))
        except (urllib.error.URLError, OSError) as error:
            last = error
            time.sleep(10 * (attempt + 1))
    raise Unreachable(f"{url}: {last}")


def central_directory(url: str, total: int) -> dict[str, tuple[int, int, int, int]]:
    """name -> (method, compressed size, size, local header offset)."""
    base = max(0, total - TAIL)
    tail = fetch(url, base, total - 1, total)
    end = tail.rfind(b"PK\x05\x06")
    if end < 0:
        raise Stale("no end of central directory in the archive's last MiB")
    count, size, offset = struct.unpack("<10xHII", tail[end : end + 20])
    if offset < base:
        tail = fetch(url, offset, total - 1, total)
        base = offset
    entries, p = {}, offset - base
    for _ in range(count):
        (sig, method, csize, usize, nlen, elen, clen, lho) = struct.unpack(
            "<I6xH8xIIHHH8xI", tail[p : p + 46]
        )
        if sig != 0x02014B50:
            raise Stale("the central directory does not parse")
        name = tail[p + 46 : p + 46 + nlen].decode("utf-8")
        extra = tail[p + 46 + nlen : p + 46 + nlen + elen]
        q = 0
        while q + 4 <= len(extra):
            tag, length = struct.unpack("<HH", extra[q : q + 4])
            if tag == 1:
                wide = iter(struct.unpack(f"<{length // 8}Q", extra[q + 4 : q + 4 + length // 8 * 8]))
                usize = next(wide) if usize == 0xFFFFFFFF else usize
                csize = next(wide) if csize == 0xFFFFFFFF else csize
                lho = next(wide) if lho == 0xFFFFFFFF else lho
            q += 4 + length
        entries[name] = (method, csize, usize, lho)
        p += 46 + nlen + elen + clen
    return entries


def extract(url: str, total: int, entry: tuple[int, int, int, int], out: Path) -> tuple[str, int]:
    method, csize, usize, lho = entry
    if method not in (0, 8):
        raise Stale(f"compression method {method} is neither stored nor deflate")
    nlen, elen = struct.unpack("<HH", fetch(url, lho + 26, lho + 29, total))
    start = lho + 30 + nlen + elen
    data = fetch(url, start, start + csize - 1, total) if csize else b""
    digest, written = hashlib.sha256(), 0
    inflate = zlib.decompressobj(-15) if method == 8 else None
    with open(out, "wb") as f:
        for i in range(0, len(data), 1 << 22):
            chunk = data[i : i + (1 << 22)]
            chunk = inflate.decompress(chunk) if inflate else chunk
            digest.update(chunk)
            written += f.write(chunk)
        if inflate:
            rest = inflate.flush()
            digest.update(rest)
            written += f.write(rest)
    if written != usize:
        raise Stale(f"{out.name} inflated to {written} bytes, the archive says {usize}")
    return digest.hexdigest(), written


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 22), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main(argv: list[str]) -> int:
    pin_mode = argv[:1] == ["--pin"]
    pins = Path(argv[1] if pin_mode else argv[0])
    url, total, pinned = read_pins(pins)
    try:
        directory = central_directory(url, total)
        if pin_mode:
            for name in argv[2:]:
                if name not in directory:
                    raise Stale(f"{name} is not in the archive")
                out = Path(Path(name).name)
                sha, size = extract(url, total, directory[name], out)
                out.unlink()
                print(f"entry\t{name}\t{sha}\t{size}")
            return 0
        target = Path(argv[1])
        target.mkdir(parents=True, exist_ok=True)
        for name, sha, size in pinned:
            out = target / Path(name).name
            if out.is_file() and out.stat().st_size == size and sha256(out) == sha:
                print(f"  [ok]        {out.name}")
                continue
            if name not in directory:
                raise Stale(f"{name} is not in the archive")
            print(f"  [fetch]     {out.name}", flush=True)
            got_sha, got_size = extract(url, total, directory[name], out)
            if (got_sha, got_size) != (sha, size):
                out.unlink()
                raise Stale(f"{out.name}: {got_size} bytes sha256 {got_sha}, pinned {size} bytes {sha}")
            print(f"  [verified]  {out.name} ({got_size} bytes)", flush=True)
        return 0
    except Stale as error:
        print(f"error: {error}; re-pin research/bim4lca-models.tsv", file=sys.stderr)
        return 1
    except Unreachable as error:
        print(f"unreachable: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
