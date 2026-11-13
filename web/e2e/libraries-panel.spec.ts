import { execFileSync } from 'node:child_process';
import type { Page } from '@playwright/test';
import { expect, openApp, openDiagnostics, readRepo, repoPath, runAndWait, test } from './fixtures';

const CORE_JS = 'fixtures/libraries/core-js/rollup-cjs.terser.js';
const SCREENSHOTS = process.env['UNTWIST_PROGRESS_SHOTS'];

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

async function load(page: Page, fixture: string): Promise<void> {
  const name = fixture.split('/').at(-1) ?? 'input.js';
  await page.locator('#open-file-input').setInputFiles({ name, mimeType: 'text/javascript', buffer: Buffer.from(readRepo(fixture)) });
  await expect(page.locator('#input-editor .cm-content')).toContainText(/\S/);
}

async function openPanel(page: Page) {
  await openDiagnostics(page);
  if ((await page.locator('#libraries-tab').getAttribute('aria-expanded')) !== 'true') {
    await page.locator('#libraries-tab').click();
  }
  const panel = page.locator('#libraries-panel');
  await expect(panel).toBeVisible();
  return panel;
}

test('with library names off the panel explains the setting and turns it on', async ({ page }) => {
  await openApp(page);
  await load(page, CORE_JS);
  await runAndWait(page);
  const panel = await openPanel(page);
  await expect(panel).toContainText('Library names is off');
  await page.locator('#libraries-turn-on').click();
  await expect(page).toHaveURL(/libs=1/);
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 120_000 });
  await expect(panel.locator('.lib-item[data-library="core-js"]')).toBeVisible();
});

test('the panel lists each library as the CLI reports it, jumps to a module and shows the tables', async ({ page }) => {
  await openApp(page, '#libs=1');
  await load(page, CORE_JS);
  await runAndWait(page);
  await expect(page.locator('#libraries-count')).toHaveText(/^\d+$/);
  const panel = await openPanel(page);
  for (const row of cliLibraries(CORE_JS)) {
    const item = panel.locator(`.lib-item[data-library="${row.library}"]`);
    await expect(item.locator('.lib-range')).toHaveText(row.range);
    await expect(item.locator('.lib-count')).toHaveText(`${row.modules} ${row.modules === 1 ? 'module' : 'modules'}`);
  }
  const core = panel.locator('.lib-item[data-library="core-js"]');
  await expect(core.locator('.lib-jump')).toHaveCount(0);
  await core.locator('summary').click();
  const jumps = core.locator('.lib-jump');
  await expect(jumps.first()).toBeVisible();
  const target = jumps.nth(Math.min(3, (await jumps.count()) - 1));
  const module = (await target.locator('.lib-module-name').textContent()) ?? '';
  await target.click();
  const active = page.locator('#output-editor .cm-activeLine');
  await expect(active).toContainText('// core-js');
  await expect(active).toContainText(module);
  await expect(active).toBeInViewport();
  const table = panel.locator('.lib-table-row[data-state="loaded"]').filter({ hasText: 'core-js' });
  await expect(table.locator('.lib-table-size')).toHaveText(/^[\d.]+ (KB|MB)$/);
  await expect(table.locator('.lib-table-ms')).toHaveText(/^\d+ ms$|^\d+\.\d\d s$/);
  await expect(panel.locator('.lib-table-row[data-state="built-in"]').first()).toBeVisible();
  if (SCREENSHOTS !== undefined) {
    await page.screenshot({ path: `${SCREENSHOTS}/libraries-panel-${test.info().project.name}.png`, caret: 'initial' });
  }
});

test('a run without library code says so and builds no module list', async ({ page }) => {
  await openApp(page, '#libs=1');
  await page.locator('#open-file-input').setInputFiles({ name: 'a.js', mimeType: 'text/javascript', buffer: Buffer.from('var a = 1;') });
  await expect(page.locator('#status-label')).toHaveText('Done');
  const panel = await openPanel(page);
  await expect(panel).toContainText('No known library code was found in this output.');
  await expect(panel.locator('.lib-item')).toHaveCount(0);
  await expect(page.locator('#libraries-count')).toHaveText('');
});
