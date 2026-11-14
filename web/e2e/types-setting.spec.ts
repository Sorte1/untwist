import { expect, openApp, openSettings, paste, runAndWait, saved, test } from './fixtures';

test.use({ acceptDownloads: true });

const SOURCE = `function greet(name) {
  let count = 0;
  for (let i = 0; i < name.length; i++) {
    count += 2;
  }
  return count;
}
export const total = greet(String(Date.now()));
console.log(total);
`;

function group(page: import('@playwright/test').Page) {
  return page.locator('#types-group');
}

test('types are off by default and the output stays untyped', async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await expect(group(page).locator('[data-types="off"]')).toHaveAttribute('aria-pressed', 'true');
  await page.keyboard.press('Escape');
  await paste(page, SOURCE);
  await runAndWait(page);
  await expect(page.locator('#output-editor .cm-content')).toContainText('function greet(name) {');
  await expect(page.locator('#download-label')).toHaveText('Download .js');
});

test('TypeScript writes the proven types, is kept in the link and downloads as .ts', async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await group(page).locator('[data-types="ts"]').click();
  await expect(group(page).locator('[data-types="ts"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('#types-hint')).toContainText('TypeScript');
  await expect(page).toHaveURL(/types=ts/);
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await openSettings(page);
  await expect(group(page).locator('[data-types="ts"]')).toHaveAttribute('aria-pressed', 'true');
  await page.keyboard.press('Escape');
  await paste(page, SOURCE);
  await runAndWait(page);
  const output = page.locator('#output-editor .cm-content');
  await expect(output).toContainText('function greet(name: string): number {');
  await expect(output).toContainText('let count: number = 0;');
  await expect(page.locator('#download-label')).toHaveText('Download .ts');
  const [download] = await Promise.all([page.waitForEvent('download'), page.locator('#download').click()]);
  expect(download.suggestedFilename()).toBe('deobfuscated.ts');
  expect((await saved(download)).toString()).toContain('function greet(name: string): number {');
});

test('JSDoc keeps JavaScript and adds comments', async ({ page }) => {
  await openApp(page, '#types=jsdoc');
  await paste(page, SOURCE);
  await runAndWait(page);
  const output = page.locator('#output-editor .cm-content');
  await expect(output).toContainText('/** @param {string} name @returns {number} */ function greet(name) {');
  await expect(page.locator('#download-label')).toHaveText('Download .js');
});
