import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';

// #1422: a piece the IFC export folds into another element's body is drawn but
// is no element of its own, so a click on it selects the element that owns it:
// its panel, its GlobalId, its tree row and no size rows, since one width,
// depth and height cannot state a body of several items. A category that is
// hidden does not take the click meant for what is drawn behind it.
//
// No other model the browser tests hold has a folded piece, so this opens the
// BIM4LCA office as built (CC BY-SA 4.0; research/bim4lca-measure.tsv), given by
// RVT_VIEWER_OFFICE. Without it the test is skipped; the workflow says so when
// the host could not be reached. The clicks are real. The capture below reads
// which mesh each click hit, so that the test knows it clicked a folded piece
// rather than assuming so. While it looks for one it holds back the panel
// requests those clicks post, each of which decodes the whole model in the
// worker (11.9 minutes in CI otherwise); the click the test asserts is not held.
//
// The JavaScript that maps a pick to its owner and the worker's panel, which
// resyncs the selection, each cover for the other, so breaking one alone does
// not fail this test; it holds the two together.

interface Hit {
  object: { visible: boolean; parent: unknown; userData: { entityIndex?: number } };
  distance: number;
}
interface Capture {
  model: {
    body_pieces?: Map<number, number[]> | Record<string, number[]>;
    global_ids?: { elements?: Map<number, string> | Record<string, string> };
    building_storeys: Array<{ name: string }>;
    entities: Array<{ kind?: string; ifc_type?: string; name?: string; type_guid?: string }>;
  } | null;
  scene: SceneNode | null;
  hits: Hit[] | null;
  holdInfo: boolean;
}
interface SceneNode {
  entity_index: number | null;
  ifc_type: string;
  storey_index?: number | null;
  children: SceneNode[];
}
declare global {
  interface Window {
    __capture: Capture;
  }
}

const office = process.env.RVT_VIEWER_OFFICE;

async function capture(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const seen: Capture = { model: null, scene: null, hits: null, holdInfo: false };
    window.__capture = seen;
    const Native = window.Worker;
    window.Worker = class extends Native {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        this.addEventListener('message', (event: MessageEvent) => {
          const message = event.data as { model?: Capture['model']; scene?: SceneNode } | null;
          if (message?.model && message.scene) {
            seen.model = message.model;
            seen.scene = message.scene;
          }
        });
      }
      override postMessage(message: unknown, ...rest: never[]): void {
        if (seen.holdInfo && (message as { type?: string } | null)?.type === 'info') return;
        (super.postMessage as (m: unknown, ...r: unknown[]) => void)(message, ...rest);
      }
    };
    // three.js sorts a raycast's hits by distance.
    const sort = Array.prototype.sort;
    Array.prototype.sort = function (this: unknown[], compare?: (a: never, b: never) => number) {
      const first = this[0] as Partial<Hit> | undefined;
      if (first?.object && typeof first.distance === 'number') seen.hits = this as Hit[];
      return sort.call(this, compare as never);
    } as unknown as typeof Array.prototype.sort;
  });
}

test.skip(!office, 'RVT_VIEWER_OFFICE is not set');

