import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, test } from './fixtures';

const REFUSAL_FIXTURE = 'fixtures/behaviour/06-rotator-in-uncalled-function.js';
const REFUSAL_PASS = 'inline-decoded-strings';

function location(text: string, offset: number): string {
  const before = text.slice(0, offset);
  const line = before.split('\n').length;
  const column = [...before.slice(before.lastIndexOf('\n') + 1)].length + 1;
  return `input ${line}:${column}`;
}

async function inputText(page: Page): Promise<string> {
  return (await page.locator('#input-editor .cm-content').textContent()) ?? '';
}

async function runWith(page: Page, text: string): Promise<void> {
  await openApp(page);
  await paste(page, text);
  await runAndWait(page);
}

test('a parse error is an error row at its input location, and the click selects it', async ({ page }) => {
  const source = 'var a = 1;\nvar b = ;\n';
  await runWith(page, source);
  const row = page.locator('#diag-list .diag-row[data-level="error"]').first();
  await expect(row.locator('.diag-src')).toHaveText('parser');
  await expect(row.locator('.diag-loc')).toHaveText(location(source, source.indexOf(';', 12)));
  await expect(page.locator('#warn-count')).not.toHaveText('0');
  await row.click();
  await page.keyboard.type('Z');
  expect(await inputText(page)).toBe('var a = 1;var b = Z');
});

test('a refusal note shows its pass, message and location, and the click selects its span', async ({ page }) => {
  const source = readRepo(REFUSAL_FIXTURE);
  await runWith(page, source);
  const row = page.locator('#diag-list .diag-row', { has: page.locator(`.diag-src:text-is("${REFUSAL_PASS}")`) }).first();
  await expect(row).toHaveAttribute('data-level', 'warning');
  await expect(row.locator('.diag-text')).toContainText('rotator is not a statement of the program body');
  const loc = (await row.locator('.diag-loc').textContent()) ?? '';
  const from = [...source].findIndex((_, at) => location(source, at) === loc);
  expect(from).toBeGreaterThanOrEqual(0);
  await row.click();
  await page.keyboard.press('Delete');
  const edited = await inputText(page);
  const flat = (text: string) => text.replace(/\n/g, '');
  const ends = Array.from({ length: source.length - from }, (_, step) => from + step + 1);
  const to = ends.find((end) => flat(source.slice(0, from) + source.slice(end)) === edited);
  expect(to, 'the deleted range starts at the shown location').toBeDefined();
  expect((to ?? from) - from).toBeGreaterThan(1);
});

test('a row of an input edited after the run shows its location but cannot be clicked', async ({ page }) => {
  await runWith(page, 'var a = 1;\nvar b = ;\n');
  await expect(page.locator('#diag-list button.diag-row').first()).toBeVisible();
  await page.locator('#input-editor .cm-content').click();
  await page.keyboard.press('End');
  await page.keyboard.type(' ');
  await expect(page.locator('#diag-list button.diag-row')).toHaveCount(0);
  await expect(page.locator('#diag-list .diag-row .diag-loc').first()).toHaveText(/^input 2:9$/);
});
