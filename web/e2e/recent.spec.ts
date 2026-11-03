import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, test } from './fixtures';

const ITEMS = '#recent-menu [role=menuitem].recent-option';

async function inputText(page: Page): Promise<string> {
  return page.evaluate(() =>
    [...(document.querySelector('#input-editor .editor-host')?.shadowRoot?.querySelectorAll('.cm-line') ?? [])].map((line) => line.textContent ?? '').join('\n'),
  );
}

async function openFile(page: Page, name: string, text: string): Promise<void> {
  await page.locator('#open-file-input').setInputFiles({ name, mimeType: 'text/javascript', buffer: Buffer.from(text) });
  await expect(page.locator('#input-name')).toHaveText(name);
  await expect(page.locator('#status-label')).toHaveText(/Done|Stopped by limit/, { timeout: 60_000 });
}

async function openRecent(page: Page): Promise<void> {
  await page.locator('#recent').click();
  await expect(page.locator('#recent-menu')).toBeVisible();
}

test('a run is kept across a reload and reopens with its input and settings', async ({ page }) => {
  await openApp(page, '#rename=unique');
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await page.locator('#rename-group [data-mode="none"]').evaluate((button: HTMLButtonElement) => button.click());
  await page.locator('#mode-group [data-mode="unsafe"]').click();
  await openRecent(page);
  await expect(page.locator(ITEMS)).toHaveCount(1);
  await expect(page.locator(ITEMS).first()).toContainText('untitled.js');
  await expect(page.locator(ITEMS).first()).toContainText('11 B · just now · Safe · --rename unique');
  await page.locator(ITEMS).first().click();
  await expect(page.locator('#recent-menu')).toHaveCount(0);
  await expect(page.locator('#status-label')).toHaveText('Done');
  await expect.poll(() => inputText(page)).toBe('var a = !0;');
  await expect(page.locator('#mode-group [data-mode="safe"]')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#rename-group [aria-pressed="true"]')).toHaveAttribute('data-mode', 'unique');
  await expect(page.locator('#input-hint')).toHaveText('Reopened from recent runs.');
});

test('the list keeps the last ten inputs, newest first, and Clear empties it', async ({ page }) => {
  await openApp(page);
  for (let index = 0; index < 12; index += 1) {
    await openFile(page, `f${index}.js`, `var v${index} = ${index};`);
  }
  await openFile(page, 'again.js', 'var v5 = 5;');
  await openRecent(page);
  await expect(page.locator(ITEMS)).toHaveCount(10);
  await expect(page.locator(`${ITEMS} .strong`)).toHaveText(['again.js', 'f11.js', 'f10.js', 'f9.js', 'f8.js', 'f7.js', 'f6.js', 'f4.js', 'f3.js', 'f2.js']);
  await page.locator('#recent-clear').click();
  await expect(page.locator(ITEMS)).toHaveCount(0);
  await expect(page.locator('#recent-menu')).toContainText('No runs yet');
  await page.reload();
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await openRecent(page);
  await expect(page.locator(ITEMS)).toHaveCount(0);
});

test('the menu works from the keyboard', async ({ page }) => {
  await openApp(page);
  await openFile(page, 'one.js', 'var one = 1;');
  await openFile(page, 'two.js', 'var two = 2;');
  await page.locator('#recent').focus();
  await page.keyboard.press('Enter');
  await expect(page.locator(ITEMS).first()).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator(ITEMS).nth(1)).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator('#recent-clear')).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.locator(ITEMS).first()).toBeFocused();
  await page.keyboard.press('End');
  await expect(page.locator('#recent-clear')).toBeFocused();
  await page.keyboard.press('Escape');
  await expect(page.locator('#recent-menu')).toHaveCount(0);
  await expect(page.locator('#recent')).toBeFocused();
  await expect(page.locator('#recent')).toHaveAttribute('aria-expanded', 'false');
  await page.keyboard.press('Enter');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  await expect(page.locator('#input-name')).toHaveText('one.js');
});

test('without browser storage the page still runs and keeps this tab’s runs', async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(window, 'localStorage', {
      get() {
        throw new DOMException('storage is off', 'SecurityError');
      },
    });
  });
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await openRecent(page);
  await expect(page.locator(ITEMS)).toHaveCount(1);
  await expect(page.locator('#recent-menu')).toContainText('Browser storage is off, so runs are kept for this tab only.');
  await page.locator('#recent-clear').click();
  await expect(page.locator(ITEMS)).toHaveCount(0);
});
