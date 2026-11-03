import { execFileSync } from 'node:child_process';
import type { Page } from '@playwright/test';
import { expect, openApp, openSettings, paste, readRepo, repoPath, runAndWait, sha256, test } from './fixtures';

const FIXTURE = 'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js';

function cli(flags: string[], fixture: string): string {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  return execFileSync(bin, [...flags, repoPath(fixture)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

test('the Passes section lists every pass the CLI lists, all on by default', async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await page.locator('#passes summary').click();
  const listed = cli(['--list-passes'], FIXTURE)
    .split('\n')
    .slice(1)
    .filter((row) => row !== '')
    .map((row) => row.split(/\s+/)[0]);
  const boxes = page.locator('#passes-list input[data-pass]');
  expect(await boxes.evaluateAll((els) => els.map((el) => (el as HTMLInputElement).dataset['pass']))).toEqual(listed);
  expect(await boxes.evaluateAll((els) => els.every((el) => (el as HTMLInputElement).checked))).toBe(true);
  await expect(page.locator('#passes-reset')).toBeDisabled();
});

test('unticking passes writes the link, runs as the CLI --disable does, and Turn all on resets', async ({ page, context }) => {
  await openApp(page);
  await openSettings(page);
  await page.locator('#passes summary').click();
  await page.locator('#passes-list input[data-pass="find-string-array"]').uncheck();
  await page.locator('#passes-list input[data-pass="unminify-booleans"]').uncheck();
  await expect(page.locator('#passes-off')).toHaveText('2 off');
  await expect(page.locator('#query')).toHaveText(/off=find-string-array%2Cunminify-booleans/);
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  const expected = cli(['--disable', 'find-string-array,unminify-booleans'], FIXTURE);
  expect(sha256(await outputText(page))).toBe(sha256(expected));
  expect(expected).not.toBe(cli([], FIXTURE));
  await openSettings(page);
  await page.locator('#copy-link').click();
  const link = await page.evaluate(() => navigator.clipboard.readText());
  const second = await context.newPage();
  await second.goto(link);
  await expect(second.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(second.locator('#passes-list input[data-pass="find-string-array"]')).not.toBeChecked();
  await expect(second.locator('#passes-off')).toHaveText('2 off');
  await page.locator('#passes-reset').click();
  await expect(page.locator('#passes-off')).toHaveText('');
  await expect(page.locator('#query')).not.toHaveText(/off=/);
  await expect(page.locator('#passes-list input[data-pass="unminify-booleans"]')).toBeChecked();
});

test('an unknown pass in the link is ignored with one info line', async ({ page }) => {
  await openApp(page, '#off=no-such-pass');
  await expect(page.locator('#passes-off')).toHaveText('');
  await expect(page.locator('#diag-list .diag-info')).toHaveCount(1);
  await expect(page.locator('#diag-list .diag-info')).toContainText('no-such-pass');
});
