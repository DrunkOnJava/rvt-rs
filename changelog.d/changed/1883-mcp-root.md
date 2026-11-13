- **`rvt-rs-mcp` reads models from, and writes exports to, one folder that its
  setup picks; a path that leaves it is refused (#1883).** The names, values
  and messages in a Revit file reach the client's model unmarked, so a crafted
  file could steer it into asking the server to open a file outside the
  project, or to overwrite one; until now the server read any file the account
  could read and wrote anywhere but inside `--output-dir`. The folder is
  `--root <folder>`, else the environment variable `RVT_MCP_ROOT` (for a
  client that passes only an environment), else the folder the server was
  started in, unless that is the top of a file system, the home folder or a
  folder that holds it (an MCP client chooses a server's working directory, and
  some start theirs from `/`): then the server starts and each tool that
  touches a file refuses with a message that names `--root`. A relative path
  is taken from the folder; a path is compared with it once `..` and links are
  resolved, and one whose text alone puts it outside is refused before any file
  is looked at, so the refusal does not say whether the file exists. The tool
  descriptions and the refusals say where the folder is and how to move it.
  `--root /` and `--unconfined` lift the limit, and `--output-dir` still sends
  the exports to a folder of their own. **Upgrade:** a setup that opens files
  outside the folder its client starts the server in now passes `--root` (or
  sets `RVT_MCP_ROOT`), or `--unconfined` for the old reach (`docs/mcp.md`,
  "The root folder"). Not defended: another process that rewrites the links
  inside the folder while a call runs.
