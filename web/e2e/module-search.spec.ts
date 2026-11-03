import type { Page } from '@playwright/test';
import { expect, openApp, test } from './fixtures';

const HITS = '#module-hits .hit-item';

async function loadBundle(page: Page): Promise<void> {
  await openApp(page);
  await page.locator('#examples').click();
  await page.locator('#examples-menu [role=option]', { hasText: 'webpack 5 bundle' }).click();
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 60_000 });
  await expect(page.locator('#module-tree')).toBeVisible();
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  const text = await page.evaluate(() => navigator.clipboard.readText());
  await expect(page.locator('#copy-label')).toHaveText('Copy');
  return text;
}

test('a search lists the hits of every module and a hit opens its module with the match selected', async ({ page }) => {
  await loadBundle(page);
  await page.locator('#module-search').fill('REQUIRE');
  await expect(page.locator('#module-list')).toBeHidden();
  await expect(page.locator('#module-hits')).toBeVisible();
  await expect(page.locator('#module-search-note')).toHaveText(/^\d+ match(es)? in \d+ modules?$/);
  const groups = page.locator('#module-hits .hit-group');
  expect(await groups.count()).toBeGreaterThan(1);
  await expect(page.locator(`${HITS} mark`).first()).toHaveText(/^require$/i);
  const hit = page.locator(HITS).last();
  const path = (await hit.getAttribute('data-path')) ?? '';
  const line = Number(await hit.getAttribute('data-line'));
  const column = Number(await hit.getAttribute('data-column'));
  await hit.click();
  await expect(page.locator('#output-meta')).toHaveText(new RegExp(`^${path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')} · `));
  await page.keyboard.type('ZZZ');
  const lines = (await outputText(page)).split('\n');
  expect(lines[line - 1]?.slice(column - 1, column + 2)).toBe('ZZZ');
  expect(lines[line - 1]?.slice(column + 2)).not.toMatch(/^require/i);
});

test('the search works from the keyboard and Escape brings the module list back', async ({ page }) => {
  await loadBundle(page);
  const search = page.locator('#module-search');
  await search.focus();
  await page.keyboard.type('exports');
  await expect(page.locator(HITS).first()).toBeVisible();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator(HITS).first()).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator(HITS).nth(1)).toBeFocused();
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  await expect(search).toBeFocused();
  await page.keyboard.press('Enter');
  const first = page.locator(HITS).first();
  await expect(page.locator('#output-meta')).toHaveText(new RegExp(`^${((await first.getAttribute('data-path')) ?? '').replace(/[.*+?^${}()|[\]\\]/g, '\\$&')} · `));
  await search.focus();
  await page.keyboard.press('Escape');
  await expect(search).toHaveValue('');
  await expect(page.locator('#module-list')).toBeVisible();
  await expect(page.locator('#module-hits')).toBeHidden();
});

test('a query without hits says so, and a hit from the diff view goes back to the code', async ({ page }) => {
  await loadBundle(page);
  await page.locator('#module-search').fill('qqqzzz');
  await expect(page.locator(HITS)).toHaveCount(0);
  await expect(page.locator('#module-search-note')).toHaveText(/^No matches in \d+ modules$/);
  await page.locator('#view-diff').click();
  await expect(page.locator('#view-diff')).toHaveAttribute('aria-pressed', 'true');
  await page.locator('#module-search').fill('exports');
  const hit = page.locator(HITS).first();
  const path = (await hit.getAttribute('data-path')) ?? '';
  await hit.click();
  await expect(page.locator('#view-output')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('#output-meta')).toHaveText(new RegExp(`^${path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')} · `));
});

test('a new run keeps the query and searches the new modules', async ({ page }) => {
  await loadBundle(page);
  await page.locator('#module-search').fill('exports');
  await expect(page.locator(HITS).first()).toBeVisible();
  const before = await page.locator(HITS).count();
  await page.locator('#run').click();
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 60_000 });
  await expect(page.locator('#module-search')).toHaveValue('exports');
  await expect(page.locator(HITS)).toHaveCount(before);
});
