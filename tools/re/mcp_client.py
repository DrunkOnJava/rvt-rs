"""A scripted client of the rvt-rs MCP server (#1327): JSON-RPC over the server's stdio, no language model.

`Server` starts the built `rvt-rs-mcp`, speaks the stateless 2026-07-28 lifecycle (every request
carries its protocol version in `_meta`, after one `server/discover`) and returns each tool's
`structuredContent`. `gather` asks the fixed query set that `tools/re/mcp_answers_vs_ifc.py`
scores and `dump` writes the answers as one deterministic JSON document (sorted keys, floats at
fixed precision, no handle, path, time or run-dependent value), which the semantic merge check
(#1334) reads on both sides of a pull request.

The queries, in order:

- `open_model` and `get_summary`, the same summary;
- `list_levels`;
- `list_elements` unfiltered, a page of 1000 at a time, so every element's row;
- `list_elements` once per IFC entity, per Revit class and per level, for the totals alone;
- `list_rooms`, a page of 1000 at a time (a server before #1883 gives every room in one answer, with no total);
- `get_element` by ElementId for the five lowest ElementIds of each IFC entity (every element when
  `details` is "all"), and by GlobalId for the first of them;
- `get_diagnostics`;
- `export_ifc` with no switch, into a folder of the caller's, for its size and hash.

Standard library only.
"""

import hashlib
import json
import os
import select
import subprocess
import sys
import tempfile
import time

PROTOCOL = "2026-07-28"
META = {
    "io.modelcontextprotocol/protocolVersion": PROTOCOL,
    "io.modelcontextprotocol/clientCapabilities": {},
}
PAGE = 1000
# Seconds a server that has decoded nothing yet is given to answer discovery and the tool list.
SHORT = 60.0
FLOAT_DIGITS = 6


class McpError(Exception):
    """The server did not answer as the protocol asks: a JSON-RPC error, a line that is not JSON, a closed pipe or no answer in time."""


class ToolError(McpError):
    """A tool answered `isError`: the message is its text."""


def takes_root(binary):
    """Whether the server has `--root` (#1883): Measure runs the scorers against the server of both sides of a run, and a side from before the flag reads any file."""
    help_text = subprocess.run([binary, "--help"], capture_output=True, text=True, timeout=60, check=True).stdout
    return "--root" in help_text


class Server:
    def __init__(self, binary, output_dir=None, timeout=2280.0, root=None):
        args = [binary]
        if root and takes_root(binary):
            # The server reads files only inside one folder; the model's own.
            args += ["--root", root]
        if output_dir:
            args += ["--output-dir", output_dir]
        self.timeout = timeout
        self.stderr = tempfile.TemporaryFile()
        self.process = subprocess.Popen(
            args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self.stderr
        )
        self.fd = self.process.stdout.fileno()
        self.buffer = b""
        self.next_id = 0
        self.checked = set()

    def line(self, deadline):
        while b"\n" not in self.buffer:
            wait = deadline - time.monotonic()
            if wait <= 0:
                raise McpError("the server gave no answer in time")
            ready, _, _ = select.select([self.fd], [], [], min(wait, 5.0))
            if not ready:
                if self.process.poll() is not None:
                    raise McpError(f"the server exited with status {self.process.returncode}")
                continue
            block = os.read(self.fd, 1 << 20)
            if not block:
                raise McpError(f"the server closed its output (status {self.process.wait()})")
            self.buffer += block
        text, _, self.buffer = self.buffer.partition(b"\n")
        return text

    def request(self, method, params=None, timeout=None):
        self.next_id += 1
        message = {
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": method,
            "params": {**(params or {}), "_meta": META},
        }
        try:
            self.process.stdin.write(json.dumps(message).encode() + b"\n")
            self.process.stdin.flush()
        except OSError as error:
            raise McpError(f"the server closed its input: {type(error).__name__}") from None
        deadline = time.monotonic() + (timeout or self.timeout)
        while True:
            raw = self.line(deadline)
            try:
                reply = json.loads(raw)
            except ValueError:
                raise McpError(f"stdout carried a line that is not JSON: {raw[:80]!r}") from None
            if reply.get("id") != self.next_id:
                continue
            if "error" in reply:
                raise McpError(f"{method}: JSON-RPC error {reply['error']}")
            return reply["result"]

    def discover(self):
        result = self.request("server/discover", timeout=SHORT)
        if PROTOCOL not in result.get("supportedVersions", []):
            raise McpError(f"server/discover lists no {PROTOCOL}: {result.get('supportedVersions')}")
        return result

    def tools(self):
        return self.request("tools/list", timeout=SHORT)["tools"]

    def call(self, tool, arguments):
        result = self.request("tools/call", {"name": tool, "arguments": arguments})
        text = "\n".join(
            block.get("text", "") for block in result.get("content", []) if block.get("type") == "text"
        )
        if result.get("isError"):
            raise ToolError(f"{tool}: {text}")
        if result.get("resultType") != "complete":
            raise McpError(f"{tool}: resultType {result.get('resultType')!r}")
        answer = result.get("structuredContent")
        if answer is None:
            raise McpError(f"{tool}: no structuredContent")
        # The text block is the same JSON: checked on a tool's first answer.
        if tool not in self.checked:
            self.checked.add(tool)
            try:
                same = json.loads(text) == answer
            except ValueError:
                same = False
            if not same:
                raise McpError(f"{tool}: the text block and structuredContent differ")
        return answer

    def close(self):
        """Close stdin as a client leaving does and return the server's exit status."""
        try:
            self.process.stdin.close()
        except OSError:
            pass
        try:
            status = self.process.wait(timeout=30)
        except subprocess.TimeoutExpired:
            self.process.kill()
            status = self.process.wait()
            raise McpError("the server did not exit after its stdin closed") from None
        finally:
            self.stderr.close()
        return status

    def kill(self):
        self.process.kill()
        self.process.wait()
        self.stderr.close()


