import { expect, test } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const coreInteriorPath = path.resolve(__dirname, '../public/demos/2024_Core_Interior.rvt');
const coreInteriorTest = fs.existsSync(coreInteriorPath) ? test : test.skip;

// B49 (#35): Core Interior's export writes Revit's parameter values into its
// Pset_ property sets, so the status panel's Parameters row reports how many
// instead of saying the parameter table is not read.
coreInteriorTest(
  'the Parameters row counts the Revit parameter values Core Interior exports',
  async ({ page }) => {
    test.slow();
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/Ready/);
    await page.locator('[data-demo-id="core-interior-2024"]').click();
    await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 300_000 });

    const row = page.locator('#status-panel .status-row', {
      has: page.locator('.status-label', { hasText: /^Parameters$/ }),
    });
    await expect(row).toHaveCount(1);
    await expect(row.locator('.status-value')).toHaveText(/^[1-9][\d,]* /);
    await expect(row.locator('.status-value')).not.toContainText('not read yet');
  },
);
