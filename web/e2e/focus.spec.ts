import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, test } from './fixtures';

const INPUT = '#input-editor';
const OUTPUT = '#output-editor';

async function paneRing(page: Page, pane: string): Promise<string> {
  return page.locator(pane).evaluate((element) => getComputedStyle(element).boxShadow);
}

async function editorStyle(page: Page, editor: string, selector: string, property: string): Promise<string> {
  return page.locator(editor).evaluate(
    (element, [inner, prop]) => {
      const root = element.querySelector('.editor-host')?.shadowRoot;
      const node = root?.querySelector<HTMLElement>(inner as string);
      return node ? getComputedStyle(node).getPropertyValue(prop as string) : 'missing';
    },
    [selector, property],
  );
}

async function editorText(page: Page, editor: string): Promise<string> {
  return page.locator(editor).evaluate((element) => element.querySelector('.editor-host')?.shadowRoot?.querySelector('.cm-content')?.textContent ?? '');
}

test('the focused editor marks its pane, and the mark moves with focus', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;\nvar b = 1;');
  await runAndWait(page);
  await page.locator(`${INPUT} .cm-content`).click();
  await expect.poll(() => paneRing(page, '#input-pane')).not.toBe('none');
  await expect.poll(() => paneRing(page, '#output-pane')).toBe('none');
  await page.locator(`${OUTPUT} .cm-content`).click();
  await expect.poll(() => paneRing(page, '#output-pane')).not.toBe('none');
  await expect.poll(() => paneRing(page, '#input-pane')).toBe('none');
});

test('the caret is a 2 px bar and only the focused editor highlights its current line', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;\nvar b = 1;');
  await runAndWait(page);
  await page.locator(`${INPUT} .cm-content`).click();
  expect(await editorStyle(page, INPUT, '.cm-cursor-primary', 'border-left-width')).toBe('2px');
  expect(await editorStyle(page, INPUT, '.cm-activeLine', 'background-color')).not.toBe('rgba(0, 0, 0, 0)');
  expect(await editorStyle(page, OUTPUT, '.cm-activeLine', 'background-color')).toBe('rgba(0, 0, 0, 0)');
});

test('Ctrl+A and Delete in the input clear the input and leave the output', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;\nvar b = 1;');
  await runAndWait(page);
  const output = await editorText(page, OUTPUT);
  await page.locator(`${INPUT} .cm-content`).click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('Delete');
  await expect.poll(() => editorText(page, INPUT)).toBe('');
  expect(await editorText(page, OUTPUT)).toBe(output);
});
