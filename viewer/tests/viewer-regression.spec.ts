import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectSamplePath = resolveProjectSamplePath();
const projectSampleTest = projectSamplePath === null ? test.skip : test;
const stagedDemoPath = resolveStagedDemoPath();
const stagedDemoId = stagedDemoPath?.id ?? 'architectural-2024';
const stagedDemoTest = stagedDemoPath === null ? test.skip : test;

test('loads the viewer shell with disabled export actions and demo gallery', async ({ page }) => {
  await page.goto('/');

  await expect(page.locator('#status')).toHaveText(/ready/);
  await expect(page.locator('#dropzone')).toBeVisible();
  await expect(page.locator('#export-quality')).toHaveText(/pending/);
  await expect(page.locator('#export-mode')).toHaveValue('scaffold');
  await expect(page.locator('#export-glb')).toBeDisabled();
  await expect(page.locator('#export-ifc')).toBeDisabled();
  await expect(page.locator('#export-svg')).toBeDisabled();
  await expect(page.locator('#download-diagnostics')).toBeDisabled();
  await expect(page.locator('#status-panel')).toContainText('No file opened');
  await expect(page.locator('#status-panel')).toContainText(/Decode|Export|IFC bar/i);
  await expect(page.locator('#mvp-workflow')).toContainText(/tier1|Open locally/i);
  await expect(page.locator('#demo-gallery')).toBeVisible();
  await expect(page.locator('#demo-attribution')).toContainText(
    /redistributable|Apache|tier1|phi-ag/i,
  );
  await expect(page.locator('[data-demo-id="architectural-2024"]')).toBeVisible();
  await expect(page.locator('[data-demo-id="structural-2023"]')).toBeVisible();
  await expect(page.locator('[data-demo-id="mep-2024"]')).toBeVisible();
  await expect(page.locator('[data-demo-id="architectural-2024"]')).toContainText(
    /expected:\s*Scaffold/i,
  );
  await expect(page.locator('[data-demo-id="synthetic-project"]')).toContainText(
    /Reference download|IFC/i,
  );
  await expect(page.getByLabel('Supported Revit file profile')).toContainText(/Synthetics/i);
  await expect(page.getByLabel('Supported Revit file profile')).toContainText(/scaffold ~25%/i);
});

test('accessibility shell: landmarks, skip link, keyboard tree activation', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/ready/);

  await expect(page.getByRole('link', { name: /Skip to viewport/i })).toBeAttached();
  await expect(page.getByRole('banner')).toBeVisible();
  await expect(page.getByRole('main', { name: /3D viewport/i })).toBeVisible();
  await expect(page.getByLabel('Scene tree and categories')).toBeVisible();
  await expect(page.getByLabel('File status and element details')).toBeVisible();
  await expect(page.getByRole('button', { name: /Choose file/i })).toBeVisible();
  await expect(page.getByLabel('Export glTF')).toBeDisabled();
  await expect(page.getByLabel('Export IFC')).toBeDisabled();
  await expect(page.getByLabel('Export plan SVG')).toBeDisabled();

  // Tab order reaches the file picker and export controls.
  await page.keyboard.press('Tab');
  await expect(page.locator(':focus')).toBeVisible();

  const demo = page.locator('[data-demo-id="architectural-2024"]');
  if (await demo.isEnabled()) {
    await demo.focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('#status')).toHaveText(/loaded/);
    const treeNode = page.locator('.tree-node[role="treeitem"]').first();
    await expect(treeNode).toBeVisible();
    await treeNode.focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('#info')).toContainText(/ifc_type|IFCPROJECT/i);
    await page.keyboard.press('Escape');
    await expect(page.locator('.tree-node.selected')).toHaveCount(0);
  }
});

stagedDemoTest(
  'MVP workflow via demo gallery: open → status/confidence → inspect → export labels',
  async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);
    await expect(page.locator(`[data-demo-id="${stagedDemoId}"]`)).toBeEnabled();

    await page.locator(`[data-demo-id="${stagedDemoId}"]`).click();
    await expect(page.locator('#status')).toHaveText(/loaded/);
    await expect(page.locator('#dropzone')).toBeHidden();

    await expect(page.locator('#export-glb')).toBeEnabled();
    await expect(page.locator('#export-ifc')).toBeEnabled();
    await expect(page.locator('#export-svg')).toBeEnabled();
    await expect(page.locator('#download-diagnostics')).toBeEnabled();
    await expect(page.locator('#export-quality')).toContainText(
      /Scaffold|Typed|Geometry|Diagnostic|Proxy|Unknown/,
    );

    await expect(page.locator('#status-panel')).toContainText(
      /Partial decode|Scaffold-only|Supported profile|unsupported model layout/i,
    );
    await expect(page.locator('#status-panel')).toContainText(/Decode/i);
    await expect(page.locator('#status-panel')).toContainText(/Export/i);
    await expect(page.locator('#status-panel')).toContainText(/IFC bar/i);
    await expect(page.locator('#status-panel')).toContainText(/Scaffold|scaffold/i);
    await expect(page.locator('#status-panel')).toContainText(/%/);

    await page.locator('#export-mode').selectOption('geometry');
    await expect(page.locator('#status-panel')).toContainText(/Selected geometry/i);
    await expect(page.locator('#status')).toContainText(/geometry/i);
    await page.locator('#export-mode').selectOption('scaffold');

    await page.locator('#diagnostics-details summary').click();
    await expect(page.locator('#diagnostics-json')).toContainText(
      /confidence|schema_version|warnings/i,
    );

    const treeNode = page.locator('.tree-node').first();
    await expect(treeNode).toBeVisible();
    await expect(treeNode).toContainText(/IFCPROJECT/i);
    await treeNode.click();
    await expect(page.locator('#info')).toContainText(/ifc_type/i);
    await expect(page.locator('#info')).toContainText(/IFCPROJECT/i);
    await expect(page.locator('.category-toggle')).toContainText(/IFCPROJECT/i);

    const ifcTitle = await page.locator('#export-ifc').getAttribute('title');
    expect(ifcTitle ?? '').toMatch(/Scaffold|Typed|Geometry|Diagnostic|Proxy|Unknown/i);
    expect(ifcTitle ?? '').toMatch(/elements/i);
    await expect(page.locator('#export-quality')).toContainText(/Scaffold/);
  },
);

