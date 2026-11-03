import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, saved, test } from './fixtures';

const OPTIONS = '#palette-list [role=option]';

async function palette(page: Page, query: string): Promise<void> {
  await page.keyboard.press('ControlOrMeta+K');
  await expect(page.locator('#palette')).toBeVisible();
  await expect(page.locator('#palette-input')).toBeFocused();
  await page.keyboard.type(query);
}

async function choose(page: Page, query: string): Promise<void> {
  await palette(page, query);
  await page.keyboard.press('Enter');
  await expect(page.locator('#palette')).toBeHidden();
}

test.use({ acceptDownloads: true });

test('Ctrl+K opens an accessible palette that filters as you type and closes on Escape', async ({ page }) => {
  await openApp(page);
  await page.locator('#examples').focus();
  await palette(page, '');
  const input = page.locator('#palette-input');
  await expect(input).toHaveAttribute('role', 'combobox');
  await expect(input).toHaveAttribute('aria-expanded', 'true');
  await expect(input).toHaveAttribute('aria-controls', 'palette-list');
  await expect(page.locator('#palette-list')).toHaveAttribute('role', 'listbox');
  const first = page.locator(OPTIONS).first();
  await expect(first).toHaveAttribute('aria-selected', 'true');
  await expect(input).toHaveAttribute('aria-activedescendant', (await first.getAttribute('id')) ?? '');
  await page.keyboard.press('ArrowDown');
  await expect(page.locator(OPTIONS).nth(1)).toHaveAttribute('aria-selected', 'true');
  await expect(input).toHaveAttribute('aria-activedescendant', (await page.locator(OPTIONS).nth(1).getAttribute('id')) ?? '');
  await page.keyboard.press('ArrowUp');
  await page.keyboard.press('ArrowUp');
  await expect(page.locator(OPTIONS).last()).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.type('webpack 5');
  await expect(page.locator(OPTIONS).first()).toContainText('webpack 5 bundle');
  await expect(page.locator(OPTIONS).first()).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.type('zzzz');
  await expect(page.locator(OPTIONS)).toHaveCount(0);
  await expect(page.locator('#palette')).toContainText('No matching command');
  await page.keyboard.press('Escape');
  await expect(page.locator('#palette')).toBeHidden();
  await expect(page.locator('#examples')).toBeFocused();
  await page.keyboard.press('ControlOrMeta+K');
  await expect(page.locator('#palette-input')).toHaveValue('');
  await page.keyboard.press('ControlOrMeta+K');
  await expect(page.locator('#palette')).toBeHidden();
});

test('an example picked in the palette loads and runs', async ({ page }) => {
  await openApp(page);
  await choose(page, 'webpack 5');
  await expect(page.locator('#example-name')).toHaveText('webpack 5 bundle');
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 60_000 });
  await expect(page.locator('#module-tree')).toBeVisible();
});

test('the palette sets the mode, switches and rename, and the link follows', async ({ page }) => {
  await openApp(page);
  await choose(page, 'unsafe mode');
  await expect(page.locator('#mode-group [data-mode="unsafe"]')).toHaveAttribute('aria-checked', 'true');
  await choose(page, 'rename unique');
  await expect(page.locator('#rename-group [aria-pressed="true"]')).toHaveAttribute('data-mode', 'unique');
  await choose(page, 'unpack bundles');
  await expect(page.locator('.switch[data-key="unpack"]')).toHaveAttribute('aria-checked', 'false');
  expect(new URL(page.url()).hash).toBe('#unpack=0&rename=unique');
  await palette(page, 'unpack bundles');
  await expect(page.locator(OPTIONS).first()).toContainText('off');
});

test('the palette runs, copies, downloads, switches the module and the theme', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await page.locator('#input-editor .cm-content').click();
  await choose(page, 'run');
  await expect(page.locator('#status-label')).toHaveText('Done');
  await choose(page, 'copy the output');
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('var a = true;\n');
  const waiting = page.waitForEvent('download');
  await choose(page, 'download');
  expect((await saved(await waiting)).toString('utf8')).toBe('var a = true;');
  const theme = await page.locator('html').getAttribute('data-theme');
  await choose(page, 'theme');
  await expect(page.locator('html')).not.toHaveAttribute('data-theme', theme ?? '');
  await choose(page, 'webpack 5');
  await expect(page.locator('#module-tree')).toBeVisible({ timeout: 60_000 });
  await palette(page, 'module ');
  const pick = page.locator(OPTIONS).nth(1);
  const path = (await pick.locator('.palette-title').textContent()) ?? '';
  await pick.click();
  await expect(page.locator('#output-meta')).toHaveText(new RegExp(`^${path.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')} · `));
});

test('commands that cannot run now are left out', async ({ page }) => {
  await openApp(page);
  await palette(page, 'copy the output');
  await expect(page.locator(OPTIONS)).toHaveCount(0);
  await page.keyboard.press('Escape');
  await palette(page, 'module');
  await expect(page.locator(OPTIONS)).toHaveCount(0);
});
