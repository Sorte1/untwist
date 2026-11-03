import { execFileSync } from 'node:child_process';
import type { Page } from '@playwright/test';
import { expect, openApp, paste, repoPath, runAndWait, test } from './fixtures';

const INPUT = '#input-editor';
const OUTPUT = '#output-editor';
const FORMAT_FLAGS = ['--no-unpack', '--no-deobfuscate', '--no-jsx', '--rename', 'none'];

const NESTED = `function outer(list) {
  for (const item of list) {
    if (item) {
      console.log(item);
    }
  }
  return list;
}
outer([1]);
`;

function cliFormat(text: string): string {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  return execFileSync(bin, FORMAT_FLAGS, { input: text, encoding: 'utf8' });
}

async function editorText(page: Page, editor: string): Promise<string> {
  await page.locator(`${editor} .cm-content`).focus();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('ControlOrMeta+C');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function typeInto(page: Page, editor: string, text: string): Promise<void> {
  await page.locator(`${editor} .cm-content`).click();
  await page.keyboard.press('ControlOrMeta+A');
  await page.keyboard.press('Delete');
  await page.keyboard.type(text);
}

test('a fold marker hides a block in the output, and its placeholder brings it back', async ({ page }) => {
  await openApp(page);
  await paste(page, NESTED);
  await runAndWait(page);
  const lines = page.locator(`${OUTPUT} .cm-line`);
  const before = await lines.count();
  await page.locator(`${OUTPUT} .cm-gutters`).hover();
  await page.locator(`${OUTPUT} .cm-fold-marker[data-open="true"]`).first().click();
  await expect(page.locator(`${OUTPUT} .cm-foldPlaceholder`)).toHaveCount(1);
  expect(await lines.count()).toBeLessThan(before);
  await expect(page.locator(`${OUTPUT} .cm-fold-marker[data-open="false"]:visible`)).toHaveCount(1);
  await page.locator(`${OUTPUT} .cm-foldPlaceholder`).click();
  await expect(page.locator(`${OUTPUT} .cm-foldPlaceholder`)).toHaveCount(0);
  await expect(lines).toHaveCount(before);
});

test('the fold keys fold and unfold the block at the cursor in the input', async ({ page }) => {
  await openApp(page);
  await paste(page, NESTED);
  await page.locator(`${INPUT} .cm-content`).click();
  await page.keyboard.press('ControlOrMeta+Home');
  await page.keyboard.press('Control+Shift+BracketLeft');
  await expect(page.locator(`${INPUT} .cm-foldPlaceholder`)).toHaveCount(1);
  await page.keyboard.press('Control+Shift+BracketRight');
  await expect(page.locator(`${INPUT} .cm-foldPlaceholder`)).toHaveCount(0);
});

test('an open bracket or quote closes itself and the pair at the cursor is marked', async ({ page }) => {
  await openApp(page);
  await typeInto(page, INPUT, 'call(');
  await page.keyboard.type("'x");
  expect(await editorText(page, INPUT)).toBe("call('x')");
  await page.keyboard.press('End');
  await expect(page.locator(`${INPUT} .cm-matchingBracket`)).toHaveCount(2);
});

test('Alt+click adds a cursor, and typing writes at every cursor', async ({ page }) => {
  await openApp(page);
  await paste(page, 'one\ntwo\n');
  const lines = page.locator(`${INPUT} .cm-line`);
  await lines.nth(0).click({ position: { x: 200, y: 8 } });
  await lines.nth(1).click({ position: { x: 200, y: 8 }, modifiers: ['Alt'] });
  await expect(page.locator(`${INPUT} .cm-cursor`)).toHaveCount(2);
  await page.keyboard.type(';');
  expect(await editorText(page, INPUT)).toBe('one;\ntwo;\n');
});

test('Ctrl+D selects the next occurrence, so one edit renames both', async ({ page }) => {
  await openApp(page);
  await typeInto(page, INPUT, 'val + val');
  await page.keyboard.press('ControlOrMeta+Home');
  await page.keyboard.press('ControlOrMeta+D');
  await page.keyboard.press('ControlOrMeta+D');
  await page.keyboard.type('next');
  expect(await editorText(page, INPUT)).toBe('next + next');
});

test('indent guides mark each nesting level of both editors', async ({ page }) => {
  await openApp(page);
  await paste(page, NESTED);
  await runAndWait(page);
  for (const editor of [INPUT, OUTPUT]) {
    await expect(page.locator(`${editor} .cm-line.cm-indent-g1`).first()).toBeVisible();
    await expect(page.locator(`${editor} .cm-line.cm-indent-g3`)).toHaveCount(1);
    await expect(page.locator(`${editor} .cm-line.cm-indent-g3`)).toContainText('console.log');
  }
});

test('Ctrl+F opens find and replace in the output, and Replace all changes every match', async ({ page }) => {
  await openApp(page);
  await paste(page, NESTED);
  await runAndWait(page);
  await page.locator(`${OUTPUT} .cm-content`).click();
  await page.keyboard.press('ControlOrMeta+F');
  const panel = page.locator(`${OUTPUT} .cm-search`);
  await expect(panel).toBeVisible();
  await panel.locator('input[name="search"]').fill('item');
  await panel.locator('input[name="replace"]').fill('entry');
  await panel.locator('button[name="replaceAll"]').click();
  await expect(page.locator('#output-edit-mark')).toBeVisible();
  const text = await editorText(page, OUTPUT);
  expect(text).not.toContain('item');
  expect(text).toContain('for (const entry of list)');
});

test('Shift+Alt+F formats the input exactly like the CLI with only unminify, and undo restores it', async ({ page }) => {
  const source = 'function f(a,b){return a+b}var x=!0,y=f(1,2);';
  await openApp(page);
  await paste(page, source);
  await page.locator(`${INPUT} .cm-content`).click();
  await page.keyboard.press('Shift+Alt+F');
  await expect.poll(() => editorText(page, INPUT)).toBe(cliFormat(source));
  await expect(page.locator('#input-meta')).toContainText(`${cliFormat(source).split('\n').length} lines`);
  await page.keyboard.press('ControlOrMeta+Z');
  await expect.poll(() => editorText(page, INPUT)).toBe(source);
});

test('Shift+Alt+F formats an edited output too', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await page.locator(`${OUTPUT} .cm-content`).click();
  await page.keyboard.press('ControlOrMeta+End');
  await page.keyboard.type('if(a){a=1}');
  await page.keyboard.press('Shift+Alt+F');
  await expect.poll(() => editorText(page, OUTPUT)).toBe(cliFormat('var a = true;\nif(a){a=1}'));
});

