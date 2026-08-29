import { expect, test } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
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
