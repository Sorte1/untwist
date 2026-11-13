import { execFileSync } from 'node:child_process';
import type { Page, Request } from '@playwright/test';
import { expect, openApp, openDiagnostics, readRepo, repoPath, runAndWait, saved, test } from './fixtures';

const CORE_JS = 'fixtures/libraries/core-js/rollup-cjs.terser.js';
const CORE_JS_OBFUSCATED = 'fixtures/libraries/core-js/babel-entry.webpack5.prod.obf.js';
const CORE_ONLY = 'fixtures/libraries/regenerator-runtime/rollup-cjs.js';
const TABLE = /\/libraries\/core-js\.table\b/;
const LIBS = '#libs=1';

interface LibraryRow {
  readonly library: string;
  readonly range: string;
  readonly modules: number;
}

function cliLibraries(fixture: string): LibraryRow[] {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  const json = execFileSync(bin, ['--report', 'json', repoPath(fixture)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
  return (JSON.parse(json) as { libraries?: LibraryRow[] }).libraries ?? [];
}

function tableRequests(page: Page): Request[] {
  const seen: Request[] = [];
  page.on('request', (request) => {
    if (TABLE.test(request.url()) && request.serviceWorker() === null) {
      seen.push(request);
    }
  });
  return seen;
}

async function load(page: Page, fixture: string): Promise<void> {
  const name = fixture.split('/').at(-1) ?? 'input.js';
  await page.locator('#open-file-input').setInputFiles({ name, mimeType: 'text/javascript', buffer: Buffer.from(readRepo(fixture)) });
  await expect(page.locator('#input-editor .cm-content')).toContainText(/\S/);
}

async function openDetails(page: Page): Promise<void> {
  await openDiagnostics(page);
  if ((await page.locator('#details-tab').getAttribute('aria-expanded')) !== 'true') {
    await page.locator('#details-tab').click();
  }
  await expect(page.locator('#details .details-section').first()).toBeVisible();
}

function librariesSection(page: Page) {
  return page.locator('#details .details-section').filter({ has: page.locator('h3', { hasText: /^Libraries$/ }) });
}

async function downloadedLibraries(page: Page): Promise<LibraryRow[]> {
  await openDetails(page);
  const waiting = page.waitForEvent('download');
  await page.locator('#download-report').click();
  const report = JSON.parse((await saved(await waiting)).toString('utf8')) as { libraries?: LibraryRow[] };
  return report.libraries ?? [];
}

async function copiedOutput(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

test.use({ acceptDownloads: true });

test('core-js code fetches the core-js table, names its modules and matches the CLI report', async ({ page }) => {
  const requests = tableRequests(page);
  await openApp(page, LIBS);
  await load(page, CORE_JS);
  await runAndWait(page);
  expect(requests).toHaveLength(1);
  await expect(page.locator('#library-code')).toBeVisible();
  await expect(page.locator('#output-editor .cm-foldPlaceholder').first()).toBeVisible();
  const copied = await copiedOutput(page);
  expect(copied).toMatch(/^ *\/\/ core-js 3\.[\d.–]+: internals\/global-this\.js\n *function \w+\(\) \{\n/m);
  await openDetails(page);
  const section = librariesSection(page);
  await expect(section.locator('dt', { hasText: /^core-js$/ })).toBeVisible();
  await expect(section).toContainText('Tables loaded');
  expect(await downloadedLibraries(page)).toEqual(cliLibraries(CORE_JS));
});

test('a second run and a reload take the table from the cache', async ({ page }) => {
  const requests = tableRequests(page);
  await openApp(page, LIBS);
  await load(page, CORE_JS);
  await runAndWait(page);
  await runAndWait(page);
  expect(requests).toHaveLength(1);
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await load(page, CORE_JS);
  await runAndWait(page);
  expect(requests).toHaveLength(1);
  await openDetails(page);
  await expect(librariesSection(page).locator('dt', { hasText: /^core-js$/ })).toBeVisible();
});

test('code with only compiled-in libraries fetches no table', async ({ page }) => {
  const requests = tableRequests(page);
  await openApp(page, LIBS);
  await load(page, CORE_ONLY);
  await runAndWait(page);
  expect(requests).toEqual([]);
  await openDetails(page);
  const section = librariesSection(page);
  await expect(section.locator('dt', { hasText: /^regenerator-runtime$/ })).toBeVisible();
  await expect(section).not.toContainText('Tables loaded');
  expect(await downloadedLibraries(page)).toEqual(cliLibraries(CORE_ONLY));
});

test('core-js hidden by a string array is found in the output and run again with its table', async ({ page }) => {
  const requests = tableRequests(page);
  await openApp(page, LIBS);
  await load(page, CORE_JS_OBFUSCATED);
  await runAndWait(page);
  expect(requests).toHaveLength(1);
  await openDetails(page);
  await expect(librariesSection(page)).toContainText('core-js (found in the output, run again)');
  expect(await downloadedLibraries(page)).toEqual(cliLibraries(CORE_JS_OBFUSCATED));
});

test('the run says it is loading tables, and still finishes when a table is broken', async ({ page }) => {
  let release: () => void = () => undefined;
  const held = new Promise<void>((resolve) => (release = resolve));
  await page.route(TABLE, async (route) => {
    await held;
    await route.fulfill({ status: 200, contentType: 'application/octet-stream', body: Buffer.from('not a table') });
  });
  await openApp(page, LIBS);
  await load(page, CORE_JS);
  await expect(page.locator('#running-stage')).toHaveText(/^Fetching library tables: core-js \([\d.]+ (KB|MB)\)…$/);
  release();
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 120_000 });
  await openDetails(page);
  const section = librariesSection(page);
  await expect(section).toContainText('Tables not loaded');
  await expect(section.locator('dt', { hasText: /^core-js$/ })).toHaveCount(0);
});
