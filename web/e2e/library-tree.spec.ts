import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import type { Page } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';
import { expect, openApp, paste, readRepo, repoPath, runAndWait, saved, test } from './fixtures';

const CORE_JS = 'fixtures/libraries/core-js/rollup-cjs.terser.js';
const PLAIN = 'fixtures/reference/unpack/webpack-5.js';
const SPLIT = '#libs=1&split=1&assume=1';

interface BundleJson {
  readonly modules: ReadonlyArray<{ readonly path: string; readonly isEntry: boolean }>;
}

function cliTree(dir: string): Map<string, string> {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  execFileSync(bin, ['-f', '--split-hoisted', '--split-assume-modules', '-o', dir, repoPath(CORE_JS)], {
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
    maxBuffer: 1 << 28,
  });
  const walk = (at: string): string[] =>
    readdirSync(at).flatMap((name) => (statSync(join(at, name)).isDirectory() ? walk(join(at, name)) : [join(at, name)]));
  return new Map(walk(dir).map((file) => [relative(dir, file), readFileSync(file, 'utf8')]));
}

function group(page: Page, key: string) {
  return page.locator(`#module-list details[data-group="${key}"]`);
}

async function loadCoreJs(page: Page): Promise<void> {
  await openApp(page, SPLIT);
  await page.locator('#open-file-input').setInputFiles({ name: 'rollup-cjs.terser.js', mimeType: 'text/javascript', buffer: Buffer.from(readRepo(CORE_JS)) });
  await expect(page.locator('#input-editor .cm-content')).toContainText(/\S/);
  await runAndWait(page);
}

test.use({ acceptDownloads: true });

test('core-js modules sit in a collapsed Libraries group apart from the app modules', async ({ page }) => {
  const files = cliTree(test.info().outputPath('cli'));
  const bundle = JSON.parse(files.get('bundle.json') ?? '{}') as BundleJson;
  const library = bundle.modules.filter((module) => module.path.startsWith('node_modules/core-js/'));
  const app = bundle.modules.filter((module) => !module.path.startsWith('node_modules/'));
  expect(library.length).toBeGreaterThan(0);
  await loadCoreJs(page);
  await expect(page.locator('#module-count')).toHaveText(`bundle · ${bundle.modules.length} modules`);
  await expect(group(page, 'app')).toHaveAttribute('open', '');
  await expect(group(page, 'app').locator(':scope > summary')).toContainText(`App${app.length} modules`);
  await expect(group(page, 'app').locator(':scope > .tree-item')).toHaveCount(app.length);
  await expect(group(page, 'app').locator('.tree-lib-badge')).toHaveCount(0);
  const libraries = group(page, 'libraries');
  await expect(libraries).not.toHaveAttribute('open', '');
  await expect(libraries.locator(':scope > summary')).toContainText(`Libraries${library.length} modules`);
  const coreJs = group(page, 'lib:core-js');
  await expect(coreJs).toBeHidden();
  await libraries.locator(':scope > summary').click();
  await expect(coreJs).toBeVisible();
  await expect(coreJs).not.toHaveAttribute('open', '');
  await expect(coreJs.locator(':scope > summary')).toContainText(/^core-js 3\.[\d.–]+/);
  await expect(coreJs.locator(':scope > summary')).toContainText(`${library.length} modules`);
  await coreJs.locator(':scope > summary').click();
  await expect(coreJs.locator('.tree-item')).toHaveCount(library.length);
  await expect(coreJs.locator('.tree-lib-badge')).toHaveCount(library.length);
  const target = library[0]?.path ?? '';
  const short = target.slice('node_modules/core-js/'.length);
  await coreJs.locator('.tree-item', { has: page.locator('.tree-name', { hasText: new RegExp(`^${short.replace(/[.]/g, '\\.')}$`) }) }).click();
  await expect(page.locator('#output-meta')).toContainText(target);
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(files.get(target));
  await expect(coreJs.locator('.tree-item[aria-current="true"]')).toHaveCount(1);
});

test('the zip keeps the CLI layout with libraries under node_modules', async ({ page }) => {
  const files = cliTree(test.info().outputPath('cli'));
  await loadCoreJs(page);
  const waiting = page.waitForEvent('download');
  await page.locator('#download').click();
  const zip = unzipSync(await saved(await waiting));
  const entries = new Map(Object.entries(zip).map(([name, data]) => [name, strFromU8(data)]));
  expect([...entries.keys()].sort()).toEqual([...files.keys()].sort());
  expect([...entries.keys()].some((name) => name.startsWith('node_modules/core-js/'))).toBe(true);
  for (const [name, text] of entries) {
    expect(text, name).toBe(files.get(name));
  }
});

test('picking a library module from the palette opens its collapsed groups', async ({ page }) => {
  await loadCoreJs(page);
  await expect(group(page, 'libraries')).not.toHaveAttribute('open', '');
  await page.locator('#output-editor .cm-content').click();
  await page.keyboard.press('ControlOrMeta+K');
  await expect(page.locator('#palette-input')).toBeFocused();
  await page.keyboard.type('node_modules/core-js/internals/global-this.js');
  await page.keyboard.press('Enter');
  await expect(page.locator('#palette')).toBeHidden();
  await expect(page.locator('#output-meta')).toContainText('node_modules/core-js/internals/global-this.js');
  await expect(group(page, 'libraries')).toHaveAttribute('open', '');
  await expect(group(page, 'lib:core-js')).toHaveAttribute('open', '');
  await expect(group(page, 'lib:core-js').locator('.tree-item[aria-current="true"]')).toBeVisible();
  await expect(group(page, 'lib:core-js').locator('.tree-item[aria-current="true"] .tree-name')).toHaveText('internals/global-this.js');
});

test('a bundle without library modules keeps the flat tree', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo(PLAIN));
  await runAndWait(page);
  await expect(page.locator('#module-list .tree-item').first()).toBeVisible();
  await expect(page.locator('#module-list details')).toHaveCount(0);
  await expect(page.locator('#module-list .tree-lib-badge')).toHaveCount(0);
});
