import type { Page } from '@playwright/test';
import { expect, openApp, openDiagnostics, readRepo, runAndWait, skipWithoutGame, test } from './fixtures';

const OBFUSCATED = 'fixtures/reference/deobfuscate/obfuscator.io.js';
const SCREENSHOTS = process.env['UNTWIST_PROGRESS_SHOTS'];

async function recordStages(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const seen: string[] = [];
    const steps: string[] = [];
    const times: string[] = [];
    Object.assign(window, { untwistSeenStages: seen, untwistSeenSteps: steps, untwistSeenTimes: times });
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
        const items = [...list.children] as HTMLElement[];
        steps.push(items.map((item) => `${item.dataset['phase']}:${item.dataset['state']}`).join(' '));
        times.push(items.map((item) => item.querySelector('.run-step-ms')?.textContent ?? '').join('|'));
      }).observe(list, { childList: true, subtree: true, characterData: true });
    });
  });
}

async function seen(page: Page): Promise<{ stages: string[]; steps: string[]; times: string[] }> {
  return page.evaluate(() => {
    const record = window as unknown as { untwistSeenStages: string[]; untwistSeenSteps: string[]; untwistSeenTimes: string[] };
    return { stages: [...record.untwistSeenStages], steps: [...record.untwistSeenSteps], times: [...record.untwistSeenTimes] };
  });
}

async function pasteFile(page: Page, text: string): Promise<void> {
  await page.locator('#open-file-input').setInputFiles({ name: 'input.js', mimeType: 'text/javascript', buffer: Buffer.from(text) });
}

async function timeline(page: Page): Promise<Record<string, string>> {
  await openDiagnostics(page);
  if ((await page.locator('#details-tab').getAttribute('aria-expanded')) !== 'true') {
    await page.locator('#details-tab').click();
  }
  const section = page.locator('#details .details-section').filter({ has: page.locator('h3', { hasText: /^Run timeline$/ }) });
  await expect(section).toBeVisible();
  const labels = await section.locator('dt').allTextContents();
  const values = await section.locator('dd').allTextContents();
  return Object.fromEntries(labels.map((label, at) => [label, values[at] ?? '']));
}

test('a run names the stages as the worker reaches them, in order, and times every phase', async ({ page }) => {
  await recordStages(page);
  await openApp(page, '#unsafe=1');
  await pasteFile(page, readRepo(OBFUSCATED));
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 60_000 });
  const { stages } = await seen(page);
  expect(stages[0]).toBe('Starting the engine…');
  const order = ['parse', 'prepare', 'deobfuscate', 'unminify', 'jsx', 'cleanup', 'print', 'unpack'];
  const shown = order.map((name) => stages.findIndex((line) => line.startsWith(`Stage: ${name}`))).filter((at) => at !== -1);
  expect([...shown].sort((a, b) => a - b), stages.join(' | ')).toEqual(shown);
  expect(stages.at(-1)).toBe('Loading the output into the editor…');
  expect(stages.some((line) => line.startsWith('Stage: library'))).toBe(false);
  await expect(page.locator('#running')).toBeHidden();
  const rows = await timeline(page);
  for (const label of ['Start worker', 'Parse', 'Prepare', 'Deobfuscate', 'Unminify', 'JSX', 'Clean up', 'Print', 'Unpack', 'Transfer output', 'Editor']) {
    expect(rows[label], `${label} in ${JSON.stringify(rows)}`).toMatch(/^\d+ ms$|^\d+\.\d\d s$/);
  }
});

test('the steps follow the settings of the run, carry their time, and end with every phase reached', async ({ page }) => {
  await recordStages(page);
  await openApp(page, '#unpack=0&rename=unique');
  await pasteFile(page, readRepo(OBFUSCATED));
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 60_000 });
  const { steps, times } = await seen(page);
  const phases = 'start parse prepare deobfuscate unminify jsx cleanup rename print transfer editor'.split(' ');
  expect(steps[0]).toBe(phases.map((phase) => `${phase}:pending`).join(' '));
  const last = steps.at(-1)?.split(' ') ?? [];
  expect(last.map((step) => step.split(':')[0])).toEqual([...phases.slice(0, -3), 'locate', ...phases.slice(-3)]);
  expect(last.at(-1)).toBe('editor:current');
  expect(last.slice(0, -1).every((step) => step.endsWith(':done'))).toBe(true);
  expect(times.at(-1)?.split('|').slice(0, -1).every((time) => /^\d+ ms$|^\d+\.\d\d s$/.test(time))).toBe(true);
  const rows = await timeline(page);
  for (const label of ['Start worker', 'Parse', 'Deobfuscate', 'Rename', 'Print', 'Transfer output', 'Editor']) {
    expect(rows[label], `${label} in ${JSON.stringify(rows)}`).toMatch(/^\d+ ms$|^\d+\.\d\d s$/);
  }
  expect(Object.keys(rows)).not.toContain('Library tables');
  expect(await page.locator('#elapsed').getAttribute('title')).toContain('Parse: ');
});

test('a long run shows its stage, the time in it and an open-ended bar while it goes, and cancel stops it', async ({ page }) => {
  skipWithoutGame();
  await openApp(page);
  await pasteFile(page, readRepo('fixtures/real-world/game.js'));
  await expect(page.locator('#running')).toBeVisible();
  await expect(page.locator('#running-stage')).toHaveText(/^Stage: /, { timeout: 30_000 });
  await expect(page.locator('#run-steps [aria-current="step"]')).toHaveCount(1);
  await expect(page.locator('#run-steps [aria-current="step"] .run-step-ms')).toHaveText(/^\d+ ms$|^\d+\.\d\d s$/);
  await expect(page.locator('#status-stage')).toHaveText(/ · (\d+ ms|\d+\.\d\d s)$/);
  const bar = page.locator('#running .run-bar');
  await expect(bar).toBeVisible();
  expect(await bar.getAttribute('aria-valuenow')).toBeNull();
  if (SCREENSHOTS !== undefined) {
    await page.waitForTimeout(1500);
    await page.screenshot({ path: `${SCREENSHOTS}/progress-${test.info().project.name}.png`, caret: 'initial' });
  }
  const asked = Date.now();
  await page.locator('#cancel').click();
  await expect(page.locator('#status-label')).toHaveText('Cancelled', { timeout: 2_000 });
  expect(Date.now() - asked).toBeLessThan(2_000);
  await expect(page.locator('#status-stage')).toHaveText('');
});

test('a second run starts its steps from the beginning', async ({ page }) => {
  await recordStages(page);
  await openApp(page);
  await pasteFile(page, 'var a = !0;');
  await expect(page.locator('#status-label')).toHaveText('Done');
  await runAndWait(page);
  const { stages, steps } = await seen(page);
  expect(stages.filter((line) => line === 'Starting the engine…')).toHaveLength(2);
  expect(steps.filter((line) => line.split(' ').every((step) => step.endsWith(':pending')))).toHaveLength(2);
});
