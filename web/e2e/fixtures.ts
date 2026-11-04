import { createHash } from 'node:crypto';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { test as base, expect, type Download, type Page } from '@playwright/test';

const HERE = dirname(fileURLToPath(import.meta.url));
export const REPO = resolve(HERE, '../..');

export function repoPath(relative: string): string {
  return resolve(REPO, relative);
}

export const GAME_JS = 'fixtures/real-world/game.js';

export function skipWithoutGame(): void {
  base.skip(!existsSync(repoPath(GAME_JS)), `${GAME_JS} is not present`);
}

export function readRepo(relative: string): string {
  return readFileSync(repoPath(relative), 'utf8');
}

export function sha256(text: string): string {
  return createHash('sha256').update(text, 'utf8').digest('hex');
}

export async function paste(page: Page, text: string): Promise<void> {
  await page.locator('#input-editor .cm-content').click();
  await page.evaluate((value) => navigator.clipboard.writeText(value), text);
  await page.keyboard.press('ControlOrMeta+V');
  await page.evaluate(() => new Promise<void>((resolve) => requestAnimationFrame(() => setTimeout(resolve, 0))));
}

export async function saved(download: Download): Promise<Buffer> {
  const path = base.info().outputPath(download.suggestedFilename());
  await download.saveAs(path);
  return readFileSync(path);
}

export function pinnedMode(hash: string): string {
  if (/(^|[#&])unsafe=/.test(hash)) {
    return hash;
  }
  return hash === '' ? '#unsafe=0' : `${hash}&unsafe=0`;
}

export async function openApp(page: Page, hash = ''): Promise<void> {
  await page.goto(`./${pinnedMode(hash)}`);
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
}

export async function openSettings(page: Page): Promise<void> {
  if (!(await page.locator('#settings').isVisible())) {
    await page.locator('#settings-toggle').click();
  }
  await expect(page.locator('#settings')).toBeVisible();
}

export async function openDiagnostics(page: Page): Promise<void> {
  if (await page.locator('#diagnostics').isHidden()) {
    await page.locator('#diag-toggle').click();
  }
  await expect(page.locator('#diagnostics')).toBeVisible();
}

export async function runAndWait(page: Page): Promise<void> {
  await page.locator('#run').click();
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 120_000 });
}

const ABORTED_WORKER_LOAD = /\/assets\/lang\.worker-[\w-]+\.js due to access control checks/;

type Fixtures = { pageErrors: string[] };

export const test = base.extend<Fixtures>({
  pageErrors: [
    async ({ page }, use) => {
      const errors: string[] = [];
      page.on('pageerror', (error) => {
        if (!ABORTED_WORKER_LOAD.test(error.message)) {
          errors.push(error.message);
        }
      });
      page.on('console', (message) => {
        if (message.type() === 'error' && !ABORTED_WORKER_LOAD.test(message.text())) {
          errors.push(message.text());
        }
      });
      await use(errors);
      expect(errors, 'page errors').toEqual([]);
    },
    { auto: true },
  ],
});

export { expect };
