import type { Page } from '@playwright/test';
import { expect, openApp, openSettings, paste, runAndWait, test } from './fixtures';

const JSX_INPUT = 'const React = require("react");\nexport const a = React.createElement("div", { id: "x" }, "hi");\n';

async function outputText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const root = document.querySelector('#output-editor .editor-host')?.shadowRoot;
    return [...(root?.querySelectorAll('.cm-line') ?? [])].map((line) => line.textContent).join('\n');
  });
}

function radio(page: Page, mode: 'safe' | 'unsafe') {
  return page.locator(`#mode-group [role="radio"][data-mode="${mode}"]`);
}

test('the one Safe / Unsafe control drives the link, the warning and the Run mark', async ({ page }) => {
  await openApp(page);
  await expect(page.locator('#mode-group')).toHaveAttribute('role', 'radiogroup');
  await expect(radio(page, 'safe')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#mode-warning')).toBeHidden();
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await expect(page.locator('#output-mode')).toHaveText('Safe');
  await radio(page, 'unsafe').click();
  await expect(radio(page, 'unsafe')).toHaveAttribute('aria-checked', 'true');
  await expect(radio(page, 'safe')).toHaveAttribute('aria-checked', 'false');
  await expect(page.locator('.switch[data-key="unsafe"]')).toHaveCount(0);
  await expect(page.locator('#mode-warning')).toBeVisible();
  await expect(page.locator('#mode-warning')).toContainText('may change behaviour in edge cases; assumes built-ins are not modified');
  await expect(page.locator('#settings-changed')).toBeVisible();
  expect(page.url()).not.toContain('unsafe=');
  await expect(page.locator('#output-mode')).toHaveText('Safe');
  await runAndWait(page);
  await expect(page.locator('#output-mode')).toHaveText('Unsafe');
  await radio(page, 'safe').click();
  await expect(radio(page, 'safe')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#mode-warning')).toBeHidden();
  expect(page.url()).toContain('unsafe=0');
});

test('a fresh visit opens in Unsafe mode, and a link with unsafe=0 opens in Safe', async ({ page }) => {
  await page.goto('./');
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(radio(page, 'unsafe')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#mode-warning')).toBeVisible();
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await expect(page.locator('#output-mode')).toHaveText('Unsafe');
  expect(page.url()).not.toContain('unsafe=');
  await page.goto('./#unsafe=0');
  await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
  await expect(radio(page, 'safe')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#mode-warning')).toBeHidden();
});

test('a link with unsafe=1 opens in Unsafe mode', async ({ page }) => {
  await openApp(page, '#unsafe=1');
  await expect(radio(page, 'unsafe')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#mode-warning')).toBeVisible();
});

test('the radiogroup is one tab stop and the arrow keys change the mode', async ({ page }) => {
  await openApp(page);
  await radio(page, 'safe').focus();
  await page.keyboard.press('ArrowRight');
  await expect(radio(page, 'unsafe')).toHaveAttribute('aria-checked', 'true');
  await expect(radio(page, 'unsafe')).toBeFocused();
  await expect(radio(page, 'unsafe')).toHaveAttribute('tabindex', '0');
  await expect(radio(page, 'safe')).toHaveAttribute('tabindex', '-1');
  await page.keyboard.press('ArrowLeft');
  await expect(radio(page, 'safe')).toHaveAttribute('aria-checked', 'true');
  await expect(radio(page, 'safe')).toBeFocused();
  await page.keyboard.press('End');
  await expect(radio(page, 'unsafe')).toBeFocused();
  await page.keyboard.press('Tab');
  await expect(radio(page, 'safe')).not.toBeFocused();
  await expect(radio(page, 'unsafe')).not.toBeFocused();
  const outline = await radio(page, 'unsafe').evaluate((element) => {
    element.focus();
    return getComputedStyle(element).outlineStyle;
  });
  expect(outline).not.toBe('none');
});

test('JSX shows as inactive in Safe mode, and Unsafe really decompiles it', async ({ page }) => {
  await openApp(page);
  await expect(page.locator('#jsx-opt')).toHaveAttribute('data-inactive', '');
  await expect(page.locator('#jsx-hint')).toHaveText(/Inactive in Safe mode/);
  await paste(page, JSX_INPUT);
  await runAndWait(page);
  expect(await outputText(page)).toContain('createElement');
  await radio(page, 'unsafe').click();
  await expect(page.locator('#jsx-opt')).not.toHaveAttribute('data-inactive', '');
  await expect(page.locator('#jsx-hint')).not.toHaveText(/Inactive/);
  await runAndWait(page);
  expect(await outputText(page)).toContain('<div');
  await openSettings(page);
  await page.locator('.switch[data-key="jsx"]').click();
  await radio(page, 'safe').click();
  await expect(page.locator('#jsx-opt')).not.toHaveAttribute('data-inactive', '');
});

test('the control stays visible at phone width and with the input hidden', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openApp(page, '#input=hidden');
  const box = await page.locator('#mode-group').boundingBox();
  expect(box).not.toBeNull();
  expect((box?.x ?? -1) >= 0 && (box?.x ?? 0) + (box?.width ?? 0) <= 390).toBe(true);
  await expect(page.locator('#mode-group')).toBeInViewport();
  await radio(page, 'unsafe').click();
  await expect(page.locator('#mode-warning')).toBeInViewport();
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(390);
});

for (const theme of ['dark', 'light'] as const) {
  test(`the mode control and its warning reach AA contrast in the ${theme} theme`, async ({ page }) => {
    await openApp(page, `#unsafe=1&theme=${theme}`);
    await paste(page, 'var a = !0;');
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
          const alpha = rgb(background)[3];
          if (background !== 'transparent' && alpha !== 0) {
            return background;
          }
        }
        return getComputedStyle(document.body).backgroundColor;
      };
      const ratio = (selector: string) => {
        const element = document.querySelector(selector);
        const [a, b] = [lum(getComputedStyle(element as Element).color), lum(surface(element))].sort((x, y) => y - x) as [number, number];
        return [selector, Math.round(((a + 0.05) / (b + 0.05)) * 100) / 100] as const;
      };
      return ['#mode-group [data-mode="safe"]', '#mode-group [data-mode="unsafe"]', '#mode-warning', '#output-mode', '#jsx-hint'].map(ratio);
    });
    for (const [selector, value] of ratios) {
      expect(value, selector).toBeGreaterThanOrEqual(4.5);
    }
  });
}
