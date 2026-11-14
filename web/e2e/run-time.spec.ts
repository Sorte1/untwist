import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, runAndWait, skipWithoutGame, test } from './fixtures';

const OBFUSCATED = 'fixtures/reference/deobfuscate/obfuscator.io.js';
const COPIES = 400;
const ROUNDS = 3;
const RATIO = 1.3;
const SLACK_MS = 300;

interface Timed {
  readonly wallMs: number;
  readonly engineMs: number;
  readonly runs: number;
}

function bigInput(): string {
  const one = readRepo(OBFUSCATED);
  return Array.from({ length: COPIES }, () => `(function () {\n${one}\n})();\n`).join('');
}

async function watchRuns(page: Page): Promise<void> {
  await page.addInitScript(() => {
    const record = { posts: 0, engineMs: [] as number[] };
    Object.assign(window, { untwistRuns: record });
    const Original = window.Worker;
    window.Worker = class extends Original {
      constructor(url: string | URL, options?: WorkerOptions) {
        super(url, options);
        if (!String(url).includes('run.worker')) {
          return;
        }
        this.addEventListener('message', (event: MessageEvent<{ kind?: string; result?: { engineMs: number } }>) => {
          if (event.data.kind === 'done' && event.data.result !== undefined) {
            record.engineMs.push(event.data.result.engineMs);
          }
        });
        const post = this.postMessage.bind(this) as (message: unknown, transfer?: Transferable[]) => void;
        this.postMessage = ((message: { kind?: string }, transfer?: Transferable[]) => {
          if (message.kind === 'run') {
            record.posts += 1;
          }
          post(message, transfer);
        }) as Worker['postMessage'];
      }
    };
  });
}

async function runs(page: Page): Promise<{ posts: number; engineMs: number[] }> {
  return page.evaluate(() => {
    const record = (window as unknown as { untwistRuns: { posts: number; engineMs: number[] } }).untwistRuns;
    return { posts: record.posts, engineMs: [...record.engineMs] };
  });
}

async function timedRun(page: Page): Promise<Timed> {
  const before = await runs(page);
  const started = await page.evaluate(() => {
    performance.clearMarks('untwist:output-shown');
    const at = performance.now();
    document.querySelector<HTMLButtonElement>('#run')?.click();
    return at;
  });
  await page.waitForFunction(() => performance.getEntriesByName('untwist:output-shown').length > 0, null, { timeout: 240_000, polling: 20 });
  const shown = await page.evaluate(() => performance.getEntriesByName('untwist:output-shown')[0]?.startTime ?? Number.NaN);
  await expect(page.locator('#status-label')).toHaveText('Done');
  const after = await runs(page);
  return { wallMs: shown - started, engineMs: after.engineMs.slice(before.engineMs.length).reduce((sum, ms) => sum + ms, 0), runs: after.posts - before.posts };
}

function median(values: readonly number[]): number {
  return [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)] ?? Number.NaN;
}

async function measure(page: Page, label: string): Promise<void> {
  const timed: Timed[] = [];
  for (let round = 0; round < ROUNDS; round += 1) {
    timed.push(await timedRun(page));
  }
  const wall = median(timed.map((t) => t.wallMs));
  const engine = median(timed.map((t) => t.engineMs));
  test.info().annotations.push({ type: label, description: `Run to output ${wall.toFixed(0)} ms, engine ${engine.toFixed(0)} ms (${(wall / engine).toFixed(2)}x)` });
  expect(timed.map((t) => t.runs)).toEqual(timed.map(() => 1));
  expect(wall).toBeLessThanOrEqual(engine * RATIO + SLACK_MS);
}

test('opening a file runs the engine once, and Run during that run starts no second one', async ({ page }) => {
  test.setTimeout(240_000);
  await watchRuns(page);
  await openApp(page, '#unsafe=1');
  await page.locator('#open-file-input').setInputFiles({ name: 'big.js', mimeType: 'text/javascript', buffer: Buffer.from(bigInput()) });
  await expect(page.locator('#status-label')).toHaveText('Running');
  await page.keyboard.press('ControlOrMeta+Enter');
  await expect(page.locator('#status-label')).toHaveText('Done', { timeout: 200_000 });
  await page.waitForTimeout(500);
  expect((await runs(page)).posts).toBe(1);
});

for (const hash of ['#unsafe=1', '#unsafe=0', '#unsafe=1&libs=1']) {
  test(`from Run to the shown output takes at most 1.3x the engine time (${hash})`, async ({ page }) => {
    test.setTimeout(600_000);
    await watchRuns(page);
    await openApp(page, hash);
    await paste(page, bigInput());
    await runAndWait(page);
    await measure(page, `generated ${hash}`);
  });
}

test('on game.js, from Run to the shown output takes at most 1.3x the engine time', async ({ page }) => {
  skipWithoutGame();
  test.setTimeout(600_000);
  await watchRuns(page);
  await openApp(page, '#unsafe=1');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await runAndWait(page);
  await measure(page, 'game.js');
});
