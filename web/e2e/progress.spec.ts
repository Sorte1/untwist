import type { Page } from '@playwright/test';
import { expect, openApp, readRepo, runAndWait, skipWithoutGame, test } from './fixtures';

const OBFUSCATED = 'fixtures/reference/deobfuscate/obfuscator.io.js';

async function recordStages(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const seen: string[] = [];
    const steps: string[] = [];
    Object.assign(window, { untwistSeenStages: seen, untwistSeenSteps: steps });
    document.addEventListener('DOMContentLoaded', () => {
      const line = document.getElementById('running-stage');
      const list = document.getElementById('run-steps');
      if (line === null || list === null) {
        return;
      }
      new MutationObserver(() => {
        if (seen.at(-1) !== line.textContent) {
          seen.push(line.textContent ?? '');
        }
      }).observe(line, { childList: true, characterData: true, subtree: true });
      new MutationObserver(() => {
        steps.push([...list.children].map((item) => `${item.firstChild?.textContent}:${item.getAttribute('data-state')}`).join(' '));
      }).observe(list, { childList: true });
    });
  });
}

async function seen(page: Page): Promise<{ stages: string[]; steps: string[] }> {
  return page.evaluate(() => {
    const record = window as unknown as { untwistSeenStages: string[]; untwistSeenSteps: string[] };
    return { stages: [...record.untwistSeenStages], steps: [...record.untwistSeenSteps] };
  });
}

async function pasteFile(page: Page, text: string): Promise<void> {
  await page.locator('#open-file-input').setInputFiles({ name: 'input.js', mimeType: 'text/javascript', buffer: Buffer.from(text) });
}

test('a run names each stage as the worker reaches it', async ({ page }) => {
  await recordStages(page);
  await openApp(page, '#unsafe=1');
  await pasteFile(page, readRepo(OBFUSCATED));
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 60_000 });
  const { stages } = await seen(page);
  const first = (name: string) => stages.indexOf(`Stage: ${name}`);
  for (const name of ['prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'unpack']) {
    expect(first(name), `${name} in ${stages.join(' | ')}`).toBeGreaterThanOrEqual(0);
  }
  expect(stages[0]).toBe('Starting the engine…');
  expect(first('prepare')).toBeLessThan(first('deobfuscate'));
  expect(first('deobfuscate')).toBeLessThan(first('unminify'));
  expect(first('cleanup')).toBeLessThan(first('unpack'));
  expect(stages).toContain('Stage: deobfuscate · round 2');
  await expect(page.locator('#running')).toBeHidden();
});

test('the steps follow the settings of the run and end with every phase reached', async ({ page }) => {
  await recordStages(page);
  await openApp(page, '#unpack=0&rename=unique');
  await pasteFile(page, readRepo(OBFUSCATED));
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 60_000 });
  const { steps } = await seen(page);
  expect(steps[0]).toBe('Prepare:pending Deobfuscate:pending Unminify:pending JSX:pending Clean up:pending Rename:pending');
  expect(steps.at(-1)).toBe('Prepare:done Deobfuscate:done Unminify:done JSX:done Clean up:done Rename:current');
  await expect(page.locator('#run-steps [aria-current="step"]')).toHaveText('Rename');
  await expect(page.locator('#run-steps .run-step[data-state="done"]').first()).toHaveText('Prepare, done');
});

test('a long run shows its stage while it goes', async ({ page }) => {
  skipWithoutGame();
  await openApp(page);
  await pasteFile(page, readRepo('fixtures/real-world/game.js'));
  await expect(page.locator('#running')).toBeVisible();
  await expect(page.locator('#running-stage')).toHaveText(/^Stage: /, { timeout: 30_000 });
  await expect(page.locator('#run-steps [aria-current="step"]')).toHaveCount(1);
  await page.locator('#cancel').click();
  await expect(page.locator('#status-label')).toHaveText('Cancelled');
});

test('a second run starts its steps from the beginning', async ({ page }) => {
  await recordStages(page);
  await openApp(page);
  await pasteFile(page, 'var a = !0;');
  await expect(page.locator('#status-label')).toHaveText('Done');
  await runAndWait(page);
  const { stages } = await seen(page);
  expect(stages.filter((line) => line === 'Starting the engine…')).toHaveLength(2);
});
