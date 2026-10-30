import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1075: a stream over the reader's resource limits is not read, and the
// viewer says so and offers "Retry with large-model limits", which reopens
// the same file with the limits `rvt-ifc --large-model` uses. A client model
// over 256 MiB is too big for CI, so `?maxStreamBytes=200` opens the first
// file under a 200-byte stream limit instead: the committed tier-1 fixture's
// BasicFileInfo is 236 bytes, which is what tests/resource_limits_cli.rs
// stops the same way for the command line.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const fixture = path.resolve(
  __dirname,
  '../../corpus/tier1/architectural-2024/architectural-2024.rvt',
);

const NOT_READ = 'Part of the file not read';

function statusRow(page: Page, label: string) {
  return page.locator('#status-panel .status-row', {
    has: page.locator('.status-label', { hasText: new RegExp(`^${label}$`) }),
  });
}

async function openFixture(page: Page): Promise<void> {
  await page.locator('#file-input').setInputFiles(fixture);
  await expect(page.locator('#status')).toHaveText(/Loaded/);
}

async function diagnosticsJson(page: Page): Promise<string> {
  const details = page.locator('#diagnostics-details');
  if ((await details.getAttribute('open')) === null) {
    await details.locator('summary').click();
  }
  return (await page.locator('#diagnostics-json').textContent()) ?? '';
}

test('streams over the limits are reported, and the large-model retry reads them', async ({
  page,
}) => {
  expect(fs.existsSync(fixture), `${fixture} is committed`).toBe(true);
  await page.goto('/?maxStreamBytes=200');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await openFixture(page);

  // Reported: the Mode row, the diagnostics and the empty-viewport note all
  // say part of the file was not read, rather than calling it a scaffold.
  const mode = statusRow(page, 'Mode');
  await expect(mode).toContainText(NOT_READ);
  await expect(mode).toContainText("over the browser's resource limits");
  const before = await diagnosticsJson(page);
  expect(before).toContain('stream_not_read_resource_limit');
  expect(before).toContain('Not read whole: stream BasicFileInfo ');
  await expect(page.locator('#scaffold-note')).toBeVisible();
  await expect(page.locator('#scaffold-note')).toContainText('Part of the file was not read');
  await expect(page.locator('#scaffold-note')).not.toContainText(/expected result/i);

  // Offered: one retry button, which names the 4 GiB browser ceiling.
  const retry = page.getByRole('button', { name: 'Retry with large-model limits' });
  await expect(retry).toHaveCount(1);
  await expect(retry).toBeVisible();
  await expect(retry).toBeEnabled();
  await expect(page.locator('#large-model-note')).toContainText('4 GiB');
  await expect(page.locator('#large-model-note')).toContainText('rvt-ifc --large-model');
  await expect(retry).toHaveAccessibleDescription(/at most 4 GiB of memory/);

  // Retried: the same file is read again under the large-model limits.
  await retry.click();
  // The loading panel replaces the button as the click lands; a click that
  // did nothing would leave it there.
  await expect(retry).toHaveCount(0);
  await expect(page.locator('#status')).toHaveText(/Loaded/);
  await expect(page.locator('#status')).not.toContainText(/error|failed/i);
  await expect(page.locator('#load-overlay')).toBeHidden();

  await expect(mode).not.toContainText(NOT_READ);
  await expect(page.getByRole('button', { name: 'Retry with large-model limits' })).toHaveCount(0);
  await expect(statusRow(page, 'File')).toContainText('Opened Revit 2024');
  const after = await diagnosticsJson(page);
  expect(after).not.toContain('stream_not_read_resource_limit');
  expect(after).not.toContain('Not read whole');
  await expect(page.locator('#file-meta')).toContainText('architectural-2024.rvt');
  await expect(page.locator('#file-meta')).toContainText(/[1-9]\d* entities/);
  await expect(page.locator('.tree-node').first()).toContainText(/IFCPROJECT/i);
  await expect(page.locator('#export-ifc')).toBeEnabled();
  // The fixture is a scaffold once read whole, and the note says that again.
  await expect(page.locator('#scaffold-note')).toContainText(/expected result/i);
});

test('the limits in the address apply to the first open only', async ({ page }) => {
  await page.goto('/?maxStreamBytes=200');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await openFixture(page);
  await expect(statusRow(page, 'Mode')).toContainText(NOT_READ);

  await page.locator('#file-input').setInputFiles({
    name: 'second-open.rvt',
    mimeType: 'application/octet-stream',
    buffer: fs.readFileSync(fixture),
  });
  await expect(page.locator('#file-meta')).toContainText('second-open.rvt');
  await expect(page.locator('#status')).toHaveText(/Loaded/);
  await expect(statusRow(page, 'Mode')).not.toContainText(NOT_READ);
  await expect(page.getByRole('button', { name: 'Retry with large-model limits' })).toHaveCount(0);
  expect(await diagnosticsJson(page)).not.toContain('Not read whole');
});

test('at the default limits the fixture is read whole and no retry is offered', async ({
  page,
}) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await openFixture(page);

  await expect(statusRow(page, 'Mode')).not.toContainText(NOT_READ);
  await expect(page.getByRole('button', { name: 'Retry with large-model limits' })).toHaveCount(0);
  await expect(statusRow(page, 'File')).toContainText('Opened Revit 2024');
  expect(await diagnosticsJson(page)).not.toContain('Not read whole');
});

test('an address with no whole-number limit leaves the defaults', async ({ page }) => {
  await page.goto('/?maxStreamBytes=200.5&maxInflateBytes=-1');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await openFixture(page);

  await expect(statusRow(page, 'Mode')).not.toContainText(NOT_READ);
  expect(await diagnosticsJson(page)).not.toContain('Not read whole');
});