projectSampleTest(
  'opens a project sample and exposes geometry diagnostics, toggles, and element info',
  async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    await page.locator('#file-input').setInputFiles(projectSamplePath!);
    await expect(page.locator('#status')).toHaveText(/loaded/);
    await expect(page.locator('#dropzone')).toBeHidden();

    await expect(page.locator('#export-glb')).toBeEnabled();
    await expect(page.locator('#export-ifc')).toBeEnabled();
    await expect(page.locator('#export-svg')).toBeEnabled();
    await expect(page.locator('#download-diagnostics')).toBeEnabled();
    await expect(page.locator('#export-quality')).toContainText(/Geometry|Typed|Scaffold/);

    // Elevation-derived ArcWall storeys remove the old missing-level gap for
    // walls; Floor/Room rows still report unsupported_geometry_missing_level
    // until Level ElementId bind (#33 leftover).
    await expect(page.locator('#status-panel')).toContainText('Partial decode');
    await expect(page.locator('#status-panel')).toContainText(/unit|thickness|storey/i);
    // #33 leftover: File Status lists recovered storey names, not counts only.
    await expect(page.locator('#status-panel')).toContainText(/Level 1|Roof/i);
    await expect(page.locator('#status-panel')).toContainText(/Materials/i);
    // Scene tree groups under IFCBUILDINGSTOREY nodes from recovered levels.
    await expect(page.locator('.tree-node.tree-storey').first()).toBeVisible();
    await expect(page.locator('.tree-node.tree-storey').first()).toContainText(
      /IFCBUILDINGSTOREY/,
    );
    await page.locator('#diagnostics-details summary').click();
    await expect(page.locator('#diagnostics-json')).toContainText('"schema_version": 1');
    await expect(page.locator('#diagnostics-json')).toContainText('"storey_count": 4');
    await expect(page.locator('#diagnostics-json')).toContainText('"storey_names"');
    // Post Finding 1 / partition Material recovery: einhoven emits 42 materials
    // (was 41). Keep in sync with tests/fixtures/project-counts/revit-ifc5-einhoven.json.
    await expect(page.locator('#diagnostics-json')).toContainText('"material_count": 42');
    await expect(page.locator('#diagnostics-json')).toContainText('lack recovered thickness');
    // ArcWalls are storey-assigned; missing_level gaps are Floor/Room only.
    await expect(page.locator('#diagnostics-json')).toContainText(
      'unsupported_geometry_missing_level',
    );

    const firstCategory = page.locator('.category-toggle').first();
    await expect(firstCategory).toBeVisible();
    const categoryCheckbox = firstCategory.locator('input');
    await expect(categoryCheckbox).toBeChecked();
    await categoryCheckbox.uncheck();
    await expect(categoryCheckbox).not.toBeChecked();
    await categoryCheckbox.check();
    await expect(categoryCheckbox).toBeChecked();

    const wallNode = page.locator('.tree-node', { hasText: 'IFCWALL' }).first();
    await expect(wallNode).toBeVisible();
    await wallNode.click();
    await expect(page.locator('#info')).toContainText('ifc_type');
    await expect(page.locator('#info')).toContainText('IFCWALL');

    expect(await viewportScreenshotHasVisibleContent(page)).toBe(true);
  },
);

function resolveProjectSamplePath(): string | null {
  const candidates = [
    process.env.RVT_VIEWER_SAMPLE,
    path.resolve(__dirname, '../../_project_corpus/Revit/Revit_IFC5_Einhoven.rvt'),
  ].filter((candidate): candidate is string => Boolean(candidate));

  for (const candidate of candidates) {
    const resolved = path.isAbsolute(candidate)
      ? candidate
      : path.resolve(process.cwd(), candidate);
    if (fs.existsSync(resolved)) return resolved;
  }
  return null;
}

function resolveStagedDemoPath(): { id: string; path: string } | null {
  const candidates = [
    { id: 'architectural-2024', path: path.resolve(__dirname, '../public/demos/architectural-2024.rvt') },
    { id: 'synthetic-mvp', path: path.resolve(__dirname, '../public/demos/synthetic-mvp.rvt') },
    { id: 'structural-2023', path: path.resolve(__dirname, '../public/demos/structural-2023.rvt') },
    { id: 'mep-2024', path: path.resolve(__dirname, '../public/demos/mep-2024.rvt') },
  ];
  for (const candidate of candidates) {
    if (fs.existsSync(candidate.path)) return candidate;
  }
  return null;
}

async function viewportScreenshotHasVisibleContent(page: Page): Promise<boolean> {
  const image = await page.locator('#viewport').screenshot();
  return image.length > 5000 && new Set(image).size > 64;
}
