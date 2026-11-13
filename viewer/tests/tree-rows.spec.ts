import { expect, test, type Page } from '@playwright/test';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// #1887: the element tree builds its rows in blocks. A group (a storey's
// categories, a category's elements, an element's parts) builds its first block
// when it is opened, or up front while the page has the rows to spend, and a
// "Show more" row at its end builds the next. Einhoven is the model: a block of
// 1 row and nothing built up front (`?treeRowsShown=1&treeRowsEager=0`) puts
// most of its groups behind a "Show more" row, which is what a file with a
// million elements in one category would do at the real limits. Its storey
// holds 3 categories, its windows' category 2 and its curtain wall 24 panels
// (RE-440): a block of 1 is what makes a group of 2 overflow.
//
// The oracle is the tree the page builds without the limits, whole and up front,
// as it did before the blocks: opened every way a reader can, the blocks must
// give the same elements in the same order, and the filter must find an element
// whose block was not built.

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const einhoven = path.resolve(__dirname, '../public/demos/Revit_IFC5_Einhoven.rvt');
const einhovenTest = fs.existsSync(einhoven) ? test : test.skip;
const BLOCKS = '?treeRowsShown=1&treeRowsEager=0';

async function openEinhoven(page: Page, query: string): Promise<void> {
  await page.goto(`/${query}`);
  await expect(page.locator('#status')).toHaveText(/Ready/);
  await page.locator('[data-demo-id="einhoven-2023"]').click();
  await expect(page.locator('#status')).toHaveText(/Loaded/, { timeout: 120_000 });
  await expect(page.locator('.tree-node.tree-storey').first()).toBeVisible();
}

/** Clicks every "Show more" row until none is left; a hidden row takes the click too. */
async function showAllRows(page: Page, level?: number): Promise<void> {
  await page.evaluate((onlyLevel) => {
    const selector = onlyLevel === undefined ? '.tree-node.tree-more' : `.tree-node.tree-more[aria-level="${onlyLevel}"]`;
    for (let guard = 0; guard < 1_000_000; guard++) {
      const more = document.querySelector<HTMLElement>(selector);
      if (!more) return;
      more.click();
    }
    throw new Error('the "Show more" rows never ran out');
  }, level);
}

/** Opens every group and every "Show more" row until the tree stops growing. */
async function openEverything(page: Page): Promise<void> {
  await page.evaluate(() => {
    // A category opens as it is clicked, any other row from its caret; both
    // take a click on the caret, and a hidden row takes one too.
    for (let guard = 0; guard < 1_000_000; guard++) {
      const closed = document.querySelector<HTMLElement>('.tree-node[aria-expanded="false"]');
      const more = document.querySelector<HTMLElement>('.tree-node.tree-more');
      if (!closed && !more) return;
      (closed?.querySelector<HTMLElement>('.tree-caret') ?? more!).click();
    }
    throw new Error('the tree was still growing after a million clicks');
  });
}

/** The element rows in the order the tree lists them. */
async function elementRows(page: Page): Promise<Array<{ index: string; title: string }>> {
  return page.evaluate(() =>
    [...document.querySelectorAll<HTMLElement>('.tree-node.tree-element[data-entity-index]')].map((row) => ({
      index: row.dataset.entityIndex!,
      title: row.title,
    })),
  );
}

einhovenTest('opened every way, the blocks give the elements the whole tree has, in its order', async ({ page }) => {
  test.setTimeout(240_000);
  await openEinhoven(page, '');
  expect(await page.locator('.tree-node.tree-more').count(), 'a model under the eager limit is built whole').toBe(0);
  const whole = await elementRows(page);
  expect(whole.length, 'Einhoven lists elements').toBeGreaterThan(20);

  await openEinhoven(page, BLOCKS);
  // Nothing was built up front but the project, its storeys and their first
  // block of categories: no element has a row yet, and a group says so.
  expect(await elementRows(page), 'no element row is built before its category is opened').toEqual([]);
  const more = page.locator('.tree-node.tree-more').first();
  await expect(more).toBeVisible();
  await expect(more).toHaveText(/^Show \d+ more [\d,]+ not listed$/);

  await openEverything(page);
  expect(await page.locator('.tree-node.tree-more').count(), 'every block was built').toBe(0);
  const blocks = await elementRows(page);
  expect(blocks.map((row) => row.index)).toEqual(whole.map((row) => row.index));
  expect(blocks.map((row) => row.title)).toEqual(whole.map((row) => row.title));
});

