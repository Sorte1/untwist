import type { Page } from '@playwright/test';
import { palettes, type Palette } from '../scripts/contrast';
import { expect, openApp, openSettings, test } from './fixtures';

const SURFACES: ReadonlyArray<readonly [string, string, 'backgroundColor' | 'color']> = [
  ['body', 'bg', 'backgroundColor'],
  ['.bar', 'bg', 'backgroundColor'],
  ['.pane-head', 'head', 'backgroundColor'],
  ['#run', 'accent', 'backgroundColor'],
  ['#settings', 'panel', 'backgroundColor'],
  [".mode-btn[aria-checked='true']", 'btn', 'backgroundColor'],
  ['.brand-name', 'text', 'color'],
  ['#examples .muted', 'muted', 'color'],
];

function rgb(hex: string): string {
  const channel = (offset: number) => Number.parseInt(hex.slice(offset, offset + 2), 16);
  return `rgb(${channel(1)}, ${channel(3)}, ${channel(5)})`;
}

function palette(id: string, mode: 'dark' | 'light'): Palette {
  const found = palettes().find((entry) => entry.id === id && entry.mode === mode);
  if (found === undefined) {
    throw new Error(`no palette ${id} (${mode})`);
  }
  return found;
}

function hex(palette: Palette, token: string): string {
  return palette.colours.get(token) ?? '';
}

async function surfaceColours(page: Page): Promise<string[]> {
  return page.evaluate(
    (surfaces) => surfaces.map(([selector, , property]) => {
      const element = document.querySelector(selector);
      return element === null ? '' : getComputedStyle(element)[property];
    }),
    SURFACES.map(([selector, token, property]) => [selector, token, property] as const),
  );
}

async function themeColours(page: Page): Promise<string[]> {
  return page.locator('head meta[name="theme-color"]').evaluateAll((metas) => metas.map((meta) => (meta.getAttribute('content') ?? '').toLowerCase()));
}

async function expectPalette(page: Page, theme: Palette): Promise<void> {
  expect(await surfaceColours(page)).toEqual(SURFACES.map(([, token]) => rgb(hex(theme, token))));
  expect(await themeColours(page)).toEqual([hex(theme, 'bg').toLowerCase(), hex(theme, 'bg').toLowerCase()]);
}

for (const mode of ['dark', 'light'] as const) {
  test.describe(`the whole page follows the picked theme, ${mode} mode`, () => {
    test.use({ colorScheme: mode });

    test(`every ${mode} theme recolours the page, header, panes, buttons, settings and browser chrome at once`, async ({ page }) => {
      await openApp(page);
      await openSettings(page);
      const seen = new Set<string>();
      for (const theme of palettes().filter((entry) => entry.mode === mode)) {
        await page.locator(`#editor-theme-${mode} [data-pick="${theme.id}"]`).click();
        await expect(page.locator(`#editor-theme-${mode} [data-pick="${theme.id}"]`)).toHaveAttribute('aria-pressed', 'true');
        await expectPalette(page, theme);
        seen.add((await surfaceColours(page)).join('|'));
      }
      expect(seen.size).toBe(palettes().filter((entry) => entry.mode === mode).length);
    });
  });
}

test.describe('no flash of the default theme', () => {
  test.use({ colorScheme: 'dark' });

  test('a reload paints the stored theme from the first frame, before the app script runs', async ({ page }) => {
    await openApp(page);
    await page.evaluate(() => localStorage.setItem('untwist.editorThemes', JSON.stringify({ dark: 'catppuccin-mocha', light: 'github-light' })));
    const theme = palette('catppuccin-mocha', 'dark');
    await page.addInitScript(() => {
      const frames: string[][] = [];
      const atBody: string[] = [];
      Object.assign(window, { untwistFrames: frames, untwistAtBody: atBody });
      new MutationObserver((records, observer) => {
        if (document.body !== null) {
          const root = document.documentElement;
          atBody.push(root.getAttribute('data-theme') ?? '', root.getAttribute('data-editor') ?? '');
          observer.disconnect();
        }
      }).observe(document, { childList: true, subtree: true });
      const sample = () => {
        const pane = document.querySelector('.pane');
        frames.push([
          document.body === null ? '' : getComputedStyle(document.body).backgroundColor,
          pane === null ? '' : getComputedStyle(pane).backgroundColor,
        ]);
        if (frames.length < 30) {
          requestAnimationFrame(sample);
        }
      };
      requestAnimationFrame(sample);
    });
    await page.reload();
    await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
    const result = await page.evaluate(() => {
      const scope = window as unknown as { untwistFrames: string[][]; untwistAtBody: string[] };
      return { frames: scope.untwistFrames, atBody: scope.untwistAtBody };
    });
    expect(result.atBody).toEqual(['dark', 'catppuccin-mocha']);
    expect(result.frames.length).toBeGreaterThan(0);
    const painted = result.frames.filter(([body]) => body !== '' && body !== 'rgba(0, 0, 0, 0)');
    expect(painted.length).toBeGreaterThan(0);
    expect(new Set(painted.map(([body]) => body))).toEqual(new Set([rgb(hex(theme, 'bg'))]));
    expect(new Set(painted.map(([, pane]) => pane).filter((pane) => pane !== ''))).toEqual(new Set([rgb(hex(theme, 'code'))]));
    await expectPalette(page, theme);
  });

  test('a link that forces light mode paints light from the first frame', async ({ page }) => {
    await page.addInitScript(() => {
      const atBody: string[] = [];
      Object.assign(window, { untwistAtBody: atBody });
      new MutationObserver((records, observer) => {
        if (document.body !== null) {
          atBody.push(document.documentElement.getAttribute('data-theme') ?? '', document.documentElement.getAttribute('data-editor') ?? '');
          observer.disconnect();
        }
      }).observe(document, { childList: true, subtree: true });
    });
    await openApp(page, '#theme=light');
    expect(await page.evaluate(() => (window as unknown as { untwistAtBody: string[] }).untwistAtBody)).toEqual(['light', 'github-light']);
    await expectPalette(page, palette('github-light', 'light'));
  });
});

test.describe('the sun and moon button', () => {
  test.use({ colorScheme: 'dark' });

  test('switches the whole page between the dark and light picks', async ({ page }) => {
    await openApp(page);
    await expectPalette(page, palette('github-dark', 'dark'));
    await page.locator('#theme').click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
    await expectPalette(page, palette('github-light', 'light'));
    await page.locator('#theme').click();
    await expectPalette(page, palette('github-dark', 'dark'));
  });
});
