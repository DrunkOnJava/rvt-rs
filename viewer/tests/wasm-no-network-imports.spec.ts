import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { auditWasmImports, canaryFailure, moduleImporting } from '../scripts/audit-wasm-imports.mjs';

/**
 * VW1-21 compile-time invariant: the packaged WASM must not import
 * network primitives. Complements the browser traffic test in
 * `no-network.spec.ts` (which needs a sample file) — this check always
 * runs against `viewer/pkg/rvt_bg.wasm` and its glue, `viewer/pkg/rvt.js`,
 * through `scripts/audit-wasm-imports.mjs`.
 */

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const wasmPath = path.resolve(__dirname, '../pkg/rvt_bg.wasm');
const gluePath = path.resolve(__dirname, '../pkg/rvt.js');

test('compiled WASM imports no network primitives (VW1-21)', () => {
  expect(fs.existsSync(wasmPath), `missing ${wasmPath} — build viewer/pkg first`).toBe(true);
  expect(fs.existsSync(gluePath), `missing ${gluePath} — build viewer/pkg first`).toBe(true);

  const { imports, hits, unread } = auditWasmImports(
    fs.readFileSync(wasmPath),
    fs.readFileSync(gluePath, 'utf8'),
  );
  expect(imports, 'the wasm has no imports to read').toBeGreaterThan(0);
  expect(unread, `imports with no shim the audit can read:\n${unread.join('\n')}`).toEqual([]);
  expect(hits, `VW1-21 violation — network imports:\n${hits.join('\n')}`).toEqual([]);
});

// wasm-bindgen names a binding of `fetch` `__wbg_fetch_<hash>`. An audit that
// does not flag a module importing one cannot fail on the real wasm either.
test('the import audit flags a module that imports fetch (VW1-21 canary)', () => {
  const { hits } = auditWasmImports(
    moduleImporting(['__wbg_fetch_0']),
    '__wbg_fetch_0: function(arg0) { return fetch(arg0); }',
  );
  expect(hits, "the audit missed the canary's fetch import").not.toEqual([]);
  expect(canaryFailure(), 'the audit script misread its own canary').toBeNull();
});
