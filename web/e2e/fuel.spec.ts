import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, test } from './fixtures';

function decoder(loops: number): string {
  const loop = (name: string, body: string) => `for (var ${name} = 0; ${name} < ${loops}; ${name}++) { ${body} }`;
  return [
    "function a() { var h = ['He', 'llo']; a = function () { return h; }; return a(); }",
    `function b(c, d) { var e = a(); b = function (f, g) { f = f - 0x0; var s = 0; ${loop('i', loop('j', loop('k', 's = s + 1;')))} return e[f]; }; return b(c, d); }`,
    'console.log(b(0x0) + b(0x1));',
  ].join('\n');
}

async function outputText(page: Page): Promise<string> {
  return page.evaluate(() => {
    const root = document.querySelector('#output-editor .editor-host')?.shadowRoot;
    return [...(root?.querySelectorAll('.cm-line') ?? [])].map((line) => line.textContent).join('\n');
  });
}

test('No limit really lifts the fuel: a decoder over 50M steps decodes only with it', async ({ page }) => {
  await openApp(page);
  await expect(page.locator('#fuel-inf')).not.toBeChecked();
  await paste(page, decoder(180));
  await runAndWait(page);
  expect(await outputText(page)).toContain('b(0) + b(1)');
  await expect(page.locator('#diag-list')).toContainText('step budget exhausted');
  await page.locator('#fuel-inf').check();
  expect(page.url()).toContain('fuel=unlimited');
  await runAndWait(page);
  expect(await outputText(page)).toContain('console.log("Hello");');
  await expect(page.locator('#diag-list')).not.toContainText('step budget exhausted');
});

test('Cancel still stops a long unlimited run promptly', async ({ page }) => {
  await openApp(page, '#fuel=unlimited');
  await paste(page, decoder(2000));
  await page.locator('#run').click();
  await expect(page.locator('#status-label')).toHaveText('Running');
  await page.waitForTimeout(1500);
  const asked = Date.now();
  await page.locator('#cancel').click();
  await expect(page.locator('#status-label')).toHaveText('Cancelled', { timeout: 2_000 });
  expect(Date.now() - asked).toBeLessThan(2_000);
  await expect(page.locator('#run')).toBeEnabled();
});

test('a limit no setting lifts is named in plain words, with why, in the chip and Details', async ({ page }) => {
  await openApp(page, '#fuel=unlimited');
  const depth = 100_000;
  await paste(page, `var x = ${'['.repeat(depth)}1${']'.repeat(depth)};`);
  await runAndWait(page);
  const chip = page.locator('#limits');
  await expect(chip).not.toHaveText('none fired');
  await expect(chip).not.toHaveText(/(^|, )(budget|refused|unpack-text|eval-cap|stack)(,|$)/);
  expect(await chip.getAttribute('title')).toMatch(/not affected by the fuel setting/);
  await page.locator('#details-tab').click();
  await expect(page.locator('#details')).toContainText('not affected by the fuel setting');
});
