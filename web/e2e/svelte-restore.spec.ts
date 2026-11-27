import { expect, openApp, openDiagnostics, openSettings, paste, readRepo, runAndWait, test } from './fixtures';

const COUNTER = readRepo('crates/untwist-svelte/tests/fixtures/Counter.js');

test('a compiled Svelte component shows up restored in the Components tab, beside the unchanged code', async ({ page }) => {
  await openApp(page);
  await paste(page, COUNTER);
  await runAndWait(page);
  await expect(page.locator('#output-editor .cm-content')).toContainText('$.from_html');
  await openDiagnostics(page);
  const tab = page.locator('#components-tab');
  await expect(tab).toBeVisible();
  await expect(page.locator('#components-count')).toHaveText('1');
  await tab.click();
  const panel = page.locator('#components-panel');
  await expect(panel.locator('.comp-path')).toHaveText('Counter.svelte');
  await expect(panel.locator('.comp-code')).toContainText('let count = $state(0);');
  await expect(panel.locator('.comp-code')).toContainText('<button onclick={increment}>clicks: {count}</button>');
  await expect(page.locator('#download-label')).toHaveText('Download .zip');
});

test('with Restore Svelte off the tab stays hidden and the link keeps the choice', async ({ page }) => {
  await openApp(page);
  await openSettings(page);
  await page.locator('.switch[data-key="svelte"]').click();
  await expect(page).toHaveURL(/svelte=0/);
  await page.keyboard.press('Escape');
  await paste(page, COUNTER);
  await runAndWait(page);
  await expect(page.locator('#components-tab')).toBeHidden();
  await expect(page.locator('#download-label')).toHaveText('Download .js');
});
