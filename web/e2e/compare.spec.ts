import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test, skipWithoutGame } from './fixtures';

const JSX_INPUT = 'const React = require("react");\nexport const a = React.createElement("div", { id: "x" }, "hi");\n';
const BUDGET_BYTES = 200 * 1024 * 1024;

async function sideText(page: Page, side: 'a' | 'b'): Promise<string> {
  return page.evaluate((which) => {
    const root = document.querySelector('#diff .editor-host')?.shadowRoot;
    return root?.querySelector(`.cm-merge-${which} .cm-content`)?.textContent ?? '';
  }, side);
}

async function compareMemory(page: Page): Promise<number> {
  return page.evaluate(() => {
    const mark = performance.getEntriesByName('untwist:compare-done').at(-1) as PerformanceMark | undefined;
    return (mark?.detail as { memoryBytes?: number } | undefined)?.memoryBytes ?? 0;
  });
}

test('Compare with Unsafe runs the other mode and opens the diff, Safe on the left', async ({ page }) => {
  await openApp(page);
  await expect(page.locator('#compare-modes')).toBeDisabled();
  await paste(page, JSX_INPUT);
  await runAndWait(page);
  await expect(page.locator('#compare-label')).toHaveText('Compare with Unsafe');
  await page.locator('#compare-modes').click();
  await expect(page.locator('#view-diff')).toHaveAttribute('aria-pressed', 'true', { timeout: 30_000 });
  await expect(page.locator('#diff-before')).toHaveText('Safe output');
  await expect(page.locator('#diff-after')).toHaveText('Unsafe output');
  await expect(page.locator('#diff')).toHaveAttribute('data-chunks', /^[1-9][0-9]*$/);
  expect(await sideText(page, 'a')).toContain('createElement');
  expect(await sideText(page, 'b')).toContain('<div');
  expect(page.workers().length).toBeLessThanOrEqual(1);
  await expect(page.locator('#compare-label')).toHaveText('Compare with Unsafe');
});

test('from an Unsafe output the action compares with Safe, still Safe on the left', async ({ page }) => {
  await openApp(page, '#unsafe=1');
  await paste(page, JSX_INPUT);
  await runAndWait(page);
  await expect(page.locator('#compare-label')).toHaveText('Compare with Safe');
  await page.locator('#compare-modes').click();
  await expect(page.locator('#diff-before')).toHaveText('Safe output', { timeout: 30_000 });
  expect(await sideText(page, 'a')).toContain('createElement');
  expect(await sideText(page, 'b')).toContain('<div');
});

test('identical modes say so, and a new run drops the comparison', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = 1;');
  await runAndWait(page);
  await page.locator('#compare-modes').click();
  await expect(page.locator('#diff-note')).toHaveText('Safe and Unsafe give the same output.', { timeout: 30_000 });
  await runAndWait(page);
  await expect(page.locator('#diff-before')).toHaveText('input');
});

test('game.js: two workers at most, each within the budget, Cancel ends the comparison, and the 10 MB diff shows side by side', async ({ page }) => {
  skipWithoutGame();
  await openApp(page, '#unpack=0');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await runAndWait(page);
  await page.locator('#compare-modes').click();
  await expect(page.locator('#compare-label')).toHaveText('Comparing…');
  await expect(page.locator('#cancel')).toBeEnabled();
  let most = 0;
  for (let index = 0; index < 10; index += 1) {
    most = Math.max(most, page.workers().length);
    await page.waitForTimeout(100);
  }
  expect(most).toBeLessThanOrEqual(2);
  await page.locator('#cancel').click();
  await expect(page.locator('#compare-label')).toHaveText('Compare with Unsafe');
  await expect(page.locator('#status-label')).toHaveText('Done');
  await expect.poll(() => page.workers().length).toBeLessThanOrEqual(1);
  await page.evaluate(() => {
    const store: number[] = [];
    (window as unknown as { longTasks: number[] }).longTasks = store;
    new PerformanceObserver((list) => list.getEntries().forEach((entry) => store.push(entry.duration))).observe({ type: 'longtask' });
  });
  await page.locator('#compare-modes').click();
  await expect(page.locator('#diff')).toHaveAttribute('data-view', 'merge', { timeout: 120_000 });
  const chunks = Number(await page.locator('#diff').getAttribute('data-chunks'));
  expect(chunks).toBeGreaterThan(100);
  expect(await sideText(page, 'a')).not.toBe('');
  const shown = await page.evaluate(() => {
    const mark = performance.getEntriesByName('untwist:diff-shown').at(-1) as PerformanceMark | undefined;
    const heap = (performance as unknown as { memory?: { usedJSHeapSize: number } }).memory?.usedJSHeapSize ?? 0;
    return { ...(mark?.detail as { workerMs: number; totalMs: number }), heap, longTasks: (window as unknown as { longTasks: number[] }).longTasks };
  });
  test.info().annotations.push({
    type: 'game.js compare',
    description: `${chunks} chunks; worker ${shown.workerMs.toFixed(0)} ms; shown ${shown.totalMs.toFixed(0)} ms; page heap ${(shown.heap / 1048576).toFixed(1)} MiB; long tasks ${JSON.stringify(shown.longTasks.map(Math.round))}`,
  });
  expect(shown.totalMs).toBeLessThanOrEqual(5000);
  const memory = await compareMemory(page);
  test.info().annotations.push({ type: 'compare-memory', description: `${(memory / 1048576).toFixed(1)} MiB` });
  expect(memory).toBeGreaterThan(0);
  expect(memory).toBeLessThanOrEqual(BUDGET_BYTES);
  await expect.poll(() => page.workers().length).toBeLessThanOrEqual(1);
});
