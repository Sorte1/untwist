import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test, skipWithoutGame } from './fixtures';

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}


test('an empty input runs and gives an empty program', async ({ page }) => {
  await openApp(page);
  await runAndWait(page);
  await expect(page.locator('#status-label')).toHaveText('Done');
  await expect(page.locator('#exit-code')).toHaveText('0');
  expect(await outputText(page)).toBe('\n');
});

test('a paste that is not JavaScript ends with a parser error row, not a crash', async ({ page }) => {
  await openApp(page);
  await paste(page, '<html><body>Hello, {{ name }}!</body></html>\n# not code');
  await runAndWait(page);
  await expect(page.locator('#exit-code')).not.toHaveText('…');
  await expect(page.locator('#diag-list')).toContainText('parser');
  await expect(page.locator('#run')).toBeEnabled();
});

test('a 10 MB paste is held whole, previewed, and runs', async ({ page }) => {
  skipWithoutGame();
  await openApp(page, '#unpack=0&jsx=0&unsafe=1');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await expect(page.locator('#input-chip')).toHaveText('10.2 MB · preview');
  await expect(page.locator('#input-meta')).toContainText('10.2 MB');
  await runAndWait(page);
  await expect(page.locator('#status-label')).toHaveText('Done');
});

test('rapid Run and Cancel settle into a consistent state, and the next run works', async ({ page }) => {
  skipWithoutGame();
  await openApp(page, '#unpack=0&jsx=0&unsafe=1');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  for (let round = 0; round < 12; round += 1) {
    await page.locator('#run').click();
    await page.locator('#cancel').click();
  }
  await expect(page.locator('#status-label')).toHaveText('Cancelled');
  await expect(page.locator('#exit-code')).toHaveText('killed');
  await expect(page.locator('#cancel')).toBeDisabled();
  const workers = await page.evaluate(() => performance.getEntriesByType('resource').filter((entry) => entry.name.includes('run.worker')).length);
  expect(workers).toBeGreaterThan(0);
  await page.locator('#clear-input').click();
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  expect(await outputText(page)).toBe('var a = true;\n');
});

test('a reload during a run starts clean', async ({ page }) => {
  skipWithoutGame();
  await openApp(page, '#unpack=0&jsx=0&unsafe=1');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await page.locator('#run').click();
  await expect(page.locator('#status-label')).toHaveText('Running');
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator('#status-label')).toHaveText('Ready');
  await expect(page.locator('#exit-code')).toHaveText('—');
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  expect(await outputText(page)).toBe('var a = true;\n');
});

test('a link whose setting names are Object.prototype names still starts and lists them as unknown', async ({ page }) => {
  await openApp(page, '#toString=1&constructor=1&__proto__=1');
  await expect(page.locator('#diag-list .diag-info')).toHaveCount(3);
  for (const name of ['toString', 'constructor', '__proto__']) {
    await expect(page.locator('#diag-list')).toContainText(`Unknown setting "${name}"`);
  }
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  expect(await outputText(page)).toBe('var a = true;\n');
});

test('a link with 200,000 unknown settings starts and lists only a few of them', async ({ page }) => {
  await openApp(page, `#${'a&'.repeat(200_000)}`);
  await expect(page.locator('#diag-list .diag-info')).toHaveCount(22);
  await expect(page.locator('#diag-list')).toContainText(/And [\d,]+ more problems with the settings in the link/);
  await expect(page.locator('#diag-list')).toContainText(/The link is longer than [\d,]+ characters/);
});

test('a bundle with thousands of modules lists one page of them and shows more on request', async ({ page }) => {
  const modules = Array.from({ length: 1_200 }, (_, at) => `${at}:function(a,b,c){}`).join(',');
  await openApp(page);
  await paste(page, `(self.webpackChunkx=self.webpackChunkx||[]).push([[15],{${modules}}]);`);
  await runAndWait(page);
  await expect(page.locator('#module-count')).toHaveText('bundle · 1200 modules');
  await expect(page.locator('#module-list .tree-name')).toHaveCount(501);
  await page.locator('#module-list .tree-more').click();
  await expect(page.locator('#module-list .tree-name')).toHaveCount(1_001);
  await page.locator('#module-list .tree-more').click();
  await expect(page.locator('#module-list .tree-name')).toHaveCount(1_201);
  await expect(page.locator('#module-list .tree-more')).toHaveCount(0);
});
