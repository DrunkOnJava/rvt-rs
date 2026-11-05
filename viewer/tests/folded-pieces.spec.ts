import { expect, test, type Page } from '@playwright/test';

// #1422: a piece the IFC export folds into another element's body is drawn but
// is no element of its own, so a click on it selects the element that owns it:
// its panel, its GlobalId, its tree row and no size rows, since one width,
// depth and height cannot state a body of several items. A category that is
// hidden does not take the click meant for what is drawn behind it.
//
// No other model the browser tests hold has a folded piece, so this opens the
// BIM4LCA office as built (CC BY-SA 4.0; research/bim4lca-measure.tsv), given by
// RVT_VIEWER_OFFICE. Without it the test is skipped; the workflow says so when
// the host could not be reached. The clicks are real, and the capture below
// only records: it reads which mesh each click hit, so that the test knows it
// clicked a folded piece rather than assuming so.

interface Hit {
  object: { visible: boolean; parent: unknown; userData: { entityIndex?: number } };
  distance: number;
}
interface Capture {
  model: {
    body_pieces?: Map<number, number[]> | Record<string, number[]>;
    entities: Array<{ kind?: string; ifc_type?: string; name?: string; type_guid?: string }>;
  } | null;
  scene: SceneNode | null;
  hits: Hit[] | null;
}
interface SceneNode {
  entity_index: number | null;
  ifc_type: string;
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
    const seen: Capture = { model: null, scene: null, hits: null };
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
    return { owners, listedByType, folded, written: elements - folded };
  });
  expect(facts.owners.length, 'this model has no folded piece').toBeGreaterThan(0);
  const ifcType = facts.owners[0]!.ifcType;

  // The schedule lists the elements the export writes, not the meshes drawn.
  await expect(page.locator('#schedule-total')).toContainText(`${facts.written} scheduled elements`);

  // The category's count is the elements the tree lists, not the meshes drawn.
  const toggles = page.locator('#categories label.category-toggle');
  const toggleTexts = await toggles.allTextContents();
  const typeName = ifcType.replace(/^IFC/, '').toLowerCase();
  const toggleText = toggleTexts.find((text) => text.toLowerCase().includes(typeName));
  expect(toggleText, `no toggle for ${ifcType} in ${toggleTexts.join(' | ')}`).toBeTruthy();
  expect(Number(/(\d+)\s*$/.exec(toggleText!.trim())![1])).toBe(facts.listedByType[ifcType]);

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
  async function expectOwnerPanel(id: string): Promise<void> {
    await expect(row('ElementId').locator('.v')).toHaveText(id, { timeout: 300_000 });
    await expect(row('GlobalId')).toBeVisible();
    await expect(info.locator('.info-group-extents')).toHaveCount(0);
    await expect(info).not.toContainText('no longer in the scene');
    await expect(info).not.toContainText(/Not recovered:[^\n]*extents/);
  }
  const first = facts.owners[0]!;
  await page.evaluate((index) => {
    const target = document.querySelector(`.tree-node[data-entity-index="${index}"]`)!;
    const openers: HTMLElement[] = [];
    for (
      let group = target.parentElement?.closest('.tree-children');
      group;
      group = group.parentElement?.closest('.tree-children')
    ) {
      openers.unshift(group.previousElementSibling as HTMLElement);
    }
    for (const opener of openers) {
      if (opener.getAttribute('aria-expanded') === 'false') opener.click();
    }
  }, first.owner);
  const treeRow = page.locator(`.tree-node[data-entity-index="${first.owner}"]`);
  await treeRow.click();
  await expectOwnerPanel(first.id);
  // A double-click frames the owner and its pieces.
  await treeRow.dblclick();
  await page.waitForTimeout(1_500);
  await testInfo.attach('framed', { body: await page.screenshot(), contentType: 'image/png' });

  // Click across the view until a click lands on a folded piece.
  const ownerOf = new Map<number, (typeof facts.owners)[number]>();
  for (const entry of facts.owners) for (const piece of entry.folded) ownerOf.set(piece, entry);
  const canvas = page.locator('#viewport canvas');
  const box = (await canvas.boundingBox())!;
  const grid = { columns: 40, rows: 26 };
  let found: { piece: number; owner: (typeof facts.owners)[number] } | null = null;
  let clicks = 0;
  let hiddenInFront = 0;
  for (let r = 0; r < grid.rows && !found; r++) {
    for (let c = 0; c < grid.columns && !found; c++) {
      await page.evaluate(() => {
        window.__capture.hits = null;
      });
      await page.mouse.click(
        box.x + (box.width * (c + 0.5)) / grid.columns,
        box.y + (box.height * (r + 0.5)) / grid.rows,
      );
      clicks++;
      const hit = await page.evaluate(() => {
        const isDrawn = (object: Hit['object']) => {
          for (let at: Hit['object'] | null = object; at; at = at.parent as Hit['object'] | null) {
            if (!at.visible) return false;
          }
          return true;
        };
        const hits = window.__capture.hits ?? [];
        const drawn = hits.find((h) => isDrawn(h.object));
        return {
          any: hits.length > 0,
          hiddenFirst: hits.length > 0 && !isDrawn(hits[0]!.object),
          entity: drawn?.object.userData.entityIndex ?? null,
        };
      });
      if (hit.hiddenFirst) hiddenInFront++;
      const owner = hit.entity === null ? undefined : ownerOf.get(hit.entity);
      if (owner) found = { piece: hit.entity!, owner };
    }
  }
  expect(found, `no click of ${clicks} landed on a folded piece`).not.toBeNull();

  // The click on the piece selected its owner.
  await expectOwnerPanel(found!.owner.id);
  await expect(row('Name').locator('.v')).toHaveText(found!.owner.name);
  await expect(
    page.locator(`.tree-node.selected[data-entity-index="${found!.owner.owner}"]`),
  ).toHaveCount(1);
  await testInfo.attach('selected', { body: await page.screenshot(), contentType: 'image/png' });

  // Some of those clicks had a hidden element in front of what they selected,
  // or this test says nothing about the hidden categories.
  expect(hiddenInFront, 'no click had a hidden element in front of it').toBeGreaterThan(0);
  console.log(
    `office: ${facts.owners.length} owners, ${facts.folded} folded pieces, ${facts.written} written; ` +
      `${ifcType} ${facts.listedByType[ifcType]} listed; click ${clicks} hit piece ${found!.piece} of ` +
      `${found!.owner.id}, ${hiddenInFront} clicks had a hidden element in front`,
  );
});
