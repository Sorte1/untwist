import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test, skipWithoutGame } from './fixtures';

async function sideText(page: Page, side: 'a' | 'b'): Promise<string> {
  return page.evaluate((which) => {
    const root = document.querySelector('#diff .editor-host')?.shadowRoot;
    return root?.querySelector(`.cm-merge-${which} .cm-content`)?.textContent ?? '';
  }, side);
}

async function changedLines(page: Page): Promise<number> {
  return page.evaluate(() => document.querySelector('#diff .editor-host')?.shadowRoot?.querySelectorAll('.cm-merge-b .cm-changedLine').length ?? 0);
}

async function chunks(page: Page): Promise<number> {
  await expect(page.locator('#diff')).toHaveAttribute('data-chunks', /^[0-9]+$/);
  return Number(await page.locator('#diff').getAttribute('data-chunks'));
}

test('with no bundle the diff sets the input against the output, and the view is kept in the hash', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;\nvar b = 1;');
  await runAndWait(page);
  await page.locator('#view-diff').click();
  expect(await chunks(page)).toBeGreaterThan(0);
  expect(await sideText(page, 'a')).toContain('!0');
  expect(await sideText(page, 'b')).toContain('true');
  expect(await changedLines(page)).toBeGreaterThan(0);
  await expect(page.locator('#input-pane')).toBeHidden();
  expect(page.url()).toContain('view=diff');
  await page.reload();
  await expect(page.locator('#view-diff')).toHaveAttribute('aria-pressed', 'true');
});

test('a module with a span is compared with its slice of the bundle; one without gets a note', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo('fixtures/reference/unpack/webpack-5.js'));
  await runAndWait(page);
  await page.locator('#view-diff').click();
  await page.locator('#module-list .tree-item', { hasText: '3.js' }).click();
  const count = await chunks(page);
  expect(count).toBeGreaterThan(0);
  expect(await changedLines(page)).toBeGreaterThan(0);
  expect(await sideText(page, 'a')).toMatch(/^\{/);
  await expect(page.locator('#diff-note')).toBeHidden();
  await page.locator('#view-output').click();
  await page.locator('#clear-input').click();
  await paste(page, readRepo('fixtures/unpack-bundlers/webpack/min-webpack5-locals.js'));
  await runAndWait(page);
  await page.locator('#view-diff').click();
  await page.locator('#module-list .tree-item', { hasText: '_runtime/webpack.js' }).click();
  await expect(page.locator('#diff-note')).toHaveText(/no span in the bundle text/);
});

test('game.js against its 10 MB output lists the changes, since its input lines are too long, and the page stays responsive', async ({ page }) => {
  skipWithoutGame();
  await openApp(page, '#unpack=0&jsx=0&unsafe=1');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await runAndWait(page);
  await page.evaluate(() => {
    const store: number[] = [];
    (window as unknown as { longTasks: number[] }).longTasks = store;
    new PerformanceObserver((list) => list.getEntries().forEach((entry) => store.push(entry.duration))).observe({ type: 'longtask' });
  });
  await page.locator('#view-diff').click();
  await expect(page.locator('#diff')).toHaveAttribute('data-view', 'summary', { timeout: 60_000 });
  await expect(page.locator('#diff-note')).toHaveText(/too long to show side by side/);
  await expect(page.locator('.diff-summary-head')).toHaveText(/^1 change · −\d/);
  await page.locator('#view-output').click();
  await page.waitForTimeout(500);
  const long = await page.evaluate(() => (window as unknown as { longTasks: number[] }).longTasks);
  expect(long.filter((ms) => ms > 100)).toEqual([]);
});

async function sideBoxes(page: Page): Promise<Array<{ left: number; right: number; contentRight: number; scrollWidth: number; clientWidth: number }>> {
  return page.evaluate(() => {
    const root = document.querySelector('#diff .editor-host')?.shadowRoot;
    return [...(root?.querySelectorAll<HTMLElement>('.cm-mergeViewEditor') ?? [])].map((side) => {
      const box = side.getBoundingClientRect();
      const visible = [...side.querySelectorAll<HTMLElement>('.cm-line')].map((line) => {
        const range = document.createRange();
        range.selectNodeContents(line);
        const rects = [...range.getClientRects()].filter((rect) => rect.top < box.bottom && rect.bottom > box.top && rect.left < box.right);
        return rects.length === 0 ? box.left : Math.max(...rects.map((rect) => Math.min(rect.right, box.right + 1)));
      });
      const scroller = side.querySelector<HTMLElement>('.cm-scroller');
      return { left: box.left, right: box.right, contentRight: Math.max(box.left, ...visible), scrollWidth: scroller?.scrollWidth ?? 0, clientWidth: scroller?.clientWidth ?? 0 };
    });
  });
}

const LONG = `var a = !0;\nvar long = ["${'x'.repeat(400)}", !1];`;

test('long diff lines scroll each side on its own by default, and wrap when Wrap is on', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await openApp(page);
  await paste(page, LONG);
  await runAndWait(page);
  await page.locator('#view-diff').click();
  await chunks(page);
  const wrap = page.locator('#diff-wrap');
  await expect(wrap).toBeVisible();
  await expect(wrap).toHaveAttribute('aria-pressed', 'false');
  const scrolled = await sideBoxes(page);
  expect(scrolled).toHaveLength(2);
  const [a, b] = scrolled as [(typeof scrolled)[number], (typeof scrolled)[number]];
  expect(Math.abs(a.right - a.left - (b.right - b.left))).toBeLessThanOrEqual(2);
  expect(a.right).toBeLessThanOrEqual(b.left + 1);
  for (const side of scrolled) {
    expect(side.scrollWidth).toBeGreaterThan(side.clientWidth);
    expect(side.contentRight).toBeLessThanOrEqual(side.right + 1);
  }
  await wrap.click();
  await expect(wrap).toHaveAttribute('aria-pressed', 'true');
  expect(page.url()).toContain('wrap=1');
  await expect.poll(async () => (await sideBoxes(page)).every((side) => side.scrollWidth <= side.clientWidth + 1)).toBe(true);
  expect(await sideText(page, 'b')).toContain('x'.repeat(400));
  await page.reload();
  await expect(page.locator('#diff-wrap')).toHaveAttribute('aria-pressed', 'true');
});
