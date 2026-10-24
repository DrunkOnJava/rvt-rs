import { expect, test, type Download, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// W9 of docs/release-1.0-criteria.md: the whole viewer journey, open →
// inspect → schedule → export → diagnostics, on each reference model the
// gallery stages. Every download is checked against the others and against
// the counts tests/project_count_fixtures.rs pins for the CLI's export of the
// same file, so the browser and the command line are held to one result.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(__dirname, '../..');

// The gallery stages Einhoven and Core Interior; the RE1 models are opened
// from the project corpus, where Deploy viewer fetches them.
const corpus = path.resolve(__dirname, '../../_project_corpus/Revit');
const journeys: Journey[] = [
  {
    file: 'Revit_IFC5_Einhoven.rvt',
    release: 2023,
    demoId: 'einhoven-2023',
    fixture: 'revit-ifc5-einhoven.json',
    loadTimeout: 120_000,
  },
  {
    file: '2024_Core_Interior.rvt',
    release: 2024,
    demoId: 'core-interior-2024',
    fixture: '2024-core-interior.json',
    loadTimeout: 300_000,
  },
  ...['Architecture', 'Mechanical', 'Plumbing', 'Electrical'].map((discipline) => ({
    file: `RE1-${discipline}.rvt`,
    release: 2025,
    loadTimeout: 120_000,
  })),
];

interface Journey {
  file: string;
  release: number;
  /** The gallery card that opens it; otherwise it is opened from the corpus. */
  demoId?: string;
  /** Its tests/fixtures/project-counts manifest, when it has one. */
  fixture?: string;
  loadTimeout: number;
}

for (const journey of journeys) {
  const source = journey.demoId
    ? path.resolve(__dirname, '../public/demos', journey.file)
    : path.join(corpus, journey.file);
  // Deploy viewer fetches every model and sets RVT_VIEWER_REQUIRE_JOURNEYS,
  // so a model missing there fails; elsewhere it is a skip.
  const required = process.env.RVT_VIEWER_REQUIRE_JOURNEYS === '1';
  const journeyTest = required || fs.existsSync(source) ? test : test.skip;

  journeyTest(`the whole journey on ${journey.file}`, async ({ page }) => {
    test.setTimeout(900_000);
    expect(fs.existsSync(source), `${source} was not staged or fetched`).toBe(true);

    // Open.
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/Ready/);
    if (journey.demoId) {
      await page.locator(`[data-demo-id="${journey.demoId}"]`).click();
    } else {
      await page.locator('#file-input').setInputFiles(source);
    }
    await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: journey.loadTimeout });
    await expect(page.locator('#status')).not.toContainText(/error|unreachable|failed/i);
    await expect(page.locator('#file-meta')).toContainText(journey.file);

    // Inspect: the first element of the first category whose IFC entity
    // carries its own ElementId as its Tag (an IfcSpace has no Tag, and an
    // opening carries its host's), kept to find it in every output.
    const category = page
      .locator(
        '.tree-node.tree-category:not([data-ifc-type="IFCSPACE"]):not([data-ifc-type="IFCOPENINGELEMENT"])',
      )
      .first();
    const ifcType = await category.getAttribute('data-ifc-type');
    expect(ifcType).toMatch(/^IFC[A-Z]+$/);
    await category.click();
    const element = page.locator(`.tree-node.tree-element[data-ifc-type="${ifcType}"]`).first();
    await expect(element).toBeVisible();
    await element.click();
    const info = page.locator('#info');
    const elementId = (
      await info.locator('.info-row', { hasText: 'ElementId' }).locator('.v').first().textContent()
    )?.trim();
    expect(elementId).toMatch(/^\d+$/);

    // Schedule.
    const schedule = await text(await downloadOf(page, '#download-schedule'));
    const rows = schedule
      .replace(/^﻿/, '')
      .split('\r\n')
      .filter((line) => line.length > 0)
      .map((line) => line.split(','));
    expect(rows[0].slice(0, 2)).toEqual(['revit_element_id', 'ifc_type']);
    const scheduled = new Map<string, number>();
    for (const row of rows.slice(1)) scheduled.set(row[1], (scheduled.get(row[1]) ?? 0) + 1);
    expect(rows.slice(1).some((row) => row[0] === elementId && row[1] === ifcType)).toBe(true);

    // Export.
    const step = await text(await downloadOf(page, '#export-ifc'));
    expect(step).toMatch(/^ISO-10303-21;/);
    expect(step).toMatch(new RegExp(`=${ifcType}\\('[^']{22}',[^\\n]*'${elementId}'`));
    const glb = await bytes(await downloadOf(page, '#export-glb'));
    expect(glb.subarray(0, 4).toString('latin1')).toBe('glTF');
    expect(glb.readUInt32LE(4)).toBe(2);
    expect(glb.readUInt32LE(8)).toBe(glb.length);
    const svg = await text(await downloadOf(page, '#export-svg'));
    expect(svg).toContain('<svg');

    // Diagnostics.
    const diagnostics = JSON.parse(await text(await downloadOf(page, '#download-diagnostics')));
    expect(diagnostics.schema_version).toBe(1);
    expect(diagnostics.input.revit_version).toBe(journey.release);
    const byType: Record<string, number> = diagnostics.exported.by_ifc_type;
    // The schedule lists every element the diagnostics count, by type.
    expect(Object.fromEntries(scheduled)).toEqual(byType);
    expect(rows.length - 1).toBe(
      diagnostics.exported.building_elements + (byType.IFCOPENINGELEMENT ?? 0),
    );
    // The status panel says how many bodies are boxes (#409).
    const bodies = Object.values(diagnostics.exported.body_sources ?? {}) as number[];
    const shapes = page.locator('#status-panel .status-row', {
      has: page.locator('.status-label', { hasText: /^Shapes$/ }),
    });
    if (bodies.length > 0) {
      const total = bodies.reduce((sum, n) => sum + n, 0);
      await expect(shapes.locator('.status-value')).toContainText(String(total));
    }

    // The CLI's pinned counts for this file: its counts, relations and
    // storeys, each as tests/project_count_fixtures.rs reads them.
    if (!journey.fixture) return;
    const manifest = JSON.parse(
      fs.readFileSync(path.join(repo, 'tests/fixtures/project-counts', journey.fixture), 'utf8'),
    );
    type Pin = [string, string | undefined, number | undefined, number];
    type Section = Record<string, Record<string, unknown>>;
    const pins = (section: Section | undefined, prefix: string, expected: string): Pin[] =>
      Object.entries(section ?? {}).map(([name, spec]) => [
        `${prefix}${name}`,
        spec.decoder_metric as string | undefined,
        spec[expected] as number | undefined,
        (spec.decoder_tolerance as number | undefined) ?? 0,
      ]);
    const pinned: Pin[] = [
      ...pins(manifest.counts, '', 'decoder_expected'),
      ...pins(manifest.relations, 'relations.', 'decoder_expected_pairs'),
      ...pins(manifest.storeys, 'storeys.', 'decoder_expected_storeys'),
    ];
    let checked = 0;
    for (const [name, decoderMetric, expected, tolerance] of pinned) {
      if (!decoderMetric || expected === undefined) continue;
      const actual = metric(decoderMetric, diagnostics, step);
      expect(
        Math.abs(actual - expected),
        `${name}: ${decoderMetric} is ${actual}, the CLI's is ${expected}`,
      ).toBeLessThanOrEqual(tolerance);
      checked += 1;
    }
    expect(checked).toBeGreaterThan(0);
    await expect(page.locator('#status')).not.toContainText(/failed/i);
  });
}

async function downloadOf(page: Page, button: string): Promise<Download> {
  await expect(page.locator(button)).toBeEnabled();
  const [download] = await Promise.all([page.waitForEvent('download'), page.locator(button).click()]);
  return download;
}

async function bytes(download: Download): Promise<Buffer> {
  return fs.readFileSync((await download.path())!);
}

async function text(download: Download): Promise<string> {
  return (await bytes(download)).toString('utf8');
}

// The metrics tests/project_count_fixtures.rs reads (`metric_actual`).
function metric(name: string, diagnostics: { exported: Record<string, unknown> }, step: string): number {
  const exported = diagnostics.exported;
  if (name.startsWith('diagnostics.exported.by_ifc_type.')) {
    const byType = exported.by_ifc_type as Record<string, number>;
    return byType[name.slice('diagnostics.exported.by_ifc_type.'.length)] ?? 0;
  }
  if (name.startsWith('diagnostics.exported.')) {
    return exported[name.slice('diagnostics.exported.'.length)] as number;
  }
  if (name.startsWith('step.')) {
    return step.split(`${name.slice('step.'.length)}(`).length - 1;
  }
  throw new Error(`unsupported decoder metric ${name}`);
}
