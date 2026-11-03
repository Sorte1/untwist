import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test } from './fixtures';

const FIXTURE = 'web/e2e/data/safe-refuses-the-decoder.js';

async function sideText(page: Page, side: 'a' | 'b'): Promise<string> {
  return page.evaluate((which) => {
    const root = document.querySelector('#diff .editor-host')?.shadowRoot;
    return root?.querySelector(`.cm-merge-${which} .cm-content`)?.textContent ?? '';
  }, side);
}

async function codeText(page: Page): Promise<string> {
  return page.evaluate(() => document.querySelector('#diff .editor-host')?.shadowRoot?.querySelector('.cm-content')?.textContent ?? '');
}

function chip(page: Page, label: string) {
  return page.locator('#steps-bar .step-chip', { hasText: new RegExp(`^${label}\\d*$`) });
}

async function steppedRun(page: Page): Promise<void> {
  await openApp(page, '#unsafe=1');
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await page.locator('#view-steps').click();
  await expect(page.locator('#steps-bar .step-chip').first()).toBeVisible({ timeout: 60_000 });
}

test('Steps records each stage on demand and opens on the first one that changed the code', async ({ page }) => {
  await steppedRun(page);
  await expect(page.locator('#view-steps')).toHaveAttribute('aria-pressed', 'true');
  await expect(page.locator('#steps-bar')).toBeVisible();
  await expect(page.locator('#steps-bar .step-chip').first()).toHaveText('Input');
  await expect(page.locator('#steps-bar .step-chip').last()).toHaveText('Output');
  await expect(chip(page, 'deobfuscate').first()).toBeVisible();
  const pressed = page.locator('#steps-bar .step-chip[aria-pressed="true"]');
  await expect(pressed).toHaveCount(1);
  await expect(page.locator('#diff')).toHaveAttribute('data-view', 'merge');
  await expect(page.locator('#diff-before')).toHaveText('Input');
  await expect(page.locator('#diff-after')).toHaveText((await pressed.textContent())?.replace(/\d+$/, '') ?? '');
  expect(page.url()).toContain('view=steps');
});

test('a chosen step is set against the step before it, another step, or shown on its own', async ({ page }) => {
  await steppedRun(page);
  await chip(page, 'deobfuscate').first().click();
  await expect(page.locator('#diff-after')).toHaveText('deobfuscate');
  await expect.poll(() => sideText(page, 'b')).toContain('"Hello"');
  expect(await sideText(page, 'a')).toContain('b(0)');
  await page.locator('#step-against').selectOption({ label: 'Input' });
  await expect(page.locator('#diff-before')).toHaveText('Input');
  await expect.poll(() => sideText(page, 'a')).toContain('0x0');
  await page.locator('#step-against').selectOption('none');
  await expect(page.locator('#diff')).toHaveAttribute('data-view', 'code');
  await expect(page.locator('#diff-legend')).toBeHidden();
  await expect.poll(() => codeText(page)).toContain('"Hello"');
  await page.locator('#step-against').selectOption('prev');
  await page.locator('#steps-bar .step-chip[aria-pressed="true"]').focus();
  await page.keyboard.press('ArrowRight');
  await expect(page.locator('#steps-bar .step-chip[aria-pressed="true"]')).toBeFocused();
  await expect(page.locator('#diff-before')).toHaveText('deobfuscate');
});

test('a step that changed nothing says so, and Code leaves the steps', async ({ page }) => {
  await steppedRun(page);
  const same = page.locator('#steps-bar .step-chip.step-same').first();
  await same.click();
  await expect(page.locator('#diff-note')).toContainText('changed nothing');
  await page.locator('#view-output').click();
  await expect(page.locator('#steps-bar')).toBeHidden();
  await expect(page.locator('#output-editor')).toBeVisible();
});

test('a new run records its own steps', async ({ page }) => {
  await steppedRun(page);
  await page.locator('#view-output').click();
  await page.locator('#clear-input').click();
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await page.locator('#view-steps').click();
  await expect(page.locator('#steps-bar .step-chip').first()).toHaveText('Input', { timeout: 60_000 });
  await chip(page, 'Output').click();
  await page.locator('#step-against').selectOption({ label: 'Input' });
  await expect.poll(() => sideText(page, 'a')).toContain('!0');
  await expect.poll(() => sideText(page, 'b')).toContain('true');
});

test('leaving Steps while it records stops the recording, and coming back records again', async ({ page }) => {
  await openApp(page, '#unsafe=1');
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await page.locator('#view-steps').click();
  await page.locator('#view-output').click();
  await page.locator('#view-steps').click();
  await expect(page.locator('#steps-bar .step-chip').first()).toHaveText('Input', { timeout: 60_000 });
});
