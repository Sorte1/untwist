import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import type { Page } from '@playwright/test';
import { expect, openApp, readRepo, repoPath, sha256, test, skipWithoutGame } from './fixtures';

const GAME = 'fixtures/real-world/game.js';
const DIST = resolve(repoPath('web'), 'dist');
const manifest = JSON.parse(readRepo('web/src/examples/manifest.json')) as ReadonlyArray<{ readonly name: string; readonly source: string }>;

function cli(fixture: string): string {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  return execFileSync(bin, [repoPath(fixture)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
}

async function finished(page: Page): Promise<void> {
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 120_000 });
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  const text = await page.evaluate(() => navigator.clipboard.readText());
  await expect(page.locator('#copy-label')).toHaveText('Copy');
  return text;
}

async function dropFile(page: Page, name: string, text: string): Promise<void> {
  const transfer = await page.evaluateHandle(
    ([fileName, body]) => {
      const data = new DataTransfer();
      data.items.add(new File([body], fileName, { type: 'text/javascript' }));
      return data;
    },
    [name, text] as const,
  );
  await page.dispatchEvent('body', 'dragenter', { dataTransfer: transfer });
  await expect(page.locator('#drop-overlay')).toContainText(`Drop to load ${name}`);
  await page.dispatchEvent('body', 'dragover', { dataTransfer: transfer });
  await page.dispatchEvent('body', 'drop', { dataTransfer: transfer });
  await expect(page.locator('#drop-overlay')).toHaveCount(0);
}


test('each example loads, runs and prints what the CLI prints', async ({ page }) => {
  await openApp(page);
  for (const example of manifest) {
    await page.locator('#examples').click();
    await page.locator('#examples-menu [role=option]', { hasText: example.name }).click();
    await expect(page.locator('#example-name')).toHaveText(example.name);
    await expect(page.locator('#input-name')).toHaveText(example.source.slice(example.source.lastIndexOf('/') + 1));
    await finished(page);
    expect(sha256(await outputText(page)), example.source).toBe(sha256(cli(example.source)));
  }
});

test('the examples are lazy chunks, not part of the initial load', async () => {
  const chunks = JSON.parse(readFileSync(join(DIST, '.vite/manifest.json'), 'utf8')) as Record<string, { file: string; isEntry?: boolean; imports?: string[] }>;
  const initial = new Set<string>();
  const pending = Object.keys(chunks).filter((key) => chunks[key]?.isEntry === true);
  for (let key = pending.pop(); key !== undefined; key = pending.pop()) {
    if (!initial.has(key)) {
      initial.add(key);
      pending.push(...(chunks[key]?.imports ?? []));
    }
  }
  const code = [...initial].map((key) => readFileSync(join(DIST, chunks[key]?.file ?? ''), 'utf8')).join('\n');
  for (const example of manifest) {
    const longest = readRepo(example.source).split('\n').reduce((a, b) => (b.trim().length > a.length ? b.trim() : a), '');
    expect(code.includes(longest), example.source).toBe(false);
  }
});

test('dropping game.js gives the preview and a full run', async ({ page }) => {
  skipWithoutGame();
  test.setTimeout(300_000);
  const text = readRepo(GAME);
  await openApp(page);
  await dropFile(page, 'game.js', text);
  await expect(page.locator('#input-name')).toHaveText('game.js');
  await expect(page.locator('#input-chip')).toHaveText(/MB · preview$/);
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 240_000 });
  expect(sha256(await outputText(page))).toBe(sha256(cli(GAME)));
});

test('the open dialog loads the first file with a note, and refuses text that is not UTF-8', async ({ page }) => {
  await openApp(page);
  await page.locator('#open-file-input').setInputFiles([
    { name: 'first.js', mimeType: 'text/javascript', buffer: Buffer.from('var a = !0;') },
    { name: 'second.js', mimeType: 'text/javascript', buffer: Buffer.from('var b = !1;') },
  ]);
  await expect(page.locator('#input-name')).toHaveText('first.js');
  await expect(page.locator('#input-hint')).toHaveText('2 files given; only the first, first.js, was loaded');
  await finished(page);
  expect(await outputText(page)).toBe('var a = true;\n');
  await page.locator('#open-file-input').setInputFiles([{ name: 'bad.bin', mimeType: 'application/octet-stream', buffer: Buffer.from([0xff, 0xfe, 0x80]) }]);
  await expect(page.locator('#input-hint')).toHaveText('bad.bin is not UTF-8 text, so it was not loaded');
  await expect(page.locator('#input-name')).toHaveText('first.js');
});

test('hide input round-trips through the link', async ({ page }) => {
  await openApp(page, '#unsafe=1');
  await page.locator('#hide-input').click();
  await expect(page.locator('#input-pane')).toBeHidden();
  await expect(page.locator('#show-input')).toBeVisible();
  await expect(page.locator('#focus-toggle')).toHaveAttribute('aria-pressed', 'true');
  expect(new URL(page.url()).hash).toBe('#input=hidden');
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(page.locator('#input-pane')).toBeHidden();
  const wide = await page.locator('#output-pane').boundingBox();
  expect(wide?.width ?? 0).toBeGreaterThan(1300);
  await page.locator('#show-input').click();
  await expect(page.locator('#input-pane')).toBeVisible();
  expect(new URL(page.url()).hash).toBe('');
  await page.locator('#focus-toggle').click();
  await expect(page.locator('#input-pane')).toBeHidden();
  await page.locator('#focus-toggle').click();
  await expect(page.locator('#input-pane')).toBeVisible();
});

test('a drag that hides its types until the drop is still accepted, and its file loads', async ({ page }) => {
  await openApp(page);
  const accepted = await page.evaluate(() => {
    const event = new DragEvent('dragover', { bubbles: true, cancelable: true, dataTransfer: new DataTransfer() });
    document.body.dispatchEvent(event);
    return event.defaultPrevented;
  });
  expect(accepted).toBe(true);
  await dropFile(page, 'hidden-types.js', 'var hidden = !0;');
  await expect(page.locator('#input-name, .mono-name').first()).toContainText('hidden-types.js');
});
