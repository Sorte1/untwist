import type { Page } from '@playwright/test';
import { differingShare, showDesign, type Rect } from './design';
import { expect, openApp, paste, readRepo, runAndWait, test, skipWithoutGame } from './fixtures';

const VIEW = { width: 1440, height: 960 };
const SCREENSHOT_STYLE_REFUSAL =
  "Refused to apply a stylesheet because its hash, its nonce, or 'unsafe-inline' does not appear in the style-src directive of the Content Security Policy.";
const MASKED = [
  '#input-editor',
  '#output-editor',
  '#output-pane .pane-head',
  '#input-name',
  '#input-chip',
  '#input-meta',
  '#output-meta',
  '#elapsed',
  '.examples',
  '.brand-name',
  '.privacy',
  '#mode-group',
  '#diag-toggle',
];

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function masks(page: Page): Promise<Rect[]> {
  const rects = await page.evaluate((selectors) => {
    const boxes = selectors.flatMap((selector) => [...document.querySelectorAll(selector)].map((el) => el.getBoundingClientRect()));
    const diagnostics = document.querySelector('.diagnostics')?.getBoundingClientRect();
    const all = diagnostics === undefined ? boxes : [...boxes, new DOMRect(diagnostics.x, diagnostics.y, diagnostics.width, innerHeight - diagnostics.y)];
    return all.map((r) => ({ x: Math.floor(r.x) - 2, y: Math.floor(r.y) - 2, width: Math.ceil(r.width) + 4, height: Math.ceil(r.height) + 4 }));
  }, MASKED);
  return rects;
}

test.use({ viewport: VIEW });

test('paste, run and read the output', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await expect(page.locator('#status-label')).toHaveText('Done');
  await expect(page.locator('#exit-code')).toHaveText('0');
  await expect(page.locator('#limits')).toHaveText('none fired');
  expect(await outputText(page)).toBe('var a = true;\n');
});

test('cancel a long run, then run again', async ({ page }) => {
  skipWithoutGame();
  await openApp(page);
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await expect(page.locator('#input-chip')).toHaveText('10.2 MB · preview');
  await page.locator('#run').click();
  await expect(page.locator('#status-label')).toHaveText('Running');
  await expect(page.locator('#running')).toBeVisible();
  await page.waitForTimeout(500);
  await page.locator('#cancel').click();
  await expect(page.locator('#status-label')).toHaveText('Cancelled');
  await expect(page.locator('#exit-code')).toHaveText('killed');
  await page.locator('#clear-input').click();
  await paste(page, 'var b = !1;');
  await runAndWait(page);
  expect(await outputText(page)).toBe('var b = false;\n');
});

for (const theme of ['dark', 'light'] as const) {
  test(`the ${theme} theme matches the design board`, async ({ page, browserName, pageErrors }) => {
    await page.emulateMedia({ colorScheme: 'dark' });
    await openApp(page);
    if (theme === 'light') {
      await page.locator('#theme').click();
    }
    await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
    await paste(page, 'var a = !0;');
    await runAndWait(page);
    await page.evaluate(() => document.fonts.ready.then(() => undefined));
    await page.mouse.move(0, VIEW.height - 1);
    const app = await page.screenshot({ caret: 'initial' });
    if (browserName === 'webkit') {
      const kept = pageErrors.filter((error) => error !== SCREENSHOT_STYLE_REFUSAL);
      pageErrors.splice(0, pageErrors.length, ...kept);
    }
    const regions = await masks(page);
    const design = await page.context().newPage();
    await showDesign(design, theme);
    const board = await design.screenshot({ caret: 'initial' });
    const share = await differingShare(design, app, board, regions);
    await test.info().attach(`app-${theme}`, { body: app, contentType: 'image/png' });
    await test.info().attach(`design-${theme}`, { body: board, contentType: 'image/png' });
    test.info().annotations.push({ type: 'differing', description: `${theme} ${(share * 100).toFixed(2)} %` });
    expect(share).toBeLessThanOrEqual(0.02);
  });
}

test('the first visit follows the colour scheme and the toggle flips it', async ({ page }) => {
  await page.emulateMedia({ colorScheme: 'light' });
  await openApp(page);
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');
  await page.locator('#theme').click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
});
