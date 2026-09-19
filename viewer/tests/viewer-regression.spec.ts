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
const realProjectDemoPath = path.resolve(__dirname, '../public/demos/Revit_IFC5_Einhoven.rvt');
const realProjectDemoTest = fs.existsSync(realProjectDemoPath) ? test : test.skip;
const largeProjectDemoPath = path.resolve(__dirname, '../public/demos/2024_Core_Interior.rvt');
const largeProjectDemoTest = fs.existsSync(largeProjectDemoPath) ? test : test.skip;

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
    await expect(page.locator('#info')).toContainText(/IFCPROJECT/i);
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
    // The panel names fields in words now, not raw record keys.
    await expect(page.locator('#info')).toContainText(/Type/);
    await expect(page.locator('#info')).toContainText(/IFCPROJECT/i);
    await expect(page.locator('#info')).not.toContainText(/ifc_type/);
    await expect(page.locator('.category-toggle')).toContainText(/IFCPROJECT/i);

    const ifcTitle = await page.locator('#export-ifc').getAttribute('title');
    expect(ifcTitle ?? '').toMatch(/Scaffold|Typed|Geometry|Diagnostic|Proxy|Unknown/i);
    expect(ifcTitle ?? '').toMatch(/elements/i);
    await expect(page.locator('#export-quality')).toContainText(/Scaffold/);
  },
);

stagedDemoTest(
  'scaffold-only result explains the empty viewport instead of leaving a bare grid',
  async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);
    await expect(page.locator('#scaffold-note')).toBeHidden();

    await page.locator(`[data-demo-id="${stagedDemoId}"]`).click();
    await expect(page.locator('#status')).toHaveText(/loaded/);

    // Synthetic tier1 fixtures decode with no drawable geometry; the
    // overlay has to say so rather than leaving an empty grid to speak.
    const note = page.locator('#scaffold-note');
    await expect(note).toBeVisible();
    await expect(note).toContainText(/Nothing to draw/i);
    await expect(note).toContainText(/no element geometry/i);
    await expect(note).toContainText(/expected result/i);

    // The loading treatment tears down once the decode lands.
    await expect(page.locator('#load-overlay')).toBeHidden();

    await page.getByRole('button', { name: /^Dismiss$/ }).click();
    await expect(note).toBeHidden();
  },
);

stagedDemoTest('a completed load raises a toast that dismisses itself', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#status')).toHaveText(/ready/);

  const region = page.locator('#toast-region');
  await expect(region).toHaveAttribute('aria-live', 'polite');
  await expect(region.locator('.toast')).toHaveCount(0);

  await page.locator(`[data-demo-id="${stagedDemoId}"]`).click();
  await expect(page.locator('#status')).toHaveText(/loaded/);

  const toast = region.locator('.toast.ok');
  await expect(toast).toBeVisible();
  await expect(toast).toContainText(/Loaded/i);

  // Non-blocking: it clears on its own without any user action.
  await expect(region.locator('.toast')).toHaveCount(0, { timeout: 15_000 });
});

stagedDemoTest(
  'reduced motion keeps the loading, toast and empty-state behaviour intact',
  async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    await page.locator(`[data-demo-id="${stagedDemoId}"]`).click();
    await expect(page.locator('#status')).toHaveText(/loaded/);
    await expect(page.locator('#load-overlay')).toBeHidden();
    await expect(page.locator('#scaffold-note')).toBeVisible();
    await expect(page.locator('#toast-region .toast')).toContainText(/Loaded/i);
  },
);

