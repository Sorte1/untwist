import { execFileSync } from 'node:child_process';
import type { Page } from '@playwright/test';
import { expect, openApp, paste, readRepo, repoPath, runAndWait, sha256, test, skipWithoutGame } from './fixtures';

const FIXTURE = 'fixtures/reference/deobfuscate/obfuscator.io-control-flow.js';
const FLAGS = '--no-unpack --no-jsx --unsafe --rename unique --fuel 1000 --no-unminify --keep-human-names --max-iterations 3 --single-file';

function cli(flags: string, fixture: string): string {
  const bin = process.env['UNTWIST_BIN'] ?? repoPath('target/release/untwist');
  return execFileSync(bin, [...flags.split(' ').filter((word) => word !== ''), repoPath(fixture)], {
    encoding: 'utf8',
    maxBuffer: 1 << 28,
    env: { ...process.env, UNTWIST_TEST_SUPERVISOR: 'off' },
  });
}

async function outputText(page: Page): Promise<string> {
  await page.locator('#copy-output').click();
  await expect(page.locator('#copy-label')).toHaveText('Copied');
  return page.evaluate(() => navigator.clipboard.readText());
}

async function setEverything(page: Page): Promise<void> {
  for (const key of ['unpack', 'jsx', 'unsafe']) {
    await page.locator(`.switch[data-key="${key}"]`).click();
  }
  await page.locator('#rename-group [data-mode="unique"]').click();
  await page.locator('#fuel-inf').uncheck();
  await page.locator('#fuel').fill('1000');
  await page.locator('#advanced summary').click();
  await page.locator('#advanced [data-key="unmin"]').check();
  await page.locator('#advanced [data-key="human"]').check();
  await page.locator('#advanced [data-key="single"]').check();
  await page.locator('#max-iterations').fill('3');
}

async function controls(page: Page): Promise<string[]> {
  return page.evaluate(() => [
    ...[...document.querySelectorAll('.switch[data-key]')].map((el) => `${el.getAttribute('data-key')}=${el.getAttribute('aria-checked')}`),
    `rename=${document.querySelector('#rename-group [aria-pressed="true"]')?.getAttribute('data-mode')}`,
    `fuel=${(document.getElementById('fuel') as HTMLInputElement).value}/${(document.getElementById('fuel-inf') as HTMLInputElement).checked}`,
    ...[...document.querySelectorAll<HTMLInputElement>('#advanced input[type=checkbox]')].map((el) => `${el.dataset['key']}=${el.checked}`),
    `iter=${(document.getElementById('max-iterations') as HTMLInputElement).value}`,
  ]);
}


test('every option round-trips through the link and runs as the CLI does', async ({ page, context }) => {
  await openApp(page);
  await setEverything(page);
  await expect(page.locator('#query')).toHaveText(/^#unpack=0&jsx=0&rename=unique&fuel=1000/);
  await page.locator('#copy-link').click();
  await expect(page.locator('#copy-link-label')).toHaveText('Link copied');
  const link = await page.evaluate(() => navigator.clipboard.readText());
  const before = await controls(page);
  const second = await context.newPage();
  await second.goto(link);
  await expect(second.locator('#run')).toBeEnabled({ timeout: 30_000 });
  expect(await controls(second)).toEqual(before);
  await paste(second, readRepo(FIXTURE));
  await runAndWait(second);
  expect(sha256(await outputText(second))).toBe(sha256(cli(FLAGS, FIXTURE)));
});

test('a settings change marks the Run button until the next run', async ({ page }) => {
  await openApp(page);
  await paste(page, 'var a = !0;');
  await runAndWait(page);
  await expect(page.locator('#settings-changed')).toBeHidden();
  await page.locator('.switch[data-key="unsafe"]').click();
  await expect(page.locator('#settings-changed')).toBeVisible();
  await expect(page.locator('#status-label')).toHaveText('Done');
  await runAndWait(page);
  await expect(page.locator('#settings-changed')).toBeHidden();
});

test('an invalid link falls back to the defaults with one info line each', async ({ page }) => {
  await openApp(page, '#bogus=1&rename=full&unsafe=1');
  await expect(page.locator('.switch[data-key="unsafe"]')).toHaveAttribute('aria-checked', 'true');
  await expect(page.locator('#rename-group [aria-pressed="true"]')).toHaveAttribute('data-mode', 'none');
  await expect(page.locator('#diag-list .diag-info')).toHaveCount(2);
});

test('a closed drawer adds at most 3 % from result to output on game.js 1b', async ({ page }) => {
  skipWithoutGame();
  await openApp(page, '#unpack=0&jsx=0&unsafe=1');
  await paste(page, readRepo('fixtures/real-world/game.js'));
  const shares: number[] = [];
  for (let round = 0; round < 5; round += 1) {
    await runAndWait(page);
    await page.waitForFunction(() => {
      const shown = performance.getEntriesByName('untwist:output-shown');
      const received = performance.getEntriesByName('untwist:result-received');
      return shown.length > 0 && received.length > 0 && (shown.at(-1)?.startTime ?? 0) > (received.at(-1)?.startTime ?? 0);
    });
    shares.push(
      await page.evaluate(() => {
        const last = (name: string) => performance.getEntriesByName(name).at(-1);
        const span = (last('untwist:output-shown')?.startTime ?? 0) - (last('untwist:result-received')?.startTime ?? 0);
        performance.clearMarks();
        const chip = last('untwist:chip')?.duration ?? 0;
        performance.clearMeasures();
        return chip / span;
      }),
    );
  }
  const median = [...shares].sort((a, b) => a - b)[2] ?? 1;
  test.info().annotations.push({ type: 'D-closed', description: `${(median * 100).toFixed(2)} % of post→shown` });
  expect(await page.locator('#details *').count()).toBe(0);
  expect(median).toBeLessThanOrEqual(0.03);
});
