import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * VW1-21 compile-time invariant: the packaged WASM must not import
 * network primitives. Complements the browser traffic test in
 * `no-network.spec.ts` (which needs a sample file) — this check always
 * runs against `viewer/pkg/rvt_bg.wasm`.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const wasmPath = path.resolve(__dirname, '../pkg/rvt_bg.wasm');
const NETWORK_IMPORT_RE =
  /"(fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)"/i;

test('compiled WASM imports no network primitives (VW1-21)', () => {
  expect(fs.existsSync(wasmPath), `missing ${wasmPath} — build viewer/pkg first`).toBe(true);

  let dump: string;
  try {
    dump = execFileSync('wasm-objdump', ['-j', 'Import', '-x', wasmPath], {
      encoding: 'utf8',
      maxBuffer: 16 * 1024 * 1024,
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    test.skip(
      true,
      `wasm-objdump unavailable or failed (${message}). Install wabt to enforce VW1-21.`,
    );
    return;
  }

  const hits = dump.split(/\r?\n/).filter((line) => NETWORK_IMPORT_RE.test(line));
  expect(hits, `VW1-21 violation — network imports:\n${hits.join('\n')}`).toEqual([]);
});

// A module with one function import per name, from `./rvt_bg.js`, the module
// wasm-bindgen's glue provides its imports under.
function moduleImporting(names: string[]): Uint8Array {
  const str = (s: string) => [s.length, ...Buffer.from(s)];
  const section = (id: number, bytes: number[]) => [id, bytes.length, ...bytes];
  const entries = names.flatMap((name) => [...str('./rvt_bg.js'), ...str(name), 0x00, 0x00]);
  return new Uint8Array([
    0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
    ...section(1, [0x01, 0x60, 0x00, 0x00]),
    ...section(2, [names.length, ...entries]),
  ]);
}

// wasm-bindgen names a binding of `fetch` `__wbg_fetch_<hash>`. An audit that
// does not flag a module importing one cannot fail on the real wasm either.
test('the import audit flags a module that imports fetch (VW1-21 canary)', () => {
  const canary = path.join(fs.mkdtempSync(path.join(os.tmpdir(), 'vw1-21-')), 'canary.wasm');
  fs.writeFileSync(canary, moduleImporting(['__wbg_fetch_0']));
  const dump = execFileSync('wasm-objdump', ['-j', 'Import', '-x', canary], { encoding: 'utf8' });
  const hits = dump.split(/\r?\n/).filter((line) => NETWORK_IMPORT_RE.test(line));
  expect(hits, `the audit missed the canary's fetch import:\n${dump}`).not.toEqual([]);
});