realProjectDemoTest(
  'real-project demo card opens Einhoven with storeys and visible geometry',
  async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    const card = page.locator('[data-demo-id="einhoven-2023"]');
    await expect(card).toBeEnabled();
    await expect(card).toContainText(/expected:\s*Geometry/i);
    await expect(card).toContainText(/MIT/);
    // Real projects are marked so they do not read as synthetic fixtures.
    await expect(card).toHaveAttribute('data-demo-real', 'true');
    await expect(card.locator('.demo-badge')).toHaveText('real project');
    await expect(page.locator('[data-demo-id="architectural-2024"]')).not.toHaveAttribute(
      'data-demo-real',
      'true',
    );
    await expect(page.locator('#demo-list [data-demo-id]').first()).toHaveAttribute(
      'data-demo-id',
      'einhoven-2023',
    );

    await card.click();
    await expect(page.locator('#status')).toHaveText(/loaded/);
    await expect(page.locator('#dropzone')).toBeHidden();
    await expect(page.locator('#file-meta')).toContainText(/Revit_IFC5_Einhoven\.rvt/);
    await expect(page.locator('#export-quality')).toContainText(/Geometry/);
    await expect(page.locator('#status-panel')).toContainText(/storey/i);
    await expect(page.locator('.tree-node.tree-storey').first()).toBeVisible();

    await page.locator('#diagnostics-details summary').click();
    await expect(page.locator('#diagnostics-json')).toContainText('"storey_count": 4');
    // Geometry decoded, so the scaffold explainer must stay out of the way.
    await expect(page.locator('#scaffold-note')).toBeHidden();
    expect(await viewportScreenshotHasVisibleContent(page)).toBe(true);
  },
);

realProjectDemoTest(
  'the scaffold explainer routes back to the real-project cards',
  async ({ page }) => {
    test.skip(stagedDemoPath === null, 'needs a staged synthetic demo to produce a scaffold');
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    await page.locator(`[data-demo-id="${stagedDemoId}"]`).click();
    await expect(page.locator('#status')).toHaveText(/loaded/);
    await expect(page.locator('#scaffold-note')).toBeVisible();

    await page.getByRole('button', { name: /Show real projects/i }).click();
    await expect(page.locator('#scaffold-note')).toBeHidden();
    await expect(page.locator('#dropzone')).toBeVisible();
    await expect(page.locator('[data-demo-id="einhoven-2023"]')).toBeFocused();
  },
);

largeProjectDemoTest(
  'large real-project demo card opens Core Interior inside wasm32 memory',
  async ({ page }) => {
    // 33.7 MB, dozens of gzip members in one stream. Before the bounded
    // inflate reservation this trapped at the 4 GiB wasm32 ceiling in
    // ~2 s; the decode itself takes tens of seconds, hence slow().
    test.slow();
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    const card = page.locator('[data-demo-id="core-interior-2024"]');
    await expect(card).toBeEnabled();
    await card.click();

    // A 30 s decode with a static status line reads as a hang. The
    // overlay has to show motion, elapsed seconds and a size-derived
    // expectation for the whole wait.
    await expect(page.locator('#load-overlay')).toBeVisible();
    await expect(page.locator('#load-hint')).toHaveText(/MB, about \d+ s/);
    await expect(page.locator('#load-elapsed')).toHaveText(/^\d+ s elapsed$/);
    await expect(card).toHaveAttribute('aria-busy', 'true');
    await expect(page.locator('#status')).toHaveClass(/is-busy/);
    // Elapsed actually advances rather than sitting at zero.
    await expect(page.locator('#load-elapsed')).toHaveText(/^[1-9]\d* s elapsed$/, {
      timeout: 15_000,
    });

    await expect(page.locator('#status')).toHaveText(/loaded/, { timeout: 300_000 });
    await expect(page.locator('#load-overlay')).toBeHidden();
    await expect(card).not.toHaveAttribute('aria-busy', 'true');
    await expect(page.locator('#status')).not.toContainText(/error|unreachable/i);
    await expect(page.locator('#file-meta')).toContainText(/2024_Core_Interior\.rvt/);
    await expect(page.locator('#export-quality')).toContainText(/Geometry/);
    await expect(page.locator('.tree-node.tree-storey').first()).toBeVisible();
    expect(await viewportScreenshotHasVisibleContent(page)).toBe(true);
  },
);

