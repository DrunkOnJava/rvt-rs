import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const projectSamplePath = resolveProjectSamplePath();
const projectSampleTest = projectSamplePath === null ? test.skip : test;
const stagedDemoPath = path.resolve(__dirname, '../public/demos/synthetic-mvp.rvt');
const stagedDemoTest = fs.existsSync(stagedDemoPath) ? test : test.skip;

test('loads the viewer shell with disabled export actions and demo gallery', async ({ page }) => {
  await page.goto('/');

  await expect(page.locator('#status')).toHaveText(/ready/);
  await expect(page.locator('#dropzone')).toBeVisible();
  await expect(page.locator('#export-quality')).toHaveText(/pending/);
  await expect(page.locator('#export-glb')).toBeDisabled();
  await expect(page.locator('#export-ifc')).toBeDisabled();
  await expect(page.locator('#export-svg')).toBeDisabled();
  await expect(page.locator('#download-diagnostics')).toBeDisabled();
  await expect(page.locator('#status-panel')).toContainText('No file opened');
  await expect(page.locator('#mvp-workflow')).toContainText('Open locally');
  await expect(page.locator('#demo-gallery')).toBeVisible();
  await expect(page.locator('#demo-attribution')).toContainText(/redistributable|Apache|phi-ag/i);
  await expect(page.locator('[data-demo-id="synthetic-mvp"]')).toBeVisible();
  await expect(page.locator('[data-demo-id="synthetic-mvp"]')).toContainText(/expected:\s*Scaffold/i);
  await expect(page.locator('[data-demo-id="synthetic-project"]')).toContainText(/Reference download|IFC/i);
});

stagedDemoTest(
  'MVP workflow via demo gallery: open → status/confidence → inspect → export labels',
  async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);
    await expect(page.locator('[data-demo-id="synthetic-mvp"]')).toBeEnabled();

    await page.locator('[data-demo-id="synthetic-mvp"]').click();
    await expect(page.locator('#status')).toHaveText(/loaded/);
    await expect(page.locator('#dropzone')).toBeHidden();

    await expect(page.locator('#export-glb')).toBeEnabled();
    await expect(page.locator('#export-ifc')).toBeEnabled();
    await expect(page.locator('#export-svg')).toBeEnabled();
    await expect(page.locator('#download-diagnostics')).toBeEnabled();
    await expect(page.locator('#export-quality')).toContainText(/Scaffold|Typed|Geometry|Diagnostic|Proxy|Unknown/);

    await expect(page.locator('#status-panel')).toContainText(
      /Partial decode|Scaffold-only|Supported profile|unsupported model layout/i,
    );

    await page.locator('#diagnostics-details summary').click();
    await expect(page.locator('#diagnostics-json')).toContainText(/confidence|schema_version|warnings/i);

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

    await expect(page.locator('#status-panel')).toContainText('Partial decode');
    await expect(page.locator('#status-panel')).toContainText('unsupported_geometry_missing_level');
    await page.locator('#diagnostics-details summary').click();
    await expect(page.locator('#diagnostics-json')).toContainText('"schema_version": 1');
    await expect(page.locator('#diagnostics-json')).toContainText(
      '"unsupported_geometry_missing_level"',
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

async function viewportScreenshotHasVisibleContent(page: Page): Promise<boolean> {
  const image = await page.locator('#viewport').screenshot();
  return image.length > 5000 && new Set(image).size > 64;
}
