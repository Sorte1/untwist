import type { Page } from '@playwright/test';
import { readFileSync } from 'node:fs';
import { expect, openApp, openSettings, paste, readRepo, repoPath, runAndWait, saved, test } from './fixtures';

const FIXTURES = 'crates/untwist/tests/wasm';
const REMOTE = 'https://wasm.example.test/pkg/app_bg.3f2a91c0.wasm';

function remoteGlue(url: string): string {
  return readRepo(`${FIXTURES}/bindgen-remote.js`).replace('pkg/app_bg.3f2a91c0.wasm', url);
}

function wasmBytes(name: string): Buffer {
  return readFileSync(repoPath(`${FIXTURES}/${name}`));
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function countWasmRequests(page: Page): Promise<string[]> {
  const seen: string[] = [];
  await page.route('https://wasm.example.test/**', async (route) => {
    seen.push(route.request().url());
    await route.fulfill({ status: 200, contentType: 'application/wasm', body: wasmBytes('bindgen.wasm') });
  });
  return seen;
}

test('an embedded wasm module is listed and downloads as its own file', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo(`${FIXTURES}/bindgen.js`));
  await runAndWait(page);
  const save = page.locator('#wasm-bar [data-wasm-path="wasm/deobfuscated.wasm"]');
  await expect(save).toBeVisible();
  await expect(page.locator('#wasm-bar')).toContainText('3 renamed');
  const [download] = await Promise.all([page.waitForEvent('download'), save.click()]);
  expect(download.suggestedFilename()).toBe('deobfuscated.wasm');
  expect(await saved(download)).toEqual(wasmBytes('bindgen.wasm'));
});

test('a declined fetch prompt sends no request', async ({ page }) => {
  const seen = await countWasmRequests(page);
  await openApp(page);
  await paste(page, remoteGlue(REMOTE));
  await runAndWait(page);
  const fetchButton = page.locator(`#wasm-bar [data-wasm-url="${REMOTE}"]`);
  await expect(fetchButton).toBeVisible();
  page.once('dialog', (dialog) => void dialog.dismiss());
  await fetchButton.click();
  await page.waitForTimeout(300);
  expect(seen).toEqual([]);
  expect(await outputText(page)).toContain('function Q(e, t)');
});

test('an accepted fetch prompt fetches that one url and names the glue', async ({ page }) => {
  const seen = await countWasmRequests(page);
  await openApp(page);
  await paste(page, remoteGlue(REMOTE));
  await runAndWait(page);
  page.once('dialog', (dialog) => {
    expect(dialog.message()).toContain(REMOTE);
    void dialog.accept();
  });
  await page.locator(`#wasm-bar [data-wasm-url="${REMOTE}"]`).click();
  await expect(page.locator('#wasm-bar')).toContainText('fetched');
  expect(seen).toEqual([REMOTE]);
  await expect.poll(() => outputText(page)).toContain('function __wbg_log_9a1b2c(e, t)');
});

test('nothing is fetched on its own unless always fetch wasm is on', async ({ page }) => {
  const seen = await countWasmRequests(page);
  await openApp(page);
  await paste(page, remoteGlue(REMOTE));
  await runAndWait(page);
  await page.waitForTimeout(300);
  expect(seen).toEqual([]);
  await openSettings(page);
  if (!(await page.locator('#advanced').evaluate((fold) => (fold as HTMLDetailsElement).open))) {
    await page.locator('#advanced summary').click();
  }
  const always = page.locator('#always-fetch-wasm');
  await expect(always).not.toBeChecked();
  await always.check();
  await page.keyboard.press('Escape');
  await runAndWait(page);
  await expect(page.locator('#wasm-bar')).toContainText('fetched');
  expect(seen).toEqual([REMOTE]);
});

test('a relative url asks for its base and is not fetched without one', async ({ page }) => {
  const seen = await countWasmRequests(page);
  await openApp(page);
  await paste(page, readRepo(`${FIXTURES}/bindgen-remote.js`));
  await runAndWait(page);
  page.once('dialog', (dialog) => {
    expect(dialog.type()).toBe('prompt');
    void dialog.dismiss();
  });
  await page.locator('#wasm-bar [data-wasm-url="pkg/app_bg.3f2a91c0.wasm"]').click();
  await page.waitForTimeout(300);
  expect(seen).toEqual([]);
});

test('several dropped wasm files are matched to the references by name', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo(`${FIXTURES}/bindgen-remote.js`));
  await runAndWait(page);
  await page.locator('#open-file-input').setInputFiles([
    { name: 'app_bg.wasm', mimeType: 'application/wasm', buffer: wasmBytes('bindgen.wasm') },
    { name: 'other.wasm', mimeType: 'application/wasm', buffer: wasmBytes('webpack.wasm') },
  ]);
  await expect(page.locator('#wasm-bar')).toContainText('supplied');
  await expect.poll(() => outputText(page)).toContain('function __wbg_log_9a1b2c(e, t)');
});
