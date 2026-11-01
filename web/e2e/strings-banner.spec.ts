import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test } from './fixtures';

const FIXTURE = 'web/e2e/data/safe-refuses-the-decoder.js';
const BANNER = 'Strings were not decoded in Safe mode. Unsafe mode decodes them.';

async function outputText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const root = document.querySelector('#output-editor .editor-host')?.shadowRoot;
    return [...(root?.querySelectorAll('.cm-line') ?? [])].map((line) => line.textContent).join('\n');
  });
}

test('a Safe run that leaves the string decoder in place says so, and Run in Unsafe decodes the same input', async ({ page }) => {
  await openApp(page, '#rename=simple');
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  const banner = page.locator('#strings-banner');
  await expect(banner).toBeVisible();
  await expect(banner).toHaveAttribute('role', 'status');
  await expect(banner).toContainText(BANNER);
  await expect(banner).toContainText('2 decoder calls were left in place.');
  expect(await outputText(page)).not.toContain('"Hello"');
  await page.locator('#clear-input').click();
  await page.locator('#run-unsafe').focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#output-mode')).toHaveText('Unsafe', { timeout: 60_000 });
  await expect(page.locator('#status-label')).toHaveText('Done');
  await expect(page.locator('#mode-group [data-mode="unsafe"]')).toHaveAttribute('aria-checked', 'true');
  expect(page.url()).not.toContain('unsafe=0');
  expect(page.url()).toContain('rename=simple');
  await expect.poll(() => outputText(page)).toContain('"Hello"');
  await expect(banner).toBeHidden();
});

test('no banner for an Unsafe run', async ({ page }) => {
  await openApp(page, '#unsafe=1');
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await expect(page.locator('#output-mode')).toHaveText('Unsafe');
  await expect(page.locator('#strings-banner')).toBeHidden();
});

test('no banner for a Safe run that decodes its strings', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo('fixtures/reference/deobfuscate/obfuscator.io-control-flow.js'));
  await runAndWait(page);
  await expect(page.locator('#output-mode')).toHaveText('Safe');
  await expect(page.locator('#strings-banner')).toBeHidden();
});

for (const theme of ['dark', 'light'] as const) {
  test(`the banner reaches AA contrast in the ${theme} theme`, async ({ page }) => {
    await openApp(page, `#theme=${theme}`);
    await paste(page, readRepo(FIXTURE));
    await runAndWait(page);
    const ratios = await page.evaluate(() => {
      const rgb = (value: string) => (value.match(/[0-9.]+/g) ?? []).map(Number);
      const lum = (value: string) => {
        const [r = 0, g = 0, b = 0] = rgb(value).map((channel) => {
          const c = channel / 255;
          return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
        });
        return 0.2126 * r + 0.7152 * g + 0.0722 * b;
      };
      const surface = (element: Element | null): string => {
        for (let node = element; node !== null; node = node.parentElement) {
          const background = getComputedStyle(node).backgroundColor;
          if (background !== 'transparent' && rgb(background)[3] !== 0) {
            return background;
          }
        }
        return getComputedStyle(document.body).backgroundColor;
      };
      return ['#strings-banner-text', '#strings-banner-detail', '#run-unsafe'].map((selector) => {
        const element = document.querySelector(selector) as Element;
        const [a, b] = [lum(getComputedStyle(element).color), lum(surface(element))].sort((x, y) => y - x) as [number, number];
        return [selector, (a + 0.05) / (b + 0.05)] as const;
      });
    });
    for (const [selector, ratio] of ratios) {
      expect(ratio, selector).toBeGreaterThanOrEqual(4.5);
    }
  });
}
