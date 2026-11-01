import { execFileSync } from 'node:child_process';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';
import type { Page } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';
import { expect, openApp, paste, readRepo, repoPath, runAndWait, saved, test } from './fixtures';

const FIXTURE = 'fixtures/reference/unpack/webpack-5.js';

interface BundleJson {
  readonly modules: ReadonlyArray<{ readonly path: string; readonly isEntry: boolean }>;
}

function cliTree(dir: string): Map<string, string> {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  execFileSync(bin, ['-f', '-o', dir, repoPath(FIXTURE)], { env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' } });
  const walk = (at: string): string[] =>
    readdirSync(at).flatMap((name) => (statSync(join(at, name)).isDirectory() ? walk(join(at, name)) : [join(at, name)]));
  return new Map(walk(dir).map((file) => [relative(dir, file), readFileSync(file, 'utf8')]));
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

test.use({ acceptDownloads: true });

test('the webpack 5 bundle shows the CLI modules, their code and the same zip', async ({ page }) => {
  const files = cliTree(test.info().outputPath('cli'));
  const bundle = JSON.parse(files.get('bundle.json') ?? '{}') as BundleJson;
  const expected = [...bundle.modules]
    .sort((a, b) => Number(b.isEntry) - Number(a.isEntry) || (a.path < b.path ? -1 : 1))
    .map((module) => module.path);
  await openApp(page);
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await expect(page.locator('#module-count')).toHaveText(`bundle · ${expected.length} modules`);
  expect(await page.locator('#module-list .tree-name').allTextContents()).toEqual(['bundle', ...expected]);
  for (const path of expected) {
    await page.locator('#module-list .tree-item', { hasText: path }).first().click();
    await expect(page.locator('#output-meta')).toContainText(path);
    expect(await outputText(page)).toBe(files.get(path));
  }
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

test('without a bundle there is no tree and the download is a .js', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await expect(page.locator('#module-tree')).toBeHidden();
  await expect(page.locator('#download-label')).toHaveText('Download .js');
});
