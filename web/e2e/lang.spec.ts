import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test, skipWithoutGame } from './fixtures';

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

const OUTPUT = '#output-editor .cm-content';
const STATUS = '#output-edit-meta span:last-child';

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function cursorAt(page: Page, needle: string, delta: number): Promise<void> {
  const point = await page.evaluate(
    ({ needle, delta }) => {
      const content = document.querySelector('#output-editor .editor-host')?.shadowRoot?.querySelector('.cm-content');
      const walker = document.createTreeWalker(content ?? document.body, NodeFilter.SHOW_TEXT);
      const nodes: Text[] = [];
      for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
        nodes.push(node as Text);
      }
      const whole = nodes.map((node) => node.data).join('');
      let target = whole.indexOf(needle) + delta;
      for (const node of nodes) {
        if (target < node.data.length) {
          const range = document.createRange();
          range.setStart(node, target);
          range.setEnd(node, target + 1);
          const rect = range.getBoundingClientRect();
          return { x: rect.left + 1, y: rect.top + rect.height / 2 };
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
  await page.mouse.click(point.x, point.y);
}

async function startEditing(page: Page, source: string): Promise<string> {
  await openApp(page);
  await paste(page, source);
  await runAndWait(page);
  const output = await outputText(page);
  await page.locator(OUTPUT).focus();
  await expect(page.locator(STATUS)).toHaveText('Language features · ready');
  return output;
}

async function watchLongTasks(page: Page): Promise<void> {
  await page.evaluate(() => {
    const store: number[] = [];
    (window as unknown as { untwistTasks: number[] }).untwistTasks = store;
    new PerformanceObserver((list) => list.getEntries().forEach((entry) => store.push(entry.duration))).observe({ type: 'longtask' });
  });
}

async function longestTask(page: Page): Promise<number> {
  return page.evaluate(() => Math.max(0, ...(window as unknown as { untwistTasks: number[] }).untwistTasks));
}


test('rename a local: the text changes at exactly its references', async ({ page }) => {
  const output = await startEditing(page, SOURCE);
  await expect(page.locator('#output-edit-mark')).toBeHidden();
  await expect(page.locator('#output-edit-meta')).toContainText('No problems');
  await watchLongTasks(page);
  await cursorAt(page, 'let sum', 4);
  await page.keyboard.press('Shift+F12');
  await expect(page.locator('#output-editor .cm-lang-reference')).toHaveCount(3);
  await page.keyboard.press('F2');
  const input = page.locator('#output-editor .cm-lang-rename input');
  await expect(input).toHaveValue('sum');
  await input.fill('subtotal');
  await input.press('Enter');
  const inside = output.slice(0, output.indexOf('\n}') + 2);
  const expected = inside.replace(/\bsum\b/g, 'subtotal') + output.slice(inside.length);
  await expect.poll(() => outputText(page)).toBe(expected);
  await expect(page.locator('#output-edit-meta')).toContainText('Edited · 3 lines');
  await page.keyboard.press('F12');
  expect(await longestTask(page)).toBeLessThanOrEqual(50);
  const analyzed = await page.evaluate(() => performance.getEntriesByName('untwist:lang-analyzed').map((m) => (m as PerformanceMark).detail as { ms: number; memoryBytes: number }).at(-1));
  test.info().annotations.push({ type: 'lang worker', description: `analyze ${analyzed?.ms.toFixed(1)} ms, linear memory ${analyzed?.memoryBytes} B` });
});

test('a refused rename says why and changes nothing', async ({ page }) => {
  const output = await startEditing(page, SOURCE);
  await cursorAt(page, 'let sum', 4);
  await page.keyboard.press('F2');
  const input = page.locator('#output-editor .cm-lang-rename input');
  await input.fill('items');
  await input.press('Enter');
  await expect(page.locator('#output-editor .cm-lang-refused')).toContainText('Not renamed');
  await page.keyboard.press('Escape');
  expect(await outputText(page)).toBe(output);
});

test('a typo shows one problem, and reset restores the output', async ({ page }) => {
  const output = await startEditing(page, SOURCE);
  await page.locator(OUTPUT).click();
  await page.keyboard.press('Control+End');
  await page.keyboard.type('let broken = ;');
  await expect(page.locator('#output-problems')).toHaveText('1 problem');
  await expect(page.locator('#output-editor .cm-lintRange-error')).toHaveCount(1);
  await page.getByRole('button', { name: 'Reset to output' }).click();
  await expect(page.locator('#output-edit-mark')).toBeHidden();
  await expect(page.locator('#output-edit-actions')).toBeHidden();
  expect(await outputText(page)).toBe(output);
});

test('completion lists the scope binding first', async ({ page }) => {
  await startEditing(page, SOURCE);
  await cursorAt(page, 'let sum = 0;', 0);
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.keyboard.type('s');
  const options = page.locator('#output-editor .cm-tooltip-autocomplete li');
  await expect(options.first()).toContainText('sum');
  await expect(options.first()).toContainText('let');
});

test('completion keeps the scope bindings while the line is half typed', async ({ page }) => {
  await startEditing(page, SOURCE);
  await cursorAt(page, 'let sum = 0;', 0);
  await page.keyboard.press('End');
  await page.keyboard.press('Enter');
  await page.keyboard.type('if (');
  await page.keyboard.press('Delete');
  await page.keyboard.type('s');
  await expect(page.locator('#output-problems')).not.toHaveText('No problems');
  await page.keyboard.press('Control+Space');
  await expect(page.locator('#output-editor .cm-tooltip-autocomplete li').first()).toContainText('sum');
});

test('go to definition jumps from a reference to its declaration', async ({ page }) => {
  const output = await startEditing(page, SOURCE);
  await cursorAt(page, 'log(sum)', 5);
  await page.keyboard.press('F12');
  await expect(page.locator('#output-editor .cm-selectionBackground')).toHaveCount(1);
  await page.keyboard.type('X');
  await page.keyboard.press('Escape');
  const declared = output.indexOf('var sum') + 4;
  const edited = await outputText(page);
  expect(edited.indexOf('X')).toBeGreaterThanOrEqual(declared);
  expect(edited.indexOf('X')).toBeLessThanOrEqual(declared + 3);
});

test("game.js's output is under the 12 MiB cap: the features are on and references answer", async ({ page }) => {
  skipWithoutGame();
  test.setTimeout(180_000);
  await openApp(page);
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await runAndWait(page);
  await page.locator(OUTPUT).focus();
  await expect(page.locator(STATUS)).toHaveText('Language features · ready', { timeout: 60_000 });
  const marks = await page.evaluate(() =>
    performance.getEntriesByName('untwist:lang-analyzed').map((mark) => (mark as PerformanceMark).detail as { ms: number; memoryBytes: number }),
  );
  const last = marks.at(-1);
  test.info().annotations.push({ type: 'game.js analyze', description: `${last?.ms.toFixed(0)} ms, ${((last?.memoryBytes ?? 0) / 1048576).toFixed(1)} MiB` });
  if (test.info().project.name === 'chromium') {
    expect(last?.ms ?? Infinity).toBeLessThanOrEqual(650);
  }
  expect(last?.ms ?? Infinity).toBeLessThanOrEqual(1000);
  await cursorAt(page, 'var ', 5);
  await page.keyboard.press('Shift+F12');
  await page.waitForFunction(
    () => (document.querySelector('#output-editor .editor-host')?.shadowRoot?.querySelectorAll('.cm-lang-reference').length ?? 0) > 0,
    undefined,
    { timeout: 10_000 },
  );
});

test('go to definition follows a required module to its export', async ({ page }) => {
  await openApp(page);
  await paste(page, readRepo('fixtures/unpack-bundlers/webpack/webpack5-prod.js'));
  await runAndWait(page);
  await page.locator('#module-list .tree-item', { hasText: 'index.js' }).first().click();
  await expect(page.locator('#output-meta')).toContainText('index.js');
  await page.locator(OUTPUT).focus();
  await expect(page.locator(STATUS)).toHaveText('Language features · ready');
  await cursorAt(page, '_0__.c)', 5);
  await page.keyboard.press('F12');
  await expect(page.locator('#output-meta')).toContainText('802.js');
  await page.keyboard.type('Z');
  await expect.poll(() => outputText(page)).toContain('function Z(line)');
});
