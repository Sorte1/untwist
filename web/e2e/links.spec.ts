import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test } from './fixtures';

const FIXTURE = 'web/e2e/data/safe-refuses-the-decoder.js';

function linkedText(page: Page, pane: 'input' | 'output') {
  return page.locator(`#${pane}-editor .cm-linked`);
}

async function linkedRun(page: Page): Promise<void> {
  await openApp(page, '#unsafe=1');
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await expect(page.locator('#output-editor .cm-line', { hasText: 'Hello' })).toBeVisible();
}

test('a click in the output marks the input statement it came from, and the reverse', async ({ page }) => {
  await linkedRun(page);
  await page.locator('#output-editor .cm-line', { hasText: 'console.log' }).click();
  await expect(linkedText(page, 'input')).toHaveText(['console.log(b(0x0) + b(0x1), window[k]);']);
  await expect(linkedText(page, 'output')).toHaveText(['console.log("Hello", window[k]);']);
  await page.locator('#input-editor .cm-line', { hasText: "var k = 'x'" }).click();
  await expect(linkedText(page, 'output')).toHaveText(['var k = "x1";']);
  await expect(linkedText(page, 'input')).toHaveText(["var k = 'x' + 1;"]);
});

test('a click on code with nothing to link clears the marks', async ({ page }) => {
  await linkedRun(page);
  await page.locator('#output-editor .cm-line', { hasText: 'console.log' }).click();
  await expect(linkedText(page, 'input')).toHaveCount(1);
  await page.locator('#output-editor .cm-line').last().click();
  await page.keyboard.press('ControlOrMeta+End');
  await expect(linkedText(page, 'input')).toHaveCount(0);
});

test('an input edited after the run is not linked', async ({ page }) => {
  await linkedRun(page);
  await page.locator('#input-editor .cm-content').click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.type('\n// edited');
  await page.locator('#output-editor .cm-line', { hasText: 'console.log' }).click();
  await expect(linkedText(page, 'input')).toHaveCount(0);
  await expect(linkedText(page, 'output')).toHaveCount(0);
});

test('a click in an open module marks the part of the bundle it came from', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo('fixtures/reference/unpack/webpack-5.js'));
  await runAndWait(page);
  await page.locator('#module-list .tree-item', { hasText: 'lib.js' }).click();
  await page.locator('#output-editor .cm-line').first().click();
  await expect(linkedText(page, 'input').first()).toHaveText('require.r(exports);');
  const marked = (await linkedText(page, 'input').allTextContents()).map((line) => line.trim());
  expect(marked).toContain("version: '2.0.0',");
  expect(marked.join('\n')).not.toContain('foo: () => b');
});
