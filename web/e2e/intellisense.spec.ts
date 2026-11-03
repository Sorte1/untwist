import type { Page } from '@playwright/test';
import { expect, openApp, paste, test } from './fixtures';

const INPUT = '#input-editor';
const OPTIONS = `${INPUT} .cm-tooltip-autocomplete li`;
const SIGNATURE = `${INPUT} .cm-lang-signature`;
const HOVER = `${INPUT} .cm-lang-hover`;

const SOURCE = `const box = { width: 1, height: 2 };
box.depth = 3;
const text = 'abc';
const greeting = "\\x68\\x69";
function add(first, second) {
  return first + second;
}
console.log(greeting, add(1, 2));
`;

async function pointAt(page: Page, needle: string, delta: number): Promise<{ x: number; y: number }> {
  const point = await page.evaluate(
    ({ needle, delta }) => {
      const content = document.querySelector('#input-editor .editor-host')?.shadowRoot?.querySelector('.cm-content');
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
    throw new Error(`no ${needle} in the input`);
  }
  return point;
}

async function ready(page: Page): Promise<void> {
  await openApp(page);
  await paste(page, SOURCE);
  await expect(page.locator('#input-lang')).toHaveText('Language features · ready');
}

async function typeAtEnd(page: Page, text: string): Promise<void> {
  await page.locator(`${INPUT} .cm-content`).click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.type(text);
}

async function hover(page: Page, needle: string, delta: number): Promise<void> {
  const point = await pointAt(page, needle, delta);
  await page.mouse.move(point.x, point.y - 30);
  await page.mouse.move(point.x, point.y);
}

test('a dot lists the properties seen on the object first', async ({ page }) => {
  await ready(page);
  await typeAtEnd(page, 'box.');
  const labels = page.locator(`${OPTIONS} .cm-completionLabel`);
  await expect(labels.first()).toBeVisible();
  const shown = await labels.allTextContents();
  expect(shown.slice(0, 3).sort()).toEqual(['depth', 'height', 'width']);
  expect(shown).toContain('hasOwnProperty');
});

test('a string constant offers the string methods with their signature and doc', async ({ page }) => {
  await ready(page);
  await typeAtEnd(page, 'text.sli');
  const option = page.locator(OPTIONS).filter({ hasText: 'slice' });
  await expect(option).toBeVisible();
  await expect(option.locator('.cm-completionDetail')).toHaveText('(start?, end?): string');
  await expect(page.locator(`${INPUT} .cm-completionInfo`)).toContainText('slice(start?, end?): string');
});

test('a built-in namespace lists its members, and a global comes with its doc', async ({ page }) => {
  await ready(page);
  await typeAtEnd(page, 'Math.flo');
  await expect(page.locator(OPTIONS).first()).toContainText('floor');
  await page.keyboard.press('Escape');
  await page.keyboard.press('Enter');
  await page.keyboard.type('parseI');
  await expect(page.locator(OPTIONS).first()).toContainText('parseInt');
  await expect(page.locator(`${INPUT} .cm-completionInfo`)).toContainText('Parses a string into an integer');
});

test('a snippet expands into a for…of loop', async ({ page }) => {
  await ready(page);
  await typeAtEnd(page, 'forof');
  await expect(page.locator(OPTIONS).first().locator('.cm-completionIcon-snippet')).toBeVisible();
  await page.waitForTimeout(150);
  await page.keyboard.press('Enter');
  await expect(page.locator(`${INPUT} .cm-content`)).toContainText('for (const name of collection) {');
});

test('parameter hints follow the argument under the cursor', async ({ page }) => {
  await ready(page);
  await typeAtEnd(page, 'add(');
  const hint = page.locator(SIGNATURE);
  await expect(hint).toContainText('add(first, second)');
  await expect(hint.locator('.cm-lang-active')).toHaveText('first');
  await page.keyboard.type('1, ');
  await expect(hint.locator('.cm-lang-active')).toHaveText('second');
  await page.keyboard.press('Escape');
  await expect(hint).toHaveCount(0);
});

test('parameter hints for a built-in show its doc', async ({ page }) => {
  await ready(page);
  await typeAtEnd(page, 'Math.max(');
  const hint = page.locator(SIGNATURE);
  await expect(hint).toContainText('Math.max(...values): number');
  await page.keyboard.type('1, 2, ');
  await expect(hint.locator('.cm-lang-active')).toHaveText('...values');
  await expect(hint).toContainText('The largest of the values.');
});

test('a hover on a constant shows its decoded value', async ({ page }) => {
  await ready(page);
  await hover(page, 'greeting, add', 2);
  const card = page.locator(HOVER);
  await expect(card).toContainText('const greeting');
  await expect(card.locator('.cm-lang-value')).toContainText('= "hi"');
  await expect(card.locator('.cm-lang-value')).toContainText('decoded');
});

test('a hover on an escaped string shows the text it decodes to', async ({ page }) => {
  await ready(page);
  await hover(page, '\\x68', 2);
  const card = page.locator(HOVER);
  await expect(card).toContainText('string');
  await expect(card.locator('.cm-lang-literal')).toHaveText('"hi"');
});

test('a hover on a function shows its parameters, and on a built-in its doc', async ({ page }) => {
  await ready(page);
  await hover(page, 'add(1, 2)', 1);
  await expect(page.locator(HOVER)).toContainText('function add(first, second)');
  await hover(page, 'log(greeting', 1);
  await expect(page.locator(HOVER)).toContainText('console.log(...data): void');
  await expect(page.locator(HOVER)).toContainText('Prints a message.');
});

test('problems show in the input as you type, and F8 jumps to the next one', async ({ page }) => {
  await ready(page);
  await expect(page.locator('#input-problems')).toHaveText('No problems');
  await typeAtEnd(page, 'let = ;');
  await expect(page.locator('#input-problems')).toHaveText('1 problem');
  await expect(page.locator(`${INPUT} .cm-lintRange-error`)).toHaveCount(1);
  await page.keyboard.press('ControlOrMeta+Home');
  await page.keyboard.press('F8');
  await page.keyboard.press('ControlOrMeta+C');
  const selected = await page.evaluate(() => navigator.clipboard.readText());
  expect(selected.length).toBeGreaterThan(0);
  expect('let = ;').toContain(selected);
});
