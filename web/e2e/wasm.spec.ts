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
  await openApp(page, '#wasm=auto');
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
    expect(dialog.message()).toContain(`Fetch ${REMOTE}?\nHost: wasm.example.test\n`);
    void dialog.accept();
  });
  await page.locator(`#wasm-bar [data-wasm-url="${REMOTE}"]`).click();
  await expect(page.locator('#wasm-bar')).toContainText('fetched');
  expect(seen).toEqual([REMOTE]);
  await expect.poll(() => outputText(page)).toContain('function __wbg_log_9a1b2c(e, t)');
});

async function countRequests(page: Page, pattern: string): Promise<string[]> {
  const seen: string[] = [];
  await page.route(pattern, async (route) => {
    seen.push(route.request().url());
    await route.fulfill({ status: 200, contentType: 'application/wasm', body: wasmBytes('bindgen.wasm'), headers: { 'access-control-allow-origin': '*' } });
  });
  return seen;
}

for (const [label, url, why] of [
  ['a user name and password', 'https://user:pw@wasm.example.test/pkg/app_bg.wasm', 'carries a user name or password'],
  ['a private address', 'https://10.0.0.1/pkg/app_bg.wasm', 'the host is the private address 10.0.0.1'],
  ['a loopback name', 'https://localhost/pkg/app_bg.wasm', 'the host localhost is a loopback name'],
] as const) {
  test(`a url with ${label} is refused without a prompt or a request`, async ({ page }) => {
    const seen = await countRequests(page, '**/pkg/app_bg.wasm');
    let dialogs = 0;
    page.on('dialog', (dialog) => {
      dialogs += 1;
      void dialog.dismiss();
    });
    await openApp(page);
    await paste(page, remoteGlue(url));
    await runAndWait(page);
    await page.locator('#wasm-bar [data-wasm-url]').click();
    await expect(page.locator('#wasm-bar')).toContainText(why);
    expect(dialogs).toBe(0);
    expect(seen).toEqual([]);
  });
}

test('always fetch wasm does not fetch a refused url either', async ({ page }) => {
  const seen = await countRequests(page, 'https://10.0.0.1/**');
  await openApp(page);
  await page.evaluate(() => window.localStorage.setItem('untwist.alwaysFetchWasm', '1'));
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await paste(page, remoteGlue('https://10.0.0.1/pkg/app_bg.wasm'));
  await runAndWait(page);
  await expect(page.locator('#wasm-bar')).toContainText('private address 10.0.0.1');
  expect(seen).toEqual([]);
});

test('a redirect is not followed, so its target gets no request', async ({ page, browserName }) => {
  test.skip(browserName === 'webkit', 'Playwright cannot fulfill a redirect status in WebKit');
  const landed = await countRequests(page, 'https://10.0.0.1/**');
  await page.route('https://wasm.example.test/**', (route) =>
    route.fulfill({ status: 302, headers: { location: 'https://10.0.0.1/a.wasm', 'access-control-allow-origin': '*' } }),
  );
  await openApp(page);
  await paste(page, remoteGlue(REMOTE));
  await runAndWait(page);
  page.once('dialog', (dialog) => void dialog.accept());
  await page.locator(`#wasm-bar [data-wasm-url="${REMOTE}"]`).click();
  await expect(page.locator('#wasm-bar')).toContainText('the playground does not follow redirects');
  expect(landed).toEqual([]);
  expect(await outputText(page)).toContain('function Q(e, t)');
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

test('with the wasm check off an embedded module is flagged and one click extracts it', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo(`${FIXTURES}/bindgen.js`));
  await runAndWait(page);
  const warning = page.locator('#wasm-bar .wasm-unchecked');
  await expect(warning).toContainText('This file may contain embedded WebAssembly. Set Wasm check to Auto or On to extract it.');
  await expect(page.locator('#wasm-bar [data-wasm-path]')).toHaveCount(0);
  await warning.getByRole('button', { name: 'Use Auto and rerun' }).click();
  await expect(page.locator('#wasm-bar [data-wasm-path="wasm/deobfuscated.wasm"]')).toBeVisible();
  await expect(warning).toHaveCount(0);
  await openSettings(page);
  await expect(page.locator('#wasm-check-group [data-check="auto"]')).toHaveAttribute('aria-pressed', 'true');
});

test('the wasm check is off by default, notes that on is slow and is kept in the link', async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  const group = page.locator('#wasm-check-group');
  await expect(group.locator('[data-check="off"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('#wasm-on-note')).toHaveText('On can be slow on large files.');
  await group.locator('[data-check="on"]').click();
  await expect(group.locator('[data-check="on"]')).toHaveAttribute('aria-pressed', 'true');
  await expect(page).toHaveURL(/wasm=on/);
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await openSettings(page);
  await expect(page.locator('#wasm-check-group [data-check="on"]')).toHaveAttribute('aria-pressed', 'true');
  await page.keyboard.press('Escape');
  await paste(page, readRepo(`${FIXTURES}/bindgen.js`));
  await runAndWait(page);
  await expect(page.locator('#wasm-bar [data-wasm-path="wasm/deobfuscated.wasm"]')).toBeVisible();
});
