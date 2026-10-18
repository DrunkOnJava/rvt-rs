// VW1-21: the compiled viewer wasm imports no network primitive.
//
// wasm-bindgen gives every import a shim in its glue (pkg/rvt.js), keyed by
// the import's name, `__wbg_<js name>_<hash>`. A binding of `fetch` or
// `sendBeacon` shows in the name; a constructor (`new WebSocket(...)`) or a
// method (`xhr.open(...)`) imports as `__wbg_new_<hash>` or
// `__wbg_open_<hash>` and shows only in its shim's body. So each import is
// checked twice: its name, and the body of the shim the glue gives it. The
// glue's own `fetch(module_or_path)`, which loads the wasm, sits in its init
// function, outside every shim, and is not an import.
//
// The audit fails, rather than passing, when:
//   - pkg/rvt_bg.wasm or pkg/rvt.js is missing;
//   - an import has no shim in the glue: a layout this script cannot read;
//   - its canary is not caught. The canary is a module importing
//     `__wbg_fetch_0`, `__wbg_new_1` and `__wbg_now_2`, whose shims call
//     fetch, new WebSocket and Date.now, beside an init-time fetch: the
//     first two must be flagged and the third and the init must not.
//
// Until October 2026 the audit grepped `wasm-objdump -j Import -x` for
// `"fetch"` and the like, quoted, and wabt prints an import as
// ` <- module.field` with no quotes (src/binary-reader-objdump.cc), so it
// could not fail.
//
// Usage: node scripts/audit-wasm-imports.mjs [--json]

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const NETWORK_NAME = /fetch|xmlhttprequest|websocket|eventsource|sendbeacon|webtransport|rtcpeerconnection/i;
const NETWORK_CALL =
  /\bfetch\s*\(|\bXMLHttpRequest\b|\bWebSocket\b|\bEventSource\b|\bsendBeacon\b|\bWebTransport\b|\bRTCPeerConnection\b/;

// The end of the string literal that opens at `start`.
function stringEnd(text, start) {
  const quote = text[start];
  for (let i = start + 1; i < text.length; i++) {
    if (text[i] === '\\') i++;
    else if (text[i] === quote) return i;
  }
  return text.length;
}

// The body of the shim the glue defines for import `name`, as
// `name: function(...) {...}` or `name = function(...) {...}`, or null.
function shimBody(glue, name) {
  const at = new RegExp(`\\b${name}\\s*[:=]\\s*function\\b`).exec(glue);
  if (!at) return null;
  const open = glue.indexOf('{', at.index + at[0].length);
  if (open < 0) return null;
  let depth = 0;
  for (let i = open; i < glue.length; i++) {
    const c = glue[i];
    if (c === '"' || c === "'" || c === '`') i = stringEnd(glue, i);
    else if (c === '{') depth++;
    else if (c === '}' && --depth === 0) return glue.slice(open, i + 1);
  }
  return null;
}

export function auditWasmImports(wasmBytes, glue) {
  const imports = WebAssembly.Module.imports(new WebAssembly.Module(wasmBytes));
  const hits = [];
  const unread = [];
  for (const { module, name } of imports) {
    const label = `${module}.${name}`;
    if (NETWORK_NAME.test(name)) hits.push(`${label}: its name`);
    const body = shimBody(glue, name);
    if (body === null) unread.push(label);
    else if (NETWORK_CALL.test(body)) hits.push(`${label}: ${body.slice(0, 200)}`);
  }
  return { imports: imports.length, hits, unread };
}

// A module with one function import per name, from `./rvt_bg.js`, the module
// wasm-bindgen's glue provides its imports under.
export function moduleImporting(names) {
  const str = (s) => [s.length, ...Buffer.from(s)];
  const section = (id, bytes) => [id, bytes.length, ...bytes];
  const entries = names.flatMap((name) => [...str('./rvt_bg.js'), ...str(name), 0x00, 0x00]);
  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...section(1, [0x01, 0x60, 0x00, 0x00]),
    ...section(2, [names.length, ...entries]),
  ]);
}

const CANARY_GLUE = `
async function __wbg_init(module_or_path) {
  module_or_path = fetch(module_or_path);
}
function __wbg_get_imports() {
  const import0 = {
    __wbg_fetch_0: function(arg0) { return fetch(arg0); },
    __wbg_new_1: function(arg0) { return new WebSocket(arg0); },
    __wbg_now_2: function() { return Date.now(); },
  };
  return { './rvt_bg.js': import0 };
}
`;

// What the audit says of its canary, or null when it says the right thing.
export function canaryFailure() {
  const { hits, unread } = auditWasmImports(
    moduleImporting(['__wbg_fetch_0', '__wbg_new_1', '__wbg_now_2']),
    CANARY_GLUE,
  );
  const flagged = (name) => hits.some((hit) => hit.startsWith(`./rvt_bg.js.${name}:`));
  if (unread.length > 0) return `the canary's shims were not read: ${unread.join(', ')}`;
  if (!flagged('__wbg_fetch_0')) return 'the canary fetch import was not flagged';
  if (!flagged('__wbg_new_1')) return 'the canary WebSocket constructor was not flagged';
  if (flagged('__wbg_now_2')) return 'the canary Date.now import was flagged';
  return null;
}

function main() {
  const json = process.argv.includes('--json');
  const pkg = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../pkg');
  const report = { canary: canaryFailure() ?? 'caught', imports: 0, hits: [], unread: [] };
  const wasm = path.join(pkg, 'rvt_bg.wasm');
  const glue = path.join(pkg, 'rvt.js');
  const missing = [wasm, glue].filter((file) => !fs.existsSync(file));
  if (missing.length > 0) {
    report.missing = missing;
  } else {
    Object.assign(report, auditWasmImports(fs.readFileSync(wasm), fs.readFileSync(glue, 'utf8')));
  }
  const ok =
    report.canary === 'caught' && !report.missing && report.hits.length === 0 && report.unread.length === 0;
  if (json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(`VW1-21: ${report.imports} imports read; canary ${report.canary}`);
    for (const file of report.missing ?? []) console.log(`missing ${file}: build viewer/pkg first`);
    for (const hit of report.hits) console.log(`network import: ${hit}`);
    for (const label of report.unread) console.log(`no shim found in the glue for ${label}`);
  }
  process.exit(ok ? 0 : 1);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) main();