largeProjectDemoTest(
  'selecting a door in Core Interior shows its host wall and jumps to it',
  async ({ page }) => {
    // Same 33.7 MB decode as the card test above — Einhoven has walls
    // only, so Core Interior is the sample with recovered openings
    // (132 doors / 6 windows, 138 host binds).
    test.slow();
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    await page.locator('[data-demo-id="core-interior-2024"]').click();
    await expect(page.locator('#status')).toHaveText(/loaded/, { timeout: 300_000 });

    // Doors nest under their host wall in the scene tree (M4-04).
    const door = page.locator('.tree-node', { hasText: 'IFCDOOR' }).first();
    await expect(door).toBeVisible();
    await door.click();

    const relations = page.locator('#info-relations');
    await expect(relations).toBeVisible();
    await expect(relations).toContainText(/Hosted by/);
    const hostRow = relations.locator('.info-relation').first();
    await expect(hostRow).toContainText(/IFCWALL/);
    const hostIndex = await hostRow.getAttribute('data-entity-index');
    expect(hostIndex).toMatch(/^\d+$/);

    // Keyboard-accessible jump: the row is a real button.
    await hostRow.focus();
    await expect(hostRow).toBeFocused();
    await page.keyboard.press('Enter');

    // The wall is now selected in the tree, and its panel lists the
    // openings it hosts — including the door we came from.
    await expect(
      page.locator(`.tree-node.selected[data-entity-index="${hostIndex}"]`),
    ).toHaveCount(1);
    await expect(relations).toContainText(/Hosts \d+ opening/);
    await expect(relations.locator('.info-relation').first()).toContainText(
      /IFCDOOR|IFCWINDOW/,
    );

    // Core Interior carries a 10-property RvtElementRecordGeometry set
    // per element, so the group collapses rather than pushing the host
    // rows off the panel.
    const details = page.locator('#info-properties-details');
    await expect(details).toBeVisible();
    await expect(details).not.toHaveAttribute('open', '');
    await expect(details.locator('summary')).toContainText(/RvtElementRecordGeometry · \d+ properties/);
    await details.locator('summary').click();
    await expect(details).toHaveAttribute('open', '');
    const rows = details.locator('.info-row');
    expect(await rows.count()).toBeGreaterThan(8);
    // Booleans read as words, lengths carry feet, and each row is
    // tagged with the value kind the Rust panel assigned it.
    await expect(
      details.locator('.info-row[data-property-kind="boolean"] .v').first(),
    ).toHaveText(/^(Yes|No)$/);
    await expect(
      details.locator('.info-row[data-property-kind="length"] .v').first(),
    ).toHaveText(/^-?\d+\.\d{3} ft$/);
    await expect(details).not.toContainText(/true|false/);
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

    const info = page.locator('#info');
    // Identity reads as words, not as the raw element record.
    await expect(info).toContainText('Name');
    await expect(info).toContainText('IFCWALL');
    await expect(info).not.toContainText('ifc_type');
    await expect(info).not.toContainText('property_set');
    await expect(info).not.toContainText('location_feet');

    // The property set is a titled group with one row per property,
    // values carrying their unit and booleans reading as words.
    const properties = info.locator('[data-group="properties"]');
    await expect(properties).toBeVisible();
    await expect(properties).toContainText('RvtArcWall');
    const propertyRows = properties.locator('.info-row');
    expect(await propertyRows.count()).toBeGreaterThan(3);
    // Booleans read as words rather than `true` / `false`.
    await expect(
      properties.locator('.info-row', { hasText: 'ThicknessResolved' }).locator('.v'),
    ).toHaveText(/^(Yes|No)$/);
    await expect(properties).not.toContainText(/true|false/);
    // Lengths carry their unit, and the row is tagged with the value
    // kind the Rust panel assigned it.
    await expect(
      properties.locator('.info-row', { hasText: 'UnconnectedHeight' }).locator('.v'),
    ).toHaveText(/^-?\d+\.\d{3} ft$/);
    await expect(
      properties.locator('.info-row[data-property-kind="length"]').first(),
    ).toBeVisible();

    // Placement and extents are labelled feet rows, not raw arrays.
    const placement = info.locator('[data-group="placement"]');
    await expect(placement).toBeVisible();
    await expect(placement.locator('.info-row').first()).toContainText(/^X/);
    await expect(placement).toContainText(/-?\d+\.\d{3} ft/);
    const extents = info.locator('[data-group="extents"]');
    await expect(extents).toBeVisible();
    await expect(extents).toContainText('Width');
    await expect(extents).toContainText('Height');
    await expect(extents).toContainText('Profile');

    // No bare dashes standing in for an absent optional.
    await expect(info.locator('.info-row .v', { hasText: /^—$/ })).toHaveCount(0);

    // The storey index resolves to a named jump target that selects
    // that storey in the scene tree (the #269 pattern, for storeys).
    const storeyLink = info.locator('[data-group="storey"] .info-relation');
    await expect(storeyLink).toBeVisible();
    await expect(storeyLink).toContainText(/\d+\.\d{3} ft/);
    const storeyIndex = await storeyLink.getAttribute('data-storey-index');
    expect(storeyIndex).toMatch(/^\d+$/);
    const storeyName = (await storeyLink.textContent())?.split(' · ')[0] ?? '';
    expect(storeyName.length).toBeGreaterThan(0);

    await storeyLink.focus();
    await expect(storeyLink).toBeFocused();
    await page.keyboard.press('Enter');
    const selectedStorey = page.locator(
      `.tree-node.selected[data-storey-index="${storeyIndex}"]`,
    );
    await expect(selectedStorey).toHaveCount(1);
    await expect(selectedStorey).toContainText(storeyName);

    // Einhoven recovers 42 materials but binds none of them to an
    // element, so the material band is omitted outright — and because
    // the diagnostics do not call material a decode gap, the panel
    // must not claim it was "not recovered" either.
    await wallNode.click();
    await expect(info.locator('[data-group="properties"]')).toBeVisible();
    await expect(info.locator('[data-group="material"]')).toHaveCount(0);
    await expect(info).not.toContainText(/Not recovered:[^\n]*material/);

    expect(await viewportScreenshotHasVisibleContent(page)).toBe(true);
  },
);