test('code that does not parse is not formatted, and the note says why', async ({ page }) => {
  await openApp(page);
  await typeInto(page, INPUT, 'let = ;');
  await page.keyboard.press('Shift+Alt+F');
  await expect(page.locator(`${INPUT} .cm-notice`)).toContainText('Not formatted');
  expect(await editorText(page, INPUT)).toBe('let = ;');
});

test('Ctrl+Enter runs without adding a line to the input', async ({ page }) => {
  await openApp(page);
  await typeInto(page, INPUT, 'var a = !0;');
  await page.keyboard.press('Control+Enter');
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 30_000 });
  expect(await editorText(page, INPUT)).toBe('var a = !0;');
});

test('F1 lists the editor shortcuts, and Escape closes the list', async ({ page }) => {
  await openApp(page);
  await page.locator(`${INPUT} .cm-content`).click();
  await page.keyboard.press('F1');
  const sheet = page.locator(`${INPUT} .cm-shortcuts`);
  await expect(sheet).toBeVisible();
  await expect(sheet.getByRole('row', { name: /Format document/ })).toContainText('Shift Alt F');
  await expect(sheet.getByRole('row', { name: /^Add a cursor Alt/ })).toContainText('Alt click');
  await page.keyboard.press('Escape');
  await expect(sheet).toHaveCount(0);
});
