import { execFileSync } from 'node:child_process';
import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, repoPath, runAndWait, sha256, test, skipWithoutGame } from './fixtures';

interface Case {
  readonly file: string;
  readonly hash: string;
  readonly flags: string;
}

const CASES: readonly Case[] = [
  { file: 'fixtures/reference/unpack/webpack-5.js', hash: '', flags: '' },
  { file: 'fixtures/reference/unpack/webpack-4.js', hash: '', flags: '' },
  { file: 'fixtures/reference/unpack/browserify.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/esbuild/esbuild-iife.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/parcel/min-parcel-entries.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/metro/metro.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/amd/rollup-amd.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/systemjs/rollup-system.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/umd/rollup-umd.js', hash: '', flags: '' },
  { file: 'fixtures/unpack-bundlers/rollup/rollup-iife.js', hash: '#single=1', flags: '--single-file' },
  { file: 'fixtures/reference/deobfuscate/obfuscator.io.js', hash: '', flags: '' },
  { file: 'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js', hash: '#unsafe=1', flags: '--unsafe' },
  { file: 'fixtures/reference/deobfuscate/obfuscator.io-high.js', hash: '#unpack=0&jsx=0&unsafe=1', flags: '--no-unpack --no-jsx --unsafe' },
  { file: 'fixtures/reference/deobfuscate/obfuscator.io-calls-transform.js', hash: '#rename=simple', flags: '--rename simple' },
  { file: 'fixtures/jsx/automatic.js', hash: '#unsafe=1', flags: '--unsafe' },
  { file: 'fixtures/jsx/classic.js', hash: '#unsafe=1', flags: '--unsafe' },
  { file: 'fixtures/untranspile/esbuild/async.0.28.1-es2015.js', hash: '#unsafe=1&unpack=0', flags: '--unsafe --no-unpack' },
  { file: 'fixtures/behaviour/06-rotator-in-uncalled-function.js', hash: '#unpack=0&jsx=0', flags: '--no-unpack --no-jsx' },
  { file: 'fixtures/corpus-npm/case-000.js', hash: '#rename=unique', flags: '--rename unique' },
  { file: 'fixtures/unminify/booleans-and-sequences.js', hash: '#rename=mangle&deob=0', flags: '--rename mangle --no-deobfuscate' },
];

function cli(flags: string, file: string): string {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  return execFileSync(bin, [...flags.split(' ').filter((word) => word !== ''), repoPath(file)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
}

async function outputDigest(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return sha256(await page.evaluate(() => navigator.clipboard.readText()));
}

async function watchLongTasks(page: Page): Promise<void> {
  await page.evaluate(() => {
    const tasks: Array<[number, number]> = [];
    (window as unknown as { untwistTasks: Array<[number, number]> }).untwistTasks = tasks;
    new PerformanceObserver((list) => list.getEntries().forEach((entry) => tasks.push([entry.startTime, entry.duration]))).observe({ type: 'longtask' });
  });
}

async function takeLongTasks(page: Page): Promise<Array<[number, number]>> {
  return page.evaluate(() => (window as unknown as { untwistTasks: Array<[number, number]> }).untwistTasks.splice(0));
}


test('20 corpus cases give the CLI stdout in Chromium', async ({ page }) => {
  test.setTimeout(300_000);
  for (const item of CASES) {
    await openApp(page, item.hash);
    await page.reload();
    await expect(page.locator('#run')).toBeEnabled({ timeout: 30_000 });
    await paste(page, readRepo(item.file));
    await runAndWait(page);
    expect(await outputDigest(page), `${item.file} ${item.flags}`).toBe(sha256(cli(item.flags, item.file)));
    await expect(page.locator('#input-lang')).not.toHaveText(/loading|analysing/, { timeout: 60_000 });
  }
});

test('game.js matches the CLI and the page stays responsive', async ({ page }) => {
  skipWithoutGame();
  test.setTimeout(300_000);
  await openApp(page);
  await paste(page, readRepo('fixtures/real-world/game.js'));
  await watchLongTasks(page);
  const labels: string[] = [];
  await page.locator('#run').click();
  while ((await page.locator('#status-label').textContent()) === 'Running') {
    labels.push((await page.locator('#elapsed').textContent()) ?? '');
    await page.waitForTimeout(250);
  }
  await page.waitForTimeout(1000);
  const timing = await page.evaluate(() => {
    const last = (name: string) => performance.getEntriesByName(name).at(-1)?.startTime ?? Number.NaN;
    return { received: last('untwist:result-received'), shownMs: last('untwist:output-shown') - last('untwist:result-received') };
  });
  const tasks = await takeLongTasks(page);
  const during = tasks.filter(([start, ms]) => start + ms < timing.received).map(([, ms]) => ms);
  const after = tasks.filter(([start, ms]) => start + ms >= timing.received).map(([, ms]) => ms);
  await page.locator('#output-editor .cm-content').hover();
  for (let burst = 0; burst < 20; burst += 1) {
    await page.mouse.wheel(0, 2000);
  }
  await page.waitForTimeout(500);
  const scroll = (await takeLongTasks(page)).map(([, ms]) => ms);
  const digest = await outputDigest(page);
  test.info().annotations.push({
    type: 'game.js responsiveness',
    description: `labels ${new Set(labels).size}, run tasks ${JSON.stringify(during.map(Math.round))}, shown ${timing.shownMs.toFixed(0)} ms, after ${JSON.stringify(after.map(Math.round))}, scroll ${JSON.stringify(scroll.map(Math.round))}`,
  });
  expect(digest).toBe(sha256(cli('', 'fixtures/real-world/game.js')));
  expect(new Set(labels).size).toBeGreaterThan(3);
  expect(during.filter((ms) => ms > 100)).toEqual([]);
  expect(timing.shownMs).toBeLessThanOrEqual(1500);
  expect(after.filter((ms) => ms > 400)).toEqual([]);
  expect(scroll.filter((ms) => ms > 50).length).toBeLessThanOrEqual(1);
});