projectSampleTest(
  'the schedule breaks down by IFC type and highlights a whole type in the scene',
  async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('#status')).toHaveText(/ready/);

    await page.locator('#file-input').setInputFiles(projectSamplePath!);
    await expect(page.locator('#status')).toHaveText(/loaded/);

    const schedule = page.locator('#schedule-summary');
    await expect(schedule.locator('#schedule-total')).toContainText(
      /\d+ scheduled elements · \d+ types?/,
    );

    const groups = schedule.locator('.schedule-row');
    expect(await groups.count()).toBeGreaterThan(1);
    await expect(schedule.locator('[data-ifc-type="IFCWALL"]')).toBeVisible();
    await expect(schedule.locator('[data-ifc-type="IFCWALL"] .schedule-count')).toHaveText(
      /^\d+$/,
    );

    // Counts add up to the total the summary line reports.
    const total = Number(
      (await schedule.locator('#schedule-total').textContent())?.match(/^(\d+)/)?.[1] ?? '0',
    );
    const counts = await schedule.locator('.schedule-count').allTextContents();
    expect(counts.reduce((sum, n) => sum + Number(n), 0)).toBe(total);

    // Einhoven decodes real geometry, so the per-type highlight is
    // offered and toggles rather than firing once.
    const wallGroup = schedule.locator('button[data-ifc-type="IFCWALL"]');
    await expect(wallGroup).toBeVisible();
    await expect(wallGroup).toHaveAttribute('aria-pressed', 'false');
    await expect(wallGroup.locator('.schedule-verb')).toHaveText('Highlight');
    await wallGroup.click();
    await expect(wallGroup).toHaveAttribute('aria-pressed', 'true');
    await expect(wallGroup.locator('.schedule-verb')).toHaveText('Clear');
    await expect(page.locator('#status')).toContainText(/highlighted \d+ IFCWALL mesh/);
    await wallGroup.click();
    await expect(wallGroup).toHaveAttribute('aria-pressed', 'false');
    await expect(wallGroup.locator('.schedule-verb')).toHaveText('Highlight');
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
