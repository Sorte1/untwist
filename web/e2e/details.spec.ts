import { execFileSync } from 'node:child_process';
import type { Page } from '@playwright/test';
import { expect, openApp, openDiagnostics, paste, readRepo, repoPath, runAndWait, saved, test, skipWithoutGame } from './fixtures';

const FIXTURE = 'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js';
const MS_FIELDS = new Set(['per_pass_ms', 'fixpoint_per_pass_ms', 'iterations_ms', 'elapsed_ms']);

type Row = Record<string, unknown>;

function masked(stages: Row[]): Row[] {
  return stages.map((stage) => Object.fromEntries(Object.entries(stage).filter(([key]) => !MS_FIELDS.has(key))));
}

function cliStages(fixture: string): Row[] {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  const json = execFileSync(bin, ['--report', 'json', repoPath(fixture)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
  return (JSON.parse(json) as { stages: Row[] }).stages;
}

async function runFixture(page: Page, text: string): Promise<void> {
  await openApp(page);
  await paste(page, text);
  await runAndWait(page);
  await openDiagnostics(page);
}

async function downloadedReport(page: Page): Promise<{ stages: Row[] }> {
  const waiting = page.waitForEvent('download');
  await page.locator('#download-report').click();
  const download = await waiting;
  return JSON.parse((await saved(download)).toString('utf8')) as { stages: Row[] };
}

test.use({ acceptDownloads: true });

test('a closed drawer adds no DOM and fetches no chunk', async ({ page }) => {
  const fetched: string[] = [];
  page.on('request', (request) => fetched.push(request.url()));
  await runFixture(page, readRepo(FIXTURE));
  await expect(page.locator('#summary-chip')).toHaveText(/^\d+ stages · \d+ rounds · \d+\.\d\d s$/);
  expect(await page.locator('#details *').count()).toBe(0);
  expect(fetched.filter((url) => /drawer/.test(url))).toEqual([]);
});

test('the drawer shows the stage rows and exports the CLI stages', async ({ page }) => {
  await runFixture(page, readRepo(FIXTURE));
  await page.locator('#details-tab').click();
  await expect(page.locator('#details .stage-row').nth(1)).toBeVisible();
  await page.locator('#details .stage-row:not(.stage-head)').first().click();
  const report = await downloadedReport(page);
  expect(masked(report.stages)).toEqual(masked(cliStages(FIXTURE)));
});

test('a run without library code lists no libraries and offers no library toggle', async ({ page }) => {
  await runFixture(page, readRepo(FIXTURE));
  await page.locator('#details-tab').click();
  const libraries = page.locator('#details .details-section').filter({ has: page.locator('h3', { hasText: /^Libraries$/ }) });
  await expect(libraries.locator('dd')).toHaveText('none detected');
  await expect(page.locator('#library-code')).toBeHidden();
});

test('the debug bundle carries settings, report and engine, never the input', async ({ page }) => {
  const marker = 'untwist_input_marker_42';
  await runFixture(page, `var ${marker} = !0;`);
  await page.locator('#details-tab').click();
  await page.locator('#copy-debug').click();
  await expect(page.locator('#copy-debug')).toHaveText('Copied');
  const text = await page.evaluate(() => navigator.clipboard.readText());
  const bundle = JSON.parse(text) as Record<string, unknown>;
  expect(Object.keys(bundle).sort()).toEqual(['engine', 'report', 'settings']);
  expect(text).not.toContain(marker);
});

test('game.js: real stage clocks within the worker time, and the drawer opens fast', async ({ page }) => {
  skipWithoutGame();
  await runFixture(page, readRepo('fixtures/real-world/game.js'));
  const openMs = await page.evaluate(async () => {
    const started = performance.now();
    document.getElementById('details-tab')?.click();
    while (document.querySelectorAll('#details .stage-row').length < 2) {
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    return performance.now() - started;
  });
  const report = await downloadedReport(page);
  const elapsed = report.stages.map((stage) => Number(stage['elapsed_ms']));
  const wall = await page.locator('#details dt:text-is("Worker wall time") + dd').textContent();
  const workerMs = Number.parseFloat(wall ?? 'NaN');
  test.info().annotations.push({ type: 'game.js', description: `stages ${elapsed.reduce((a, b) => a + b, 0).toFixed(0)} ms of worker ${workerMs} ms; drawer ${openMs.toFixed(0)} ms` });
  expect(elapsed.filter((ms) => ms > 0).length).toBeGreaterThan(elapsed.length / 2);
  expect(elapsed.reduce((a, b) => a + b, 0)).toBeLessThanOrEqual(workerMs);
  expect(openMs).toBeLessThanOrEqual(100);
});

test('Details shows the fuel used, the peak call depth, the collision reasons and the warnings per class, as the CLI reports them', async ({ page }) => {
  await openApp(page, '#rename=simple');
  await paste(page, readRepo(FIXTURE));
  await runAndWait(page);
  await openDiagnostics(page);
  await page.locator('#details-tab').click();
  const pair = (label: string) => page.locator('#details .details-pairs dt', { hasText: new RegExp(`^\\s*${label}$`) }).locator('xpath=following-sibling::dd[1]');
  const cli = JSON.parse(
    execFileSync(process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist'), ['--rename', 'simple', '--report', 'json', repoPath(FIXTURE)], {
      encoding: 'utf8',
      maxBuffer: 1 << 28,
      env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
    }),
  ) as { stats: { decode: { steps: number; units: number; peakCallDepth: number }; collisions: Record<string, number> } };
  const steps = cli.stats.decode.steps.toLocaleString('en-US');
  expect(cli.stats.decode.units).toBeGreaterThan(0);
  await expect(pair('Fuel used')).toHaveText(new RegExp(`^${steps} steps over ${cli.stats.decode.units} decode unit`));
  await expect(pair('Evaluator peak call depth')).toHaveText(new RegExp(`^${cli.stats.decode.peakCallDepth} \\(cap \\d+\\)$`));
  const skipped = Object.values(cli.stats.collisions).reduce((sum, n) => sum + n, 0);
  await expect(pair('Collision reasons')).toHaveText(skipped === 0 ? 'none' : `${skipped.toLocaleString('en-US')} ${skipped === 1 ? 'name' : 'names'} skipped`);
  await expect(page.locator('#details section', { hasText: 'Warnings by class' })).toBeVisible();
  await expect(page.locator('#details')).not.toContainText('not recorded');
});
