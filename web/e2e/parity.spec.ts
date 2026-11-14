import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import type { Page } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';
import { expect, openApp, readRepo, repoPath, runAndWait, saved, sha256, test } from './fixtures';

const MODERN = 'fixtures/modern/babel-usage.core-js-3.39.0.rollup-cjs.js';
const BOUNDARIES = 'fixtures/unpack-rollup-boundaries/synthetic-rollup3-wrapped/index.js';
const PLAIN = 'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js';
const MAP_COMMENT = '//# sourceMappingURL=data:application/json;';

function cli(args: readonly string[], fixture: string): string {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  return execFileSync(bin, [...args, repoPath(fixture)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
}

function cliTree(args: readonly string[], fixture: string, dir: string): Map<string, string> {
  cli([...args, '-f', '-o', dir], fixture);
  const walk = (at: string): string[] =>
    readdirSync(at).flatMap((name) => (statSync(join(at, name)).isDirectory() ? walk(join(at, name)) : [join(at, name)]));
  return new Map(walk(dir).map((file) => [relative(dir, file), readFileSync(file, 'utf8')]));
}

async function load(page: Page, fixture: string): Promise<void> {
  const name = fixture.split('/').at(-1) ?? 'input.js';
  await page.locator('#open-file-input').setInputFiles({ name, mimeType: 'text/javascript', buffer: Buffer.from(readRepo(fixture)) });
  await expect(page.locator('#input-editor .cm-content')).toContainText(/\S/);
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

test.use({ acceptDownloads: true });

test('Modern baseline removes core-js polyfills exactly as --modern does', async ({ page }) => {
  const expected = cli(['--modern', 'es2022'], MODERN);
  expect(expected).not.toBe(cli([], MODERN));
  await openApp(page, '#libs=1&modern=es2022');
  await load(page, MODERN);
  await runAndWait(page);
  expect(sha256(await outputText(page))).toBe(sha256(expected));
});

test('Modern baseline fetches the core-js table with Library names off', async ({ page }) => {
  const expected = cli(['--disable', 'library-names,library-rename', '--modern', 'es2022'], MODERN);
  expect(expected).not.toBe(cli(['--disable', 'library-names,library-rename'], MODERN));
  await openApp(page, '#modern=es2022');
  await load(page, MODERN);
  await runAndWait(page);
  expect(sha256(await outputText(page))).toBe(sha256(expected));
});

test('a baseline of engines with spaces reaches the engine whole', async ({ page }) => {
  const expected = cli(['--modern', 'chrome 120, firefox 121'], MODERN);
  await openApp(page, `#libs=1&modern=${encodeURIComponent('chrome 120, firefox 121')}`);
  await load(page, MODERN);
  await runAndWait(page);
  expect(sha256(await outputText(page))).toBe(sha256(expected));
});

test('Split hoisted bundles with Assume ES modules gives the CLI tree, inferred boundaries included', async ({ page }) => {
  const args = ['--split-hoisted', '--split-assume-modules'];
  const files = cliTree(args, BOUNDARIES, test.info().outputPath('cli'));
  const modules = (JSON.parse(files.get('bundle.json') ?? '{}') as { modules: unknown[] }).modules;
  expect(modules.length).toBeGreaterThanOrEqual(10);
  await openApp(page, '#libs=1&split=1&assume=1');
  await load(page, BOUNDARIES);
  await runAndWait(page);
  await expect(page.locator('#module-count')).toHaveText(`bundle · ${modules.length} modules`);
  await expect(page.locator('#download-label')).toHaveText('Download .zip');
  const waiting = page.waitForEvent('download');
  await page.locator('#download').click();
  const zip = unzipSync(await saved(await waiting));
  const entries = new Map(Object.entries(zip).map(([name, data]) => [name, strFromU8(data)]));
  expect([...entries.keys()].sort()).toEqual([...files.keys()].sort());
  for (const [name, text] of entries) {
    expect(text, name).toBe(files.get(name));
  }
});

test('Assume ES modules does nothing without the split, as on the CLI', async ({ page }) => {
  await openApp(page, '#libs=1&assume=1');
  await load(page, BOUNDARIES);
  await runAndWait(page);
  expect(sha256(await outputText(page))).toBe(sha256(cli([], BOUNDARIES)));
});

test('Source map appends the same inline map as --source-map inline', async ({ page }) => {
  const expected = cli(['--source-map', 'inline'], PLAIN);
  expect(expected).toContain(MAP_COMMENT);
  await openApp(page, '#libs=1&smap=1');
  await load(page, PLAIN);
  await runAndWait(page);
  expect(sha256(await outputText(page))).toBe(sha256(expected));
});

test('a bundle that splits only under the assumption offers it, and the button turns it on and reruns', async ({ page }) => {
  const files = cliTree(['--split-hoisted', '--split-assume-modules'], BOUNDARIES, test.info().outputPath('cli'));
  const modules = (JSON.parse(files.get('bundle.json') ?? '{}') as { modules: unknown[] }).modules;
  await openApp(page, '#libs=1');
  await load(page, BOUNDARIES);
  await runAndWait(page);
  await expect(page.locator('#split-banner')).toBeVisible();
  await expect(page.locator('#split-banner-text')).toHaveText("This bundle can be split with 'Assume ES modules'.");
  await page.locator('#split-assume').click();
  await expect(page.locator('#status-label')).toHaveText(/Done/, { timeout: 120_000 });
  await expect(page.locator('#module-count')).toHaveText(`bundle · ${modules.length} modules`);
  await expect(page.locator('#split-banner')).toBeHidden();
  await expect(page.locator('#query')).toContainText('split=1&assume=1');
});

test('the modern presets and a custom baseline set the flag', async ({ page }) => {
  await openApp(page);
  await page.locator('#settings-toggle').click();
  await page.locator('#modern-preset').selectOption('es2022');
  await expect(page.locator('#query')).toContainText('modern=es2022');
  await expect(page.locator('#modern')).toBeHidden();
  await page.locator('#modern-preset').selectOption('custom');
  await page.locator('#modern').fill('node 20');
  await expect(page.locator('#query')).toContainText('modern=node%2020');
  await page.locator('#modern-preset').selectOption('');
  await expect(page.locator('#query')).not.toContainText('modern=');
});