test('a click on a folded piece selects the element that owns it', async ({ page }, testInfo) => {
  test.setTimeout(900_000);
  await capture(page);
  await page.goto('./');
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('#file-input').setInputFiles(office!);
  await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 600_000 });

  // What the worker handed over: the pieces the tree does not list are folded.
  const facts = await page.evaluate(() => {
    const { model, scene } = window.__capture;
    const listed = new Set<number>();
    const listedByType: Record<string, number> = {};
    const walk = (node: SceneNode) => {
      if (node.entity_index !== null) {
        listed.add(node.entity_index);
        listedByType[node.ifc_type] = (listedByType[node.ifc_type] ?? 0) + 1;
      }
      node.children.forEach(walk);
    };
    walk(scene!);
    const raw = model!.body_pieces ?? {};
    const pairs: Array<[number, number[]]> =
      raw instanceof Map
        ? [...raw]
        : Object.entries(raw).map(([owner, list]) => [Number(owner), list as number[]]);
    const owners = pairs
      .map(([owner, list]) => ({
        owner,
        folded: list.filter((piece) => !listed.has(piece)),
        id: model!.entities[owner]!.type_guid ?? '',
        name: model!.entities[owner]!.name ?? '',
        ifcType: model!.entities[owner]!.ifc_type ?? '',
      }))
      .filter((entry) => entry.folded.length > 0)
      .sort((a, b) => b.folded.length - a.folded.length);
    const elements = model!.entities.filter((entity) => entity.kind === 'BuildingElement').length;
    const folded = owners.reduce((sum, entry) => sum + entry.folded.length, 0);
    // A part of a stair, on the storey the export writes it on: its own record
    // names none, so it read as no storey before #1423.
    let flight: { index: number; id: string; storey: string } | null = null;
    const findFlight = (node: SceneNode, storey: number | null) => {
      const here = node.ifc_type === 'IFCBUILDINGSTOREY' ? (node.storey_index ?? null) : storey;
      if (!flight && node.ifc_type === 'IFCSTAIRFLIGHT' && node.entity_index !== null && here !== null) {
        flight = {
          index: node.entity_index,
          id: model!.entities[node.entity_index]!.type_guid ?? '',
          storey: model!.building_storeys[here]!.name,
        };
      }
      node.children.forEach((child) => findFlight(child, here));
    };
    findFlight(scene!, null);
    return { owners, listedByType, folded, written: elements - folded, flight };
  });
  expect(facts.owners.length, 'this model has no folded piece').toBeGreaterThan(0);
  const ifcType = facts.owners[0]!.ifcType;

  // The schedule lists the elements the export writes, not the meshes drawn.
  await expect(page.locator('#schedule-total')).toHaveText(
    new RegExp(`^${facts.written} scheduled elements`),
  );

  // The category's count is the elements the tree lists, not the meshes drawn.
  const toggles = page.locator('#categories label.category-toggle');
  const toggleTexts = await toggles.allTextContents();
  const typeName = ifcType.replace(/^IFC/, '').toLowerCase();
  const toggleText = toggleTexts.find((text) => text.toLowerCase().includes(typeName));
  expect(toggleText, `no toggle for ${ifcType} in ${toggleTexts.join(' | ')}`).toBeTruthy();
  expect(Number(/(\d+)\s*$/.exec(toggleText!.trim())![1])).toBe(facts.listedByType[ifcType]);

  // Wait for the meshes: `Loaded` comes before the glTF has been parsed, and a
  // schedule row becomes a button only once there are meshes to light up.
  await expect(page.locator('#schedule-groups button').first()).toBeVisible({ timeout: 120_000 });
  const canvas = page.locator('#viewport canvas');
  const box = (await canvas.boundingBox())!;

  // Only that category is drawn: every other one is hidden, and still in front.
  await page.evaluate((name) => {
    for (const row of document.querySelectorAll<HTMLLabelElement>('#categories label.category-toggle')) {
      const box = row.querySelector<HTMLInputElement>('input[type="checkbox"]')!;
      if (!row.textContent!.toLowerCase().includes(name) && box.checked) box.click();
    }
  }, typeName);

  // The owner from the tree: no size rows, and its GlobalId.
  const info = page.locator('#info');
  const row = (name: string) =>
    info
      .locator('.info-group-identity .info-row')
      .filter({ has: page.locator('.k', { hasText: new RegExp(`^${name}$`) }) });
  async function expectOwnerPanel(owner: number, id: string): Promise<void> {
    await expect(row('ElementId').locator('.v')).toHaveText(id, { timeout: 300_000 });
    const globalId = await page.evaluate((index) => {
      const ids = window.__capture.model!.global_ids?.elements;
      return ids instanceof Map ? ids.get(index) : ids?.[String(index)];
    }, owner);
    expect(globalId, 'the owner has no GlobalId in the model').toBeTruthy();
    await expect(row('GlobalId').locator('.v')).toHaveText(globalId!);
    await expect(info.locator('.info-group-extents')).toHaveCount(0);
    await expect(info).not.toContainText('no longer in the scene');
    await expect(info).not.toContainText(/Not recovered:[^\n]*extents/);
  }
  const first = facts.owners[0]!;
  async function openTreeRow(index: number) {
    await page.evaluate((target) => {
      const row = document.querySelector(`.tree-node[data-entity-index="${target}"]`)!;
      const openers: HTMLElement[] = [];
      for (
        let group = row.parentElement?.closest('.tree-children');
        group;
        group = group.parentElement?.closest('.tree-children')
      ) {
        openers.unshift(group.previousElementSibling as HTMLElement);
      }
      for (const opener of openers) {
        // A category opens as it is clicked, any other row only from its caret.
        if (opener.getAttribute('aria-expanded') === 'false') {
          opener.querySelector<HTMLElement>('.tree-caret')!.click();
        }
      }
    }, index);
    return page.locator(`.tree-node[data-entity-index="${index}"]`);
  }
  const treeRow = await openTreeRow(first.owner);
  await treeRow.click();
  await expectOwnerPanel(first.owner, first.id);
  // A double-click frames the owner and its pieces.
  await treeRow.dblclick();
  await page.waitForTimeout(1_500);
  await testInfo.attach('framed', { body: await page.screenshot(), contentType: 'image/png' });

  // Click across the view, from the middle out, until a click lands on a folded
  // piece with a hidden element in front of it.
  const ownerOf = new Map<number, (typeof facts.owners)[number]>();
  for (const entry of facts.owners) for (const piece of entry.folded) ownerOf.set(piece, entry);
  const columns = 60;
  const rows = 40;
  const points: Array<{ x: number; y: number }> = [];
  for (let r = 0; r < rows; r++) {
    for (let c = 0; c < columns; c++) {
      points.push({ x: box.x + (box.width * (c + 0.5)) / columns, y: box.y + (box.height * (r + 0.5)) / rows });
    }
  }
  const middle = { x: box.x + box.width / 2, y: box.y + box.height / 2 };
  points.sort((a, b) => Math.hypot(a.x - middle.x, a.y - middle.y) - Math.hypot(b.x - middle.x, b.y - middle.y));
  async function clickAndRead(point: { x: number; y: number }) {
    await page.evaluate(() => {
      window.__capture.hits = null;
    });
    await page.mouse.click(point.x, point.y);
    return page.evaluate(() => {
      const isDrawn = (object: Hit['object']) => {
        for (let at: Hit['object'] | null = object; at; at = at.parent as Hit['object'] | null) {
          if (!at.visible) return false;
        }
        return true;
      };
      const hits = window.__capture.hits ?? [];
      return {
        any: hits.length > 0,
        hiddenFirst: hits.length > 0 && !isDrawn(hits[0]!.object),
        entity: hits.find((h) => isDrawn(h.object))?.object.userData.entityIndex ?? null,
      };
    });
  }
  await page.evaluate(() => {
    window.__capture.holdInfo = true;
  });
  let found: { point: { x: number; y: number }; piece: number; owner: (typeof facts.owners)[number] } | null = null;
  let clicks = 0;
  let anyHit = false;
  let hiddenInFront = 0;
  for (const point of points) {
    const hit = await clickAndRead(point);
    clicks++;
    anyHit ||= hit.any;
    // A capture that sees nothing would otherwise spend the whole grid.
    if (clicks === 200) expect(anyHit, 'no click hit anything: is the capture still reading the raycast?').toBe(true);
    if (hit.hiddenFirst) hiddenInFront++;
    const owner = hit.entity === null ? undefined : ownerOf.get(hit.entity);
    if (owner && hit.hiddenFirst) {
      found = { point, piece: hit.entity!, owner };
      break;
    }
  }
  expect(found, `no click of ${clicks} landed on a folded piece behind a hidden element`).not.toBeNull();
  await page.evaluate(() => {
    window.__capture.holdInfo = false;
  });

  // From an unrelated selection, that click selects the piece's owner: not the
  // hidden element in front of it, and not the piece.
  const other = facts.owners.find((entry) => entry.owner !== found!.owner.owner)!;
  await (await openTreeRow(other.owner)).click();
  await expectOwnerPanel(other.owner, other.id);
  const again = await clickAndRead(found!.point);
  expect(again.hiddenFirst, 'the view moved: the hidden element is no longer in front').toBe(true);
  expect(again.entity).toBe(found!.piece);
  await expectOwnerPanel(found!.owner.owner, found!.owner.id);
  await expect(row('Name').locator('.v')).toHaveText(found!.owner.name);
  await expect(
    page.locator(`.tree-node.selected[data-entity-index="${found!.owner.owner}"]`),
  ).toHaveCount(1);
  await testInfo.attach('selected', { body: await page.screenshot(), contentType: 'image/png' });

  // A part of a stair is on its whole's storey, where its own record has none.
  expect(facts.flight, 'this model has no stair flight on a storey').not.toBeNull();
  await (await openTreeRow(facts.flight!.index)).click();
  await expect(row('ElementId').locator('.v')).toHaveText(facts.flight!.id, { timeout: 300_000 });
  await expect(info.locator('.info-group-storey')).toContainText(facts.flight!.storey);

  // #35: the saved parameters of the first owner, selected again, on the largest
  // model the suite opens. Whether the page read them on its own or waited to
  // be asked, and what the open's worker and the read held, is printed; a read
  // that fails is not a model that got worse, but it is not a pass either.
  await (await openTreeRow(first.owner)).click();
  await expect(row('ElementId').locator('.v')).toHaveText(first.id, { timeout: 300_000 });
  const parameters = page.locator('#info-parameters');
  const settledStates = /^(ready|empty|unsupported|error|on-request|no-id)$/;
  await expect(parameters).toHaveAttribute('data-state', settledStates, { timeout: 300_000 });
  const read = () =>
    parameters.evaluate((el) => ({
      state: (el as HTMLElement).dataset.state ?? '',
      readMs: (el as HTMLElement).dataset.readMs ?? null,
      wasmMemoryBytes: (el as HTMLElement).dataset.wasmMemoryBytes ?? null,
      entries: el.querySelectorAll('[data-parameter]').length,
    }));
  let parameterRead = await read();
  const askedFirst = parameterRead.state === 'on-request';
  if (askedFirst) {
    await page.locator('#read-saved-parameters').click();
    await expect(parameters).toHaveAttribute(
      'data-state',
      /^(ready|empty|unsupported|error|no-id)$/,
      { timeout: 300_000 },
    );
    parameterRead = await read();
  }
  const loadStats = JSON.parse(
    (await page.locator('#status').getAttribute('data-load-stats')) ?? 'null',
  ) as { openMs: number; loadMs: number; wasmMemoryBytes: number; estimatedPeakBytes: number };
  const mib = (bytes: number | string | null) => Math.round(Number(bytes ?? 0) / 2 ** 20);
  const parametersLine =
    `saved parameters of ${first.ifcType} ${first.id} on the office (${mib(fs.statSync(office!).size)} MiB, ` +
    `estimated open ${mib(loadStats.estimatedPeakBytes)} MiB): ${askedFirst ? 'the page asked first, then read' : 'read on selection'}: ` +
    `${parameterRead.state}, ${parameterRead.entries} entries, ${parameterRead.readMs ?? 'no'} ms, ` +
    `that read's wasm memory ${mib(parameterRead.wasmMemoryBytes)} MiB; the open's worker ${mib(loadStats.wasmMemoryBytes)} MiB, ` +
    `${Math.round(loadStats.openMs)} ms in the open call`;
  console.log(parametersLine);
  if (process.env.GITHUB_STEP_SUMMARY) {
    fs.appendFileSync(process.env.GITHUB_STEP_SUMMARY, `${parametersLine}\n\n`);
  }
  expect(parameterRead.state, parametersLine).not.toBe('error');

  console.log(
    `office: ${facts.owners.length} owners, ${facts.folded} folded pieces, ${facts.written} written; ` +
      `${ifcType} ${facts.listedByType[ifcType]} listed; click ${clicks} hit piece ${found!.piece} of ` +
      `${found!.owner.id}, ${hiddenInFront} clicks had a hidden element in front`,
  );
});
