import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, test } from './fixtures';

interface Focused {
  readonly id: string;
  readonly tag: string;
  readonly inEditor: string | null;
  readonly outline: string;
}

async function focused(page: Page): Promise<Focused> {
  return page.evaluate(() => {
    let element = document.activeElement;
    let inEditor: string | null = null;
    while (element?.shadowRoot?.activeElement) {
      inEditor = element.closest('.pane-body')?.id ?? 'editor';
      element = element.shadowRoot.activeElement;
    }
    const target = element as HTMLElement | null;
    const style = target === null ? null : getComputedStyle(target);
    const host = inEditor === null ? null : document.activeElement;
    const ring = inEditor === null ? style : host === null ? null : getComputedStyle(host);
    return {
      id: target?.id || target?.getAttribute('data-key') || target?.getAttribute('data-mode') || target?.className || '',
      tag: target?.tagName.toLowerCase() ?? '',
      inEditor,
      outline: ring === null ? 'none' : `${ring.outlineStyle} ${ring.outlineWidth}`,
    };
  });
}

async function tabCycle(page: Page, presses: number): Promise<Focused[]> {
  const seen: Focused[] = [];
  for (let index = 0; index < presses; index += 1) {
    await page.keyboard.press('Tab');
    seen.push(await focused(page));
  }
  return seen;
}


test('every control has an accessible name', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  const unnamed = await page.evaluate(() =>
    [...document.querySelectorAll<HTMLElement>('button, input, select, textarea, [role="switch"], a[href]')]
      .filter((element) => element.offsetParent !== null)
      .filter((element) => {
        const labelled = element.getAttribute('aria-label') ?? '';
        const text = element.textContent?.trim() ?? '';
        const id = element.id;
        const forLabel = id === '' ? '' : document.querySelector(`label[for="${id}"]`)?.textContent?.trim() ?? '';
        const wrapped = element.closest('label')?.textContent?.trim() ?? '';
        return `${labelled}${text}${forLabel}${wrapped}` === '';
      })
      .map((element) => element.outerHTML.slice(0, 120)),
  );
  expect(unnamed).toEqual([]);
});

test('the keyboard reaches every control with a visible focus ring, and leaves the editors', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await page.locator('body').click({ position: { x: 5, y: 5 } });
  const main = await tabCycle(page, 40);
  const ids = new Set(main.map((item) => item.id));
  for (const id of ['examples', 'safe', 'settings-toggle', 'theme', 'run', 'clear-input', 'view-diff', 'copy-output', 'download', 'diag-toggle']) {
    expect(ids, id).toContain(id);
  }
  expect(main.some((item) => item.inEditor === 'input-editor')).toBe(true);
  expect(main.some((item) => item.inEditor === 'output-editor')).toBe(true);
  const afterInput = main.findIndex((item) => item.inEditor === 'input-editor');
  expect(main[afterInput + 1]?.inEditor ?? null).not.toBe('input-editor');
  await page.locator('#settings-toggle').focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#settings')).toBeVisible();
  const settings = await tabCycle(page, 26);
  const settingIds = new Set(settings.map((item) => item.id));
  for (const id of ['settings-close', 'unpack', 'jsx', 'none', 'mangle', 'fuel', 'fuel-inf', 'copy-link']) {
    expect(settingIds, id).toContain(id);
  }
  await page.keyboard.press('Escape');
  await expect(page.locator('#settings')).toBeHidden();
  await page.locator('#diag-toggle').focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#diagnostics')).toBeVisible();
  const panel = await tabCycle(page, 4);
  expect(panel.map((item) => item.id)).toContain('details-tab');
  const unringed = [...main, ...settings, ...panel].filter((item) => item.tag !== 'body' && !/^(solid|auto) [1-9]/.test(item.outline));
  expect(unringed).toEqual([]);
});

test('a keyboard-only run: tab into the input, type, Ctrl+Enter, copy', async ({ page }) => {
  await openApp(page);
  await page.locator('body').click({ position: { x: 5, y: 5 } });
  for (let index = 0; index < 80 && (await focused(page)).inEditor !== 'input-editor'; index += 1) {
    await page.keyboard.press('Tab');
  }
  await page.keyboard.type('var a = !0;');
  await page.keyboard.press('Control+Enter');
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 30_000 });
  await page.locator('#copy-output').focus();
  await page.keyboard.press('Enter');
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('var a = true;\n');
});

test('at phone width the panes stack and nothing scrolls sideways', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  const layout = await page.evaluate(() => {
    const input = document.getElementById('input-pane')?.getBoundingClientRect();
    const output = document.getElementById('output-pane')?.getBoundingClientRect();
    const overflowing = [...document.querySelectorAll<HTMLElement>('button')]
      .filter((button) => button.offsetParent !== null)
      .filter((button) => button.getBoundingClientRect().right > innerWidth + 0.5)
      .map((button) => button.id || button.textContent);
    return { scroll: document.documentElement.scrollWidth, width: innerWidth, stacked: (output?.top ?? 0) >= (input?.bottom ?? 0), overflowing };
  });
  expect(layout.scroll).toBeLessThanOrEqual(layout.width);
  expect(layout.stacked).toBe(true);
  expect(layout.overflowing).toEqual([]);
});