einhovenTest('a "Show more" row builds the next block and keeps the keyboard where it was', async ({ page }) => {
  test.setTimeout(240_000);
  await openEinhoven(page, BLOCKS);
  // The categories of every storey first (level 3), so the one chosen below is built.
  await showAllRows(page, 3);
  // A category with more than 1 element: opened, its group lists 1 and a row.
  const category = page
    .locator('.tree-node.tree-category')
    .filter({ has: page.locator('.tree-count', { hasText: /^([2-9]|\d{2,})$/ }) })
    .first();
  await expect(category).toBeAttached();
  await category.evaluate((row) => (row as HTMLElement).click());
  const group = category.locator('xpath=following-sibling::*[contains(@class, "tree-children")]');
  const elements = group.locator(':scope > .tree-item > .tree-node.tree-element');
  await expect(elements).toHaveCount(1);
  const more = group.locator(':scope > .tree-item > .tree-node.tree-more');
  await expect(more).toHaveCount(1);
  const remaining = Number(/([\d,]+) not listed/.exec((await more.textContent()) ?? '')![1]!.replace(/,/g, ''));

  // Enter on the row builds the next block; the focus stays on the row, which says what is left.
  await more.evaluate((row) => (row as HTMLElement).focus());
  await more.press('Enter');
  await expect(elements).toHaveCount(Math.min(2, 1 + remaining));
  if (remaining > 1) {
    await expect(more).toBeFocused();
    await expect(more).toContainText(`${remaining - 1} not listed`);
  }
});

einhovenTest('the filter lists an element whose block was not built, and says when it lists only the first', async ({
  page,
}) => {
  test.setTimeout(240_000);
  await openEinhoven(page, '');
  // An element of a category that is not the first of its group, with a name only it has.
  const target = await page.evaluate(() => {
    const rows = [...document.querySelectorAll<HTMLElement>('.tree-node.tree-element[data-entity-index]')];
    const names = rows.map((row) => row.title.split(' · ')[0]!.toLowerCase());
    for (let i = rows.length - 1; i >= 0; i--) {
      const wrap = rows[i]!.parentElement!;
      const group = wrap.parentElement!;
      const position = [...group.children].indexOf(wrap);
      const ownerIsCategory = group.previousElementSibling?.classList.contains('tree-category') ?? false;
      if (position >= 1 && ownerIsCategory && names.filter((other) => other.includes(names[i]!)).length === 1) {
        return { index: rows[i]!.dataset.entityIndex!, name: rows[i]!.title.split(' · ')[0]! };
      }
    }
    return null;
  });
  expect(target, 'Einhoven has an element past the first of its category with a name of its own').not.toBeNull();

  await openEinhoven(page, BLOCKS);
  expect(await page.locator(`.tree-node[data-entity-index="${target!.index}"]`).count()).toBe(0);
  const filter = page.locator('#tree-filter');
  await filter.fill(target!.name);
  await expect(page.locator('#tree-filter-status')).toHaveText('1 element matches');
  await expect(page.locator(`.tree-node[data-entity-index="${target!.index}"]`)).toBeVisible();
  // The rest of its category stays out of the way while the filter holds.
  await expect(page.locator('.tree-node.tree-element:visible')).toHaveCount(1);

  // Every element's row says its IFC class, so "ifc" matches them all: 4 are
  // listed (4 blocks of 1), and the status line says that there are more.
  await filter.fill('ifc');
  await expect(page.locator('#tree-filter-status')).toHaveText(/^\d+ elements match; the first 4 are listed$/);
  expect(await page.locator('.tree-node.tree-element:visible').count()).toBeGreaterThanOrEqual(4);

  // Clearing it gives the tree back, with the row the filter built kept.
  await filter.press('Escape');
  await expect(page.locator('#tree-filter-status')).toBeHidden();
  expect(await page.locator(`.tree-node[data-entity-index="${target!.index}"]`).count()).toBe(1);
});
