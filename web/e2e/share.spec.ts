import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, test } from './fixtures';

const CODE = 'var greeting = "h\\u00e9llo";\nfunction f() { return !0; }\nconsole.log(greeting, f());';

async function inputText(page: Page): Promise<string> {
  return page.evaluate(() =>
    [...(document.querySelector('#input-editor .editor-host')?.shadowRoot?.querySelectorAll('.cm-line') ?? [])].map((line) => line.textContent ?? '').join('\n'),
  );
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function copiedLink(page: Page): Promise<string> {
  await page.locator('#share-link').click();
  await expect(page.locator('#share-link-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

function noise(length: number): string {
  let seed = 7;
  let text = '';
  while (text.length < length) {
    seed = (seed * 1103515245 + 12345) % 2147483648;
    text += seed.toString(36);
  }
  return `var blob = "${text.slice(0, length)}";`;
}

test('Copy link carries the code and the settings, and the link restores both without running', async ({ page, context }) => {
  await openApp(page, '#rename=unique');
  await paste(page, CODE);
  await runAndWait(page);
  const expected = await outputText(page);
  const link = await copiedLink(page);
  expect(link).toMatch(/#unsafe=0&rename=unique&code=[A-Za-z0-9_-]+$/);
  await expect(page.locator('#input-hint')).toHaveText('Link copied with the code and settings. Nothing is uploaded.');
  const second = await context.newPage();
  await second.goto(link);
  await expect(second.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect.poll(() => inputText(second)).toBe(CODE);
  await expect(second.locator('#input-hint')).toHaveText('Opened from a link. Press Run.');
  await expect(second.locator('#rename-group [aria-pressed="true"]')).toHaveAttribute('data-mode', 'unique');
  await expect(second.locator('#status-label')).toHaveText('Ready');
  await expect(second.locator('#output-meta')).toHaveText('output.js · 0 lines');
  expect(new URL(second.url()).hash).toBe('#unsafe=0&rename=unique');
  await runAndWait(second);
  expect(await outputText(second)).toBe(expected);
});

test('a link keeps the file name of an opened file', async ({ page, context }) => {
  await openApp(page);
  await page.locator('#open-file-input').setInputFiles({ name: 'my bundle.js', mimeType: 'text/javascript', buffer: Buffer.from('var a = !0;') });
  await expect(page.locator('#input-name')).toHaveText('my bundle.js');
  const link = await copiedLink(page);
  expect(link).toContain('&name=my%20bundle.js');
  const second = await context.newPage();
  await second.goto(link);
  await expect(second.locator('#input-name')).toHaveText('my bundle.js');
  await expect.poll(() => inputText(second)).toBe('var a = !0;');
});

test('an input too large for a link copies the settings only and says so', async ({ page }) => {
  await openApp(page, '#rename=simple');
  await paste(page, noise(120_000));
  const link = await copiedLink(page);
  expect(new URL(link).hash).toBe('#unsafe=0&rename=simple');
  await expect(page.locator('#input-hint')).toHaveText('Too large for a link; it holds the settings only.');
});

test('an empty input copies the settings only', async ({ page }) => {
  await openApp(page);
  const link = await copiedLink(page);
  expect(new URL(link).hash).toBe('#unsafe=0');
  await expect(page.locator('#input-hint')).toHaveText('The input is empty; the link holds the settings only.');
});

test('a link whose code cannot be read keeps its settings and says so', async ({ page }) => {
  await openApp(page, '#rename=unique&code=not*base64');
  await expect(page.locator('#rename-group [aria-pressed="true"]')).toHaveAttribute('data-mode', 'unique');
  await expect(page.locator('#diag-list .diag-info')).toHaveText(/The code in the link could not be read/);
  expect(await inputText(page)).toBe('');
});

test('pasting a code link into the open page loads it', async ({ page, context }) => {
  await openApp(page);
  await paste(page, CODE);
  const link = await copiedLink(page);
  await page.locator('#clear-input').click();
  const fresh = await context.newPage();
  await openApp(fresh);
  await fresh.evaluate((hash) => {
    window.location.hash = hash;
  }, new URL(link).hash);
  await expect.poll(() => inputText(fresh)).toBe(CODE);
});
