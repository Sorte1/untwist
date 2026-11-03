import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, saved, test } from './fixtures';

const OUTPUT = '#output-editor .cm-content';

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function sideB(page: Page): Promise<string> {
  return page.evaluate(() => document.querySelector('#diff .editor-host')?.shadowRoot?.querySelector('.cm-merge-b .cm-content')?.textContent ?? '');
}

test.use({ acceptDownloads: true });

test('the output takes typing straight after a run; Copy, Download and Diff use the edit, and Run replaces it', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await expect(page.locator('#edit-output')).toHaveCount(0);
  await expect(page.locator('#output-readonly')).toHaveCount(0);
  await expect(page.locator('#output-edit-mark')).toBeHidden();
  await page.locator(OUTPUT).click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.type('var b = 2;');
  await expect(page.locator('#output-edit-mark')).toHaveText('Edited · 1 line');
  await expect(page.locator('#reset-output')).toHaveAttribute('title', 'Run replaces your edits');
  expect(await outputText(page)).toBe('var a = true;\nvar b = 2;');
  const waiting = page.waitForEvent('download');
  await page.locator('#download').click();
  expect((await saved(await waiting)).toString('utf8')).toBe('var a = true;\nvar b = 2;');
  await page.locator('#view-diff').click();
  await expect(page.locator('#diff-after')).toHaveText('output.js (edited)');
  await expect.poll(() => sideB(page)).toContain('var b = 2;');
  await page.locator('#view-output').click();
  expect(await outputText(page)).toBe('var a = true;\nvar b = 2;');
  await runAndWait(page);
  expect(await outputText(page)).toBe('var a = true;\n');
  await expect(page.locator('#output-edit-mark')).toBeHidden();
  await expect(page.locator('#output-edit-actions')).toBeHidden();
});

test('Reset to output drops the edits and keeps the output editable', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await page.locator(OUTPUT).click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.type('x();');
  await page.getByRole('button', { name: 'Reset to output' }).click();
  await expect(page.locator('#output-edit-mark')).toBeHidden();
  expect(await outputText(page)).toBe('var a = true;\n');
  await page.locator(OUTPUT).click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.type('y();');
  expect(await outputText(page)).toBe('var a = true;\ny();');
});
