import type { Page } from '@playwright/test';
import { resolve } from 'node:path';
import { expect, repoPath, test } from './fixtures';
import { serveDist, type DistServer } from './serve-dist';

const DIST = resolve(repoPath('web'), 'dist');
const NEXT_VERSION = 'e2e0000000000002';

let server: DistServer;

test.beforeAll(async () => {
  server = await serveDist(DIST);
});

test.afterAll(async () => {
  await server.close();
});

test.afterEach(() => {
  server.patchWorker = null;
});

async function openControlled(page: Page): Promise<void> {
  await page.goto(`${server.url}#unsafe=0`);
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null, null, { timeout: 60_000 });
}

async function cacheState(page: Page): Promise<{ names: string[]; paths: string[] }> {
  return page.evaluate(async () => {
    const names = await caches.keys();
    const cache = await caches.open(names[0] ?? '');
    return { names, paths: (await cache.keys()).map((request) => new URL(request.url).pathname) };
  });
}

test('under the deployed headers the page is cross-origin isolated and a service worker precaches it', async ({ page }) => {
  await openControlled(page);
  expect(await page.evaluate(() => self.crossOriginIsolated)).toBe(true);
  const { names, paths } = await cacheState(page);
  expect(names).toHaveLength(1);
  expect(names[0]).toMatch(/^untwist-[0-9a-f]{16}$/);
  expect(paths).toContain('/');
  expect(paths.some((path) => /^\/assets\/wasm\/.*\.wasm$/.test(path))).toBe(true);
  expect(paths.some((path) => /^\/assets\/run\.worker-.*\.js$/.test(path))).toBe(true);
  expect(paths).not.toContain('/sw.js');
});

test('offline, a reload comes from the cache, stays isolated and runs an example', async ({ page, context }) => {
  await openControlled(page);
  await context.setOffline(true);
  try {
    await page.reload();
    await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
    expect(await page.evaluate(() => self.crossOriginIsolated)).toBe(true);
    await page.locator('#examples').click();
    await page.locator('#examples-menu [role=option]', { hasText: 'webpack 5 bundle' }).click();
    await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 60_000 });
    await expect(page.locator('#module-tree')).toBeVisible();
  } finally {
    await context.setOffline(false);
  }
});

test('a new version waits for the prompt, Later hides it, and Reload switches to it', async ({ page }) => {
  await openControlled(page);
  const first = (await cacheState(page)).names[0];
  server.patchWorker = (code) => code.replace(/"[0-9a-f]{16}"/, `"${NEXT_VERSION}"`);
  await page.evaluate(() => navigator.serviceWorker.getRegistration().then((registration) => registration?.update()));
  await expect(page.locator('#update-prompt')).toBeVisible({ timeout: 60_000 });
  await expect(page.locator('#update-prompt')).toHaveText(/A new version of Untwist is ready/);
  await page.locator('#update-dismiss').click();
  await expect(page.locator('#update-prompt')).toHaveCount(0);
  await page.reload();
  await expect(page.locator('#update-prompt')).toBeVisible({ timeout: 60_000 });
  expect((await cacheState(page)).names).toContain(first);
  const reloaded = page.waitForEvent('load');
  await page.locator('#update-reload').click();
  await reloaded;
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect.poll(async () => (await cacheState(page)).names).toEqual([`untwist-${NEXT_VERSION}`]);
  await expect(page.locator('#update-prompt')).toHaveCount(0);
});