def id_key(row):
    text = row.get("id") or ""
    return (0, int(text), "") if text.isdigit() else (1, 0, text)


def gather(server, model, export_dir, per_entity=5, details="sample", progress=None):
    """The answers of the fixed query set on `model`, as one dict. An export is
    written to `export_dir/model.ifc` (None to skip it)."""
    say = progress or (lambda text: None)
    answers = {}
    discover = server.discover()
    answers["discover_versions"] = discover["supportedVersions"]
    tools = server.tools()
    answers["tools"] = [tool["name"] for tool in tools]
    answers["tools_without_output_schema"] = [t["name"] for t in tools if not t.get("outputSchema")]
    opened = server.call("open_model", {"model": model})
    say("open_model")
    answers["summary"] = opened
    handle = opened["model_id"]
    answers["summary_again"] = server.call("get_summary", {"model_id": handle})
    answers["levels"] = server.call("list_levels", {"model": handle})["levels"]

    rows, offset = [], 0
    total = None
    while total is None or offset < total:
        page = server.call("list_elements", {"model": handle, "limit": PAGE, "offset": offset})
        total = page["total"]
        rows += page["elements"]
        if not page["elements"]:
            break
        offset += len(page["elements"])
    say(f"list_elements: {len(rows)} rows")
    answers["rows"] = rows
    answers["rows_total"] = total

    def count(**filters):
        return server.call("list_elements", {"model": handle, "limit": 1, **filters})["total"]

    summary = opened["elements"]
    answers["totals"] = {
        "category": {name: count(category=name) for name in sorted(summary["by_ifc_type"])},
        "class": {name: count(**{"class": name}) for name in sorted(summary["by_revit_class"])},
        "level": {level["name"]: count(level=level["name"]) for level in opened["levels"]},
    }
    rooms, offset = [], 0
    while True:
        page = server.call("list_rooms", {"model": handle, "limit": PAGE, "offset": offset})
        rooms += page["rooms"]
        # A server that does not page (before #1883) answers with every room and says no total.
        if "total" not in page or not page["rooms"]:
            break
        offset += len(page["rooms"])
        if offset >= page["total"]:
            break
    answers["rooms"] = rooms

    by_category = {}
    for row in rows:
        if row.get("id"):
            by_category.setdefault(row["provenance"]["category"], []).append(row)
    chosen = []
    for category in sorted(by_category):
        group = sorted(by_category[category], key=id_key)
        chosen += group if details == "all" else group[:per_entity]
    answers["details"] = {}
    for row in chosen:
        answers["details"][row["id"]] = server.call("get_element", {"model": handle, "id": row["id"]})
    say(f"get_element: {len(chosen)} elements")
    if chosen:
        first = chosen[0]
        gid = first["provenance"]["global_id"]
        answers["detail_by_global_id"] = {
            "id": first["id"],
            "answer": server.call("get_element", {"model": handle, "id": gid}),
        }
    answers["diagnostics"] = server.call("get_diagnostics", {"model": handle})
    if export_dir:
        target = os.path.join(export_dir, "model.ifc")
        answers["export"] = server.call("export_ifc", {"model": handle, "out_path": target})
        answers["export"]["path"] = target
    return answers


def canonical(value):
    """The value with its floats at fixed precision and no negative zero, so two runs of the same code dump the same bytes."""
    if isinstance(value, float):
        rounded = round(value, FLOAT_DIGITS)
        return 0.0 if rounded == 0 else rounded
    if isinstance(value, dict):
        return {key: canonical(item) for key, item in value.items()}
    if isinstance(value, list):
        return [canonical(item) for item in value]
    return value


def drop_handles(value):
    """The answers without `model_id`, which hashes the file's modification time."""
    if isinstance(value, dict):
        return {
            key: drop_handles(item)
            for key, item in value.items()
            if key != "model_id"
        }
    if isinstance(value, list):
        return [drop_handles(item) for item in value]
    return value


def dump(answers):
    """The semantic dump: the answers gathered, sorted and canonical, as JSON text."""
    keep = {
        key: answers[key]
        for key in ("summary", "levels", "totals", "rooms", "details", "diagnostics")
        if key in answers
    }
    keep["rows"] = sorted(answers["rows"], key=lambda row: (id_key(row), row["provenance"].get("global_id", "")))
    keep["details"] = {key: keep["details"][key] for key in sorted(keep.get("details", {}), key=lambda k: id_key({"id": k}))}
    return json.dumps(canonical(drop_handles(keep)), sort_keys=True, indent=1, ensure_ascii=True) + "\n"


def sha256_without_export_time(path):
    """The hash tools/ci/measure-reference-models.sh writes to ifc.sha256: the IFC without its FILE_NAME and IFCOWNERHISTORY lines."""
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for line in handle:
            if line.startswith(b"FILE_NAME(") or b"IFCOWNERHISTORY" in line:
                continue
            digest.update(line if line.endswith(b"\n") else line + b"\n")
    return digest.hexdigest()


def main(argv):
    if len(argv) < 3:
        print("usage: mcp_client.py <rvt-rs-mcp> <model.rvt> [--all]  (prints the semantic dump)", file=sys.stderr)
        return 2
    with tempfile.TemporaryDirectory() as scratch:
        server = Server(argv[1], output_dir=scratch, root=os.path.dirname(os.path.abspath(argv[2])))
        try:
            answers = gather(server, argv[2], scratch, details="all" if "--all" in argv[3:] else "sample")
        finally:
            server.close()
    sys.stdout.write(dump(answers))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
