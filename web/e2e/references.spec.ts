import type { Page } from '@playwright/test';
import { expect, openApp, paste, runAndWait, test } from './fixtures';

const SOURCE = `function total(items) {
  let sum = 0;
  for (const item of items) {
    sum += item.price;
  }
  return sum;
}
var sum = total([{ price: 1 }]);
console.log(sum);
`;

const OUTPUT = '#output-editor';
const PANEL = `${OUTPUT} .cm-lang-refs`;
const errors: string[] = [];

test.beforeEach(({ page }) => {
  errors.length = 0;
  page.on('console', (message) => {
    if (message.type() === 'error') {
      errors.push(message.text());
    }
  });
});

test.afterEach(() => {
  expect(errors).toEqual([]);
});

async function pointAt(page: Page, needle: string, delta: number): Promise<{ x: number; y: number }> {
  const point = await page.evaluate(
    ({ needle, delta }) => {
      const content = document.querySelector('#output-editor .editor-host')?.shadowRoot?.querySelector('.cm-content');
      const walker = document.createTreeWalker(content ?? document.body, NodeFilter.SHOW_TEXT);
      const nodes: Text[] = [];
      for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
        nodes.push(node as Text);
      }
      let target = nodes.map((node) => node.data).join('').indexOf(needle) + delta;
      for (const node of nodes) {
        if (target < node.data.length) {
          const range = document.createRange();
          range.setStart(node, target);
          range.setEnd(node, target + 1);
          const rect = range.getBoundingClientRect();
          return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
        }
        target -= node.data.length;
      }
      return null;
    },
    { needle, delta },
  );
  if (point === null) {
    throw new Error(`no ${needle} in the output`);
  }
  return point;
}

async function clickAt(page: Page, needle: string, delta: number): Promise<void> {
  const point = await pointAt(page, needle, delta);
  await page.mouse.click(point.x, point.y);
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function start(page: Page): Promise<void> {
  await openApp(page);
  await paste(page, SOURCE);
  await runAndWait(page);
  await page.locator(`${OUTPUT} .cm-content`).focus();
  await expect(page.locator('#output-lang')).toHaveText('Language features · ready');
}

test('Shift+F12 lists the references in a panel, and a row jumps to its line', async ({ page }) => {
  await start(page);
  await clickAt(page, 'let sum', 5);
  await page.keyboard.press('Shift+F12');
  const panel = page.locator(PANEL);
  await expect(panel).toBeVisible();
  await expect(panel.locator('.cm-lang-refs-head')).toContainText('3 references to sum');
  const rows = panel.getByRole('option');
  await expect(rows).toHaveCount(3);
  await expect(rows.nth(0)).toContainText('let sum = 0;');
  await expect(rows.nth(0)).toContainText('2');
  await rows.nth(2).click();
  await expect(rows.nth(2)).toHaveAttribute('aria-selected', 'true');
  await page.keyboard.type('X');
  expect(await outputText(page)).toContain('return X;');
});

test('the panel steps with F4 and Shift+F4, and Escape closes it with its marks', async ({ page }) => {
  await start(page);
  await clickAt(page, 'let sum', 5);
  await page.keyboard.press('Shift+F12');
  await expect(page.locator(PANEL).getByRole('option')).toHaveCount(3);
  const current = page.locator(`${PANEL} [aria-selected="true"]`);
  await expect(current).toContainText('let sum = 0;');
  await page.keyboard.press('F4');
  await expect(current).toContainText('sum += item.price;');
  await page.keyboard.press('F4');
  await page.keyboard.press('F4');
  await expect(current).toContainText('let sum = 0;');
  await page.keyboard.press('Shift+F4');
  await expect(current).toContainText('return sum;');
  await page.keyboard.press('Escape');
  await expect(page.locator(PANEL)).toHaveCount(0);
  await expect(page.locator(`${OUTPUT} .cm-lang-reference`)).toHaveCount(0);
});

test('F12 on a declaration lists its references instead of staying put', async ({ page }) => {
  await start(page);
  await clickAt(page, 'let sum', 5);
  await page.keyboard.press('F12');
  await expect(page.locator(`${PANEL} .cm-lang-refs-head`)).toContainText('3 references to sum');
});

test('rename shows the edits as you type, and a refused name says why before Enter', async ({ page }) => {
  await start(page);
  await clickAt(page, 'let sum', 5);
  await page.keyboard.press('F2');
  const input = page.locator(`${OUTPUT} .cm-lang-rename input`);
  await input.fill('subtotal');
  const preview = page.locator(`${OUTPUT} .cm-lang-rename-preview`);
  await expect(page.locator(`${OUTPUT} .cm-lang-rename .cm-lang-note`)).toHaveText('3 references on 3 lines · Enter to rename');
  await expect(preview.locator('.cm-lang-rename-line')).toHaveCount(3);
  await expect(preview.locator('.cm-lang-rename-line').first()).toContainText('let subtotal = 0;');
  await input.fill('items');
  await expect(page.locator(`${OUTPUT} .cm-lang-refused`)).toContainText('Not renamed');
  await expect(preview.locator('.cm-lang-rename-line')).toHaveCount(0);
  await input.fill('subtotal');
  await input.press('Enter');
  await expect.poll(() => outputText(page)).toContain('let subtotal = 0;');
});

test('the symbol under the cursor marks its other occurrences, and an edit clears them', async ({ page }) => {
  await start(page);
  await clickAt(page, 'let sum', 5);
  const marks = page.locator(`${OUTPUT} .cm-lang-occurrence`);
  await expect(marks).toHaveCount(3);
  await clickAt(page, 'var sum', 5);
  await expect(marks).toHaveCount(2);
  await clickAt(page, 'console', 2);
  await expect(marks).toHaveCount(0);
  await clickAt(page, 'var sum', 5);
  await expect(marks).toHaveCount(2);
  await page.keyboard.type('m');
  await expect(marks).toHaveCount(0);
});

test('Ctrl+hover underlines a symbol that has a definition', async ({ page }) => {
  await start(page);
  const point = await pointAt(page, 'log(sum)', 5);
  await page.keyboard.down('Control');
  await page.mouse.move(point.x, point.y - 20);
  await page.mouse.move(point.x, point.y);
  await expect(page.locator(`${OUTPUT} .cm-lang-link`)).toHaveText('sum');
  await page.keyboard.up('Control');
  await expect(page.locator(`${OUTPUT} .cm-lang-link`)).toHaveCount(0);
});
