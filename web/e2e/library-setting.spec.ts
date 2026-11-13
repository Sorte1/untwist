import type { Page, Request } from '@playwright/test';
import { expect, openApp, openDiagnostics, openSettings, readRepo, runAndWait, test } from './fixtures';

const CORE_JS = 'fixtures/libraries/core-js/rollup-cjs.terser.js';
const LIBRARY_FILES = /\/libraries\/(index\.json|[\w-]+\.table)\b/;
const SCREENSHOTS = process.env['UNTWIST_PROGRESS_SHOTS'];

function libraryRequests(page: Page): Request[] {
  const seen: Request[] = [];
  page.on('request', (request) => {
    if (LIBRARY_FILES.test(request.url()) && request.serviceWorker() === null) {
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

async function detailsSection(page: Page, title: string) {
  await openDiagnostics(page);
  if ((await page.locator('#details-tab').getAttribute('aria-expanded')) !== 'true') {
    await page.locator('#details-tab').click();
  }
  return page.locator('#details .details-section').filter({ has: page.locator('h3', { hasText: new RegExp(`^${title}$`) }) });
}

function librarySwitch(page: Page) {
  return page.locator('.switch[data-key="libs"]');
}

test('library names are off by default: no scan, no table, no comments and no folds', async ({ page }) => {
  const requests = libraryRequests(page);
  await openApp(page);
  await openSettings(page);
  await expect(librarySwitch(page)).toHaveAttribute('aria-checked', 'false');
  await page.keyboard.press('Escape');
  await load(page, CORE_JS);
  await runAndWait(page);
  expect(requests.map((request) => request.url())).toEqual([]);
  await expect(page.locator('#library-code')).toBeHidden();
  await expect(page.locator('#output-editor .cm-content')).not.toContainText('// core-js');
  const libraries = await detailsSection(page, 'Libraries');
  await expect(libraries.locator('dd').first()).toHaveText('none detected');
  const timeline = await detailsSection(page, 'Run timeline');
  await expect(timeline).not.toContainText('Library tables');
  await expect(timeline).not.toContainText('Library names');
});

test('turning library names on fetches the table, folds the library and is remembered after a reload', async ({ page }) => {
  const requests = libraryRequests(page);
  await openApp(page);
  await openSettings(page);
  await librarySwitch(page).click();
  await expect(librarySwitch(page)).toHaveAttribute('aria-checked', 'true');
  await expect(page).toHaveURL(/libs=1/);
  await page.keyboard.press('Escape');
  await load(page, CORE_JS);
  await runAndWait(page);
  expect(requests.some((request) => /core-js\.table/.test(request.url()))).toBe(true);
  await expect(page.locator('#library-code')).toBeVisible();
  await expect(page.locator('#output-editor .cm-foldPlaceholder').first()).toBeVisible();
  const timeline = await detailsSection(page, 'Run timeline');
  await expect(timeline.locator('dt', { hasText: /^Library tables$/ })).toBeVisible();
  await expect(timeline.locator('dt', { hasText: /^Library names$/ })).toBeVisible();
  await page.goto('./#unsafe=0');
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(page).toHaveURL(/libs=1/);
  await openSettings(page);
  await expect(librarySwitch(page)).toHaveAttribute('aria-checked', 'true');
  await librarySwitch(page).click();
  await expect(page).not.toHaveURL(/libs=1/);
  await page.goto('./#unsafe=0');
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await openSettings(page);
  await expect(librarySwitch(page)).toHaveAttribute('aria-checked', 'false');
});

test('the progress panel names the table it fetches with its size, and cancel stops the run while it waits', async ({ page }) => {
  let release: () => void = () => undefined;
  const held = new Promise<void>((resolve) => (release = resolve));
  await page.route(/\/libraries\/core-js\.table\b/, async (route) => {
    await held;
    await route.continue().catch(() => undefined);
  });
  await openApp(page, '#libs=1');
  await load(page, CORE_JS);
  await expect(page.locator('#running-stage')).toHaveText(/^Fetching library tables: core-js \([\d.]+ (KB|MB)\)…$/);
  await expect(page.locator('#run-steps [aria-current="step"]')).toHaveAttribute('data-phase', 'tables');
  await expect(page.locator('#status-stage')).toHaveText(/^Library tables · /);
  if (SCREENSHOTS !== undefined) {
    await page.screenshot({ path: `${SCREENSHOTS}/progress-tables-${test.info().project.name}.png` });
  }
  const asked = Date.now();
  await page.locator('#cancel').click();
  await expect(page.locator('#status-label')).toHaveText('Cancelled', { timeout: 2_000 });
  expect(Date.now() - asked).toBeLessThan(2_000);
  release();
  await runAndWait(page);
  await expect(page.locator('#library-code')).toBeVisible();
});
