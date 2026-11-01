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
