import type { Page } from '@playwright/test';
import { expect, openApp, openDiagnostics, paste, readRepo, runAndWait, test } from './fixtures';

const FIXTURE = 'web/e2e/data/safe-refuses-the-decoder.js';

async function inputText(page: Page): Promise<string> {
  return (await page.locator('#input-editor .cm-content').textContent()) ?? '';
}

async function decodedRun(page: Page, hash: string): Promise<void> {
  await openApp(page, hash);
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await openDiagnostics(page);
}

test('Strings lists each decoded call with its value, finds by search and selects the call in the input', async ({ page }) => {
  await decodedRun(page, '#unsafe=1');
  const tab = page.locator('#strings-tab');
  await expect(tab).toBeVisible();
  await expect(page.locator('#strings-count')).toHaveText('2');
  await tab.click();
  await expect(tab).toHaveAttribute('aria-expanded', 'true');
  const rows = page.locator('#strings-table .string-row');
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(0).locator('.string-call')).toHaveText('b(0x0)');
  await expect(rows.nth(0).locator('.string-value')).toHaveText('"He"');
  await expect(rows.nth(0).locator('.string-loc')).toHaveText('4:13');
  await expect(rows.nth(1).locator('.string-value')).toHaveText('"llo"');
  await expect(page.locator('#strings-table .string-summary')).toHaveText('2 decoded strings · 2 distinct');
  await page.locator('#strings-search').fill('LLO');
  await expect(rows).toHaveCount(1);
  await expect(page.locator('#strings-table .string-summary')).toHaveText('1 of 2 decoded strings · 2 distinct');
  await rows.first().click();
  await page.keyboard.type('Z');
  expect(await inputText(page)).toContain('console.log(b(0x0) + Z, window[k]);');
  await expect(page.locator('#strings-table button.string-row')).toHaveCount(0);
  await expect(page.locator('#strings-table')).toContainText('The input changed after this run');
  await tab.click();
  await expect(page.locator('#strings-table')).toBeHidden();
});

test('a run that decodes nothing has no Strings tab', async ({ page }) => {
  await decodedRun(page, '');
  await expect(page.locator('#output-mode')).toHaveText('Safe');
  await expect(page.locator('#strings-tab')).toBeHidden();
});
