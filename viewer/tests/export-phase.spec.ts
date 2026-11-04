import { expect, test, type Download, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1339: the export phase in the viewer, as `rvt-ifc --phase` and the Python
// API have it, on RE1 Architecture (Revit 2025, Drshelden/IFC-ECS, MIT). Its
// phases are Existing (ElementId 12589) and New Construction (86961, the
// last), and its 11 rooms are in the second. This is the browser twin of
// `ifc_phase_option_chooses_the_export_phase_of_a_real_file` in
// tests/cli_real_files.rs: the select lists the file's phases with the last as
// the default; the default export has every room; the first phase leaves the 11
// out; choosing the default again gives the first export back.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const source = path.resolve(__dirname, '../../_project_corpus/Revit/RE1-Architecture.rvt');
// Deploy viewer fetches the RE1 models and sets this, so a model missing there
// fails; elsewhere it is a skip.
const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';
const phaseTest = required || fs.existsSync(source) ? test : test.skip;

async function stepOf(page: Page): Promise<string> {
  await expect(page.locator('#export-ifc')).toBeEnabled();
  const [download]: [Download, void] = await Promise.all([
    page.waitForEvent('download'),
    page.locator('#export-ifc').click(),
  ]);
  return fs.readFileSync((await download.path())!).toString('utf8');
}

/** The STEP text without the two places the wall clock is written. */
function body(step: string): string {
  return step
    .split('\n')
    .filter((line) => !line.includes('FILE_NAME(') && !line.includes('IFCOWNERHISTORY('))
    .join('\n');
}

function spaces(step: string): number {
  return step.split('=IFCSPACE(').length - 1;
}

/** Changes the select and waits for the file to be read again with the phase. */
async function choosePhase(page: Page, value: string): Promise<void> {
  await page.locator('#export-phase').selectOption({ value });
  // The page clears the last open's stats as the reopen starts and sets them
  // when the model is ready.
  await expect(page.locator('#status')).toHaveAttribute('data-load-stats', /.+/, {
    timeout: 120_000,
  });
  await expect(page.locator('#status')).toHaveText(/Loaded/);
}

phaseTest('the export phase is chosen in the viewer as rvt-ifc --phase chooses it', async ({
  page,
}) => {
  test.setTimeout(600_000);
  expect(fs.existsSync(source), `${source} was not fetched`).toBe(true);

  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  // A file with no choice shows none: the field appears with the model.
  await expect(page.locator('#export-phase-field')).toBeHidden();
  await page.locator('#file-input').setInputFiles(source);
  await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 120_000 });

  // The file's phases in order, the last the default.
  const select = page.locator('#export-phase');
  await expect(select).toBeVisible();
  await expect(select).toBeEnabled();
  await expect(select.locator('option')).toHaveText(['Existing', 'New Construction (last, default)']);
  await expect(select).toHaveValue('');

  // The default export is the last phase, with every room.
  const defaultStep = await stepOf(page);
  expect(spaces(defaultStep)).toBe(11);
  await expect(page.locator('#download-rooms')).toBeEnabled();

  // The first phase leaves the 11 rooms out.
  await choosePhase(page, '12589');
  await expect(select).toHaveValue('12589');
  await expect(select.locator('option')).toHaveCount(2);
  const firstStep = await stepOf(page);
  expect(spaces(firstStep)).toBe(0);
  expect(body(firstStep)).not.toBe(body(defaultStep));
  await expect(page.locator('#download-rooms')).toBeDisabled();

  // The default again is the first export back.
  await choosePhase(page, '');
  await expect(select).toHaveValue('');
  expect(body(await stepOf(page))).toBe(body(defaultStep));
});
