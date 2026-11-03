import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test } from './fixtures';

const FIXTURE = 'web/e2e/data/safe-refuses-the-decoder.js';

async function inputText(page: Page): Promise<string> {
  return (await page.locator('#input-editor .cm-content').textContent()) ?? '';
}

async function refusedRun(page: Page, hash = ''): Promise<void> {
  await openApp(page, hash);
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await expect(page.locator('#strings-banner')).toBeVisible();
}

test('Why? names the line that kept the decoder, quotes it, and the click selects it in the input', async ({ page }) => {
  await refusedRun(page);
  const why = page.locator('#refusal-why');
  const panel = page.locator('#refusal-why-panel');
  await expect(why).toHaveAttribute('aria-expanded', 'false');
  await expect(panel).toBeHidden();
  await why.click();
  await expect(why).toHaveAttribute('aria-expanded', 'true');
  await expect(panel).toBeVisible();
  await expect(panel).toContainText('Why Safe mode left the decoder calls in place');
  const row = panel.locator('button.why-row', { has: page.locator('.why-code', { hasText: 'window[k]' }) }).first();
  await expect(row.locator('.why-loc')).toHaveText('Line 4');
  await expect(row.locator('.why-text')).toContainText('so the decoder calls were left alone');
  await row.click();
  await page.keyboard.type('Z');
  expect(await inputText(page)).toContain('console.log(b(0x0) + b(0x1), Z);');
  await expect(panel.locator('button.why-row')).toHaveCount(0);
  await expect(panel).toContainText('The input changed after this run, so the lines are not linked.');
  await why.click();
  await expect(panel).toBeHidden();
});

test('a jump from Why? shows a hidden input first', async ({ page }) => {
  await refusedRun(page, '#input=hidden');
  await expect(page.locator('#input-pane')).toBeHidden();
  await page.locator('#refusal-why').click();
  await page.locator('#refusal-why-panel button.why-row').first().click();
  await expect(page.locator('#input-pane')).toBeVisible();
  await page.keyboard.type('Z');
  expect(await inputText(page)).toContain('Z');
});

test('the explanation closes with the banner when a run decodes the strings', async ({ page }) => {
  await refusedRun(page);
  await page.locator('#refusal-why').click();
  await expect(page.locator('#refusal-why-panel')).toBeVisible();
  await page.locator('#run-unsafe').click();
  await expect(page.locator('#output-mode')).toHaveText('Unsafe', { timeout: 60_000 });
  await expect(page.locator('#strings-banner')).toBeHidden();
  await expect(page.locator('#refusal-why-panel')).toBeHidden();
  await expect(page.locator('#refusal-why')).toHaveAttribute('aria-expanded', 'false');
});
